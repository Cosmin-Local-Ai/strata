//! Running app windows for the Apps tile (taskbar parity, brief §4.3).
//!
//! Event-driven: a WinEvent hook (create/destroy/show/hide/name/foreground/minimise/cloak) marks the
//! list dirty; a worker recomputes it at most every 120 ms and emits it only when it changed.
//!
//! Which windows count, like the taskbar: visible, top-level, not tool windows, unowned (or explicitly
//! WS_EX_APPWINDOW), with a title. Cloaking: komorebi hides windows on inactive workspaces by cloaking
//! them (DWM_CLOAKED_APP); those stay in the list, flagged `cloaked`. Windows cloaked by the shell
//! (other virtual desktops, pre-launched Store "ghost" windows) are dropped.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppWindow {
    pub hwnd: i64,
    pub title: String,
    /// Full exe path (icon + grouping + "new window").
    pub exe: String,
    /// AppUserModelID when the window sets one (Store apps, some Win32 apps): groups and icons.
    pub aumid: String,
    pub minimized: bool,
    pub foreground: bool,
    /// Hidden by komorebi on another workspace.
    pub cloaked: bool,
}

static DIRTY: AtomicBool = AtomicBool::new(true);
/// Title-only changes: handled at most every 2 s (terminals and browsers retitle constantly).
static TITLE_DIRTY: AtomicBool = AtomicBool::new(false);

fn emitter() -> &'static Mutex<Option<Box<dyn Fn(&[AppWindow]) + Send>>> {
    static E: OnceLock<Mutex<Option<Box<dyn Fn(&[AppWindow]) + Send>>>> = OnceLock::new();
    E.get_or_init(|| Mutex::new(None))
}

pub fn last() -> &'static Mutex<Vec<AppWindow>> {
    static L: OnceLock<Mutex<Vec<AppWindow>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(Vec::new()))
}

unsafe extern "system" fn on_event(_h: HWINEVENTHOOK, _e: u32, hwnd: HWND, id_object: i32, _c: i32, _t: u32, _ms: u32) {
    // Only window-level events (OBJID_WINDOW = 0), not every caret/cursor/child object.
    if id_object == 0 && !hwnd.0.is_null() {
        if _e == EVENT_OBJECT_NAMECHANGE {
            TITLE_DIRTY.store(true, Ordering::Relaxed);
        } else {
            DIRTY.store(true, Ordering::Relaxed);
        }
    }
}

/// Install hooks (main thread: it has a message loop) and start the worker.
pub fn start(emit: impl Fn(&[AppWindow]) + Send + 'static) {
    *emitter().lock().unwrap() = Some(Box::new(emit));
    unsafe {
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        // Create/destroy/show/hide (not LOCATIONCHANGE: windows move constantly under komorebi).
        let _ = SetWinEventHook(EVENT_OBJECT_CREATE, EVENT_OBJECT_HIDE, None, Some(on_event), 0, 0, flags);
        let _ = SetWinEventHook(EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE, None, Some(on_event), 0, 0, flags);
        let _ = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_event), 0, 0, flags);
        let _ = SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, None, Some(on_event), 0, 0, flags);
        let _ = SetWinEventHook(EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED, None, Some(on_event), 0, 0, flags);
    }
    std::thread::Builder::new()
        .name("strata-winlist".into())
        .spawn(|| {
          let mut last_title_pass = std::time::Instant::now();
          loop {
            std::thread::sleep(std::time::Duration::from_millis(120));
            let structural = DIRTY.swap(false, Ordering::Relaxed);
            let titles = last_title_pass.elapsed() > std::time::Duration::from_secs(2) && TITLE_DIRTY.swap(false, Ordering::Relaxed);
            if !structural && !titles {
                continue;
            }
            if titles { last_title_pass = std::time::Instant::now(); }
            let list = enumerate();
            let changed = {
                let mut l = last().lock().unwrap();
                let c = *l != list;
                *l = list.clone();
                c
            };
            if changed {
                if let Some(e) = emitter().lock().unwrap().as_ref() {
                    e(&list);
                }
            }
          }
        })
        .expect("winlist thread");
}

fn aumid_of(hwnd: HWND) -> String {
    use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
    use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
    unsafe {
        SHGetPropertyStoreForWindow::<IPropertyStore>(hwnd)
            .and_then(|ps| ps.GetValue(&PKEY_AppUserModel_ID))
            .map(|v| v.to_string())
            .unwrap_or_default()
    }
}

