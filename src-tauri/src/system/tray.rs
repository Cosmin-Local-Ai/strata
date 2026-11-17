//! Tray (notification area) icons via a tray hook derived from YASB (MIT, see tray/LICENSE-YASB):
//! (tray/strata_tray_hook.cpp, MIT) is injected into Explorer's tray thread with SetWindowsHookEx. It
//! subclasses Shell_TrayWnd and copies every Shell_NotifyIcon message (WM_COPYDATA, dwData = 1) to our
//! pipe, with the icon pre-rendered to RGBA, then lets Explorer handle it as usual, so the real tray
//! keeps working. A watchdog in the DLL waits on our liveness mutex: if Strata exits or crashes, the
//! DLL unsubclasses and unloads itself.
//!
//! Clicks are forwarded the way the taskbar does: AllowSetForegroundWindow(owner) then
//! SendNotifyMessage(hWnd, uCallbackMessage, ...) with NOTIFYICON_VERSION_4 semantics when the app
//! asked for them.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_IO_PENDING, ERROR_PIPE_CONNECTED, HANDLE, HWND, LPARAM, WAIT_OBJECT_0, WPARAM};
use windows::Win32::Storage::FileSystem::{ReadFile, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_INBOUND};
use windows::Win32::System::IO::{CancelIo, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Pipes::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE, PIPE_WAIT};
use windows::Win32::System::Threading::{CreateEventW, CreateMutexW, WaitForSingleObject};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayIcon {
    pub key: String,
    pub tooltip: String,
    /// data:image/png;base64,...
    pub icon: String,
    #[serde(skip)]
    pub hwnd: u32,
    #[serde(skip)]
    pub uid: u32,
    #[serde(skip)]
    pub callback: u32,
    #[serde(skip)]
    pub version: u32,
    pub hidden: bool,
    /// Process exe name (for stable identity / pinning later).
    pub exe: String,
    #[serde(skip)]
    order: u64,
}

fn icons() -> &'static Mutex<HashMap<String, TrayIcon>> {
    static I: OnceLock<Mutex<HashMap<String, TrayIcon>>> = OnceLock::new();
    I.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn list() -> Vec<TrayIcon> {
    let mut v: Vec<TrayIcon> = icons().lock().unwrap().values().filter(|i| !i.hidden).cloned().collect();
    v.sort_by_key(|i| i.order);
    v
}

/// Our own TaskbarCreated broadcast (to make apps re-add icons) must not look like an Explorer restart.
pub static SELF_BROADCAST_UNTIL: Mutex<Option<std::time::Instant>> = Mutex::new(None);

fn u32_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).unwrap_or(0)
}

fn wstr_at(b: &[u8], o: usize, chars: usize) -> String {
    let end = (o + chars * 2).min(b.len());
    if o >= end {
        return String::new();
    }
    let w: Vec<u16> = b[o..end].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
    String::from_utf16_lossy(&w)
}

fn png_data_url(rgba: &[u8], w: u32, h: u32) -> Option<String> {
    use windows::core::Interface;
    use windows::Win32::Graphics::Imaging::*;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, STREAM_SEEK_CUR};
    if w == 0 || h == 0 || rgba.len() < (w * h * 4) as usize {
        return None;
    }
    // The hook converts pixels to RGBA (straight alpha, top-down).
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let bmp = wic.CreateBitmapFromMemory(w, h, &GUID_WICPixelFormat32bppRGBA, w * 4, &rgba[..(w * h * 4) as usize]).ok()?;
        let mut buf = vec![0u8; (w * h * 4 + 4096) as usize];
        let stream = wic.CreateStream().ok()?;
        stream.InitializeFromMemory(&mut buf).ok()?;
        let enc = wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null()).ok()?;
        enc.Initialize(&stream, WICBitmapEncoderNoCache).ok()?;
        let mut fr = None;
        enc.CreateNewFrame(&mut fr, std::ptr::null_mut()).ok()?;
        let fr = fr?;
        fr.Initialize(None).ok()?;
        fr.SetSize(w, h).ok()?;
        let mut fmt = GUID_WICPixelFormat32bppBGRA;
        fr.SetPixelFormat(&mut fmt).ok()?;
        fr.WriteSource(&bmp.cast::<IWICBitmapSource>().ok()?, std::ptr::null()).ok()?;
        fr.Commit().ok()?;
        enc.Commit().ok()?;
        let mut pos = 0u64;
        stream.Seek(0, STREAM_SEEK_CUR, Some(&mut pos)).ok()?;
        buf.truncate(pos as usize);
        Some(format!("data:image/png;base64,{}", crate::system::media::b64_pub(&buf)))
    }
}

