//! Small system features behind single-purpose tiles: Recycle Bin, light/dark theme, microphone mute,
//! Show Desktop, Notification Center, On-Screen Keyboard, keyboard layout, battery.

use serde::Serialize;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
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

fn mic() -> Option<windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume> {
    use windows::Win32::Media::Audio::*;
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let dev = en.GetDefaultAudioEndpoint(eCapture, eCommunications).or_else(|_| en.GetDefaultAudioEndpoint(eCapture, eConsole)).ok()?;
        dev.Activate(CLSCTX_ALL, None).ok()
    }
}

/// None = no microphone.
pub fn mic_muted() -> Option<bool> {
    mic().and_then(|m| unsafe { m.GetMute().ok().map(|b| b.as_bool()) })
}

pub fn set_mic_muted(muted: bool) -> Result<(), String> {
    let m = mic().ok_or("No microphone found")?;
    unsafe { m.SetMute(muted, std::ptr::null()).map_err(|e| e.to_string()) }
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

/// Show Desktop that komorebi keeps up with. Win+D hides windows in a way komorebi loses, and pausing
/// komorebi made it ignore the windows coming back. So: minimise normally (komorebi sees each one),
/// and on the second click restore the same windows one at a time, oldest first, pausing briefly so
/// komorebi registers every window, then put the focused one back in front and retile.
struct Desk { hwnds: Vec<isize>, fg: isize }
static DESK: std::sync::Mutex<Option<Desk>> = std::sync::Mutex::new(None);

fn restore_desktop(d: Desk) {
    for &h in d.hwnds.iter().rev() {
        let hw = windows::Win32::Foundation::HWND(h as _);
        unsafe {
            if IsWindow(Some(hw)).as_bool() && IsIconic(hw).as_bool() {
                let _ = ShowWindow(hw, SW_SHOWNOACTIVATE);
                std::thread::sleep(std::time::Duration::from_millis(45));
            }
        }
    }
    unsafe {
        let fg = windows::Win32::Foundation::HWND(d.fg as _);
        if d.fg != 0 && IsWindow(Some(fg)).as_bool() { let _ = SetForegroundWindow(fg); }
    }
    if crate::util::process_running(&["komorebi.exe"]) {
        std::thread::sleep(std::time::Duration::from_millis(150));
        let _ = crate::komorebi::komorebic(&["retile"]);
    }
}

pub fn show_desktop() {
    let mut g = DESK.lock().unwrap();
    if let Some(d) = g.take() {
        drop(g);
        restore_desktop(d);
        return;
    }
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
    *g = Some(Desk { hwnds: hwnds.clone(), fg });
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
