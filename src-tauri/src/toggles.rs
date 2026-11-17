//! Quick Toggles and friends: brightness (DDC/CI on the main monitor only),
//! Wi-Fi and Bluetooth radios, Keep Awake, desktop icons, Do Not Disturb, camera access.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Registry::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// ---------------------------------------------------------------- registry helpers
fn reg_dword(path: &str, name: &str) -> Option<u32> {
    let mut v = 0u32;
    let mut n = 4u32;
    unsafe { RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(path), &HSTRING::from(name), RRF_RT_REG_DWORD, None, Some(&mut v as *mut _ as _), Some(&mut n)).ok().ok()? };
    Some(v)
}
fn set_reg_dword(path: &str, name: &str, v: u32) -> Result<(), String> {
    unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(path), &HSTRING::from(name), REG_DWORD.0, Some(&v as *const _ as _), 4).ok().map_err(|e| e.to_string()) }
}
fn set_reg_sz(path: &str, name: &str, s: &str) -> Result<(), String> {
    let w: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(path), &HSTRING::from(name), REG_SZ.0, Some(w.as_ptr() as _), (w.len() * 2) as u32).ok().map_err(|e| e.to_string()) }
}

// ---------------------------------------------------------------- brightness (DDC/CI)
fn with_main_monitor<T>(f: impl FnOnce(windows::Win32::Foundation::HANDLE) -> Result<T, String>) -> Result<T, String> {
    use windows::Win32::Devices::Display::*;
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTOPRIMARY};
    unsafe {
        let hm = MonitorFromPoint(windows::Win32::Foundation::POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut n = 0u32;
        GetNumberOfPhysicalMonitorsFromHMONITOR(hm, &mut n).map_err(|e| e.to_string())?;
        if n == 0 { return Err("no monitor".into()) }
        let mut mons = vec![PHYSICAL_MONITOR::default(); n as usize];
        GetPhysicalMonitorsFromHMONITOR(hm, &mut mons).map_err(|e| e.to_string())?;
        let r = f(mons[0].hPhysicalMonitor);
        let _ = DestroyPhysicalMonitors(&mons);
        r
    }
}

pub fn brightness_get() -> Result<u32, String> {
    use windows::Win32::Devices::Display::GetVCPFeatureAndVCPFeatureReply;
    with_main_monitor(|h| unsafe {
        let (mut cur, mut max) = (0u32, 0u32);
        if GetVCPFeatureAndVCPFeatureReply(h, 0x10, None, &mut cur, Some(&mut max)) == 0 {
            return Err("This monitor doesn't allow brightness control (DDC/CI is off in its menu)".into());
        }
        Ok(if max > 0 { cur * 100 / max } else { cur })
    })
}

pub fn brightness_set(v: u32) -> Result<(), String> {
    use windows::Win32::Devices::Display::SetVCPFeature;
    with_main_monitor(|h| unsafe {
        if SetVCPFeature(h, 0x10, v.min(100)) == 0 { Err("The monitor refused the change".into()) } else { Ok(()) }
    })
}

// ---------------------------------------------------------------- radios
fn radio(kind: windows::Devices::Radios::RadioKind) -> Option<windows::Devices::Radios::Radio> {
    use windows::Devices::Radios::Radio;
    let _ = Radio::RequestAccessAsync().ok()?.join();
    let list = Radio::GetRadiosAsync().ok()?.join().ok()?;
    (0..list.Size().ok()?).filter_map(|i| list.GetAt(i).ok()).find(|r| r.Kind().ok() == Some(kind))
}

// ---------------------------------------------------------------- keep awake
static AWAKE: AtomicBool = AtomicBool::new(false);
static AWAKE_THREAD: std::sync::Once = std::sync::Once::new();
fn keep_awake(on: bool) {
    use windows::Win32::System::Power::*;
    AWAKE.store(on, Ordering::Relaxed);
    AWAKE_THREAD.call_once(|| {
        std::thread::spawn(|| loop {
            unsafe {
                SetThreadExecutionState(if AWAKE.load(Ordering::Relaxed) { ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED } else { ES_CONTINUOUS });
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        });
    });
}

// ---------------------------------------------------------------- desktop icons
fn defview() -> Option<(HWND, HWND)> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), PCWSTR::null()).ok()?;
        let mut dv = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), PCWSTR::null()).ok();
        if dv.is_none() {
            let mut worker: Option<HWND> = None;
            while let Ok(wk) = FindWindowExW(None, worker, w!("WorkerW"), PCWSTR::null()) {
                if let Ok(d) = FindWindowExW(Some(wk), None, w!("SHELLDLL_DefView"), PCWSTR::null()) { dv = Some(d); break }
                worker = Some(wk);
            }
        }
        let dv = dv?;
        let list = FindWindowExW(Some(dv), None, w!("SysListView32"), PCWSTR::null()).ok()?;
        Some((dv, list))
    }
}

