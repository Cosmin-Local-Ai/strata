//! System actions used by tiles: launching, power, Start / Win+X, uptime.

pub mod audio;
pub mod audio_ext;
pub mod everything;
pub mod icons;
pub mod media;
pub mod misc;
pub mod stats;
pub mod net;
pub mod netinfo;
pub mod previews;
pub mod thumbs;
pub mod tray;
pub mod winlist;

use std::os::windows::process::CommandExt;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_LWIN, VK_X};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Open a file, shortcut, exe, folder, URL or `shell:AppsFolder\<AUMID>` the way Explorer would.
pub fn launch(target: &str, args: Option<&str>) -> Result<(), String> {
    let t = HSTRING::from(expand_env(target));
    let a = args.map(HSTRING::from);
    let r = unsafe {
        ShellExecuteW(None, w!("open"), &t, a.as_ref().map(|a| PCWSTR(a.as_ptr())).unwrap_or(PCWSTR::null()), PCWSTR::null(), SW_SHOWNORMAL)
    };
    if r.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("Windows could not open {target} (code {})", r.0 as isize))
    }
}

pub fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        if let Some(j) = after.find('%') {
            let name = &after[..j];
            match std::env::var(name) {
                Ok(v) => out.push_str(&v),
                Err(_) => {
                    out.push('%');
                    out.push_str(name);
                    out.push('%');
                }
            }
            rest = &after[j + 1..];
        } else {
            out.push('%');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() }, ..Default::default() } },
    }
}

/// Open the Start menu (same as pressing the Windows key).
pub fn open_start() {
    let inputs = [key(VK_LWIN, false), key(VK_LWIN, true)];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Open Windows' quick-link menu (Win+X), the Start tile's right-click.
pub fn open_winx() {
    let inputs = [key(VK_LWIN, false), key(VK_X, false), key(VK_X, true), key(VK_LWIN, true)];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Power actions: shut down, restart, sign out, hibernate, sleep, lock.
pub fn power(action: &str) -> Result<(), String> {
    let run = |args: &[&str]| {
        std::process::Command::new("shutdown").args(args).creation_flags(CREATE_NO_WINDOW).spawn().map(|_| ()).map_err(|e| e.to_string())
    };
    match action {
        "lock" => unsafe { windows::Win32::System::Shutdown::LockWorkStation().map_err(|e| e.to_string()) },
        "signout" => run(&["/l"]),
        "sleep" => unsafe {
            // Sleep, not hibernate; wake events allowed.
            if windows::Win32::System::Power::SetSuspendState(false, false, false) { Ok(()) } else { Err("sleep was refused by Windows".into()) }
        },
        "hibernate" => run(&["/h"]),
        "restart" => run(&["/r", "/t", "0"]),
        "shutdown" => run(&["/s", "/hybrid", "/t", "0"]),
        _ => Err(format!("unknown power action {action}")),
    }
}

/// Run as administrator (UAC prompt).
pub fn launch_admin(target: &str) -> Result<(), String> {
    let t = HSTRING::from(expand_env(target));
    let r = unsafe { ShellExecuteW(None, w!("runas"), &t, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    if r.0 as isize > 32 { Ok(()) } else { Err("cancelled or failed".into()) }
}

/// Show a file in Explorer, selected.
pub fn reveal(target: &str) -> Result<(), String> {
    std::process::Command::new("explorer.exe").arg(format!("/select,{}", expand_env(target))).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Seconds since boot.
pub fn uptime() -> u64 {
    unsafe { windows::Win32::System::SystemInformation::GetTickCount64() / 1000 }
}
