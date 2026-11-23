//! Vault widget: passwords, API keys, secure notes, cards, Wi-Fi, IDs and small files.
//!
//! * Secrets: one Windows Credential Manager entry per item ("Strata/vault/<id>"). Items larger than
//!   Credential Manager allows (about 2.5 KB: long notes) and stored files are
//!   stored as DPAPI-encrypted files in %APPDATA%\Strata\vault (same protection: your Windows sign-in).
//! * The list the UI sees (title, username, URL, kind, file name) holds no secrets.
//! * Unlock: Windows Hello, an optional master password, or both. The master password is stored only
//!   as a PBKDF2-SHA256 hash (600,000 rounds); it gates access in Strata, it does not re-encrypt data.
//! * Copies skip clipboard history / cloud clipboard and are cleared after 30 s.
//! * Import: CSV exports of Google Password Manager, Apple Passwords / iCloud Keychain, Vivaldi,
//!   Chrome, Edge, Brave, Firefox, Bitwarden, 1Password (columns are matched by name).
//! * Health: weak, reused and old passwords, plus an opt-in breach check against Have I Been Pwned's
//!   k-anonymity range API (only the first 5 hex characters of each password's SHA-1 leave the PC).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Mutex;
use windows::core::{HSTRING, PWSTR};
use windows::Win32::Security::Credentials::*;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Entry {
    pub id: String,
    /// "password" | "api" | "note" | "card" | "wifi" | "identity" | "recovery" | "file"
    pub kind: String,
    pub title: String,
    pub username: String,
    pub url: String,
    pub updated: u64,
    /// Name of the attached file ("" = none). The file itself is encrypted on disk.
    pub file_name: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Secret {
    pub secret: String,
    pub notes: String,
    /// Kind-specific secret fields (card number, CVV, Wi-Fi password, ID number, OTP URI, ...).
    pub fields: BTreeMap<String, String>,
    /// Stored in an encrypted file because it is too large for Credential Manager.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub external: bool,
}

static UNLOCKED_UNTIL: Mutex<Option<std::time::Instant>> = Mutex::new(None);

fn dir() -> std::path::PathBuf {
    let d = crate::config::data_dir().join("vault");
    let _ = std::fs::create_dir_all(&d);
    d
}
fn index_path() -> std::path::PathBuf { crate::config::data_dir().join("vault-index.json") }
pub fn list() -> Vec<Entry> {
    std::fs::read(index_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}
fn save_list(v: &[Entry]) -> Result<(), String> {
    std::fs::write(index_path(), serde_json::to_vec_pretty(v).unwrap()).map_err(|e| e.to_string())
}

fn target(id: &str) -> HSTRING { HSTRING::from(format!("Strata/vault/{id}")) }
fn safe_id(id: &str) -> Result<&str, String> {
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { Ok(id) } else { Err("bad id".into()) }
}

// ---- DPAPI (CryptProtectData, current user) -------------------------------------------------

fn dpapi(data: &[u8], protect: bool) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::*;
    let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
    let mut out = CRYPT_INTEGER_BLOB::default();
    unsafe {
        let r = if protect {
            CryptProtectData(&input, &HSTRING::from("Strata vault"), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out)
        } else {
            CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out)
        };
        r.map_err(|e| format!("Windows data protection failed: {e}"))?;
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out.pbData as _)));
        Ok(v)
    }
}

// ---- secret storage ---------------------------------------------------------------------------

fn cred_write(id: &str, blob: &[u8]) -> Result<(), String> {
    let mut blob = blob.to_vec();
    let t = target(id);
    let user = HSTRING::from("Strata vault");
    let cred = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(t.as_ptr() as *mut u16),
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: PWSTR(user.as_ptr() as *mut u16),
        ..Default::default()
    };
    unsafe { CredWriteW(&cred, 0).map_err(|e| e.to_string()) }
}

fn write_secret(id: &str, s: &Secret) -> Result<(), String> {
    let id = safe_id(id)?;
    let blob = serde_json::to_vec(&Secret { external: false, ..s.clone() }).unwrap();
    let big = dir().join(format!("{id}.bin"));
    if blob.len() <= 2500 {
        let _ = std::fs::remove_file(&big);
        return cred_write(id, &blob);
    }
    if blob.len() > 1 << 20 { return Err("Too long: keep notes and fields under 1 MB".into()) }
    std::fs::write(&big, dpapi(&blob, true)?).map_err(|e| e.to_string())?;
    cred_write(id, &serde_json::to_vec(&Secret { external: true, ..Default::default() }).unwrap())
}

