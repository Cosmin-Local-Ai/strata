//! Cards and menus: one reusable popup window (label "card") that opens over the tile that asked for it
//! (brief §4.4: "opens its card positioned over that widget, not mid-dock"; must fit small screens).
//!
//! Flow: dock UI → `card_open(uid, widget, anchor)` → card window gets `card:open` → it renders, measures
//! its content and calls `card_ready(w, h)` → we size, place and show it. Focus loss or Esc closes it.
//! The window is created once and kept hidden between uses, so opening is instant.

use crate::config::Edge;
use crate::shell::winstyle;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
use windows::Win32::Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW};

pub const LABEL: &str = "card";

/// Anchor rectangle in the dock window's CSS px.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Anchor {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardOpen {
    pub uid: String,
    pub widget: String,
    /// "card" or "menu"
    pub kind: String,
    pub payload: serde_json::Value,
    pub dock: String,
    pub edge: Edge,
    pub light: bool,
    /// Dock style id (cards follow it: black, white, midnight, glass...).
    pub style: String,
}

struct Open {
    uid: String,
    kind: String,
    dock: String,
    /// Anchor in screen physical px.
    anchor: RECT,
    edge: Edge,
    scale: f64,
}

#[derive(Default)]
pub struct Cards {
    open: Mutex<Option<Open>>,
    /// Do not close on focus loss (an app's own menu, opened from the tray card, has focus).
    pub sticky: std::sync::atomic::AtomicBool,
    /// Last card closed by focus loss, and when: a click on the same tile right after is a close.
    blur_closed: Mutex<Option<(String, std::time::Instant)>>,
    /// Overlay kind waiting for the overlay page (it may still be loading when first opened).
    pub overlay_pending: Mutex<Option<String>>,
}

fn ensure_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = app.get_webview_window(LABEL) {
        return Ok(w);
    }
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html#card".into()))
        .title("Strata card")
        .transparent(true)
        .decorations(false)
        .shadow(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .focused(false)
        .inner_size(360.0, 240.0)
        .effects(tauri::window::EffectsBuilder::new().effect(tauri::window::Effect::Acrylic).color(tauri::window::Color(24, 24, 28, 140)).build())
        .build()?;
    let hwnd = HWND(w.hwnd()?.0 as _);
    winstyle::apply(hwnd, winstyle::ShellStyle { no_activate: false });
    unsafe {
        let pref = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &pref as *const _ as _, std::mem::size_of_val(&pref) as u32);
    }
    Ok(w)
}

pub const OVERLAY: &str = "overlay";
/// The power panel has its own window with a blur created once (switching effects at runtime flickered).
pub const POWER: &str = "power";

fn overlay_label(kind: &str) -> &'static str {
    if kind == "power" { POWER } else { OVERLAY }
}

/// Full-screen panel (power menu, region capture).
fn ensure_overlay(app: &AppHandle, kind: &str) -> tauri::Result<WebviewWindow> {
    let label = overlay_label(kind);
    if let Some(w) = app.get_webview_window(label) {
        return Ok(w);
    }
    let mut b = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html#overlay".into()));
    let w = b
        .title("Strata panel")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .focused(false)
        .build()?;
    winstyle::apply(HWND(w.hwnd()?.0 as _), winstyle::ShellStyle { no_activate: false });
    if kind == "power" && app.state::<crate::AppState>().store.get().power_backdrop != "dim" {
        // Blurred desktop (see accent_blur).
        winstyle::accent_blur(HWND(w.hwnd()?.0 as _), 0x01202020);
    }
    Ok(w)
}

impl Cards {
    pub fn overlay_open(&self, app: &AppHandle, dock: &WebviewWindow, kind: &str) -> tauri::Result<()> {
        self.hide(app);
        let w = ensure_overlay(app, kind)?;
        let hwnd = HWND(w.hwnd()?.0 as _);
        let mon = unsafe {
            let hm = windows::Win32::Graphics::Gdi::MonitorFromWindow(HWND(dock.hwnd()?.0 as _), MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            let _ = GetMonitorInfoW(hm, &mut mi);
            // The Widgets picker leaves the dock visible and usable (badges, drag to reorder): it
            // covers only the work area. Other panels (power) cover the whole monitor.
            if kind == "widgets" { mi.rcWork } else { mi.rcMonitor }
        };
        *self.overlay_pending.lock().unwrap() = Some(kind.to_string());
        w.emit_to(overlay_label(kind), "overlay:open", serde_json::json!({ "kind": kind }))?;
        // Shown cloaked; overlay_show() uncloaks once the page has painted (fallback after 400 ms).
        winstyle::cloak(hwnd, true);
        let _ = w.set_size(tauri::PhysicalSize::new((mon.right - mon.left) as u32, (mon.bottom - mon.top) as u32));
        set_client_rect(hwnd, mon.left, mon.top, mon.right - mon.left, mon.bottom - mon.top, true);
        let _ = app.emit("picker:state", kind == "widgets");
        winstyle::uncloak_after(hwnd, 400);
        if kind == "power" && app.state::<crate::AppState>().store.get().power_backdrop != "dim" {
            // Re-applied on every show.
            winstyle::accent_blur(hwnd, 0x01202020);
        }
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
        let _ = w.set_focus();
        Ok(())
    }

    pub fn overlay_show(&self, app: &AppHandle) {
        let Some(kind) = self.overlay_pending.lock().unwrap().clone() else { return };
        let Some(w) = app.get_webview_window(overlay_label(&kind)) else { return };
        if let Ok(h) = w.hwnd() {
            winstyle::cloak(HWND(h.0 as _), false);
        }
    }

    pub fn overlay_close(&self, app: &AppHandle) {
        for label in [OVERLAY, POWER] {
        if let Some(w) = app.get_webview_window(label) {
            let was = self.overlay_pending.lock().unwrap().take();
            if was.as_deref() == Some("widgets") {
                let _ = app.emit("picker:state", false);
            }
            if let Ok(h) = w.hwnd() {
                unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(HWND(h.0 as _), windows::Win32::UI::WindowsAndMessaging::SW_HIDE); }
            }
            {
                let _ = w.hide();
                let _ = w.emit_to(label, "overlay:close", ());
            }
        }
        }
    }

