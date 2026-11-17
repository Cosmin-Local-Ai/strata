//! Yahoo Mail over IMAP (TLS) with an app password (brief §7). Strictly read-only: the mailbox is
//! opened with EXAMINE and headers are fetched with BODY.PEEK, so nothing is ever marked as read.

use serde::Serialize;

const HOST: &str = "imap.mail.yahoo.com";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Msg {
    pub uid: u32,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub seen: bool,
    pub flagged: bool,
    pub message_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub email: String,
    pub unread: u32,
    pub total: u32,
    pub messages: Vec<Msg>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Login { pub email: String, pub password: String }

pub fn save_login(email: &str, password: &str) -> Result<(), String> {
    let l = Login { email: email.trim().into(), password: password.replace(' ', "") };
    // Verify before saving.
    session(&l)?.logout().ok();
    crate::secrets::set("yahoo-imap", &serde_json::to_string(&l).unwrap())
}

fn login() -> Result<Login, String> {
    let s = crate::secrets::get("yahoo-imap")?.ok_or("not-connected")?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

fn session(l: &Login) -> Result<imap::Session<native_tls::TlsStream<std::net::TcpStream>>, String> {
    let tls = native_tls::TlsConnector::builder().build().map_err(|e| e.to_string())?;
    let client = imap::connect((HOST, 993), HOST, &tls).map_err(|e| format!("Couldn't reach Yahoo: {e}"))?;
    client.login(&l.email, &l.password).map_err(|e| {
        let m = e.0.to_string();
        if m.to_ascii_lowercase().contains("auth") || m.contains("LOGIN") { "Yahoo rejected the app password (create a new one in Yahoo Account security)".to_string() } else { m }
    })
}

/// `view`: "inbox" | "unread" | "flagged"; `query` searches subject and sender.
pub fn summary(view: &str, query: Option<&str>, limit: usize) -> Result<Summary, String> {
    let l = login()?;
    let mut s = session(&l)?;
    let mb = s.examine("INBOX").map_err(|e| e.to_string())?;
    let unread = s.search("UNSEEN").map_err(|e| e.to_string())?.len() as u32;
    let total = mb.exists;
    let mut seqs: Vec<u32> = match (view, query.map(str::trim).filter(|q| !q.is_empty())) {
        (_, Some(q)) => {
            let q = q.replace('"', "");
            s.search(format!("OR SUBJECT \"{q}\" FROM \"{q}\"")).map_err(|e| e.to_string())?.into_iter().collect()
        }
        ("unread", _) => s.search("UNSEEN").map_err(|e| e.to_string())?.into_iter().collect(),
        ("flagged", _) => s.search("FLAGGED").map_err(|e| e.to_string())?.into_iter().collect(),
        _ => (total.saturating_sub(limit as u32 - 1).max(1)..=total).collect(),
    };
    seqs.sort_unstable();
    let seqs: Vec<u32> = seqs.into_iter().rev().take(limit).collect();
    let mut messages = vec![];
    if !seqs.is_empty() && total > 0 {
        let set = seqs.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",");
        let fetched = s.fetch(set, "(UID FLAGS BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE MESSAGE-ID)])").map_err(|e| e.to_string())?;
        for f in fetched.iter() {
            let head = String::from_utf8_lossy(f.header().unwrap_or_default()).to_string();
            let field = |name: &str| -> String {
                let mut out = String::new();
                let mut on = false;
                for line in head.lines() {
                    if on && (line.starts_with(' ') || line.starts_with('\t')) { out.push_str(line.trim_start()); continue }
                    on = false;
                    if let Some((k, v)) = line.split_once(':') { if k.eq_ignore_ascii_case(name) { out = v.trim().to_string(); on = true } }
                }
                decode_words(&out)
            };
            let flags = f.flags();
            let from = field("From");
            let from = from.split('<').next().map(|x| x.trim().trim_matches('"').to_string()).filter(|x| !x.is_empty()).unwrap_or(from);
            messages.push(Msg {
                uid: f.uid.unwrap_or(0),
                from,
                subject: field("Subject"),
                date: field("Date"),
                seen: flags.iter().any(|x| matches!(x, imap::types::Flag::Seen)),
                flagged: flags.iter().any(|x| matches!(x, imap::types::Flag::Flagged)),
                message_id: field("Message-ID"),
            });
        }
        messages.sort_by(|a, b| b.uid.cmp(&a.uid));
    }
    s.logout().ok();
    Ok(Summary { email: l.email, unread, total, messages })
}

/// RFC 2047 encoded words (=?charset?B|Q?text?=), UTF-8 and Latin-1.
fn decode_words(s: &str) -> String {
    use base64::Engine;
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("=?") {
        let (before, after) = rest.split_at(i);
        if !before.trim().is_empty() || out.is_empty() { out.push_str(before) }
        let Some(end) = after[2..].find("?=").map(|e| e + 2) else { out.push_str(after); return out };
        let parts: Vec<&str> = after[2..end].splitn(3, '?').collect();
        if parts.len() == 3 {
            let bytes = match parts[1].to_ascii_uppercase().as_str() {
                "B" => base64::engine::general_purpose::STANDARD.decode(parts[2]).unwrap_or_default(),
                _ => {
                    let mut v = vec![];
                    let b = parts[2].as_bytes();
                    let mut j = 0;
                    while j < b.len() {
                        if b[j] == b'_' { v.push(b' ') } else if b[j] == b'=' && j + 2 < b.len() {
                            if let Ok(x) = u8::from_str_radix(&parts[2][j + 1..j + 3], 16) { v.push(x); j += 3; continue }
                        } else { v.push(b[j]) }
                        j += 1;
                    }
                    v
                }
            };
            if parts[0].to_ascii_lowercase().contains("8859") || parts[0].eq_ignore_ascii_case("windows-1252") {
                out.extend(bytes.iter().map(|&c| c as char));
            } else {
                out.push_str(&String::from_utf8_lossy(&bytes));
            }
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}
