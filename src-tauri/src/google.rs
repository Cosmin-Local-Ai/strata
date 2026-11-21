//! Google sign-in shared by Google Analytics, Search Console and Gmail (brief §7: one Google auth
//! module). Desktop OAuth with a loopback redirect and PKCE, using the user's own OAuth client (a
//! "Desktop app" client from their Google Cloud project). Read-only scopes. The client and the
//! refresh token live in Windows Credential Manager; access tokens only in memory.

use base64::Engine;
use sha2::Digest;
use std::io::{Read, Write};
use std::sync::Mutex;

const SCOPES: &str = "https://www.googleapis.com/auth/analytics.readonly https://www.googleapis.com/auth/webmasters.readonly https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/drive.metadata.readonly openid email";

static TOKEN: Mutex<Option<(String, std::time::Instant)>> = Mutex::new(None);

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
fn random(n: usize) -> String {
    let mut b = vec![0u8; n];
    getrandom::fill(&mut b).expect("random");
    b64(&b)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Client { id: String, secret: String }

fn client() -> Result<Client, String> {
    let s = crate::secrets::get("google-client")?.ok_or("not-connected")?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(std::time::Duration::from_secs(20))).build().new_agent()
}

fn token_call(form: &[(&str, &str)]) -> Result<serde_json::Value, String> {
    let mut r = agent().post("https://oauth2.googleapis.com/token").send_form(form.iter().copied()).map_err(|e| format!("network: {e}"))?;
    let v: serde_json::Value = r.body_mut().read_json().map_err(|e| e.to_string())?;
    if let Some(e) = v.get("error") {
        return Err(format!("Google refused: {} {}", e.as_str().unwrap_or(""), v["error_description"].as_str().unwrap_or("")));
    }
    Ok(v)
}

/// A valid access token (refreshed when needed).
pub fn access_token() -> Result<String, String> {
    if let Some((t, until)) = TOKEN.lock().unwrap().clone() {
        if std::time::Instant::now() < until { return Ok(t) }
    }
    let c = client()?;
    let refresh = crate::secrets::get("google-token")?.ok_or("not-connected")?;
    let v = token_call(&[("client_id", &c.id), ("client_secret", &c.secret), ("refresh_token", &refresh), ("grant_type", "refresh_token")])?;
    let t = v["access_token"].as_str().ok_or("no access token")?.to_string();
    let secs = v["expires_in"].as_u64().unwrap_or(3600).saturating_sub(120);
    *TOKEN.lock().unwrap() = Some((t.clone(), std::time::Instant::now() + std::time::Duration::from_secs(secs)));
    Ok(t)
}

pub fn connected() -> bool {
    matches!(crate::secrets::get("google-token"), Ok(Some(_)))
}

pub fn disconnect() {
    let _ = crate::secrets::delete("google-token");
    *TOKEN.lock().unwrap() = None;
}

/// Opens the browser to Google's consent page and waits (up to 5 minutes) for the redirect.
pub fn connect(id: &str, secret: &str) -> Result<(), String> {
    // Empty fields: sign in again with the client saved last time (e.g. to grant a new scope).
    let saved: Option<Client> = if id.trim().is_empty() && secret.trim().is_empty() {
        crate::secrets::get("google-client").ok().flatten().and_then(|s| serde_json::from_str(&s).ok())
    } else { None };
    let (id, secret) = match &saved { Some(c) => (c.id.as_str(), c.secret.as_str()), None => (id.trim(), secret.trim()) };
    if id.is_empty() || secret.is_empty() { return Err("Paste both the client ID and the client secret".into()) }
    crate::secrets::set("google-client", &serde_json::to_string(&Client { id: id.into(), secret: secret.into() }).unwrap())?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");
    let verifier = random(48);
    let challenge = b64(&sha2::Sha256::digest(verifier.as_bytes()));
    let state = random(16);
    let enc = crate::services::urlencode;
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={challenge}&code_challenge_method=S256&access_type=offline&prompt=consent&state={state}",
        enc(id), enc(&redirect), enc(SCOPES)
    );
    crate::system::launch(&url, None)?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
    let (mut sock, _) = loop {
        match listener.accept() {
            Ok(s) => break s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() > deadline { return Err("Timed out waiting for Google sign-in".into()) }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(e) => return Err(e.to_string()),
        }
    };
    let _ = sock.set_nonblocking(false);
    let mut buf = [0u8; 8192];
    let n = sock.read(&mut buf).unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req.split_whitespace().nth(1).unwrap_or("");
    let q: std::collections::HashMap<String, String> = path.split_once('?').map(|(_, q)| q).unwrap_or("").split('&')
        .filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), urldecode(v))).collect();
    let ok = q.get("state") == Some(&state) && q.contains_key("code");
    let page = if ok { "Strata is connected to Google. You can close this tab." } else { "Sign-in was cancelled or failed. You can close this tab and try again from Strata." };
    let html = format!("<!doctype html><meta charset=utf-8><body style=\"font:16px system-ui;background:#111;color:#eee;display:grid;place-items:center;height:90vh\">{page}</body>");
    let _ = write!(sock, "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len());
    if !ok { return Err(q.get("error").cloned().unwrap_or_else(|| "Sign-in was cancelled".into())) }
    let v = token_call(&[
        ("code", &q["code"]), ("client_id", id), ("client_secret", secret), ("redirect_uri", &redirect),
        ("grant_type", "authorization_code"), ("code_verifier", &verifier),
    ])?;
    let refresh = v["refresh_token"].as_str().ok_or("Google didn't return a refresh token")?;
    crate::secrets::set("google-token", refresh)?;
    if let Some(t) = v["access_token"].as_str() {
        *TOKEN.lock().unwrap() = Some((t.into(), std::time::Instant::now() + std::time::Duration::from_secs(3000)));
    }
    Ok(())
}

fn urldecode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() + 0 && i + 2 <= b.len() - 1 => {
                if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) { out.push(v); i += 3; continue }
                out.push(b'%');
            }
            b'+' => out.push(b' '),
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