    /// Kind of the open overlay, if any.
    pub fn overlay_kind(&self) -> Option<String> {
        self.overlay_pending.lock().unwrap().clone()
    }
}

/// Invisible border between the outer window rect and the client area (left, top).
fn border_offsets(hwnd: HWND) -> (i32, i32) {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    unsafe {
        let mut wr = RECT::default();
        let mut origin = POINT::default();
        let _ = GetWindowRect(hwnd, &mut wr);
        let _ = ClientToScreen(hwnd, &mut origin);
        (origin.x - wr.left, origin.y - wr.top)
    }
}

/// Pointer is over an open hover preview (keeps it open). Reset whenever a card closes.
pub static PREVIEW_HOVER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Place a window so its *client area* covers (x, y, w, h). Windows with a DWM shadow keep invisible
/// resize borders, so the outer rect must be grown by the border sizes.
pub fn set_client_rect(hwnd: HWND, x: i32, y: i32, w: i32, h: i32, show: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};
    unsafe {
        let mut wr = RECT::default();
        let mut cr = RECT::default();
        let mut origin = POINT::default();
        let _ = GetWindowRect(hwnd, &mut wr);
        let _ = GetClientRect(hwnd, &mut cr);
        let _ = ClientToScreen(hwnd, &mut origin);
        let left = origin.x - wr.left;
        let top = origin.y - wr.top;
        let right = (wr.right - wr.left) - (cr.right - cr.left) - left;
        let bottom = (wr.bottom - wr.top) - (cr.bottom - cr.top) - top;
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x - left, y - top, w + left + right, h + top + bottom, if show { SWP_SHOWWINDOW | SWP_NOACTIVATE } else { SWP_NOACTIVATE });
    }
}

impl Cards {
    /// Pre-create the hidden card window so the first open is instant.
    pub fn warm(&self, app: &AppHandle) {
        if let Err(e) = ensure_window(app) {
            log::warn!("card window: {e}");
        }
    }

    /// Open (or toggle closed) a card for `uid` anchored to a tile of dock window `dock`.
    pub fn open(&self, app: &AppHandle, dock: &WebviewWindow, req: CardOpen, anchor: Anchor, scale: f64) -> tauri::Result<()> {
        {
            let mut g = self.open.lock().unwrap();
            if let Some((uid, at)) = self.blur_closed.lock().unwrap().take() {
                if uid == req.uid && at.elapsed() < std::time::Duration::from_millis(400) {
                    return Ok(()); // this click is what took focus away: treat as toggle-close
                }
            }
            if let Some(o) = g.as_ref() {
                if o.uid == req.uid {
                    *g = None;
                    drop(g);
                    self.hide(app);
                    return Ok(());
                }
            }
        }
        let w = ensure_window(app)?;
        let dock_hwnd = HWND(dock.hwnd()?.0 as _);
        let mut origin = POINT::default();
        unsafe {
            let _ = ClientToScreen(dock_hwnd, &mut origin);
        }
        let a = RECT {
            left: origin.x + (anchor.x * scale).round() as i32,
            top: origin.y + (anchor.y * scale).round() as i32,
            right: origin.x + ((anchor.x + anchor.w) * scale).round() as i32,
            bottom: origin.y + ((anchor.y + anchor.h) * scale).round() as i32,
        };
        *self.open.lock().unwrap() = Some(Open { uid: req.uid.clone(), kind: req.kind.clone(), dock: req.dock.clone(), anchor: a, edge: req.edge, scale });
        w.emit_to(LABEL, "card:open", &req)?;
        Ok(())
    }

