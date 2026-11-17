//! strata-guard: a tiny watchdog that puts the Windows taskbar back if Strata exits for any reason
//! (normal quit, crash, Task Manager kill, update). It shares the restore code with Strata.
//!
//!   strata-guard --watch <pid>   wait for that process to end, then restore if it left changes behind
//!   strata-guard --restore       restore now if the process that changed the taskbar is gone
//!                                (used from HKCU\...\Run at logon and on demand)
#![windows_subsystem = "windows"]

#[path = "../../src-tauri/src/shell/taskbar_core.rs"]
#[allow(dead_code)]
mod taskbar_core;

use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, WaitForSingleObject, INFINITE, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE};

const STILL_ACTIVE: u32 = 259;

fn alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
        let mut code = 0u32;
        let ok = GetExitCodeProcess(h, &mut code).is_ok() && code == STILL_ACTIVE;
        let _ = CloseHandle(h);
        ok
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("--watch") => {
            let Some(pid) = args.get(2).and_then(|p| p.parse::<u32>().ok()) else { return };
            unsafe {
                if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
                    let r = WaitForSingleObject(h, INFINITE);
                    let _ = CloseHandle(h);
                    if r != WAIT_OBJECT_0 {
                        return;
                    }
                }
            }
            // Strata is gone. If it exited cleanly it already restored and deleted the state file.
            if let Some((_, applied)) = taskbar_core::read_state() {
                if applied.pid == pid || !alive(applied.pid) {
                    taskbar_core::restore();
                }
            }
        }
        Some("--restore") => {
            if let Some((_, applied)) = taskbar_core::read_state() {
                if !alive(applied.pid) {
                    taskbar_core::restore();
                }
            }
        }
        _ => {}
    }
}
