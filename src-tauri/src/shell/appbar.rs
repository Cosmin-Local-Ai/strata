//! AppBar space reservation and shell notifications for a dock window.
//!
//! Komorebi re-reads monitor work areas when they change (docs/research/integrations.md §6), so a real
//! AppBar keeps tiled windows clear of the dock without any work-area offsets (D-005).
//!
//! The dock window is taller than the reserved strip (headroom for magnification and hover names); the
//! window region (see `set_input_region`) keeps that headroom click-through.

use std::collections::HashMap;
use std::sync::{mpsc::Sender, Mutex, OnceLock};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, HRGN, RGN_OR};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass, SHAppBarMessage, ABE_BOTTOM, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, ABN_FULLSCREENAPP, ABN_POSCHANGED, ABN_STATECHANGE, APPBARDATA};
use windows::Win32::UI::WindowsAndMessaging::{RegisterWindowMessageW, WM_APP, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_SETTINGCHANGE, WM_POWERBROADCAST, WM_WTSSESSION_CHANGE};

use crate::config::Edge;

pub const CALLBACK_MSG: u32 = WM_APP + 0x51;
const SUBCLASS_ID: usize = 0x5354_4142; // "STAB"

#[derive(Debug, Clone, Copy)]
pub enum ShellEvent {
    /// A fullscreen app opened (true) or closed (false); from ABN_FULLSCREENAPP.
    Fullscreen { hwnd: isize, on: bool },
    /// The shell asks AppBars to re-check their position (taskbar moved/resized, etc.).
    PosChanged { hwnd: isize },
    /// Taskbar auto-hide/always-on-top state changed.
    TaskbarState,
    /// Monitors added/removed/resolution changed.
    DisplayChange,
    /// DPI changed for the dock's monitor.
    DpiChange { hwnd: isize },
    /// Work area or system setting changed (wallpaper changes come through here too).
    SettingChange { spi: u32 },
    /// Explorer restarted: AppBars and taskbar state must be re-applied.
    ExplorerRestarted,
    /// Resume from sleep.
    Resumed,
    /// Session unlock.
    Unlocked,
}

fn senders() -> &'static Mutex<HashMap<isize, Sender<ShellEvent>>> {
    static S: OnceLock<Mutex<HashMap<isize, Sender<ShellEvent>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn taskbar_created_msg() -> u32 {
    static M: OnceLock<u32> = OnceLock::new();
    *M.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

fn send(hwnd: HWND, ev: ShellEvent) {
    if let Some(tx) = senders().lock().unwrap().get(&(hwnd.0 as isize)) {
        let _ = tx.send(ev);
    }
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, _data: usize) -> LRESULT {
    const PBT_APMRESUMEAUTOMATIC: usize = 0x12;
    const WTS_SESSION_UNLOCK: usize = 0x8;
    let h = hwnd.0 as isize;
    match msg {
        CALLBACK_MSG => match wp.0 as u32 {
            ABN_FULLSCREENAPP => send(hwnd, ShellEvent::Fullscreen { hwnd: h, on: lp.0 != 0 }),
            ABN_POSCHANGED => send(hwnd, ShellEvent::PosChanged { hwnd: h }),
            ABN_STATECHANGE => send(hwnd, ShellEvent::TaskbarState),
            _ => {}
        },
        WM_DISPLAYCHANGE => send(hwnd, ShellEvent::DisplayChange),
        WM_DPICHANGED => send(hwnd, ShellEvent::DpiChange { hwnd: h }),
        WM_SETTINGCHANGE => send(hwnd, ShellEvent::SettingChange { spi: wp.0 as u32 }),
        WM_POWERBROADCAST if wp.0 == PBT_APMRESUMEAUTOMATIC => send(hwnd, ShellEvent::Resumed),
        WM_WTSSESSION_CHANGE if wp.0 == WTS_SESSION_UNLOCK => send(hwnd, ShellEvent::Unlocked),
        m if m == taskbar_created_msg() => send(hwnd, ShellEvent::ExplorerRestarted),
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}

/// Start forwarding shell events for `hwnd` to `tx`.
pub fn attach(hwnd: HWND, tx: Sender<ShellEvent>) {
    let _ = taskbar_created_msg();
    senders().lock().unwrap().insert(hwnd.0 as isize, tx);
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(proc_), SUBCLASS_ID, 0);
        // Session-unlock notifications are opt-in per window.
        let _ = windows::Win32::System::RemoteDesktop::WTSRegisterSessionNotification(hwnd, 0);
    }
}

fn abd(hwnd: HWND) -> APPBARDATA {
    APPBARDATA { cbSize: std::mem::size_of::<APPBARDATA>() as u32, hWnd: hwnd, uCallbackMessage: CALLBACK_MSG, ..Default::default() }
}

pub fn register(hwnd: HWND) -> bool {
    let mut d = abd(hwnd);
    unsafe { SHAppBarMessage(ABM_NEW, &mut d) != 0 }
}

pub fn unregister(hwnd: HWND) {
    let mut d = abd(hwnd);
    unsafe {
        SHAppBarMessage(ABM_REMOVE, &mut d);
    }
}

/// Reserve `thickness` physical px along `edge` of `monitor`. Returns the rect the shell granted.
pub fn reserve(hwnd: HWND, monitor: RECT, edge: Edge, thickness: i32) -> RECT {
    let mut d = abd(hwnd);
    d.uEdge = if edge == Edge::Top { ABE_TOP } else { ABE_BOTTOM };
    d.rc = monitor;
    match edge {
        Edge::Top => d.rc.bottom = d.rc.top + thickness,
        Edge::Bottom => d.rc.top = d.rc.bottom - thickness,
    }
    unsafe {
        SHAppBarMessage(ABM_QUERYPOS, &mut d);
        // The shell may have moved our rect to avoid other AppBars (e.g. another bar); keep our thickness.
        match edge {
            Edge::Top => d.rc.bottom = d.rc.top + thickness,
            Edge::Bottom => d.rc.top = d.rc.bottom - thickness,
        }
        SHAppBarMessage(ABM_SETPOS, &mut d);
    }
    d.rc
}

/// Restrict where the dock window takes mouse input (and draws): the union of `rects` (window px),
/// each with its own corner radius. Everything else is click-through to the windows below.
pub fn set_input_region(hwnd: HWND, rects: &[(RECT, i32)]) {
    unsafe {
        let mut rgn: Option<HRGN> = None;
        for (r, radius) in rects {
            let part = if *radius > 0 {
                CreateRoundRectRgn(r.left, r.top, r.right + 1, r.bottom + 1, radius * 2, radius * 2)
            } else {
                CreateRectRgn(r.left, r.top, r.right, r.bottom)
            };
            match rgn {
                None => rgn = Some(part),
                Some(acc) => {
                    CombineRgn(Some(acc), Some(acc), Some(part), RGN_OR);
                    let _ = DeleteObject(part.into());
                }
            }
        }
        let rgn = rgn.unwrap_or_else(|| CreateRectRgn(0, 0, 0, 0));
        // The system owns the region after this call.
        SetWindowRgn(hwnd, Some(rgn), true);
    }
}