    /// The card measured its content (CSS px): size, place, show and focus it.
    pub fn ready(&self, app: &AppHandle, width: f64, height: f64) -> tauri::Result<()> {
        let Some(w) = app.get_webview_window(LABEL) else { return Ok(()) };
        let g = self.open.lock().unwrap();
        let Some(o) = g.as_ref() else { return Ok(()) };
        let s = o.scale;
        let gap = (10.0 * s).round() as i32;
        let margin = (8.0 * s).round() as i32;
        let cx = (o.anchor.left + o.anchor.right) / 2;
        let mon = unsafe {
            let hm = MonitorFromPoint(POINT { x: cx, y: (o.anchor.top + o.anchor.bottom) / 2 }, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            let _ = GetMonitorInfoW(hm, &mut mi);
            mi.rcMonitor
        };
        // Fit small screens: never larger than the space between the dock and the far edge.
        let avail_h = match o.edge {
            Edge::Bottom => o.anchor.top - gap - mon.top - margin,
            Edge::Top => mon.bottom - margin - (o.anchor.bottom + gap),
        };
        let pw = ((width * s).round() as i32).min(mon.right - mon.left - 2 * margin);
        let ph = ((height * s).round() as i32).min(avail_h.max(100));
        let x = (cx - pw / 2).clamp(mon.left + margin, mon.right - margin - pw);
        let y = match o.edge {
            Edge::Bottom => o.anchor.top - gap - ph,
            Edge::Top => o.anchor.bottom + gap,
        };
        let hwnd = HWND(w.hwnd()?.0 as _);
        // Shown cloaked, uncloaked a moment later: the first visible frame is the new content.
        let was_visible = w.hwnd().map(|h| unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(HWND(h.0 as _)).as_bool() }).unwrap_or(false);
        if !was_visible {
            winstyle::cloak(hwnd, true);
        }
        // Size and place only through tao, so the WebView's bounds always follow the window (a raw
        // SetWindowPos could leave the WebView at its old, smaller size: card drawn only partly).
        let (bl, bt) = border_offsets(hwnd);
        let _ = w.set_size(tauri::PhysicalSize::new(pw.max(1) as u32, ph.max(1) as u32));
        let _ = w.set_position(tauri::PhysicalPosition::new(x - bl, y - bt));
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE, SWP_NOMOVE, SWP_NOSIZE};
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
        if !was_visible {
            winstyle::uncloak_after(hwnd, 40);
        }
        if o.kind == "preview" {
            let uid = o.uid.clone();
            let app2 = app.clone();
            let card_raw = hwnd.0 as isize;
            std::thread::spawn(move || {
                use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect, WindowFromPoint, GetAncestor, GA_ROOT};
                let mut away = 0;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    let st = app2.state::<crate::AppState>();
                    if st.cards.open_uid().as_deref() != Some(uid.as_str()) { return; }
                    let over = unsafe {
                        let mut p = POINT::default();
                        let _ = GetCursorPos(&mut p);
                        let mut r = RECT::default();
                        let _ = GetWindowRect(HWND(card_raw as _), &mut r);
                        let in_card = p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom;
                        let root = GetAncestor(WindowFromPoint(p), GA_ROOT);
                        let in_dock = st.docks.labels().iter().any(|l| st.docks.hwnd_for(l).map(|h| h == root).unwrap_or(false));
                        in_card || in_dock
                    };
                    away = if over { 0 } else { away + 1 };
                    if away >= 2 { st.cards.hide(&app2); return; }
                }
            });
        }
        // Hover previews must not take focus (the app you're about to switch to keeps it).
        if o.kind != "preview" {
            unsafe {
                let _ = SetForegroundWindow(hwnd);
            }
            let _ = w.set_focus();
        }
        let _ = app.emit("card:shown", serde_json::json!({ "uid": o.uid, "dock": o.dock }));
        Ok(())
    }

    /// Closed because the card lost focus (remember it for the toggle check).
    pub fn hide_on_blur(&self, app: &AppHandle) {
        if self.sticky.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        if let Some(o) = self.open.lock().unwrap().as_ref() {
            *self.blur_closed.lock().unwrap() = Some((o.uid.clone(), std::time::Instant::now()));
        }
        self.hide(app);
    }

    pub fn hide(&self, app: &AppHandle) {
        self.sticky.store(false, std::sync::atomic::Ordering::SeqCst);
        crate::system::previews::clear();
        PREVIEW_HOVER.store(false, std::sync::atomic::Ordering::SeqCst);
        let was = self.open.lock().unwrap().take();
        if let Some(w) = app.get_webview_window(LABEL) {
            // We show it with SetWindowPos, so tao believes it is hidden and its hide() would no-op.
            if let Ok(h) = w.hwnd() {
                unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(HWND(h.0 as _), windows::Win32::UI::WindowsAndMessaging::SW_HIDE); }
            }
            let _ = w.hide();
            let _ = w.emit_to(LABEL, "card:close", ());
        }
        if let Some(o) = was {
            let _ = app.emit("card:closed", serde_json::json!({ "uid": o.uid, "dock": o.dock }));
        }
    }

    /// Uid of the open card, if any.
    pub fn open_uid(&self) -> Option<String> {
        self.open.lock().unwrap().as_ref().map(|o| o.uid.clone())
    }

    pub fn is_open(&self) -> bool {
        self.open.lock().unwrap().is_some()
    }
}
