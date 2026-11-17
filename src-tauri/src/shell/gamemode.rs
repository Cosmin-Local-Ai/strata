//! Game / local-AI yield mode (decision D-011).
//!
//! Triggers, evaluated on foreground changes (WinEvent hook, no polling), on AppBar fullscreen
//! notifications, and every 5 s for background processes:
//! * fullscreen (borderless or exclusive) on a monitor: docks on that monitor hide;
//! * Windows' own "D3D fullscreen / presentation / busy" notification state;
//! * a running Steam game (image under any library's `steamapps\common`);
//! * a running process from the user's "Yield GPU to" list (plain exe names).
//!
//! Entries of the form `exe:argument` match only when the process command line contains `argument`
//! (e.g. `ollama.exe:runner`: Ollama's tray server idles all day, its model runner is the GPU user).
//!
//! NOT YET: the GPU-utilisation trigger (other processes using > 60% 3D for 3 s). Listed as Partial
//! in docs/parity.md.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL};
use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS};

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YieldState {
    pub active: bool,
    pub reasons: Vec<String>,
    /// Monitors (HMONITOR as isize) with a fullscreen window: docks there hide.
    #[serde(skip)]
    pub fullscreen_monitors: Vec<isize>,
}

pub enum Trigger {
    Foreground,
    FullscreenHint,
    Settings { yield_apps: Vec<String>, steam: bool },
}

fn sender() -> &'static Mutex<Option<Sender<Trigger>>> {
    static S: OnceLock<Mutex<Option<Sender<Trigger>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None))
}

pub fn notify(t: Trigger) {
    if let Some(tx) = sender().lock().unwrap().as_ref() {
        let _ = tx.send(t);
    }
}

unsafe extern "system" fn on_foreground(_h: HWINEVENTHOOK, _e: u32, _hwnd: HWND, _o: i32, _c: i32, _t: u32, _ms: u32) {
    notify(Trigger::Foreground);
}

/// Must be called on a thread with a message loop (the main thread).
pub fn install_foreground_hook() {
    unsafe {
        let _ = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_foreground), 0, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS);
    }
}

/// Full executable path of a process, or "".
pub fn process_path(pid: u32) -> String {
    image_path(pid).unwrap_or_default()
}

/// Executable file name of a process ("Spotify.exe"), or "".
/// Full-screen *overlays* that are not games: snipping, colour pickers, launchers.
pub fn is_overlay_tool(exe: &str) -> bool {
    const TOOLS: &[&str] = &["screenclippinghost.exe", "snippingtool.exe", "screensketch.exe", "flow.launcher.exe", "powertoys.powerlauncher.exe",
        "powertoys.colorpickerui.exe", "powertoys.screenruler.exe", "sharex.exe", "greenshot.exe", "lightshot.exe", "flameshot.exe", "textinputhost.exe"];
    TOOLS.contains(&exe.to_ascii_lowercase().as_str())
}

pub fn process_name(pid: u32) -> String {
    image_path(pid).map(|p| p.rsplit('\\').next().unwrap_or("").to_string()).unwrap_or_default()
}