fn read_secret(id: &str) -> Result<Secret, String> {
    let id = safe_id(id)?;
    let t = target(id);
    let s: Secret = unsafe {
        let mut p: *mut CREDENTIALW = std::ptr::null_mut();
        CredReadW(&t, CRED_TYPE_GENERIC, None, &mut p).map_err(|_| "This entry's secret is missing from Credential Manager")?;
        let c = &*p;
        let bytes = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec();
        CredFree(p as _);
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?
    };
    if !s.external { return Ok(s) }
    let enc = std::fs::read(dir().join(format!("{id}.bin"))).map_err(|_| "This entry's encrypted file is missing")?;
    serde_json::from_slice(&dpapi(&enc, false)?).map_err(|e| e.to_string())
}

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64 }

fn require_unlocked() -> Result<(), String> {
    match *UNLOCKED_UNTIL.lock().unwrap() {
        Some(t) if std::time::Instant::now() < t => Ok(()),
        _ => Err("locked".into()),
    }
}

pub fn is_unlocked() -> bool { require_unlocked().is_ok() }
pub fn lock() { *UNLOCKED_UNTIL.lock().unwrap() = None }

// ---- unlock: Windows Hello and/or master password ----------------------------------------------

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
struct LockConfig {
    /// PBKDF2-SHA256 of the master password ("" = no master password).
    hash: String,
    salt: String,
    rounds: u32,
    /// With a master password: also ask for Windows Hello.
    hello_too: bool,
}

fn lock_path() -> std::path::PathBuf { dir().join("lock.json") }
fn lock_config() -> LockConfig {
    std::fs::read(lock_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockInfo { pub master: bool, pub hello_too: bool }
pub fn lock_info() -> LockInfo { let c = lock_config(); LockInfo { master: !c.hash.is_empty(), hello_too: c.hello_too } }

fn derive(pw: &str, salt: &[u8], rounds: u32) -> [u8; 32] {
    let mut out = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(pw.as_bytes(), salt, rounds, &mut out);
    out
}

fn check_master(c: &LockConfig, pw: Option<&str>) -> Result<(), String> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let Some(pw) = pw.filter(|p| !p.is_empty()) else { return Err("master-required".into()) };
    let salt = b64.decode(&c.salt).map_err(|e| e.to_string())?;
    let want = b64.decode(&c.hash).map_err(|e| e.to_string())?;
    let got = derive(pw, &salt, c.rounds.max(1));
    // Constant-time compare.
    let diff = want.iter().zip(got.iter()).fold((want.len() != got.len()) as u8, |d, (a, b)| d | (a ^ b));
    if diff == 0 { Ok(()) } else { std::thread::sleep(std::time::Duration::from_millis(600)); Err("Wrong master password".into()) }
}

fn hello(hwnd: isize) -> Result<&'static str, String> { hello_with(hwnd, "Unlock your Strata vault") }

fn hello_with(hwnd: isize, message: &str) -> Result<&'static str, String> {
    use windows::Security::Credentials::UI::*;
    let avail = UserConsentVerifier::CheckAvailabilityAsync().and_then(|a| a.join()).map_err(|e| e.to_string())?;
    if avail != UserConsentVerifierAvailability::Available { return Ok("no-hello") }
    let interop: windows::Win32::System::WinRT::IUserConsentVerifierInterop =
        windows::core::factory::<UserConsentVerifier, _>().map_err(|e| e.to_string())?;
    let op: windows_future::IAsyncOperation<UserConsentVerificationResult> = unsafe {
        interop.RequestVerificationForWindowAsync(windows::Win32::Foundation::HWND(hwnd as _), &HSTRING::from(message))
    }.map_err(|e| e.to_string())?;
    let r = op.join().map_err(|e| e.to_string())?;
    if r != UserConsentVerificationResult::Verified { return Err("Not verified".into()) }
    Ok("hello")
}

