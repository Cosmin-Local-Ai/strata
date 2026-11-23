//! Small system features behind single-purpose tiles: Recycle Bin, light/dark theme, microphone mute,
//! Show Desktop, Notification Center, On-Screen Keyboard, keyboard layout, battery.

use serde::Serialize;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Shell::{SHEmptyRecycleBinW, SHQueryRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHQUERYRBINFO};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bin {
    pub items: i64,
    pub bytes: i64,
}

pub fn recycle_bin() -> Bin {
    unsafe {
        let mut i = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, ..Default::default() };
        let _ = SHQueryRecycleBinW(PCWSTR::null(), &mut i);
        Bin { items: i.i64NumItems, bytes: i.i64Size }
    }
}

pub fn empty_recycle_bin() -> Result<(), String> {
    unsafe { SHEmptyRecycleBinW(None, PCWSTR::null(), SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND).map_err(|e| e.to_string()) }
}

const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

pub fn is_light() -> bool {
    use windows::Win32::System::Registry::*;
    unsafe {
        let mut v = 1u32;
        let mut n = 4u32;
        let _ = RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(PERSONALIZE), w!("AppsUseLightTheme"), RRF_RT_REG_DWORD, None, Some(&mut v as *mut _ as _), Some(&mut n));
        v != 0
    }
}

/// Switch Windows apps and system to light or dark (the same two values Settings > Colours writes).
pub fn set_light(light: bool) -> Result<(), String> {
    use windows::Win32::System::Registry::*;
    unsafe {
        let v: u32 = light as u32;
        for name in [w!("AppsUseLightTheme"), w!("SystemUsesLightTheme")] {
            RegSetKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(PERSONALIZE), name, REG_DWORD.0, Some(&v as *const _ as _), 4).ok().map_err(|e| e.to_string())?;
        }
        let topic = HSTRING::from("ImmersiveColorSet");
        let _ = SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, WPARAM(0), LPARAM(topic.as_ptr() as isize), SMTO_ABORTIFHUNG, 200, None);
    }
    Ok(())
}

/// Every active microphone with its device id. Apps record from the device *they* picked (Discord,
/// Teams, games), not necessarily the Windows default, so mute acts on all of them.
fn mics() -> Vec<(String, windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume)> {
    use windows::Win32::Media::Audio::*;
    let mut out = vec![];
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return out };
        let Ok(list) = en.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) else { return out };
        for i in 0..list.GetCount().unwrap_or(0) {
            let Ok(dev) = list.Item(i) else { continue };
            let id = dev.GetId().ok().and_then(|p| { let s = p.to_string().ok(); CoTaskMemFree(Some(p.0 as _)); s }).unwrap_or_default();
            if let Ok(v) = dev.Activate(CLSCTX_ALL, None) { out.push((id, v)) }
        }
    }
    out
}

/// Microphones Strata muted, so unmuting leaves the ones you had muted yourself alone.
static MUTED_BY_US: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// None = no microphone; muted = every microphone is muted.
pub fn mic_muted() -> Option<bool> {
    let m = mics();
    if m.is_empty() { return None }
    Some(m.iter().all(|(_, v)| unsafe { v.GetMute().map(|b| b.as_bool()).unwrap_or(false) }))
}

pub fn set_mic_muted(muted: bool) -> Result<(), String> {
    let m = mics();
    if m.is_empty() { return Err("No microphone found".into()) }
    let mut ours = MUTED_BY_US.lock().unwrap();
    let mut err = None;
    for (id, v) in &m {
        let now = unsafe { v.GetMute().map(|b| b.as_bool()).unwrap_or(false) };
        if muted && !now {
            match unsafe { v.SetMute(true, std::ptr::null()) } { Ok(()) => ours.push(id.clone()), Err(e) => err = Some(e.to_string()) }
        } else if !muted && now && (ours.is_empty() || ours.contains(id)) {
            if let Err(e) = unsafe { v.SetMute(false, std::ptr::null()) } { err = Some(e.to_string()) }
        }
    }
    if !muted { ours.clear() }
    drop(ours);
    match err { Some(e) if mic_muted() != Some(muted) => Err(e), _ => Ok(()) }
}

fn keys(vks: &[VIRTUAL_KEY]) {
    let mut inputs: Vec<INPUT> = vec![];
    let k = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() }, ..Default::default() } },
    };
    for &v in vks { inputs.push(k(v, false)); }
    for &v in vks.iter().rev() { inputs.push(k(v, true)); }
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32); }
}

/// Show Desktop that komorebi keeps up with.
///
/// What goes wrong otherwise (reproduced 17 Nov): komorebi's transparency adds WS_EX_LAYERED to
/// every unfocused window it manages. Minimising makes komorebi drop the window from its layout
/// (normal). On restore komorebi re-checks it, sees the layered flag *it* added, and refuses it
/// unless the app is on its layered allow-list, so the window ends up floating and unmanaged until
/// it's reopened. So: remember what komorebi managed, minimise normally, and on restore (1) remove
/// the layered flag where komorebi's own transparency put it, (2) restore one window at a time,
/// (3) force-manage any window komorebi still missed with its own `manage` command, then put the
/// original window back in front and retile.
struct Desk { hwnds: Vec<isize>, fg: isize, managed: Vec<isize> }
static DESK: std::sync::Mutex<Option<Desk>> = std::sync::Mutex::new(None);

