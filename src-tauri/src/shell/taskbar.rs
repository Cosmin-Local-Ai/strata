//! Strata's side of taskbar control (decision D-006). The restore logic itself is in taskbar_core.rs,
//! shared with strata-guard.exe.

use super::taskbar_core as core;
use crate::config::TaskbarMode;
use std::os::windows::process::CommandExt;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::System::Registry::{RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};

const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const RUN_VALUE: PCWSTR = w!("StrataTaskbarRestore");

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum TaskbarStatus {
    /// Strata changed the taskbar as configured.
    Managed { hidden: bool },
    /// Strata leaves the taskbar alone (user setting).
    Left,
    /// Another tool manages the taskbar; Strata will not fight it.
    Blocked { by: String },
}

fn guard_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // Next to strata.exe (development) or in resources\ (installed).
    let dir = exe.parent()?;
    let p = [dir.join("strata-guard.exe"), dir.join("resources").join("strata-guard.exe")].into_iter().find(|p| p.exists())?;
    p.exists().then_some(p)
}

fn set_run_entry(on: bool) {
    unsafe {
        if on {
            let Some(g) = guard_path() else { return };
            let cmd = HSTRING::from(format!("\"{}\" --restore", g.display()));
            let bytes = (cmd.len() + 1) * 2;
            let _ = RegSetKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE, REG_SZ.0, Some(cmd.as_ptr() as _), bytes as u32);
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE);
        }
    }
}

fn spawn_guard() {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    let Some(g) = guard_path() else {
        log::error!("strata-guard.exe missing next to strata.exe; taskbar will still be restored at next launch");
        return;
    };
    let pid = std::process::id().to_string();
    let r = std::process::Command::new(&g)
        .args(["--watch", &pid])
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_BREAKAWAY_FROM_JOB)
        .spawn()
        .or_else(|_| std::process::Command::new(&g).args(["--watch", &pid]).creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS).spawn());
    match r {
        Ok(_) => log::info!("taskbar guard watching pid {pid}"),
        Err(e) => log::error!("could not start strata-guard: {e}"),
    }
}

/// Called once at startup, before any dock is shown.
pub fn start(mode: TaskbarMode) -> TaskbarStatus {
    // Leftovers from a crash or kill: put the taskbar back first, then re-apply cleanly.
    if let Some((_, applied)) = core::read_state() {
        if applied.pid != std::process::id() {
            log::warn!("restoring taskbar state left by a previous Strata run (pid {})", applied.pid);
            core::restore();
        }
    }
    if crate::util::process_running(&["thide.exe"]) {
        log::warn!("thide.exe is running and manages the taskbar; Strata leaves it alone");
        return TaskbarStatus::Blocked { by: "Taskbar Hide (thide.exe)".into() };
    }
    match mode {
        TaskbarMode::Leave => TaskbarStatus::Left,
        TaskbarMode::AutoHide | TaskbarMode::Hidden => {
            let hidden = mode == TaskbarMode::Hidden;
            spawn_guard();
            set_run_entry(true);
            core::apply(hidden, std::process::id());
            TaskbarStatus::Managed { hidden }
        }
    }
}

/// Explorer restarted or a setting flipped the taskbar back: re-apply without touching the saved
/// original state.
pub fn reapply(status: &TaskbarStatus) {
    if let TaskbarStatus::Managed { hidden } = status {
        core::apply(*hidden, std::process::id());
    }
}

/// Clean exit.
pub fn stop() {
    core::restore();
    set_run_entry(false);
}