/// One message from the hook. Returns true when the icon list changed.
fn handle(msg: &[u8], counter: &mut u64) -> bool {
    if u32_at(msg, 0) != 2 || msg.len() < 28 {
        return false;
    }
    let dw_data = u64::from_le_bytes(msg[4..12].try_into().unwrap());
    let cb = u32_at(msg, 12) as usize;
    let (iw, ih, isz) = (u32_at(msg, 16), u32_at(msg, 20), u32_at(msg, 24) as usize);
    if dw_data != 1 || msg.len() < 28 + cb {
        return false;
    }
    let d = &msg[28..28 + cb];
    let icon_bytes = &msg[28 + cb..(28 + cb + isz).min(msg.len())];
    let message = u32_at(d, 4);
    let n = 8; // NOTIFYICONDATA32 starts after dwSignature + dwMessage
    let flags = u32_at(d, n + 12);
    let hwnd = u32_at(d, n + 4);
    let uid = u32_at(d, n + 8);
    // guidItem is at offset 936 of the (packed) NOTIFYICONDATA32.
    let key = if flags & 0x20 != 0 && d.len() >= n + 952 {
        format!("guid:{}", d[n + 936..n + 952].iter().map(|b| format!("{b:02x}")).collect::<String>())
    } else {
        format!("{hwnd:x}:{uid}")
    };
    let mut map = icons().lock().unwrap();
    match message {
        2 => map.remove(&key).is_some(), // NIM_DELETE
        0 | 1 => {
            // NIM_ADD / NIM_MODIFY
            *counter += 1;
            let order = *counter;
            let e = map.entry(key.clone()).or_insert_with(|| {
                let mut pid = 0u32;
                unsafe { GetWindowThreadProcessId(HWND(hwnd as usize as _), Some(&mut pid)) };
                TrayIcon { key: key.clone(), tooltip: String::new(), icon: String::new(), hwnd, uid, callback: 0, version: 0, hidden: false, exe: crate::shell::gamemode::process_name(pid), order }
            });
            e.hwnd = hwnd;
            e.uid = uid;
            if flags & 0x1 != 0 { e.callback = u32_at(d, n + 16); }
            if flags & 0x4 != 0 { e.tooltip = wstr_at(d, n + 24, 128); }
            if flags & 0x8 != 0 {
                let state = u32_at(d, n + 280);
                let mask = u32_at(d, n + 284);
                if mask & 1 != 0 { e.hidden = state & 1 != 0; }
            }
            if flags & 0x2 != 0 && isz > 0 {
                if let Some(url) = png_data_url(icon_bytes, iw, ih) { e.icon = url; }
            }
            true
        }
        4 => {
            // NIM_SETVERSION
            if let Some(e) = map.get_mut(&key) { e.version = u32_at(d, n + 800); }
            false
        }
        _ => false,
    }
}

fn dll_path() -> Option<std::path::PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    [dir.join("strata-tray-hook.dll"), dir.join("resources").join("strata-tray-hook.dll")].into_iter().find(|p| p.exists())
}

fn inject() -> Result<(), String> {
    let dll = dll_path().ok_or("strata-tray-hook.dll not found next to strata.exe")?;
    unsafe {
        let module = LoadLibraryW(&HSTRING::from(dll.as_os_str())).map_err(|e| e.to_string())?;
        let proc_ = GetProcAddress(module, windows::core::s!("GetMsgProc")).ok_or("GetMsgProc missing")?;
        let tray = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()).map_err(|_| "Explorer's tray window not found")?;
        let tid = GetWindowThreadProcessId(tray, None);
        let hook = SetWindowsHookExW(WH_GETMESSAGE, Some(std::mem::transmute(proc_)), Some(module.into()), tid).map_err(|e| e.to_string())?;
        // Make Explorer's tray thread process a message so the hook (and our DLL) loads.
        let _ = PostThreadMessageW(tid, WM_NULL, WPARAM(0), LPARAM(0));
        // The DLL keeps itself loaded (it adds its own reference); our hook can go after a moment.
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let _ = UnhookWindowsHookEx(hook);
    }
    Ok(())
}

fn broadcast_taskbar_created() {
    *SELF_BROADCAST_UNTIL.lock().unwrap() = Some(std::time::Instant::now() + std::time::Duration::from_secs(3));
    unsafe {
        let msg = RegisterWindowMessageW(w!("TaskbarCreated"));
        let _ = SendNotifyMessageW(HWND_BROADCAST, msg, WPARAM(0), LPARAM(0));
    }
}