unsafe extern "system" fn each(h: HWND, lp: LPARAM) -> BOOL {
    let out = unsafe { &mut *(lp.0 as *mut Vec<AppWindow>) };
    unsafe {
        if !IsWindowVisible(h).as_bool() {
            return true.into();
        }
        let ex = GetWindowLongW(h, GWL_EXSTYLE) as u32;
        let owner = GetWindow(h, GW_OWNER).unwrap_or_default();
        let app_window = ex & WS_EX_APPWINDOW.0 != 0;
        if ex & WS_EX_TOOLWINDOW.0 != 0 && !app_window {
            return true.into();
        }
        if !owner.0.is_null() && !app_window {
            return true.into();
        }
        if ex & WS_EX_NOACTIVATE.0 != 0 && !app_window {
            return true.into();
        }
        let len = GetWindowTextLengthW(h);
        if len == 0 {
            return true.into();
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut _ as _, 4);
        // DWM_CLOAKED_SHELL = 2: other virtual desktop / pre-launched UWP ghost: not a taskbar window.
        if cloaked & 2 != 0 {
            return true.into();
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        if pid == std::process::id() {
            return true.into();
        }
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(h, &mut buf);
        let title = String::from_utf16_lossy(&buf[..n as usize]);
        // exe and AUMID never change for a window: cache them (they are the expensive part).
        let (exe, aumid) = {
            let mut c = ident_cache().lock().unwrap();
            c.entry(h.0 as i64).or_insert_with(|| (crate::shell::gamemode::process_path(pid), aumid_of(h))).clone()
        };
        out.push(AppWindow {
            hwnd: h.0 as i64,
            title,
            aumid,
            exe,
            minimized: IsIconic(h).as_bool(),
            foreground: GetForegroundWindow() == h,
            cloaked: cloaked != 0,
        });
    }
    true.into()
}

fn ident_cache() -> &'static Mutex<std::collections::HashMap<i64, (String, String)>> {
    static C: OnceLock<Mutex<std::collections::HashMap<i64, (String, String)>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

pub fn enumerate() -> Vec<AppWindow> {
    let mut v: Vec<AppWindow> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut v as *mut _ as isize));
    }
    // Forget identities of windows that are gone (hwnds get reused).
    ident_cache().lock().unwrap().retain(|h, _| v.iter().any(|w| w.hwnd == *h));
    // ApplicationFrameHost hosts UWP windows: use the AUMID for identity, the exe path is useless.
    v
}

/// Click on an app window: bring it forward, or minimise it if it already is (taskbar behaviour).
/// Windows on another komorebi workspace: focus that workspace first, then the window.
pub fn activate(hwnd: i64, toggle_minimize: bool, komorebi_location: Option<(usize, usize)>) {
    let h = HWND(hwnd as _);
    unsafe {
        if toggle_minimize && GetForegroundWindow() == h && !IsIconic(h).as_bool() {
            // A plain minimise is what komorebi handles (it un-tiles and re-tiles on restore).
            // Async: a hung app must never block the caller.
            let _ = ShowWindowAsync(h, SW_MINIMIZE);
            return;
        }
        if let Some((m, w)) = komorebi_location {
            let _ = crate::komorebi::komorebic(&["focus-monitor-workspace", &m.to_string(), &w.to_string()]);
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        if IsIconic(h).as_bool() {
            let _ = ShowWindowAsync(h, SW_RESTORE);
        }
        if !SetForegroundWindow(h).as_bool() || GetForegroundWindow() != h {
            // Windows refuses foreground changes from a process that doesn't own the foreground
            // (the dock never takes focus). A zero-distance synthetic mouse move makes Strata the
            // last input receiver, which lifts the lock (an Alt tap would open apps' menu bars).
            use windows::Win32::UI::Input::KeyboardAndMouse::*;
            let nudge = INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dwFlags: MOUSEEVENTF_MOVE, ..Default::default() } } };
            SendInput(&[nudge], std::mem::size_of::<INPUT>() as i32);
            let _ = SetForegroundWindow(h);
            let _ = BringWindowToTop(h);
        }
    }
}

pub fn close(hwnd: i64) {
    unsafe {
        let _ = PostMessageW(Some(HWND(hwnd as _)), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

/// Every top-level window handle (unfiltered).
pub fn all_top_level() -> Vec<HWND> {
    unsafe extern "system" fn cb(h: HWND, lp: LPARAM) -> windows::core::BOOL {
        unsafe { (*(lp.0 as *mut Vec<HWND>)).push(h) };
        true.into()
    }
    let mut v: Vec<HWND> = vec![];
    unsafe { let _ = EnumWindows(Some(cb), LPARAM(&mut v as *mut _ as isize)); }
    v
}
