//! Vault widget: passwords and API keys. Each secret is a Windows Credential Manager entry
//! ("Strata/vault/<id>"); the list the UI sees (title, username, URL, kind) holds no secrets. Copying
//! goes through here: the text is marked "not for clipboard history / cloud clipboard" and cleared
//! after 30 s if it is still on the clipboard. Opening the vault asks for Windows Hello.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use windows::core::{HSTRING, PWSTR};
use windows::Win32::Security::Credentials::*;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Entry {
    pub id: String,
    pub kind: String, // "password" | "api"
    pub title: String,
    pub username: String,
    pub url: String,
    pub updated: u64,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Secret {
    pub secret: String,
    pub notes: String,
}

static UNLOCKED_UNTIL: Mutex<Option<std::time::Instant>> = Mutex::new(None);

fn index_path() -> std::path::PathBuf { crate::config::data_dir().join("vault-index.json") }
pub fn list() -> Vec<Entry> {
    std::fs::read(index_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}
fn save_list(v: &[Entry]) -> Result<(), String> {
    std::fs::write(index_path(), serde_json::to_vec_pretty(v).unwrap()).map_err(|e| e.to_string())
}

fn target(id: &str) -> HSTRING { HSTRING::from(format!("Strata/vault/{id}")) }

fn write_secret(id: &str, s: &Secret) -> Result<(), String> {
    let mut blob = serde_json::to_vec(s).unwrap();
    if blob.len() > 2560 { return Err("Too long: secret and notes together must stay under about 2,500 characters".into()) }
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

fn read_secret(id: &str) -> Result<Secret, String> {
    let t = target(id);
    unsafe {
        let mut p: *mut CREDENTIALW = std::ptr::null_mut();
        CredReadW(&t, CRED_TYPE_GENERIC, None, &mut p).map_err(|_| "This entry's secret is missing from Credential Manager")?;
        let c = &*p;
        let bytes = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec();
        CredFree(p as _);
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
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

/// Windows Hello (PIN, fingerprint or face). If Hello isn't set up on this PC the vault still opens
/// (the secrets remain protected by your Windows sign-in), and the result says so.
pub fn unlock(hwnd: isize, minutes: u64) -> Result<String, String> {
    use windows::Security::Credentials::UI::*;
    let avail = UserConsentVerifier::CheckAvailabilityAsync().and_then(|a| a.join()).map_err(|e| e.to_string())?;
    let mode = if avail == UserConsentVerifierAvailability::Available {
        let interop: windows::Win32::System::WinRT::IUserConsentVerifierInterop =
            windows::core::factory::<UserConsentVerifier, _>().map_err(|e| e.to_string())?;
        let op: windows_future::IAsyncOperation<UserConsentVerificationResult> = unsafe {
            interop.RequestVerificationForWindowAsync(windows::Win32::Foundation::HWND(hwnd as _), &HSTRING::from("Unlock your Strata vault"))
        }.map_err(|e| e.to_string())?;
        let r = op.join().map_err(|e| e.to_string())?;
        if r != UserConsentVerificationResult::Verified { return Err("Not verified".into()) }
        "hello"
    } else {
        "no-hello"
    };
    *UNLOCKED_UNTIL.lock().unwrap() = Some(std::time::Instant::now() + std::time::Duration::from_secs(minutes.clamp(1, 120) * 60));
    Ok(mode.into())
}

pub fn save(mut e: Entry, secret: Option<String>, notes: Option<String>) -> Result<Entry, String> {
    require_unlocked()?;
    let mut v = list();
    if e.id.is_empty() { e.id = format!("{:x}", now_ms()) }
    e.updated = now_ms();
    let mut s = read_secret(&e.id).unwrap_or_default();
    if let Some(x) = secret { s.secret = x }
    if let Some(n) = notes { s.notes = n }
    write_secret(&e.id, &s)?;
    v.retain(|x| x.id != e.id);
    v.push(e.clone());
    save_list(&v)?;
    Ok(e)
}

pub fn delete(id: &str) -> Result<(), String> {
    require_unlocked()?;
    unsafe { let _ = CredDeleteW(&target(id), CRED_TYPE_GENERIC, None); }
    let v: Vec<Entry> = list().into_iter().filter(|x| x.id != id).collect();
    save_list(&v)
}

pub fn reveal(id: &str) -> Result<Secret, String> {
    require_unlocked()?;
    read_secret(id)
}

/// Copies `field` ("secret" | "username") without it entering clipboard history; clears it after 30 s.
pub fn copy(id: &str, field: &str) -> Result<(), String> {
    require_unlocked()?;
    let text = match field {
        "username" => list().into_iter().find(|x| x.id == id).map(|x| x.username).ok_or("not found")?,
        _ => read_secret(id)?.secret,
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