/// Start hosting: liveness mutex, pipe server, injection, read loop (reinjects after Explorer restarts).
pub fn start(emit: impl Fn(Vec<TrayIcon>) + Send + 'static) {
    std::thread::Builder::new().name("strata-tray".into()).spawn(move || unsafe {
        // Held for Strata's lifetime; the DLL's watchdog unloads itself when it's abandoned.
        let _alive = CreateMutexW(None, true, w!("Local\\StrataTrayHookAlive"));
        let mut counter = 0u64;
        loop {
            let pipe = CreateNamedPipeW(w!("\\\\.\\pipe\\strata_tray_monitor"), PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT, 1, 0, 1 << 20, 0, None);
            if pipe.is_invalid() {
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
            let ev = CreateEventW(None, true, false, None).unwrap_or_default();
            let mut ov = OVERLAPPED { hEvent: ev, ..Default::default() };
            let pending = match ConnectNamedPipe(pipe, Some(&mut ov)) {
                Ok(()) => false,
                Err(_) => GetLastError() == ERROR_IO_PENDING,
            };
            if let Err(e) = inject() {
                log::warn!("tray: {e}");
            }
            let connected = if pending { WaitForSingleObject(ev, 6000) == WAIT_OBJECT_0 } else { GetLastError() == ERROR_PIPE_CONNECTED || !pending };
            if !connected {
                let _ = CancelIo(pipe);
                let _ = CloseHandle(ev);
                let _ = CloseHandle(pipe);
                log::warn!("tray: hook did not connect; retrying in 10 s");
                std::thread::sleep(std::time::Duration::from_secs(10));
                continue;
            }
            log::info!("tray: hook connected");
            icons().lock().unwrap().clear();
            broadcast_taskbar_created();
            let mut buf = vec![0u8; 1 << 20];
            let mut dirty = false;
            let mut last_emit = std::time::Instant::now();
            loop {
                let mut rov = OVERLAPPED { hEvent: ev, ..Default::default() };
                let mut n = 0u32;
                // Never cancel a read in flight: a message completing during CancelIo was lost (that is
                // how icons went missing during the TaskbarCreated burst). Instead keep waiting on the
                // same read, flushing batched changes every 150 ms while idle.
                let ok = match ReadFile(pipe, Some(&mut buf), None, Some(&mut rov)) {
                    Ok(()) => GetOverlappedResult(pipe, &rov, &mut n, true).is_ok(),
                    Err(_) if GetLastError() == ERROR_IO_PENDING => loop {
                        if WaitForSingleObject(ev, 150) == WAIT_OBJECT_0 {
                            break GetOverlappedResult(pipe, &rov, &mut n, false).is_ok();
                        }
                        if dirty {
                            emit(list());
                            dirty = false;
                            last_emit = std::time::Instant::now();
                        }
                    },
                    Err(_) => false,
                };
                if !ok {
                    break;
                }
                if n > 0 && handle(&buf[..n as usize], &mut counter) {
                    dirty = true;
                }
                if dirty && (n == 0 || last_emit.elapsed() > std::time::Duration::from_millis(150)) {
                    emit(list());
                    dirty = false;
                    last_emit = std::time::Instant::now();
                }
            }
            log::warn!("tray: hook disconnected (Explorer restarted?), reinjecting");
            let _ = DisconnectNamedPipe(pipe);
            let _ = CloseHandle(ev);
            let _ = CloseHandle(pipe);
            icons().lock().unwrap().clear();
            emit(Vec::new());
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }).expect("tray thread");
}

/// Forward a click to the icon's owner like the taskbar does. `x, y` = screen position (physical px).
pub fn click(key: &str, button: &str, x: i32, y: i32) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let i = icons().lock().unwrap().get(key).cloned().ok_or("icon is gone")?;
    if i.callback == 0 {
        return Err("this icon takes no clicks".into());
    }
    const NIN_SELECT: u32 = WM_USER;
    const NIN_KEYSELECT: u32 = WM_USER + 1;
    let _ = NIN_KEYSELECT;
    let v3 = i.version >= 3;
    let msgs: Vec<u32> = match button {
        "left" if v3 => vec![WM_LBUTTONDOWN, WM_LBUTTONUP, NIN_SELECT],
        "left" => vec![WM_LBUTTONDOWN, WM_LBUTTONUP],
        "double" => vec![WM_LBUTTONDBLCLK],
        "right" if v3 => vec![WM_RBUTTONDOWN, WM_RBUTTONUP, WM_CONTEXTMENU],
        "right" => vec![WM_RBUTTONDOWN, WM_RBUTTONUP],
        "middle" => vec![WM_MBUTTONDOWN, WM_MBUTTONUP],
        _ => return Err("unknown button".into()),
    };
    let _ = (x, y);
    let mut pt = windows::Win32::Foundation::POINT::default();
    unsafe { let _ = GetCursorPos(&mut pt); }
    unsafe {
        let hwnd = HWND(i.hwnd as usize as _);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let _ = AllowSetForegroundWindow(pid);
        for &m in &msgs {
            // Version 4: wParam = anchor point (cursor), lParam = (message, uID). Older: wParam = uID.
            let (wp, lp) = if i.version > 3 {
                (((pt.y as u32 & 0xffff) << 16 | (pt.x as u32 & 0xffff)) as usize, ((i.uid & 0xffff) << 16 | (m & 0xffff)) as isize)
            } else {
                (i.uid as usize, (m & 0xffff) as isize)
            };
            let _ = SendNotifyMessageW(hwnd, i.callback, WPARAM(wp), LPARAM(lp));
        }
    }
    Ok(())
}