fn image_path(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        r.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

fn class_of(h: HWND) -> String {
    let mut cls = [0u16; 128];
    let n = unsafe { GetClassNameW(h, &mut cls) } as usize;
    String::from_utf16_lossy(&cls[..n])
}

struct Evaluator {
    steam_common: Vec<String>,
    yield_apps: Vec<String>,
    steam: bool,
    /// pid → lowercase image path, so the 5 s scan only opens new processes.
    paths: HashMap<u32, String>,
}

impl Evaluator {
    fn foreground(&self, reasons: &mut BTreeSet<String>, fs_monitors: &mut Vec<isize>) {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return;
            }
            let cls = class_of(fg);
            // The desktop, the taskbar and our own windows are never "fullscreen apps".
            if matches!(cls.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "Tauri Window") {
                return;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            if pid == std::process::id() {
                return;
            }
            let exe = image_path(pid).unwrap_or_default();
            let name = exe.rsplit('\\').next().unwrap_or("").to_string();
            if is_overlay_tool(&name) {
                return;
            }
            let mon = MonitorFromWindow(fg, MONITOR_DEFAULTTONULL);
            if !mon.0.is_null() {
                let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
                let mut wr = RECT::default();
                if GetMonitorInfoW(mon, &mut mi).as_bool() && GetWindowRect(fg, &mut wr).is_ok() {
                    let m = mi.rcMonitor;
                    // Covers the whole monitor: borderless or exclusive fullscreen. Komorebi monocle
                    // fills the *work area*, which excludes our AppBar, so it is not mistaken for this.
                    if wr.left <= m.left && wr.top <= m.top && wr.right >= m.right && wr.bottom >= m.bottom {
                        reasons.insert(format!("Fullscreen: {name}"));
                        fs_monitors.push(mon.0 as isize);
                    }
                }
            }
        }
    }

    fn notification_state(&self, reasons: &mut BTreeSet<String>) {
        // Same for screen-snipping and launcher overlays.
        unsafe {
            let fg = GetForegroundWindow();
            let mut pid = 0u32;
            if !fg.0.is_null() { GetWindowThreadProcessId(fg, Some(&mut pid)); }
            if pid != 0 && is_overlay_tool(&process_name(pid)) { return; }
        }
        // Windows reports "busy" for ANY full-screen foreground window, including our own power panel;
        // that must not count as a game (it made the dock yield and flicker behind the panel).
        unsafe {
            let fg = GetForegroundWindow();
            let mut pid = 0u32;
            if !fg.0.is_null() { GetWindowThreadProcessId(fg, Some(&mut pid)); }
            if pid == std::process::id() { return; }
        }
        if let Ok(s) = unsafe { SHQueryUserNotificationState() } {
            if s == QUNS_RUNNING_D3D_FULL_SCREEN {
                reasons.insert("Direct3D fullscreen".into());
            } else if s == QUNS_PRESENTATION_MODE {
                reasons.insert("Presentation mode".into());
            } else if s == QUNS_BUSY {
                reasons.insert("Fullscreen app".into());
            }
        }
    }

    fn processes(&mut self, reasons: &mut BTreeSet<String>) {
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return };
            let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            let mut alive = Vec::new();
            let windowed = visible_window_pids();
            if Process32FirstW(snap, &mut e).is_ok() {
                loop {
                    let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                    let exe = String::from_utf16_lossy(&e.szExeFile[..len]);
                    let pid = e.th32ProcessID;
                    alive.push(pid);
                    for entry in &self.yield_apps {
                        let (name, arg) = match entry.split_once(':') { Some((n, a)) => (n, Some(a)), None => (entry.as_str(), None) };
                        if !name.eq_ignore_ascii_case(&exe) {
                            continue;
                        }
                        let hit = match arg {
                            None => true,
                            Some(a) => command_line(pid).map(|c| c.to_lowercase().contains(&a.to_lowercase())).unwrap_or(false),
                        };
                        if hit {
                            reasons.insert(format!("Local AI / yield list: {entry}"));
                        }
                    }
                    if self.steam && !self.steam_common.is_empty() {
                        let path = match self.paths.get(&pid) {
                            Some(p) => p.clone(),
                            None => {
                                let p = image_path(pid).unwrap_or_default().to_lowercase();
                                self.paths.insert(pid, p.clone());
                                p
                            }
                        };
                        if let Some(dir) = self.steam_common.iter().find_map(|c| path.strip_prefix(c.as_str())).and_then(|rest| rest.split('\\').next()) {
                            // A game: lives in a Steam game folder that isn't a known tool, isn't a
                            // helper exe, and owns a visible top-level window (background helpers don't).
                            if !NON_GAME_STEAM_DIRS.contains(&dir) && !is_launcher_helper(&exe) && windowed.contains(&pid) {
                                reasons.insert(format!("Game: {exe}"));
                            }
                        }
                    }
                    if Process32NextW(snap, &mut e).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
            self.paths.retain(|pid, _| alive.contains(pid));
        }
    }
}

/// Command line of another process (NtQueryInformationProcess, ProcessCommandLineInformation = 60).
/// Only used for the few processes whose exe matches an `exe:argument` entry.
fn command_line(pid: u32) -> Option<String> {
    use windows::Wdk::System::Threading::{NtQueryInformationProcess, PROCESSINFOCLASS};
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u8; 8192];
        let mut ret = 0u32;
        let st = NtQueryInformationProcess(h, PROCESSINFOCLASS(60), buf.as_mut_ptr() as _, buf.len() as u32, &mut ret);
        let _ = CloseHandle(h);
        if st.is_err() {
            return None;
        }
        // Buffer starts with a UNICODE_STRING { Length: u16, MaximumLength: u16, Buffer: *u16 }.
        let us = &*(buf.as_ptr() as *const windows::Win32::Foundation::UNICODE_STRING);
        let n = (us.Length / 2) as usize;
        Some(String::from_utf16_lossy(std::slice::from_raw_parts(us.Buffer.0, n)))
    }
}