/// Without a master password: Windows Hello (if Hello isn't set up the vault still opens and the
/// result says so). With one: the master password, plus Hello when `hello_too` is on.
pub fn unlock(hwnd: isize, minutes: u64, password: Option<String>) -> Result<String, String> {
    let c = lock_config();
    let mode = if c.hash.is_empty() {
        hello(hwnd)?
    } else {
        check_master(&c, password.as_deref())?;
        if c.hello_too { hello(hwnd)?; "master+hello" } else { "master" }
    };
    *UNLOCKED_UNTIL.lock().unwrap() = Some(std::time::Instant::now() + std::time::Duration::from_secs(minutes.clamp(1, 120) * 60));
    Ok(mode.into())
}

/// Forgot the master password. The items are encrypted with your Windows account (DPAPI); the master
/// password is an extra lock on top. Proving you are that Windows user again (Windows Hello: face,
/// fingerprint or PIN) removes it and opens the vault, so a new one can be set. Nothing is lost.
/// Without Hello there is no second way to prove it: "no-hello".
pub fn reset_master(hwnd: isize, minutes: u64) -> Result<(), String> {
    if lock_config().hash.is_empty() { return Ok(()) }
    if hello_with(hwnd, "Confirm it's you to reset the Strata vault's master password")? == "no-hello" {
        return Err("no-hello".into());
    }
    std::fs::write(lock_path(), serde_json::to_vec_pretty(&LockConfig::default()).unwrap()).map_err(|e| e.to_string())?;
    *UNLOCKED_UNTIL.lock().unwrap() = Some(std::time::Instant::now() + std::time::Duration::from_secs(minutes.clamp(1, 120) * 60));
    log::warn!("vault master password reset after Windows Hello verification");
    Ok(())
}

/// Set or change the master password (Some), or turn it off (None). Only while unlocked.
pub fn set_master(password: Option<String>, hello_too: bool) -> Result<(), String> {
    require_unlocked()?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let c = match password {
        None => LockConfig::default(),
        Some(p) => {
            if p.chars().count() < 8 { return Err("Use at least 8 characters".into()) }
            let mut salt = [0u8; 16];
            getrandom::fill(&mut salt).map_err(|e| e.to_string())?;
            let rounds = 600_000;
            LockConfig { hash: b64.encode(derive(&p, &salt, rounds)), salt: b64.encode(salt), rounds, hello_too }
        }
    };
    std::fs::write(lock_path(), serde_json::to_vec_pretty(&c).unwrap()).map_err(|e| e.to_string())
}

// ---- items ----------------------------------------------------------------------------------------

pub fn save(mut e: Entry, secret: Option<String>, notes: Option<String>, fields: Option<BTreeMap<String, String>>) -> Result<Entry, String> {
    require_unlocked()?;
    let mut v = list();
    if e.id.is_empty() { e.id = new_id() }
    safe_id(&e.id)?;
    e.updated = now_ms();
    // The file name is owned by attach_file / remove_file, never by the editor.
    e.file_name = v.iter().find(|x| x.id == e.id).map(|x| x.file_name.clone()).unwrap_or_default();
    let mut s = read_secret(&e.id).unwrap_or_default();
    if let Some(x) = secret { s.secret = x }
    if let Some(n) = notes { s.notes = n }
    if let Some(f) = fields { s.fields = f.into_iter().filter(|(_, v)| !v.is_empty()).collect() }
    write_secret(&e.id, &s)?;
    v.retain(|x| x.id != e.id);
    v.push(e.clone());
    save_list(&v)?;
    Ok(e)
}

fn new_id() -> String {
    let mut r = [0u8; 3];
    let _ = getrandom::fill(&mut r);
    format!("{:x}-{:02x}{:02x}{:02x}", now_ms(), r[0], r[1], r[2])
}

pub fn delete(id: &str) -> Result<(), String> {
    require_unlocked()?;
    let id = safe_id(id)?;
    unsafe { let _ = CredDeleteW(&target(id), CRED_TYPE_GENERIC, None); }
    let _ = std::fs::remove_file(dir().join(format!("{id}.bin")));
    let _ = std::fs::remove_file(dir().join(format!("{id}.file")));
    let v: Vec<Entry> = list().into_iter().filter(|x| x.id != id).collect();
    save_list(&v)
}

pub fn reveal(id: &str) -> Result<Secret, String> {
    require_unlocked()?;
    read_secret(id)
}

