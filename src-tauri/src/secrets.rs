//! Secrets (API keys, tokens, app passwords) in Windows Credential Manager, never in settings.json or
//! the repo (brief §4.7). Stored as generic credentials named `Strata/<name>`, visible to the user in
//! Control Panel > Credential Manager > Windows Credentials, where they can also be removed.

use windows::core::{HSTRING, PWSTR};
use windows::Win32::Security::Credentials::{CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC};

/// Only these names may be read/written from the UI.
const ALLOWED: &[&str] = &["unsplash", "google-client", "google-token", "yahoo-imap", "openweather", "calendars"];

fn target(name: &str) -> Result<HSTRING, String> {
    if !ALLOWED.contains(&name) && !crate::services::is_service_secret(name) {
        return Err(format!("unknown secret {name}"));
    }
    Ok(HSTRING::from(format!("Strata/{name}")))
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    let t = target(name)?;
    let mut blob: Vec<u8> = value.as_bytes().to_vec();
    let user = HSTRING::from("Strata");
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

pub fn get(name: &str) -> Result<Option<String>, String> {
    let t = target(name)?;
    unsafe {
        let mut p: *mut CREDENTIALW = std::ptr::null_mut();
        if CredReadW(&t, CRED_TYPE_GENERIC, None, &mut p).is_err() {
            return Ok(None);
        }
        let c = &*p;
        let bytes = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec();
        CredFree(p as _);
        Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
    }
}

pub fn delete(name: &str) -> Result<(), String> {
    let t = target(name)?;
    unsafe {
        let _ = CredDeleteW(&t, CRED_TYPE_GENERIC, None);
    }
    Ok(())
}
