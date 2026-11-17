//! One request path for every online widget (brief §4.7): the page names a *service*, Strata checks
//! the URL belongs to that service, adds the credential on this side (keys never reach the web UI)
//! and returns the JSON. Unknown services or foreign hosts are refused, so a widget can only talk to
//! its own vendor.

use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Copy)]
enum Auth {
    /// No credential (public APIs).
    None,
    /// `Authorization: Bearer <secret>`
    Bearer,
    /// `<header>: <secret>`
    Header(&'static str),
    /// `?<param>=<secret>` appended to the URL.
    Query(&'static str),
    /// Google OAuth access token (refreshed from the stored refresh token).
    Google,
    /// Claude Code's own sign-in (~/.claude/.credentials.json), read-only.
    Claude,
    /// Codex CLI sign-in (~/.codex/auth.json), read-only.
    Codex,
}

struct Service {
    name: &'static str,
    prefixes: &'static [&'static str],
    auth: Auth,
}

/// `*` in a prefix matches one host label (e.g. a Shopify store name).
const SERVICES: &[Service] = &[
    Service { name: "public", prefixes: &[
        "https://api.frankfurter.dev/", "https://api.frankfurter.app/", "https://api.binance.com/",
        "https://leetcode.com/graphql", "https://query1.finance.yahoo.com/", "https://query2.finance.yahoo.com/",
        "https://api.open-meteo.com/", "https://geocoding-api.open-meteo.com/", "https://api.sunrise-sunset.org/",
        "http://localhost:8085/data.json", "http://127.0.0.1:8085/data.json",
    ], auth: Auth::None },
    Service { name: "github", prefixes: &["https://api.github.com/"], auth: Auth::Bearer },
    Service { name: "stripe", prefixes: &["https://api.stripe.com/"], auth: Auth::Bearer },
    Service { name: "polar", prefixes: &["https://api.polar.sh/"], auth: Auth::Bearer },
    Service { name: "shopify", prefixes: &["https://*.myshopify.com/admin/api/"], auth: Auth::Header("X-Shopify-Access-Token") },
    Service { name: "lemonsqueezy", prefixes: &["https://api.lemonsqueezy.com/"], auth: Auth::Bearer },
    Service { name: "paddle", prefixes: &["https://api.paddle.com/"], auth: Auth::Bearer },
    Service { name: "gumroad", prefixes: &["https://api.gumroad.com/"], auth: Auth::Query("access_token") },
    Service { name: "dodo", prefixes: &["https://live.dodopayments.com/"], auth: Auth::Bearer },
    Service { name: "plausible", prefixes: &["https://plausible.io/api/"], auth: Auth::Bearer },
    Service { name: "dub", prefixes: &["https://api.dub.co/"], auth: Auth::Bearer },
    Service { name: "resend", prefixes: &["https://api.resend.com/"], auth: Auth::Bearer },
    Service { name: "linear", prefixes: &["https://api.linear.app/"], auth: Auth::Header("Authorization") },
    Service { name: "supabase", prefixes: &["https://api.supabase.com/"], auth: Auth::Bearer },
    Service { name: "datafast", prefixes: &["https://datafa.st/api/"], auth: Auth::Bearer },
    Service { name: "youtube", prefixes: &["https://www.googleapis.com/youtube/v3/"], auth: Auth::Query("key") },
    Service { name: "instagram", prefixes: &["https://graph.instagram.com/"], auth: Auth::Query("access_token") },
    Service { name: "x", prefixes: &["https://api.x.com/2/", "https://api.twitter.com/2/"], auth: Auth::Bearer },
    Service { name: "tiktok", prefixes: &["https://open.tiktokapis.com/"], auth: Auth::Bearer },
    Service { name: "finnhub", prefixes: &["https://finnhub.io/api/v1/"], auth: Auth::Query("token") },
    Service { name: "deepseek", prefixes: &["https://api.deepseek.com/"], auth: Auth::Bearer },
    Service { name: "glm", prefixes: &["https://api.z.ai/", "https://open.bigmodel.cn/"], auth: Auth::Header("Authorization") },
    Service { name: "meta", prefixes: &["https://graph.facebook.com/"], auth: Auth::Query("access_token") },
    Service { name: "google", prefixes: &[
        "https://analyticsdata.googleapis.com/", "https://analyticsadmin.googleapis.com/",
        "https://searchconsole.googleapis.com/", "https://www.googleapis.com/webmasters/", "https://gmail.googleapis.com/",
    ], auth: Auth::Google },
    Service { name: "claude", prefixes: &["https://api.anthropic.com/api/oauth/"], auth: Auth::Claude },
    Service { name: "codex", prefixes: &["https://chatgpt.com/backend-api/"], auth: Auth::Codex },
];

/// Secret names the UI may *set* or *test for* (never read back), one per keyed service.
pub fn is_service_secret(name: &str) -> bool {
    SERVICES.iter().any(|s| s.name == name && matches!(s.auth, Auth::Bearer | Auth::Header(_) | Auth::Query(_)))
}

fn matches(prefix: &str, url: &str) -> bool {
    match prefix.split_once('*') {
        None => url.starts_with(prefix),
        Some((a, b)) => url.strip_prefix(a).and_then(|rest| {
            let label_end = rest.find('.')?;
            let label = &rest[..label_end];
            (!label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then(|| &rest[label_end..])
        }).is_some_and(|rest| rest.starts_with(b)),
    }
}

fn home() -> std::path::PathBuf {
    std::env::var_os("USERPROFILE").map(Into::into).unwrap_or_default()
}

pub fn claude_token() -> Result<String, String> {
    let dir = std::env::var_os("CLAUDE_CONFIG_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| home().join(".claude"));
    let v: Value = serde_json::from_slice(&std::fs::read(dir.join(".credentials.json")).map_err(|_| "Claude Code isn't signed in on this PC")?)
        .map_err(|e| e.to_string())?;
    let o = &v["claudeAiOauth"];
    if let Some(exp) = o["expiresAt"].as_f64() {
        if exp / 1000.0 < std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64() {
            return Err("expired: Claude Code's sign-in has expired. Open Claude Code once to refresh it.".into());
        }
    }
    o["accessToken"].as_str().map(String::from).ok_or_else(|| "Claude Code isn't signed in on this PC".into())
}

fn codex_auth() -> Result<(String, Option<String>), String> {
    let dir = std::env::var_os("CODEX_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| home().join(".codex"));
    let v: Value = serde_json::from_slice(&std::fs::read(dir.join("auth.json")).map_err(|_| "Codex isn't signed in on this PC")?)
        .map_err(|e| e.to_string())?;
    let t = v["tokens"]["access_token"].as_str().ok_or("Codex isn't signed in with ChatGPT")?;
    Ok((t.to_string(), v["tokens"]["account_id"].as_str().map(String::from)))
}

#[derive(serde::Serialize)]
pub struct Reply {
    pub status: u16,
    pub body: Value,
}

pub fn request(service: &str, method: &str, url: &str, body: Option<Value>, headers: HashMap<String, String>) -> Result<Reply, String> {
    let svc = SERVICES.iter().find(|s| s.name == service).ok_or("unknown service")?;
    if !svc.prefixes.iter().any(|p| matches(p, url)) {
        return Err("This address doesn't belong to that service".into());
    }
    let mut url = url.to_string();
    let mut hdrs: Vec<(String, String)> = headers.into_iter().filter(|(k, _)| !k.eq_ignore_ascii_case("authorization")).collect();
    let secret = |name: &str| crate::secrets::get(name)?.ok_or_else(|| "not-connected".to_string());
    match svc.auth {
        Auth::None => {}
        Auth::Bearer => hdrs.push(("Authorization".into(), format!("Bearer {}", secret(svc.name)?))),
        Auth::Header(h) => hdrs.push((h.into(), secret(svc.name)?)),
        Auth::Query(p) => {
            let s = secret(svc.name)?;
            url.push(if url.contains('?') { '&' } else { '?' });
            url.push_str(&format!("{p}={}", urlencode(&s)));
        }
        Auth::Google => hdrs.push(("Authorization".into(), format!("Bearer {}", crate::google::access_token()?))),
        Auth::Claude => {
            hdrs.push(("Authorization".into(), format!("Bearer {}", claude_token()?)));
            hdrs.push(("anthropic-beta".into(), "oauth-2025-04-20".into()));
        }
        Auth::Codex => {
            let (t, acct) = codex_auth()?;
            hdrs.push(("Authorization".into(), format!("Bearer {t}")));
            if let Some(a) = acct { hdrs.push(("ChatGPT-Account-Id".into(), a)); }
        }
    }
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .new_agent();
    let res = match method.to_ascii_uppercase().as_str() {
        "GET" => { let mut r = agent.get(&url); for (k, v) in &hdrs { r = r.header(k, v); } r.call() }
        m @ ("POST" | "PUT" | "PATCH") => {
            let mut r = match m { "POST" => agent.post(&url), "PUT" => agent.put(&url), _ => agent.patch(&url) };
            for (k, v) in &hdrs { r = r.header(k, v); }
            match body { Some(b) => r.send_json(b), None => r.send_empty() }
        }
        _ => return Err("method not allowed".into()),
    };
    let mut res = res.map_err(|e| format!("network: {e}"))?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(32 << 20).read_to_string().unwrap_or_default();
    let body = serde_json::from_str(&text).unwrap_or(Value::String(text.chars().take(2000).collect()));
    Ok(Reply { status, body })
}

pub fn urlencode(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
        _ => format!("%{b:02X}"),
    }).collect()
}