/// Copies `field` ("secret" | "username" | a field name) without it entering clipboard history;
/// clears it after 30 s.
pub fn copy(id: &str, field: &str) -> Result<(), String> {
    require_unlocked()?;
    let text = match field {
        "username" => list().into_iter().find(|x| x.id == id).map(|x| x.username).ok_or("not found")?,
        "secret" => read_secret(id)?.secret,
        f => read_secret(id)?.fields.get(f).cloned().ok_or("not found")?,
    };
    let seq = clipboard_private(&text)?;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        use windows::Win32::System::DataExchange::*;
        unsafe {
            if GetClipboardSequenceNumber() == seq && OpenClipboard(None).is_ok() {
                let _ = EmptyClipboard();
                let _ = CloseClipboard();
            }
        }
    });
    Ok(())
}

// ---- files ----------------------------------------------------------------------------------------

const MAX_FILE: u64 = 10 << 20;

/// Encrypt `path` into the vault and attach it to item `id`.
pub fn attach_file(id: &str, path: &str) -> Result<Entry, String> {
    require_unlocked()?;
    let id = safe_id(id)?;
    let p = std::path::Path::new(path);
    let len = std::fs::metadata(p).map_err(|e| e.to_string())?.len();
    if len > MAX_FILE { return Err("Files up to 10 MB fit in the vault".into()) }
    let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
    std::fs::write(dir().join(format!("{id}.file")), dpapi(&bytes, true)?).map_err(|e| e.to_string())?;
    let mut v = list();
    let e = v.iter_mut().find(|x| x.id == id).ok_or("Save the item first")?;
    e.file_name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    e.updated = now_ms();
    let out = e.clone();
    save_list(&v)?;
    Ok(out)
}

/// Decrypt item `id`'s file to `dest`.
pub fn export_file(id: &str, dest: &str) -> Result<(), String> {
    require_unlocked()?;
    let id = safe_id(id)?;
    let enc = std::fs::read(dir().join(format!("{id}.file"))).map_err(|_| "The file is missing")?;
    std::fs::write(dest, dpapi(&enc, false)?).map_err(|e| e.to_string())
}

pub fn remove_file(id: &str) -> Result<(), String> {
    require_unlocked()?;
    let id = safe_id(id)?;
    let _ = std::fs::remove_file(dir().join(format!("{id}.file")));
    let mut v = list();
    if let Some(e) = v.iter_mut().find(|x| x.id == id) { e.file_name.clear(); e.updated = now_ms(); }
    save_list(&v)
}

// ---- import / export ------------------------------------------------------------------------------

/// RFC 4180 CSV: quoted fields, doubled quotes, newlines inside quotes, BOM.
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let text = text.trim_start_matches('\u{feff}');
    let mut rows = vec![];
    let mut row = vec![];
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => { field.push('"'); chars.next(); }
                '"' => quoted = false,
                _ => field.push(c),
            }
        } else {
            match c {
                '"' => quoted = true,
                ',' => row.push(std::mem::take(&mut field)),
                '\r' => {}
                '\n' => { row.push(std::mem::take(&mut field)); rows.push(std::mem::take(&mut row)); }
                _ => field.push(c),
            }
        }
    }
    if !field.is_empty() || !row.is_empty() { row.push(field); rows.push(row); }
    rows.retain(|r| r.iter().any(|f| !f.trim().is_empty()));
    rows
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult { pub imported: usize, pub duplicates: usize, pub skipped: usize, pub source: String }

fn host_of(url: &str) -> String {
    let u = url.trim();
    let u = u.split("://").nth(1).unwrap_or(u);
    u.split(['/', '?', '#']).next().unwrap_or("").trim_start_matches("www.").to_lowercase()
}

