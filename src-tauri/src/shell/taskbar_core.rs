//! Windows taskbar hide/restore, shared by strata.exe and strata-guard.exe (included by path, so it must
//! only depend on `std` and `windows`). Brief §7: the taskbar must come back on normal exit, crash, kill,
//! update, reboot and Explorer restart, without depending on the main process surviving.
//!
//! Mechanism (no Windows files are modified):
//! * Auto-hide: `SHAppBarMessage(ABM_SETSTATE, ABS_AUTOHIDE)` on the taskbar: the documented API, the
//!   same switch as Taskbar settings > "Automatically hide the taskbar".
//! * Hidden: auto-hide plus `ShowWindow(SW_HIDE)` on the primary and secondary taskbar windows.
//! * Before the first change we write the original state to `%LOCALAPPDATA%\Strata\taskbar-state.txt`.
//!   Restore reads that file, puts everything back and deletes it. If the file already exists (crash
//!   last time), the original state in it is kept, never overwritten with our own modified state.

use std::path::PathBuf;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Shell::{SHAppBarMessage, ABM_GETSTATE, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, FindWindowW, GetClassNameW, ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Original {
    pub autohide: bool,
    /// Whether the primary taskbar window was visible (another tool may have hidden it).
    pub visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applied {
    pub hidden: bool,
    pub pid: u32,
}

pub fn state_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    // Not "Strata": that is the install folder; app data stays separate from program files.
    let d = base.join("local.strata.dock");
    let _ = std::fs::create_dir_all(&d);
    d.join("taskbar-state.txt")
}

pub fn read_state() -> Option<(Original, Applied)> {
    let text = std::fs::read_to_string(state_path()).ok()?;
    let mut autohide = None;
    let mut visible = true;
    let mut hidden = false;
    let mut pid = 0;
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "original_autohide" => autohide = Some(v.trim() == "1"),
                "original_visible" => visible = v.trim() == "1",
                "hidden" => hidden = v.trim() == "1",
                "pid" => pid = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    Some((Original { autohide: autohide?, visible }, Applied { hidden, pid }))
}

fn write_state(o: Original, a: Applied) {
    let text = format!("original_autohide={}\noriginal_visible={}\nhidden={}\npid={}\n", o.autohide as u8, o.visible as u8, a.hidden as u8, a.pid);
    let path = state_path();
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

fn tray() -> Option<HWND> {
    unsafe { FindWindowW(w!("Shell_TrayWnd"), None).ok() }
}

pub fn get_autohide() -> bool {
    unsafe {
        let mut d = APPBARDATA { cbSize: std::mem::size_of::<APPBARDATA>() as u32, ..Default::default() };
        d.hWnd = tray().unwrap_or_default();
        (SHAppBarMessage(ABM_GETSTATE, &mut d) as u32) & ABS_AUTOHIDE != 0
    }
}

pub fn set_autohide(on: bool) {
    unsafe {
        let mut d = APPBARDATA { cbSize: std::mem::size_of::<APPBARDATA>() as u32, ..Default::default() };
        d.hWnd = tray().unwrap_or_default();
        // Keep the other state bits (always-on-top is implicit on Windows 10/11).
        let cur = SHAppBarMessage(ABM_GETSTATE, &mut d) as u32;
        let new = if on { cur | ABS_AUTOHIDE } else { cur & !ABS_AUTOHIDE };
        d.lParam = windows::Win32::Foundation::LPARAM(new as isize);
        SHAppBarMessage(ABM_SETSTATE, &mut d);
    }
}

unsafe extern "system" fn each_tray(h: HWND, lp: LPARAM) -> windows::core::BOOL {
    let mut cls = [0u16; 64];
    let n = unsafe { GetClassNameW(h, &mut cls) } as usize;
    let name = String::from_utf16_lossy(&cls[..n]);
    if name == "Shell_TrayWnd" || name == "Shell_SecondaryTrayWnd" {
        let show = lp.0 != 0;
        unsafe {
            let _ = ShowWindow(h, if show { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }
    true.into()
}

/// Is the primary or any secondary taskbar showing?
pub fn any_visible() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, IsWindowVisible};
    unsafe {
        if tray_visible() { return true }
        let mut after: Option<HWND> = None;
        while let Ok(h) = FindWindowExW(None, after, w!("Shell_SecondaryTrayWnd"), None) {
            if h.0.is_null() { break }
            if IsWindowVisible(h).as_bool() { return true }
            after = Some(h);
        }
        false
    }
}

pub fn tray_visible() -> bool {
    tray().map(|h| unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(h).as_bool() }).unwrap_or(true)
}

/// Show or hide the primary and all secondary taskbars.
pub fn set_visible(show: bool) {
    unsafe {
        let _ = EnumWindows(Some(each_tray), LPARAM(show as isize));
    }
}

/// Apply a mode. `hidden=false` means auto-hide only.
pub fn apply(hidden: bool, pid: u32) {
    let original = match read_state() {
        Some((o, _)) => o, // stale file from a crash: its original is the truth
        None => Original { autohide: get_autohide(), visible: tray_visible() },
    };
    write_state(original, Applied { hidden, pid });
    set_autohide(true);
    set_visible(!hidden);
}

/// Put the taskbar back exactly as it was before Strata touched it. Safe to call repeatedly and when
/// nothing was changed.
pub fn restore() -> bool {
    let Some((o, a)) = read_state() else { return false };
    if a.hidden || o.visible {
        set_visible(o.visible);
    }
    if !o.autohide {
        set_autohide(false);
    }
    let _ = std::fs::remove_file(state_path());
    true
}
