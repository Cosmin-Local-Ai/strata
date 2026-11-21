//! Keep shell surfaces (dock, cards, previews) as non-activating tool popups.
//!
//! tao re-applies its own idea of the extended style on several state changes (show, focus,
//! always-on-top), which brings back WS_EX_APPWINDOW. Those make the window look like a
//! normal app window to the taskbar, Alt+Tab and komorebi (which then tiles it). We subclass the
//! window and rewrite every style change so the shell flags always win.

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::*;

const SUBCLASS_ID: usize = 0x5354_5241; // "STRA"

#[derive(Clone, Copy)]
pub struct ShellStyle {
    /// Never take focus when clicked (dock). Cards with text inputs set this to false.
    pub no_activate: bool,
}

/// tao owns the frame styles of its windows (it implements undecorated windows with its own
/// non-client handling), so we leave WS_STYLE alone; changing it leaves a stale caption strip.
fn fix_style(style: u32) -> u32 {
    style
}

fn fix_ex(ex: u32, s: ShellStyle) -> u32 {
    let mut ex = (ex & !(WS_EX_APPWINDOW.0 | WS_EX_WINDOWEDGE.0 | WS_EX_DLGMODALFRAME.0)) | WS_EX_TOOLWINDOW.0;
    if s.no_activate {
        ex |= WS_EX_NOACTIVATE.0;
    } else {
        ex &= !WS_EX_NOACTIVATE.0;
    }
    ex
}

unsafe extern "system" fn subclass_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, data: usize) -> LRESULT {
    unsafe { subclass_inner(hwnd, msg, wp, lp, data) }
}

unsafe fn subclass_inner(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, data: usize) -> LRESULT {
  unsafe {
    let s = ShellStyle { no_activate: data & 1 == 1 };
    match msg {
        WM_STYLECHANGING => {
            let ss = &mut *(lp.0 as *mut STYLESTRUCT);
            if wp.0 as i32 == GWL_STYLE.0 {
                ss.styleNew = fix_style(ss.styleNew);
            } else if wp.0 as i32 == GWL_EXSTYLE.0 {
                ss.styleNew = fix_ex(ss.styleNew, s);
            }
            return DefSubclassProc(hwnd, msg, wp, lp);
        }
        WM_WINDOWPOSCHANGING => {
            // A pinned window (the dock) only ever sits where Strata put it. Anything else trying to
            // move or resize it (a tiling manager that adopted it during a start-up race, Windows
            // cascading it to the centre) is overruled.
            let wp_ = &mut *(lp.0 as *mut WINDOWPOS);
            if let Some(r) = pinned(hwnd) {
                if wp_.flags.0 & SWP_NOMOVE.0 == 0 || wp_.flags.0 & SWP_NOSIZE.0 == 0 {
                    wp_.x = r.left;
                    wp_.y = r.top;
                    wp_.cx = r.right - r.left;
                    wp_.cy = r.bottom - r.top;
                }
            }
        }
        WM_MOUSEACTIVATE if s.no_activate => return LRESULT(MA_NOACTIVATE as isize),
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wp, lp)
  }
}

static PINS: std::sync::Mutex<Vec<(isize, windows::Win32::Foundation::RECT)>> = std::sync::Mutex::new(Vec::new());

fn pinned(hwnd: HWND) -> Option<windows::Win32::Foundation::RECT> {
    PINS.lock().ok()?.iter().find(|p| p.0 == hwnd.0 as isize).map(|p| p.1)
}

/// Pin a window to `r` (physical px). Call before Strata's own SetWindowPos so that move is allowed.
pub fn pin(hwnd: HWND, r: windows::Win32::Foundation::RECT) {
    let mut g = PINS.lock().unwrap();
    g.retain(|p| p.0 != hwnd.0 as isize);
    g.push((hwnd.0 as isize, r));
}

pub fn unpin(hwnd: HWND) {
    PINS.lock().unwrap().retain(|p| p.0 != hwnd.0 as isize);
}

pub fn apply(hwnd: HWND, s: ShellStyle) {
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, s.no_activate as usize);
        let st = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_STYLE, fix_style(st) as isize);
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, fix_ex(ex, s) as isize);
        let _ = SetWindowPos(hwnd, None, 0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
    }
}

/// DWM cloak: the window stays "shown" (so WebView2 keeps rendering) but DWM doesn't display it.
/// Used to show surfaces only once their first frame is ready, so nothing flashes empty or stale.
pub fn cloak(hwnd: HWND, on: bool) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
    let v: windows::core::BOOL = on.into();
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_CLOAK, &v as *const _ as _, std::mem::size_of_val(&v) as u32);
    }
}

/// Uncloak after `ms` on a helper thread (fallback when a surface never reports ready).
pub fn uncloak_after(hwnd: HWND, ms: u64) {
    let raw = hwnd.0 as isize;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        cloak(HWND(raw as _), false);
    });
}

/// Blur-behind via the (undocumented, long-stable) SetWindowCompositionAttribute accent policy, the
/// classic blur-behind way. Unlike the Windows 11 acrylic *system backdrop*, it renders the same
/// whether or not the window is active, so it never flashes to a grey fallback while focus settles.
pub fn accent_blur(hwnd: HWND, tint_abgr: u32) {
    #[repr(C)]
    struct AccentPolicy { state: u32, flags: u32, gradient: u32, animation: u32 }
    #[repr(C)]
    struct Data { attrib: u32, data: *mut core::ffi::c_void, size: usize }
    type Swca = unsafe extern "system" fn(HWND, *mut Data) -> i32;
    unsafe {
        let Ok(user32) = windows::Win32::System::LibraryLoader::GetModuleHandleW(windows::core::w!("user32.dll")) else { return };
        let Some(f) = windows::Win32::System::LibraryLoader::GetProcAddress(user32, windows::core::s!("SetWindowCompositionAttribute")) else { return };
        let f: Swca = std::mem::transmute(f);
        // ACCENT_ENABLE_BLURBEHIND = 3; WCA_ACCENT_POLICY = 19.
        let mut p = AccentPolicy { state: 3, flags: 0, gradient: tint_abgr, animation: 0 };
        let mut d = Data { attrib: 19, data: &mut p as *mut _ as _, size: std::mem::size_of::<AccentPolicy>() };
        f(hwnd, &mut d);
    }
}