/// Import a password CSV. Columns are matched by header name, so the exports of Google Password
/// Manager / Chrome / Vivaldi / Edge / Brave (name,url,username,password,note), Apple Passwords /
/// iCloud Keychain (Title,URL,Username,Password,Notes,OTPAuth), Firefox, Bitwarden and 1Password all
/// work. Entries identical to an existing one (site, username and password) are skipped.
pub fn import_csv(path: &str, delete_after: bool) -> Result<ImportResult, String> {
    require_unlocked()?;
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let rows = parse_csv(&String::from_utf8_lossy(&bytes));
    let Some((head, body)) = rows.split_first() else { return Err("The file is empty".into()) };
    let h: Vec<String> = head.iter().map(|x| x.trim().to_lowercase()).collect();
    let col = |names: &[&str]| names.iter().find_map(|n| h.iter().position(|x| x == n));
    let c_title = col(&["name", "title", "account", "login_name"]);
    let c_url = col(&["url", "website", "login_uri", "web site", "origin"]);
    let c_user = col(&["username", "user name", "login", "login_username", "email", "user"]);
    let c_pass = col(&["password", "login_password", "pass"]);
    let c_note = col(&["note", "notes", "comment", "extra"]);
    let c_otp = col(&["otpauth", "totp", "login_totp", "otp"]);
    let Some(c_pass) = c_pass else { return Err("This doesn't look like a password export: no \"password\" column".into()) };
    let source = if h.iter().any(|x| x == "otpauth") { "Apple Passwords" } else if h.first().map(|x| x == "name").unwrap_or(false) && h.len() <= 5 { "Google / Chromium (Vivaldi, Chrome, Edge)" } else { "CSV" }.to_string();

    // Fingerprints of what's already in the vault: host|username|password.
    let mut existing: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in list().iter().filter(|e| e.kind == "password") {
        if let Ok(s) = read_secret(&e.id) { existing.insert(format!("{}|{}|{}", host_of(&e.url), e.username, s.secret)); }
    }
    let mut out = ImportResult { source, ..Default::default() };
    let mut v = list();
    let get = |r: &Vec<String>, c: Option<usize>| c.and_then(|i| r.get(i)).map(|s| s.trim().to_string()).unwrap_or_default();
    for r in body {
        let pass = r.get(c_pass).cloned().unwrap_or_default();
        let url = get(r, c_url);
        let user = get(r, c_user);
        if pass.is_empty() && user.is_empty() { out.skipped += 1; continue }
        let key = format!("{}|{}|{}", host_of(&url), user, pass);
        if !existing.insert(key) { out.duplicates += 1; continue }
        let mut title = get(r, c_title);
        if title.is_empty() { title = host_of(&url) }
        if title.is_empty() { title = "Imported login".into() }
        let mut fields = BTreeMap::new();
        let otp = get(r, c_otp);
        if !otp.is_empty() { fields.insert("otp".to_string(), otp); }
        let e = Entry { id: new_id(), kind: "password".into(), title, username: user, url, updated: now_ms(), file_name: String::new() };
        write_secret(&e.id, &Secret { secret: pass, notes: get(r, c_note), fields, external: false })?;
        v.push(e);
        out.imported += 1;
    }
    save_list(&v)?;
    if delete_after && out.imported + out.duplicates > 0 {
        let _ = std::fs::remove_file(path);
    }
    Ok(out)
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

/// Export to `dest`. "csv": logins in the Google / Chromium format (name,url,username,password,note),
/// which Google Password Manager, Vivaldi, Chrome, Edge, Bitwarden and 1Password import; Apple
/// Passwords imports it too. "json": every item with all fields (not attached files). Plain text!
pub fn export(dest: &str, format: &str) -> Result<usize, String> {
    require_unlocked()?;
    let items = list();
    let text = if format == "json" {
        #[derive(Serialize)]
        struct Full { #[serde(flatten)] entry: Entry, #[serde(flatten)] secret: Secret }
        let all: Vec<Full> = items.iter().filter_map(|e| read_secret(&e.id).ok().map(|s| Full { entry: e.clone(), secret: s })).collect();
        serde_json::to_string_pretty(&all).unwrap()
    } else {
        let mut t = String::from("name,url,username,password,note\n");
        for e in items.iter().filter(|e| e.kind == "password" || e.kind == "api") {
            let s = read_secret(&e.id).unwrap_or_default();
            t += &[e.title.as_str(), &e.url, &e.username, &s.secret, &s.notes].iter().map(|x| csv_field(x)).collect::<Vec<_>>().join(",");
            t.push('\n');
        }
        t
    };
    std::fs::write(dest, text).map_err(|e| e.to_string())?;
    Ok(items.len())
}

// ---- health ---------------------------------------------------------------------------------------

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub id: String,
    pub weak: bool,
    /// How many other items share this password.
    pub reused: usize,
    /// Not changed for more than a year.
    pub old: bool,
    /// Times seen in known breaches (None = not checked).
    pub breached: Option<u64>,
}

fn weak(p: &str) -> bool {
    let sets = [p.chars().any(|c| c.is_ascii_lowercase()), p.chars().any(|c| c.is_ascii_uppercase()), p.chars().any(|c| c.is_ascii_digit()), p.chars().any(|c| !c.is_ascii_alphanumeric())]
        .iter().filter(|x| **x).count();
    let pool = [0.0, 26.0, 52.0, 62.0, 90.0][sets];
    let bits = p.chars().count() as f64 * f64::log2(f64::max(pool, 10.0));
    bits < 50.0 || p.chars().count() < 10
}

/// Have I Been Pwned range API: send the first 5 hex chars of SHA-1, compare suffixes locally.
fn pwned(p: &str, agent: &ureq::Agent) -> Result<u64, String> {
    use sha1::Digest;
    let hex: String = sha1::Sha1::digest(p.as_bytes()).iter().map(|b| format!("{b:02X}")).collect();
    let (prefix, suffix) = hex.split_at(5);
    let body = agent.get(&format!("https://api.pwnedpasswords.com/range/{prefix}"))
        .header("Add-Padding", "true").header("User-Agent", "Strata-vault")
        .call().map_err(|e| format!("Breach check failed: {e}"))?
        .body_mut().read_to_string().map_err(|e| e.to_string())?;
    Ok(body.lines().find_map(|l| l.split_once(':').filter(|(s, _)| s.eq_ignore_ascii_case(suffix)).and_then(|(_, n)| n.trim().parse().ok())).unwrap_or(0))
}

pub fn health(check_breaches: bool) -> Result<Vec<Health>, String> {
    require_unlocked()?;
    let items: Vec<(Entry, String)> = list().into_iter()
        .filter(|e| e.kind == "password")
        .filter_map(|e| read_secret(&e.id).ok().map(|s| (e, s.secret)))
        .filter(|(_, p)| !p.is_empty())
        .collect();
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (_, p) in &items { *counts.entry(p.as_str()).or_default() += 1; }
    let year = 365 * 24 * 3600 * 1000u64;
    let agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(15))).build().new_agent();
    let mut cache: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut out = vec![];
    for (e, p) in &items {
        let breached = if check_breaches {
            match cache.get(p) {
                Some(n) => Some(*n),
                None => { let n = pwned(p, &agent)?; cache.insert(p.clone(), n); Some(n) }
            }
        } else { None };
        out.push(Health { id: e.id.clone(), weak: weak(p), reused: counts[p.as_str()] - 1, old: now_ms().saturating_sub(e.updated) > year, breached });
    }
    Ok(out)
}