/// Steam "apps" that are tools, not games (folder names under steamappscommon, lowercase).
const NON_GAME_STEAM_DIRS: &[&str] = &["wallpaper_engine", "steamworks shared", "steam controller configs", "steamvr", "soundpad", "lossless scaling", "obs studio"];

/// PIDs owning at least one visible, non-tool, top-level window.
fn visible_window_pids() -> std::collections::HashSet<u32> {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowLongW, IsWindowVisible, GWL_EXSTYLE, WS_EX_TOOLWINDOW};
    unsafe extern "system" fn cb(h: HWND, lp: LPARAM) -> windows::core::BOOL {
        let set = unsafe { &mut *(lp.0 as *mut std::collections::HashSet<u32>) };
        unsafe {
            if IsWindowVisible(h).as_bool() && (GetWindowLongW(h, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0) == 0 {
                let mut pid = 0u32;
                GetWindowThreadProcessId(h, Some(&mut pid));
                set.insert(pid);
            }
        }
        true.into()
    }
    let mut set = std::collections::HashSet::new();
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut set as *mut _ as isize));
    }
    set
}

/// Steam tools that live under steamapps\common but are not games.
fn is_launcher_helper(exe: &str) -> bool {
    let e = exe.to_ascii_lowercase();
    // Wallpaper Engine is installed as a Steam "game" but must never count as one.
    e.contains("wallpaper") || e.starts_with("ui32") || e == "launcher.exe" || e.contains("crashhandler") || e.contains("redist") || e == "steamwebhelper.exe"
}

/// Start the evaluator thread. `on_change` runs on that thread whenever the yield state changes.
pub fn start(yield_apps: Vec<String>, steam: bool, on_change: impl Fn(&YieldState) + Send + 'static) {
    let (tx, rx): (Sender<Trigger>, Receiver<Trigger>) = channel();
    *sender().lock().unwrap() = Some(tx);
    std::thread::Builder::new()
        .name("strata-gamemode".into())
        .spawn(move || {
            let steam_common: Vec<String> = crate::wallpaper_engine::steam_libraries()
                .into_iter()
                .map(|l| {
                    let mut p: PathBuf = l.join("steamapps").join("common");
                    p.push("");
                    p.to_string_lossy().to_lowercase()
                })
                .collect();
            let mut ev = Evaluator { steam_common, yield_apps, steam, paths: HashMap::new() };
            let mut last = YieldState::default();
            let mut calm_since: Option<std::time::Instant> = None;
            loop {
                match rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(Trigger::Settings { yield_apps, steam }) => {
                        ev.yield_apps = yield_apps;
                        ev.steam = steam;
                    }
                    Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                let mut reasons = BTreeSet::new();
                let mut fs = Vec::new();
                ev.foreground(&mut reasons, &mut fs);
                ev.notification_state(&mut reasons);
                ev.processes(&mut reasons);
                let active = !reasons.is_empty();
                // Enter immediately; leave only after 5 s without triggers (avoids flapping on alt-tab).
                let next = if active {
                    calm_since = None;
                    YieldState { active: true, reasons: reasons.into_iter().collect(), fullscreen_monitors: fs }
                } else if last.active {
                    let since = *calm_since.get_or_insert_with(std::time::Instant::now);
                    if since.elapsed() < Duration::from_secs(5) {
                        // Still calming down; re-check soon.
                        let _ = sender().lock().unwrap().as_ref().map(|t| {
                            let t = t.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(Duration::from_secs(5));
                                let _ = t.send(Trigger::Foreground);
                            })
                        });
                        continue;
                    }
                    YieldState::default()
                } else {
                    YieldState::default()
                };
                if next != last {
                    log::info!("yield state: active={} reasons={:?}", next.active, next.reasons);
                    on_change(&next);
                    last = next;
                }
            }
        })
        .expect("gamemode thread");
}