/// hwnds komorebi currently manages (all monitors and workspaces).
fn komorebi_managed() -> Vec<isize> {
    let Ok(o) = crate::komorebi::komorebic(&["state"]) else { return vec![] };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) else { return vec![] };
    let mut out = vec![];
    fn walk(v: &serde_json::Value, out: &mut Vec<isize>) {
        match v {
            serde_json::Value::Object(m) => {
                if let (Some(h), Some(_)) = (m.get("hwnd").and_then(|x| x.as_i64()), m.get("exe")) { out.push(h as isize) }
                for x in m.values() { walk(x, out) }
            }
            serde_json::Value::Array(a) => for x in a { walk(x, out) },
            _ => {}
        }
    }
    if let Some(ms) = v.get("monitors") { walk(ms, &mut out) }
    out.sort_unstable();
    out.dedup();
    out
}

/// komorebi's transparency alpha and the apps it accepts as layered anyway.
fn komorebi_transparency() -> Option<(u8, Vec<String>)> {
    let o = crate::komorebi::komorebic(&["global-state"]).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).ok()?;
    if v["transparency_enabled"].as_bool() != Some(true) { return None }
    let alpha = v["transparency_alpha"].as_u64().unwrap_or(255) as u8;
    let mut allow = vec![];
    fn ids(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(m) => if m.get("kind").and_then(|k| k.as_str()) == Some("Exe") { if let Some(i) = m.get("id").and_then(|i| i.as_str()) { out.push(i.to_ascii_lowercase()) } },
            serde_json::Value::Array(a) => for x in a { ids(x, out) },
            _ => {}
        }
    }
    ids(&v["layered_whitelist"], &mut allow);
    Some((alpha, allow))
}

/// Remove WS_EX_LAYERED if komorebi's transparency put it there (same alpha, app not allow-listed).
fn strip_komorebi_layer(hw: windows::Win32::Foundation::HWND, alpha: u8, allow: &[String]) {
    use windows::Win32::UI::WindowsAndMessaging::{GetLayeredWindowAttributes, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, LAYERED_WINDOW_ATTRIBUTES_FLAGS, WS_EX_LAYERED};
    unsafe {
        let ex = GetWindowLongPtrW(hw, GWL_EXSTYLE);
        if ex & WS_EX_LAYERED.0 as isize == 0 { return }
        let mut pid = 0u32;
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hw, Some(&mut pid));
        if allow.contains(&crate::shell::gamemode::process_name(pid).to_ascii_lowercase()) { return }
        let (mut key, mut a, mut flags) = (windows::Win32::Foundation::COLORREF(0), 0u8, LAYERED_WINDOW_ATTRIBUTES_FLAGS(0));
        if GetLayeredWindowAttributes(hw, Some(&mut key), Some(&mut a), Some(&mut flags)).is_ok() && flags.0 == 2 /* LWA_ALPHA */ && a == alpha {
            SetWindowLongPtrW(hw, GWL_EXSTYLE, ex & !(WS_EX_LAYERED.0 as isize));
        }
    }
}