fn clipboard_private(t: &str) -> Result<u32, String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::*;
    use windows::Win32::System::Memory::*;
    unsafe fn global(bytes: &[u8]) -> Option<windows::Win32::Foundation::HGLOBAL> {
        let hg = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)) }.ok()?;
        let p = unsafe { GlobalLock(hg) } as *mut u8;
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len()); let _ = GlobalUnlock(hg); }
        Some(hg)
    }
    let w: Vec<u8> = t.encode_utf16().chain(std::iter::once(0)).flat_map(|c| c.to_le_bytes()).collect();
    unsafe {
        OpenClipboard(None).map_err(|e| e.to_string())?;
        let _ = EmptyClipboard();
        if let Some(h) = global(&w) { let _ = SetClipboardData(13, Some(HANDLE(h.0))); }
        // Tell Windows clipboard history and cloud clipboard to skip this item.
        let zero = 0u32.to_le_bytes();
        for (name, data) in [("ExcludeClipboardContentFromMonitorProcessing", &[0u8][..]), ("CanIncludeInClipboardHistory", &zero[..]), ("CanUploadToCloudClipboard", &zero[..])] {
            let f = RegisterClipboardFormatW(&HSTRING::from(name));
            if let Some(h) = global(data) { let _ = SetClipboardData(f, Some(HANDLE(h.0))); }
        }
        let _ = CloseClipboard();
        Ok(GetClipboardSequenceNumber())
    }
}

/// Random password from the OS random generator.
pub fn generate(len: usize, symbols: bool) -> String {
    const A: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    const S: &[u8] = b"!#$%&*+-=?@^_~";
    let set: Vec<u8> = if symbols { [A, S].concat() } else { A.to_vec() };
    let mut out = String::new();
    let mut buf = vec![0u8; len.clamp(8, 128) * 2];
    getrandom::fill(&mut buf).expect("random");
    for b in buf.chunks(2) {
        let n = u16::from_le_bytes([b[0], b[1]]) as usize;
        if n < (65536 / set.len()) * set.len() { out.push(set[n % set.len()] as char) }
        if out.len() >= len.clamp(8, 128) { break }
    }
    out
}