// ---------------------------------------------------------------- public: states + set
const NOTIF: &str = r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings";
const CAMERA: &str = r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\webcam";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toggle { pub id: &'static str, pub on: bool, pub available: bool }

pub fn states() -> Vec<Toggle> {
    use windows::Devices::Radios::{RadioKind, RadioState};
    let r = |k| radio(k).map(|r| r.State().ok() == Some(RadioState::On));
    let wifi = r(RadioKind::WiFi);
    let bt = r(RadioKind::Bluetooth);
    let icons = defview().map(|(_, l)| unsafe { IsWindowVisible(l).as_bool() });
    let camera_on = crate::util::reg_string(HKEY_CURRENT_USER, CAMERA, "Value").map(|v| v != "Deny");
    vec![
        Toggle { id: "wifi", on: wifi.unwrap_or(false), available: wifi.is_some() },
        Toggle { id: "bluetooth", on: bt.unwrap_or(false), available: bt.is_some() },
        Toggle { id: "dnd", on: reg_dword(NOTIF, "NOC_GLOBAL_SETTING_TOASTS_ENABLED") == Some(0), available: true },
        Toggle { id: "microphone", on: !crate::system::misc::mic_muted().unwrap_or(false), available: true },
        Toggle { id: "camera", on: camera_on.unwrap_or(true), available: true },
        Toggle { id: "keep-awake", on: AWAKE.load(Ordering::Relaxed), available: true },
        Toggle { id: "desktop-icons", on: icons.unwrap_or(true), available: icons.is_some() },
    ]
}

pub fn set(id: &str, on: bool) -> Result<(), String> {
    use windows::Devices::Radios::{RadioKind, RadioState};
    match id {
        "wifi" | "bluetooth" => {
            let r = radio(if id == "wifi" { RadioKind::WiFi } else { RadioKind::Bluetooth }).ok_or("not available on this PC")?;
            let res = r.SetStateAsync(if on { RadioState::On } else { RadioState::Off }).map_err(|e| e.to_string())?.join().map_err(|e| e.to_string())?;
            if res != windows::Devices::Radios::RadioAccessStatus::Allowed { return Err("Windows didn't allow the change".into()) }
            Ok(())
        }
        "dnd" => {
            set_reg_dword(NOTIF, "NOC_GLOBAL_SETTING_TOASTS_ENABLED", if on { 0 } else { 1 })?;
            unsafe { let _ = SendNotifyMessageW(HWND_BROADCAST, WM_SETTINGCHANGE, WPARAM(0), LPARAM(w!("Notifications").as_ptr() as isize)); }
            Ok(())
        }
        "microphone" => crate::system::misc::set_mic_muted(!on),
        "camera" => set_reg_sz(CAMERA, "Value", if on { "Allow" } else { "Deny" }),
        "keep-awake" => { keep_awake(on); Ok(()) }
        "desktop-icons" => {
            let (dv, list) = defview().ok_or("desktop not found")?;
            unsafe {
                if IsWindowVisible(list).as_bool() != on { let _ = SendMessageW(dv, WM_COMMAND, Some(WPARAM(0x7402)), Some(LPARAM(0))); }
            }
            Ok(())
        }
        _ => Err("unknown toggle".into()),
    }
}

