//! The notification window: a transparent, never-focused strip above the dock (below it when the dock
//! is at the top), at the right of the dock's monitor. The page (src/notices/Toasts.svelte) shows the
//! stack and reports its size; an empty stack hides the window.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const LABEL: &str = "toast";

/// Notifications that arrived before the page was ready (it collects them when it loads).
static PENDING: std::sync::Mutex<Vec<crate::notices::Toast>> = std::sync::Mutex::new(Vec::new());
/// 0 = no window, 1 = being created / page loading, 2 = page ready. One window, created once: its
/// label only becomes known when creation has finished, so asking for it isn't enough.
static STATE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub fn take_pending() -> Vec<crate::notices::Toast> {
    STATE.store(2, std::sync::atomic::Ordering::SeqCst);
    std::mem::take(&mut *PENDING.lock().unwrap())
}

/// Create the window ahead of time (hidden), so the first notification shows at once.
pub fn warm(app: &AppHandle) {
    if STATE.compare_exchange(0, 1, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst).is_ok() {
        if let Err(e) = create(app) { log::warn!("toast window: {e}"); STATE.store(0, std::sync::atomic::Ordering::SeqCst); }
    }
}

fn create(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html#toast".into()))
        .additional_browser_args(crate::BROWSER_ARGS)
        .title("Strata notifications")
        .transparent(true)
        .decorations(false)
        .shadow(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .inner_size(400.0, 10.0)
        // The same glass as the cards; the page tints it for the dock style.
        .effects(tauri::window::EffectsBuilder::new().effect(tauri::window::Effect::Acrylic).color(tauri::window::Color(24, 24, 28, 140)).build())
        .build()?;
    let hwnd = HWND(w.hwnd()?.0 as _);
    crate::shell::winstyle::apply(hwnd, crate::shell::winstyle::ShellStyle { no_activate: true });
    unsafe {
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
        let pref = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &pref as *const _ as _, std::mem::size_of_val(&pref) as u32);
    }
    Ok(w)
}

/// Show a notification (creates the window the first time; the page keeps a short queue until ready).
pub fn push(app: &AppHandle, t: crate::notices::Toast) {
    crate::notices::remember(&t);
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        if STATE.load(std::sync::atomic::Ordering::SeqCst) == 2 {
            let _ = h.emit_to(LABEL, "toast:push", t);
        } else {
            PENDING.lock().unwrap().push(t);
            warm(&h);
        }
    });
}

/// The page's content size in CSS px; 0 high = nothing to show.
pub fn layout(app: &AppHandle, width: f64, height: f64) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    let Ok(raw) = w.hwnd() else { return };
    let hwnd = HWND(raw.0 as _);
    if height < 1.0 {
        unsafe { let _ = ShowWindow(hwnd, SW_HIDE); }
        return;
    }
    let s = app.state::<crate::AppState>().store.get();
    unsafe {
        // The work area already leaves out the dock's reserved strip.
        let mon = MonitorFromPoint(windows::Win32::Foundation::POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(mon, &mut mi);
        let scale = w.scale_factor().unwrap_or(1.0);
        let (pw, ph) = ((width * scale).round() as i32, (height * scale).round() as i32);
        let gap = (10.0 * scale) as i32;
        let r = mi.rcWork;
        // At the right, like Windows' own notifications.
        let x = r.right - gap - pw;
        let y = if matches!(s.edge, crate::config::Edge::Top) { r.top + gap } else { r.bottom - gap - ph };
        let _ = w.set_size(tauri::PhysicalSize::new(pw.max(1) as u32, ph.max(1) as u32));
        let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}