fn restore_desktop(d: Desk) {
    log::info!("show-desktop: restoring {:?}", d.hwnds);
    let komorebi = crate::util::process_running(&["komorebi.exe"]);
    let transparency = if komorebi { komorebi_transparency() } else { None };
    for &h in d.hwnds.iter().rev() {
        let hw = windows::Win32::Foundation::HWND(h as _);
        unsafe {
            if IsWindow(Some(hw)).as_bool() && IsIconic(hw).as_bool() {
                if let Some((alpha, allow)) = &transparency { if d.managed.contains(&h) { strip_komorebi_layer(hw, *alpha, allow) } }
                let _ = ShowWindow(hw, SW_SHOWNOACTIVATE);
                std::thread::sleep(std::time::Duration::from_millis(60));
            }
        }
    }
    if komorebi {
        std::thread::sleep(std::time::Duration::from_millis(350));
        // Anything komorebi managed before but didn't take back: hand it over explicitly.
        let now = komorebi_managed();
        let missed: Vec<isize> = d.managed.iter().copied().filter(|h| d.hwnds.contains(h) && !now.contains(h)).collect();
        for &h in &missed {
            let hw = windows::Win32::Foundation::HWND(h as _);
            unsafe {
                if !IsWindow(Some(hw)).as_bool() || IsIconic(hw).as_bool() { continue }
                let _ = SetForegroundWindow(hw);
            }
            std::thread::sleep(std::time::Duration::from_millis(80));
            let _ = crate::komorebi::komorebic(&["manage"]);
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        if !missed.is_empty() { log::info!("show-desktop: komorebi missed {missed:?}; managed them explicitly") }
    }
    unsafe {
        let fg = windows::Win32::Foundation::HWND(d.fg as _);
        if d.fg != 0 && IsWindow(Some(fg)).as_bool() { let _ = SetForegroundWindow(fg); }
    }
    if komorebi {
        std::thread::sleep(std::time::Duration::from_millis(120));
        let _ = crate::komorebi::komorebic(&["retile"]);
    }
}

// Show Desktop minimises every window (like Win+D) and the second click restores them, also with
// komorebi running: you use it to see what komorebi does not tile (desktop, HYTE Nexus, widgets),
// which a switch to an empty komorebi workspace never showed (D-029, replaces D-028's workspace
// switch). Windows komorebi managed are handed back to it on restore (restore_desktop).
pub fn show_desktop() {
    let mut g = DESK.lock().unwrap();
    if let Some(d) = g.take() {
        drop(g);
        restore_desktop(d);
        return;
    }
    let managed = if crate::util::process_running(&["komorebi.exe"]) { komorebi_managed() } else { vec![] };
    let mut hwnds = vec![];
    let fg = unsafe { GetForegroundWindow().0 as isize };
    // winlist::enumerate() is in z-order (top first); cloaked = on another komorebi workspace.
    for w in crate::system::winlist::enumerate() {
        if !w.minimized && !w.cloaked {
            unsafe { let _ = ShowWindow(windows::Win32::Foundation::HWND(w.hwnd as _), SW_MINIMIZE); }
            hwnds.push(w.hwnd as isize);
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
    }
    log::info!("show-desktop: minimised {:?} (foreground {fg}, komorebi managed {:?})", hwnds, managed);
    *g = Some(Desk { hwnds: hwnds.clone(), fg, managed });
    drop(g);
    // If you bring one window back yourself (taskbar, Alt+Tab, Apps tile), forget the rest: they stay
    // minimised like normal, and the next click starts a fresh Show Desktop.
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(400));
        let mut g = DESK.lock().unwrap();
        let Some(d) = g.as_ref() else { return };
        if d.hwnds != hwnds { return }
        let back = d.hwnds.iter().any(|&h| unsafe {
            let hw = windows::Win32::Foundation::HWND(h as _);
            IsWindow(Some(hw)).as_bool() && !IsIconic(hw).as_bool()
        });
        if back { *g = None; return }
    });
}

/// Windows projection menu (Win+P): PC screen only / Duplicate / Extend / Second screen only.
pub fn project_menu() { keys(&[VK_LWIN, VIRTUAL_KEY(b'P' as u16)]); }
pub fn notification_center() { keys(&[VK_LWIN, VIRTUAL_KEY(b'N' as u16)]); }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub id: String,
    /// e.g. "en-US"
    pub locale: String,
    pub current: bool,
}

fn locale_of(hkl: isize) -> String {
    use windows::Win32::Globalization::LCIDToLocaleName;
    let lcid = (hkl as u32) & 0xffff;
    let mut buf = [0u16; 85];
    let n = unsafe { LCIDToLocaleName(lcid, Some(&mut buf), 0) };
    if n > 1 { String::from_utf16_lossy(&buf[..(n - 1) as usize]) } else { format!("{lcid:04x}") }
}

/// Keyboard layouts and which one the foreground app is using.
pub fn layouts() -> Vec<Layout> {
    unsafe {
        let fg = GetForegroundWindow();
        let tid = GetWindowThreadProcessId(fg, None);
        let cur = GetKeyboardLayout(tid).0 as isize;
        let n = GetKeyboardLayoutList(None);
        let mut list = vec![HKL::default(); n.max(0) as usize];
        GetKeyboardLayoutList(Some(&mut list));
        list.iter().map(|h| Layout { id: format!("{:x}", h.0 as isize), locale: locale_of(h.0 as isize), current: h.0 as isize == cur }).collect()
    }
}

/// Ask the foreground window to switch to a layout (the way the language bar does).
pub fn set_layout(id: &str) -> Result<(), String> {
    let v = isize::from_str_radix(id, 16).map_err(|_| "bad layout id")?;
    unsafe {
        let fg = GetForegroundWindow();
        PostMessageW(Some(fg), WM_INPUTLANGCHANGEREQUEST, WPARAM(0), LPARAM(v)).map_err(|e| e.to_string())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub present: bool,
    pub percent: u8,
    pub charging: bool,
    pub plugged: bool,
}

pub fn battery() -> Battery {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { let _ = GetSystemPowerStatus(&mut s); }
    Battery {
        present: s.BatteryFlag != 128 && s.BatteryFlag != 255,
        percent: if s.BatteryLifePercent == 255 { 0 } else { s.BatteryLifePercent },
        charging: s.BatteryFlag & 8 != 0,
        plugged: s.ACLineStatus == 1,
    }
}