// ---------------------------------------------------------------- Flow Launcher toggle
/// Opens Flow Launcher's search box, or hides it if it is already showing (the dock never takes focus,
/// so clicking the tile again doesn't make Flow lose focus by itself).
pub fn flow_toggle() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let mut found: Option<HWND> = None;
    for w in crate::system::winlist::all_top_level() {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(w, Some(&mut pid)) };
        if unsafe { IsWindowVisible(w).as_bool() } && crate::shell::gamemode::process_name(pid).eq_ignore_ascii_case("Flow.Launcher.exe") {
            let mut r = windows::Win32::Foundation::RECT::default();
            unsafe { let _ = GetWindowRect(w, &mut r); }
            if r.right - r.left > 200 { found = Some(w); break }
        }
    }
    if let Some(w) = found {
        unsafe {
            let _ = SetForegroundWindow(w);
            let k = |up: bool| INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_ESCAPE, dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, ..Default::default() } } };
            SendInput(&[k(false), k(true)], std::mem::size_of::<INPUT>() as i32);
        }
        return Ok(());
    }
    let p = std::env::var("LOCALAPPDATA").unwrap_or_default() + r"\FlowLauncher\Flow.Launcher.exe";
    crate::system::launch(&p, None)
}

// ---------------------------------------------------------------- software brightness / warmth
// For monitors without DDC/CI (or with it off): the main monitor's colour ramp, like f.lux. Monitor
// settings are untouched; Strata puts the original ramp back when it exits or when reset.
static ORIGINAL: std::sync::Mutex<Option<Vec<u16>>> = std::sync::Mutex::new(None);

fn main_dc() -> Option<windows::Win32::Graphics::Gdi::HDC> {
    use windows::Win32::Graphics::Gdi::*;
    unsafe {
        let hm = MonitorFromPoint(windows::Win32::Foundation::POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFOEXW::default();
        mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if !GetMonitorInfoW(hm, &mut mi as *mut _ as *mut MONITORINFO).as_bool() { return None }
        let dc = CreateDCW(w!("DISPLAY"), PCWSTR(mi.szDevice.as_ptr()), PCWSTR::null(), None);
        (!dc.is_invalid()).then_some(dc)
    }
}

pub fn gamma_set(brightness: u32, warmth: u32) -> Result<(), String> {
    use windows::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};
    let dc = main_dc().ok_or("main monitor not found")?;
    unsafe {
        let mut g = ORIGINAL.lock().unwrap();
        if g.is_none() {
            let mut cur = vec![0u16; 768];
            if GetDeviceGammaRamp(dc, cur.as_mut_ptr() as _).as_bool() { *g = Some(cur) }
        }
        let b = (brightness.clamp(40, 100) as f32) / 100.0;
        let w = (warmth.min(100) as f32) / 100.0;
        let (kr, kg, kb) = (1.0, 1.0 - 0.22 * w, 1.0 - 0.55 * w);
        let mut ramp = vec![0u16; 768];
        for i in 0..256 {
            let v = i as f32 * 257.0 * b;
            ramp[i] = (v * kr).min(65535.0) as u16;
            ramp[256 + i] = (v * kg).min(65535.0) as u16;
            ramp[512 + i] = (v * kb).min(65535.0) as u16;
        }
        let ok = SetDeviceGammaRamp(dc, ramp.as_ptr() as _).as_bool();
        let _ = windows::Win32::Graphics::Gdi::DeleteDC(dc);
        if ok { Ok(()) } else { Err("Windows refused that level (it limits how far the colour ramp may go, and HDR ignores it)".into()) }
    }
}

pub fn gamma_reset() {
    use windows::Win32::UI::ColorSystem::SetDeviceGammaRamp;
    let Some(orig) = ORIGINAL.lock().unwrap().take() else { return };
    if let Some(dc) = main_dc() {
        unsafe {
            let _ = SetDeviceGammaRamp(dc, orig.as_ptr() as _);
            let _ = windows::Win32::Graphics::Gdi::DeleteDC(dc);
        }
    }
}

// ---------------------------------------------------------------- start with Windows
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// Registers (or removes) Strata in the user's Run key, which is what Task Manager → Startup apps lists.
pub fn set_autostart(on: bool) -> Result<(), String> {
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        set_reg_sz(RUN, "Strata", &format!("\"{}\"", exe.display()))
    } else {
        unsafe { let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(RUN), w!("Strata")); }
        Ok(())
    }
}
pub fn autostart() -> bool { crate::util::reg_string(HKEY_CURRENT_USER, RUN, "Strata").is_some() }
