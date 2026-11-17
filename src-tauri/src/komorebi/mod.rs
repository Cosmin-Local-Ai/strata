//! Komorebi integration (brief §5).
//!
//! * Live state through komorebi's subscription mechanism: we host a named pipe and ask komorebi to
//!   connect to it (`komorebic subscribe-pipe <name>`); every event arrives as `{event, state}` JSON.
//!   No polling of `komorebic state`.
//! * Reconnects when komorebi restarts (pipe breaks → offline → wait for komorebi.exe → resubscribe).
//! * Re-applies Strata's ignore rule on every (re)connect: runtime rules die with komorebi.
//! * Commands go through `komorebic` with an allow-list of subcommands.

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::process::{Command, Output};
use std::time::Duration;
use windows::core::HSTRING;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_IO_PENDING, ERROR_PIPE_CONNECTED, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{ReadFile, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_INBOUND};
use windows::Win32::System::IO::{CancelIo, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Pipes::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const PIPE_NAME: &str = "strata-komorebi";

/// Run `komorebic` with arguments, without flashing a console window.
pub fn komorebic(args: &[&str]) -> std::io::Result<Output> {
    Command::new("komorebic").args(args).creation_flags(CREATE_NO_WINDOW).output()
}

/// The ignore rule is registered for the komorebi instance currently running. Cleared when the
/// subscription breaks (komorebi stopped/restarted), so a new instance gets the rule again.
static RULE_SET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Make sure komorebi never tiles, borders, animates or cloaks any Strata surface (D-004).
/// Sent once per komorebi instance: re-sending makes komorebi retile, which visibly moves windows.
pub fn ensure_ignore_rules() {
    use std::sync::atomic::Ordering;
    if RULE_SET.load(Ordering::SeqCst) {
        return;
    }
    match komorebic(&["ignore-rule", "exe", "strata.exe"]) {
        Ok(o) if o.status.success() => {
            RULE_SET.store(true, Ordering::SeqCst);
            log::info!("komorebi ignore rule registered")
        }
        Ok(o) => log::warn!("komorebic ignore-rule failed: {}", String::from_utf8_lossy(&o.stderr)),
        Err(e) => log::info!("komorebic not available: {e}"),
    }
}

/// Subcommands the UI may run. Anything else is refused.
const ALLOWED: &[&str] = &[
    "focus-monitor-workspace", "focus-workspace", "cycle-workspace", "cycle-monitor", "focus-named-workspace",
    "change-layout", "cycle-layout", "flip-layout", "toggle-monocle", "toggle-maximize", "toggle-float",
    "toggle-pause", "toggle-tiling", "retile", "cycle-stack", "focus-stack-window", "workspace-name",
    "workspace-layout", "move-to-monitor-workspace", "send-to-monitor-workspace", "move-to-workspace",
    "send-to-workspace", "focus", "move", "stack", "unstack", "start", "stop", "close-workspace", "new-workspace",
];

/// Run a komorebic command asynchronously (never blocks the UI).
pub fn run(args: Vec<String>) -> Result<(), String> {
    let Some(sub) = args.first() else { return Err("empty command".into()) };
    if !ALLOWED.contains(&sub.as_str()) {
        return Err(format!("komorebic {sub} is not allowed from the dock"));
    }
    std::thread::spawn(move || {
        let a: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        match komorebic(&a) {
            Ok(o) if !o.status.success() => log::warn!("komorebic {:?} failed: {}", a, String::from_utf8_lossy(&o.stderr).trim()),
            Err(e) => log::warn!("komorebic {:?}: {e}", a),
            _ => {}
        }
    });
    Ok(())
}

/// Start / stop / restart komorebi (+ whkd).
pub fn control(action: &str) -> Result<(), String> {
    let args: Vec<String> = match action {
        "start" => vec!["start".into(), "--whkd".into()],
        "stop" => vec!["stop".into(), "--whkd".into()],
        "restart" => {
            std::thread::spawn(|| {
                let _ = komorebic(&["stop", "--whkd"]);
                std::thread::sleep(Duration::from_millis(800));
                let _ = komorebic(&["start", "--whkd"]);
            });
            return Ok(());
        }
        _ => return Err(format!("unknown action {action}")),
    };
    run(args)
}

// ---------------------------------------------------------------------------------------------
// State reduced to what the dock needs. Emitted only when it changes.

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KWindow {
    pub hwnd: i64,
    pub exe: String,
    pub title: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KWorkspace {
    pub index: usize,
    pub name: String,
    pub layout: String,
    pub monocle: bool,
    pub maximized: bool,
    pub windows: Vec<KWindow>,
    pub floating: usize,
    /// Windows in the focused container when it is a stack (more than one window).
    pub stack: Vec<KWindow>,
    pub stack_focused: usize,
    pub flip: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KMonitor {
    pub index: usize,
    /// HMONITOR as reported by komorebi; matches the dock's display.
    pub hmonitor: i64,
    pub name: String,
    pub focused_workspace: usize,
    pub workspaces: Vec<KWorkspace>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KState {
    pub online: bool,
    pub paused: bool,
    pub focused_monitor: usize,
    pub monitors: Vec<KMonitor>,
    pub focused_window: Option<KWindow>,
}

fn layout_name(v: &serde_json::Value) -> String {
    if let Some(d) = v.get("Default").and_then(|d| d.as_str()) {
        return d.to_string();
    }
    if v.get("Custom").is_some() {
        return "Custom".into();
    }
    v.as_str().unwrap_or("Unknown").to_string()
}

fn window_of(v: &serde_json::Value) -> KWindow {
    KWindow {
        hwnd: v.get("hwnd").and_then(|h| h.as_i64()).unwrap_or(0),
        exe: v.get("exe").and_then(|s| s.as_str()).unwrap_or("").to_string(),
        title: v.get("title").and_then(|s| s.as_str()).unwrap_or("").to_string(),
    }
}

fn ring<'a>(v: &'a serde_json::Value, key: &str) -> (Vec<&'a serde_json::Value>, usize) {
    let r = &v[key];
    let el = r["elements"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    (el, r["focused"].as_u64().unwrap_or(0) as usize)
}

pub fn reduce(state: &serde_json::Value) -> KState {
    let (mons, fm) = ring(state, "monitors");
    let mut out = KState { online: true, paused: state["is_paused"].as_bool().unwrap_or(false), focused_monitor: fm, ..Default::default() };
    for (mi, m) in mons.iter().enumerate() {
        let (wss, fw) = ring(m, "workspaces");
        let mut km = KMonitor {
            index: mi,
            hmonitor: m["id"].as_i64().unwrap_or(0),
            name: m["name"].as_str().unwrap_or("").to_string(),
            focused_workspace: fw,
            workspaces: vec![],
        };
        for (wi, w) in wss.iter().enumerate() {
            let (containers, fc) = ring(w, "containers");
            let mut windows = vec![];
            let mut stack = vec![];
            let mut stack_focused = 0;
            for (ci, c) in containers.iter().enumerate() {
                let (ws, fwi) = ring(c, "windows");
                for x in &ws {
                    windows.push(window_of(x));
                }
                if ci == fc && ws.len() > 1 {
                    stack = ws.iter().map(|x| window_of(x)).collect();
                    stack_focused = fwi;
                }
            }
            let monocle = !w["monocle_container"].is_null();
            if monocle {
                let (ws, _) = ring(&w["monocle_container"], "windows");
                windows.extend(ws.iter().map(|x| window_of(x)));
            }
            let maximized = !w["maximized_window"].is_null();
            if maximized {
                windows.push(window_of(&w["maximized_window"]));
            }
            let floating = w["floating_windows"]["elements"].as_array().map(|a| a.len()).or_else(|| w["floating_windows"].as_array().map(|a| a.len())).unwrap_or(0);
            if wi == fw && mi == fm {
                if let Some(c) = containers.get(fc) {
                    let (ws, fwi) = ring(c, "windows");
                    out.focused_window = ws.get(fwi).map(|x| window_of(x));
                }
            }
            km.workspaces.push(KWorkspace {
                index: wi,
                name: w["name"].as_str().map(|s| s.to_string()).unwrap_or_else(|| format!("{}", wi + 1)),
                layout: layout_name(&w["layout"]),
                monocle,
                maximized,
                windows,
                floating,
                stack,
                stack_focused,
                flip: w["layout_flip"].as_str().map(|s| s.to_string()),
            });
        }
        out.monitors.push(km);
    }
    out
}

/// Komorebi's current work area for a monitor (HMONITOR), as (top, bottom) in physical px.
/// None when komorebi isn't running or doesn't know the monitor. Note: komorebi reports
/// work_area_size as {left, top, right: width, bottom: height}.
pub fn work_area(hmonitor: i64) -> Option<(i32, i32)> {
    if !crate::util::process_running(&["komorebi.exe"]) {
        return None;
    }
    let o = komorebic(&["state"]).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).ok()?;
    let mons = v["monitors"]["elements"].as_array()?;
    let m = mons.iter().find(|m| m["id"].as_i64() == Some(hmonitor))?;
    let w = &m["work_area_size"];
    let top = w["top"].as_i64()? as i32;
    Some((top, top + w["bottom"].as_i64()? as i32))
}

// ---------------------------------------------------------------------------------------------
// Subscription thread.

struct Pipe(HANDLE);
impl Drop for Pipe {
    fn drop(&mut self) {
        unsafe {
            let _ = DisconnectNamedPipe(self.0);
            let _ = CloseHandle(self.0);
        }
    }
}

fn create_pipe() -> windows::core::Result<Pipe> {
    let name = HSTRING::from(format!(r"\\.\pipe\{PIPE_NAME}"));
    let h = unsafe {
        CreateNamedPipeW(&name, PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED, PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT, 1, 0, 1 << 20, 0, None)
    };
    if h.is_invalid() {
        return Err(windows::core::Error::from_thread());
    }
    Ok(Pipe(h))
}

/// Overlapped wait for a client, with a timeout so a missing komorebi never blocks us forever.
fn wait_connect(p: &Pipe, timeout_ms: u32) -> bool {
    unsafe {
        let Ok(ev) = CreateEventW(None, true, false, None) else { return false };
        let mut ov = OVERLAPPED { hEvent: ev, ..Default::default() };
        let ok = match ConnectNamedPipe(p.0, Some(&mut ov)) {
            Ok(()) => true,
            Err(_) => {
                let e = GetLastError();
                if e == ERROR_PIPE_CONNECTED {
                    true
                } else if e == ERROR_IO_PENDING {
                    if WaitForSingleObject(ev, timeout_ms) == WAIT_OBJECT_0 {
                        let mut n = 0u32;
                        GetOverlappedResult(p.0, &ov, &mut n, false).is_ok()
                    } else {
                        let _ = CancelIo(p.0);
                        false
                    }
                } else {
                    false
                }
            }
        };
        let _ = CloseHandle(ev);
        ok
    }
}

/// Blocking read of the next chunk (overlapped + wait). None = pipe closed.
fn read_chunk(p: &Pipe, buf: &mut [u8]) -> Option<usize> {
    unsafe {
        let ev = CreateEventW(None, true, false, None).ok()?;
        let mut ov = OVERLAPPED { hEvent: ev, ..Default::default() };
        let mut n = 0u32;
        let r = match ReadFile(p.0, Some(buf), None, Some(&mut ov)) {
            Ok(()) => GetOverlappedResult(p.0, &ov, &mut n, true).ok().map(|_| n as usize),
            Err(_) if GetLastError() == ERROR_IO_PENDING => GetOverlappedResult(p.0, &ov, &mut n, true).ok().map(|_| n as usize),
            Err(_) => None,
        };
        let _ = CloseHandle(ev);
        r.filter(|&n| n > 0)
    }
}

/// Start the subscription thread. `emit` receives every changed state (including offline).
pub fn start(emit: impl Fn(&KState) + Send + 'static) {
    std::thread::Builder::new()
        .name("strata-komorebi".into())
        .spawn(move || {
            let mut last = KState::default();
            let mut announce = |s: KState, last: &mut KState| {
                if &s != last {
                    emit(&s);
                    *last = s;
                }
            };
            loop {
                if !crate::util::process_running(&["komorebi.exe"]) {
                    announce(KState::default(), &mut last);
                    std::thread::sleep(Duration::from_secs(3));
                    continue;
                }
                let pipe = match create_pipe() {
                    Ok(p) => p,
                    Err(e) => {
                        log::warn!("komorebi pipe: {e}");
                        std::thread::sleep(Duration::from_secs(3));
                        continue;
                    }
                };
                // Ask komorebi to connect to us.
                std::thread::spawn(|| {
                    let _ = komorebic(&["subscribe-pipe", PIPE_NAME]);
                });
                if !wait_connect(&pipe, 5000) {
                    log::info!("komorebi did not connect to the pipe; retrying");
                    drop(pipe);
                    std::thread::sleep(Duration::from_secs(3));
                    continue;
                }
                log::info!("komorebi connected");
                ensure_ignore_rules();
                // komorebi only sends on events; fetch the current state once so the dock is
                // correct immediately.
                if let Ok(o) = komorebic(&["state"]) {
                    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                        announce(reduce(&v), &mut last);
                    }
                }
                let mut acc: Vec<u8> = Vec::with_capacity(1 << 16);
                let mut buf = vec![0u8; 1 << 16];
                while let Some(n) = read_chunk(&pipe, &mut buf) {
                    acc.extend_from_slice(&buf[..n]);
                    // Messages are JSON objects back to back (newline separated in practice).
                    let mut consumed = 0;
                    {
                        let mut it = serde_json::Deserializer::from_slice(&acc).into_iter::<serde_json::Value>();
                        while let Some(Ok(v)) = it.next() {
                            consumed = it.byte_offset();
                            if let Some(st) = v.get("state") {
                                announce(reduce(st), &mut last);
                            }
                        }
                    }
                    acc.drain(..consumed);
                    if acc.len() > (8 << 20) {
                        acc.clear(); // never let a malformed stream grow unbounded
                    }
                }
                log::warn!("komorebi disconnected");
                RULE_SET.store(false, std::sync::atomic::Ordering::SeqCst);
                announce(KState::default(), &mut last);
                drop(pipe);
                std::thread::sleep(Duration::from_secs(2));
            }
        })
        .expect("komorebi thread");
}
