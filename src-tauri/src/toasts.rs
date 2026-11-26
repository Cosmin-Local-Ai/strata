//! The notification window: a transparent, never-focused strip above the dock (below it when the dock
//! is at the top), at the right of the dock's monitor. The page (src/notices/Toasts.svelte) shows the
//! stack and reports its size; an empty stack hides the window.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const LABEL: &str = "toast";

/// Notifications that arrived while the window was still loading (the page collects them).
static PENDING: std::sync::Mutex<Vec<crate::notices::Toast>> = std::sync::Mutex::new(Vec::new());
pub fn take_pending() -> Vec<crate::notices::Toast> { std::mem::take(&mut *PENDING.lock().unwrap()) }

fn ensure(app: &AppHandle) -> tauri::Result<(tauri::WebviewWindow, bool)> {
    if let Some(w) = app.get_webview_window(LABEL) { return Ok((w, false)) }
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html#toast".into()))
        .additional_browser_args(crate::BROWSER_ARGS)
        .title("Strata notifications")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .inner_size(400.0, 10.0)
        .build()?;
    let hwnd = HWND(w.hwnd()?.0 as _);
    crate::shell::winstyle::apply(hwnd, crate::shell::winstyle::ShellStyle { no_activate: true });
    Ok((w, true))
}

/// Show a notification (creates the window the first time; the page keeps a short queue until ready).
pub fn push(app: &AppHandle, t: crate::notices::Toast) {
    crate::notices::remember(&t);
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        match ensure(&h) {
            Ok((_, true)) => PENDING.lock().unwrap().push(t),
            Ok((_, false)) => { let _ = h.emit_to(LABEL, "toast:push", t); }
            Err(e) => log::warn!("toast window: {e}"),
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