/// Passphrase of `words` random words (EFF-style short list), separated by '-', with a digit.
pub fn passphrase(words: usize) -> String {
    const W: &[&str] = &["acid","acorn","acre","acts","afar","aged","agent","ajar","alarm","album","alert","alike","alive","aloe","aloft","alone","amend","amino","ample","amuse","angel","anger","ankle","apple","april","apron","arena","argue","arise","armor","army","aroma","array","arson","ashen","atlas","atom","attic","audio","avert","avoid","awake","award","awoke","axis","bacon","badge","bagel","baker","banjo","barge","basil","batch","beach","beads","beefy","begin","being","bench","berry","bike","bingo","birch","blade","blank","blast","blaze","bleak","blend","bless","blimp","blink","bloat","blob","blog","bloom","blot","blue","bluff","blunt","blurt","blush","boast","boat","body","boil","bok","bolt","boned","boney","bonus","booth","boots","boss","botch","both","boxer","brain","brand","brave","bread","brick","bride","brief","brink","brisk","broad","broil","brook","broom","brush","buck","bud","buggy","bulge","bulk","bunch","bunny","bunt","bush","bust","buzz","cable","cache","cadet","cage","cake","calm","camel","candy","canal","canon","cargo","carol","carry","carve","case","cash","cause","cedar","chain","chair","chalk","champ","chant","chaos","charm","chase","cheek","cheer","chef","chess","chest","chew","chief","chili","chill","chip","chomp","chop","chow","chuck","chunk","churn","cider","cinch","city","civic","civil","clad","claim","clamp","clap","clash","clasp","class","claw","clay","clean","clear","cleat","cleft","clerk","click","cling","clink","clip","cloak","clock","clone","cloth","cloud","clump","coach","coast","cocoa","coil","comet","comic","coral","corn","couch","cover","crab","craft","cramp","crane","crank","crate","crave","crawl","crisp","crook","crop","cross","crowd","crown","crumb","crush","crust","cub","cult","cupid","cure","curl","curry","curve","cushy","cycle","dab","dad","daily","dairy","daisy","dance","dandy","darn","dart","dash","data","date","dawn","deal","debit","debut","decal","decay","decoy","deed","delay","denim","dense","dent","depth","derby","desk","dial","diary","dice","dig","dill","dime","diner","dingy","disco","dish","disk","ditch","ditto","dive","dizzy","dock","dodge","doing","doll","dome","donor","donut","dose","dot","dove","down","dowry","doze","drab","drama","drank","draw","dress","dried","drift","drill","drink","drive","drone","droop","drove","drum","dry","duck","duct","dude","dug","duke","duo","dusk","dust","duty","dwarf","dwell","eagle","early","earth","easel","east","eaten","eats","ebay","ebony","echo","edge","eel","egg","elbow","elder","elf","elk","elm","elope","elude","elves","email","emit","empty","emu","enter","entry","envoy","equal","erase","error","erupt","essay","etch","evade","even","evict","evil","evoke","exact","exit","fable","faced","fact","fade","fall","false","fancy","fang","fax","feast","feed","femur","fence","fend","ferry","fetal","fetch","fever","fiber","fifth","fifty","film","filth","final","finch","fit","five","flag","flaky","flame","flap","flask","fled","flick","fling","flint","flip","flirt","float","flock","flop","floss","flyer","foam","foe","fog","foil","folic","folk","food","fool","found","fox","foyer","frail","frame","fray","fresh","fried","frill","frisk","from","front","frost","froth","frown","froze","fruit"];
    let mut r = vec![0u8; words.clamp(3, 10) * 2 + 1];
    getrandom::fill(&mut r).expect("random");
    let mut out: Vec<String> = r.chunks(2).take(words.clamp(3, 10)).map(|b| W[u16::from_le_bytes([b[0], *b.get(1).unwrap_or(&0)]) as usize % W.len()].to_string()).collect();
    let d = r.last().copied().unwrap_or(0) % 10;
    if let Some(last) = out.last_mut() { last.push_str(&d.to_string()); }
    out.join("-")
}
