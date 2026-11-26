//! Dock windows: one per enabled display (D-008), placed along the chosen edge, with AppBar space
//! reservation, the glass layer, and the input region that keeps transparent headroom click-through.

use crate::config::{Edge, Settings, Store};
use crate::glass::{self, Glass, GlassRect};
use crate::shell::appbar::{self, ShellEvent};
use crate::shell::displays::{self, Display};
use crate::shell::winstyle;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW};

/// Geometry sent to the dock's web UI. Lengths in CSS (logical) px unless named `*Px`.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockLayout {
    pub display_id: String,
    pub display_name: String,
    /// HMONITOR, matches komorebi's monitor `id`.
    pub hmonitor: i64,
    pub edge: Edge,
    pub scale: f64,
    pub body_height: f64,
    pub edge_distance: f64,
    pub headroom: f64,
    pub padding: f64,
    pub spacing: f64,
    pub full_width: bool,
    pub max_width: f64,
    pub window_width: f64,
    pub window_height: f64,
}

struct DockInstance {
    label: String,
    hwnd: isize,
    display: Display,
    layout: DockLayout,
    glass: Option<Glass>,
    appbar: bool,
    /// Last applied (window rect, reserved thickness, edge): re-placing with identical geometry is
    /// skipped so komorebi doesn't retile and nothing flickers.
    placed: Option<(RECT, i32, Edge)>,
    /// Last input region, to skip redundant SetWindowRgn calls while hovering.
    region: Vec<(RECT, i32)>,
    /// The web UI has painted once; until then the window stays hidden (no empty first frame).
    ready: bool,
}

#[derive(Clone)]
pub struct Docks {
    inner: Arc<Mutex<HashMap<String, DockInstance>>>,
    app: AppHandle,
    store: Store,
    events: Sender<ShellEvent>,
}

pub fn compute_layout(d: &Display, s: &Settings) -> DockLayout {
    let scale = d.scale();
    let body = s.dock_height.clamp(24, 120) as f64;
    let dist = s.edge_distance.min(24) as f64;
    // Room above the body for magnified icons and hover names (click-through unless hovered).
    let headroom = (body * (s.magnification_scale.max(1.0) as f64 - 1.0) + 34.0).ceil();
    let mw = (d.rect[2] - d.rect[0]) as f64 / scale;
    DockLayout {
        display_id: d.id.clone(),
        display_name: d.name.clone(),
        hmonitor: d.hmonitor as i64,
        edge: s.edge,
        scale,
        body_height: body,
        edge_distance: dist,
        headroom,
        padding: s.padding.min(24) as f64,
        spacing: s.spacing.min(24) as f64,
        full_width: s.full_width,
        max_width: if s.full_width { mw - 2.0 * dist } else { (mw * s.max_width_fraction.clamp(0.2, 1.0) as f64).floor() },
        window_width: mw,
        window_height: body + dist + headroom,
    }
}

/// Is any ordinary visible window (not ours, not cloaked by komorebi, not minimised, not maximised
/// or fullscreen over the whole monitor) overlapping `strip`?
fn windows_overlap(strip: RECT, mon: RECT) -> bool {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsZoomed, GWL_EXSTYLE, WS_EX_TOOLWINDOW};
    struct Ctx { strip: RECT, mon: RECT, hit: bool }
    unsafe extern "system" fn cb(h: HWND, lp: LPARAM) -> windows::core::BOOL {
        let c = unsafe { &mut *(lp.0 as *mut Ctx) };
        unsafe {
            if !IsWindowVisible(h).as_bool() || IsIconic(h).as_bool() || IsZoomed(h).as_bool() { return true.into(); }
            if GetWindowLongW(h, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 { return true.into(); }
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, Some(&mut pid));
            if pid == std::process::id() { return true.into(); }
            let mut cloaked = 0u32;
            let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut _ as _, 4);
            if cloaked != 0 { return true.into(); }
            // Visible frame (GetWindowRect includes ~9 px of invisible resize border).
            let mut r = RECT::default();
            if DwmGetWindowAttribute(h, windows::Win32::Graphics::Dwm::DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as _, std::mem::size_of::<RECT>() as u32).is_err()
                && GetWindowRect(h, &mut r).is_err() { return true.into(); }
            let s = c.strip;
            // Ignore windows covering the entire monitor (fullscreen apps, the desktop).
            let m = c.mon;
            let full = r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom;
            if !full && r.left < s.right && r.right > s.left && r.top < s.bottom - 2 && r.bottom > s.top + 2 {
                c.hit = true;
                return false.into();
            }
        }
        true.into()
    }
    let mut c = Ctx { strip, mon, hit: false };
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut c as *mut _ as isize));
    }
    c.hit
}

/// Glass shader parameters per dock style (brief §4.2: Black, White, Liquid Glass, iOS Glass, Midnight,
/// Clear). Tuned by eye.
pub fn style_params(style: crate::config::DockStyle) -> glass::GlassStyle {
    use crate::config::DockStyle::*;
    let g = glass::GlassStyle::default();
    match style {
        LiquidGlass => g,
        IosGlass => glass::GlassStyle { tint: [1.0, 1.0, 1.0, 0.16], refraction: 0.25, blur_lod: 3.6, rim: 0.25, dispersion: 0.0, ..g },
        // Clear: you see straight through (no frost, a light edge bend), tiles have no background.
        Clear => glass::GlassStyle { tint: [1.0, 1.0, 1.0, 0.0], refraction: 0.12, blur_lod: 0.0, rim: 0.30, dispersion: 0.0, ..g },
        Black => glass::GlassStyle { tint: [0.0, 0.0, 0.0, 0.0], refraction: 0.0, blur_lod: 3.0, rim: 0.10, dispersion: 0.0, shade: [0.06, 0.06, 0.07, 0.93] },
        White => glass::GlassStyle { tint: [1.0, 1.0, 1.0, 0.0], refraction: 0.0, blur_lod: 3.0, rim: 0.15, dispersion: 0.0, shade: [0.96, 0.96, 0.97, 0.90] },
        Midnight => glass::GlassStyle { tint: [0.0, 0.0, 0.0, 0.0], refraction: 0.3, blur_lod: 3.2, rim: 0.22, dispersion: 0.0, shade: [0.05, 0.07, 0.16, 0.72] },
    }
}

impl Docks {
    pub fn new(app: AppHandle, store: Store, events: Sender<ShellEvent>) -> Self {
        Docks { inner: Arc::new(Mutex::new(HashMap::new())), app, store, events }
    }

    /// Create/remove/re-place docks to match current displays and settings. Main thread only.
    pub fn sync(&self) {
        let settings = self.store.get();
        let displays = displays::enumerate();
        for d in &displays {
            log::info!("display {} '{}' primary={} rect={:?} dpi={} dock={}", d.gdi_name, d.name, d.primary, d.rect, d.dpi, d.dock_enabled(&settings));
        }
        let wanted: Vec<&Display> = displays.iter().filter(|d| d.dock_enabled(&settings)).collect();
        // Remove docks whose display is gone or disabled.
        let stale: Vec<String> = {
            let g = self.inner.lock().unwrap();
            g.keys().filter(|id| !wanted.iter().any(|d| &d.id == *id)).cloned().collect()
        };
        for id in stale {
            self.destroy(&id);
        }
        for d in wanted {
            let exists = self.inner.lock().unwrap().contains_key(&d.id);
            if !exists {
                if let Err(e) = self.create(d, &settings) {
                    log::error!("could not create dock for {}: {e}", d.gdi_name);
                    continue;
                }
            }
            self.place(&d.id, d.clone(), &settings);
        }
        let style = style_params(settings.style);
        self.for_each_glass(|g| g.set_style(style));
    }

    /// Destroy and rebuild every dock (komorebi adopted one before its ignore rule existed; a new
    /// window is created under the rule). Main thread only.
    pub fn recreate(&self) {
        let ids: Vec<String> = self.inner.lock().unwrap().keys().cloned().collect();
        for id in ids {
            self.destroy(&id);
        }
        self.sync();
    }

    fn create(&self, d: &Display, s: &Settings) -> tauri::Result<()> {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let label = format!("dock-{}", NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        let layout = compute_layout(d, s);
        let w = WebviewWindowBuilder::new(&self.app, &label, WebviewUrl::App("index.html".into()))
            .additional_browser_args(crate::BROWSER_ARGS)
            .title("Strata")
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(false)
            .inner_size(layout.window_width, layout.window_height)
            .build()?;
        let hwnd = HWND(w.hwnd()?.0 as _);
        winstyle::apply(hwnd, winstyle::ShellStyle { no_activate: true });
        appbar::attach(hwnd, self.events.clone());
        let glass_mode = if s.live_glass { glass::Background::Live } else { glass::Background::Image };
        let glass = match Glass::new(hwnd, glass_mode) {
            Ok(g) => Some(g),
            Err(e) => {
                log::warn!("glass unavailable for {}: {e}", d.gdi_name);
                None
            }
        };
        self.inner.lock().unwrap().insert(d.id.clone(), DockInstance { label, hwnd: hwnd.0 as isize, display: d.clone(), layout, glass, appbar: false, placed: None, region: vec![], ready: false });
        Ok(())
    }

    fn destroy(&self, id: &str) {
        let inst = self.inner.lock().unwrap().remove(id);
        if let Some(inst) = inst {
            let hwnd = HWND(inst.hwnd as _);
            winstyle::unpin(hwnd);
            if inst.appbar {
                appbar::unregister(hwnd);
            }
            if let Some(w) = self.app.get_webview_window(&inst.label) {
                let _ = w.destroy();
            }
        }
    }

    /// Position the window, reserve AppBar space, refresh glass, tell the UI its layout.
    fn place(&self, id: &str, d: Display, s: &Settings) {
        let layout = compute_layout(&d, s);
        let scale = layout.scale;
        let win_h = (layout.window_height * scale).round() as i32;
        let mon = d.rect();
        let y = match layout.edge {
            Edge::Bottom => mon.bottom - win_h,
            Edge::Top => mon.top,
        };
        let target = RECT { left: mon.left, top: y, right: mon.right, bottom: y + win_h };
        let thickness = if s.auto_hide { 0 } else { ((layout.body_height + layout.edge_distance) * scale).round() as i32 };
        let (hwnd, label, glass, had_appbar, ready, unchanged) = {
            let mut g = self.inner.lock().unwrap();
            let Some(inst) = g.get_mut(id) else { return };
            let unchanged = inst.placed == Some((target, thickness, layout.edge)) && inst.appbar == !s.auto_hide;
            inst.display = d.clone();
            inst.layout = layout.clone();
            inst.placed = Some((target, thickness, layout.edge));
            (HWND(inst.hwnd as _), inst.label.clone(), inst.glass.clone(), inst.appbar, inst.ready, unchanged)
        };
        if unchanged {
            // Same window geometry, but padding / spacing / width settings may have changed: the UI
            // still needs the new layout (skipping this made those sliders do nothing).
            let _ = self.app.emit_to(label.as_str(), "dock:layout", layout);
            return;
        }
        if !ready {
            // Shown but cloaked until the web UI has painted (mark_ready), with a safety timeout.
            winstyle::cloak(hwnd, true);
            winstyle::uncloak_after(hwnd, 4000);
        }
        winstyle::pin(hwnd, RECT { bottom: target.top + win_h, ..target });
        unsafe {
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), target.left, target.top, target.right - target.left, win_h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        }
        // Reserve body + edge distance; the headroom above stays usable by other windows.
        if s.auto_hide {
            if had_appbar {
                appbar::unregister(hwnd);
                self.set_appbar(id, false);
            }
        } else {
            if !had_appbar {
                appbar::register(hwnd);
                self.set_appbar(id, true);
            }
            let granted = appbar::reserve(hwnd, mon, layout.edge, thickness);
            log::info!("appbar reserved {:?} on {}", granted, d.gdi_name);
            // Another AppBar on the same edge (e.g. another bar at the top) pushes our strip inward: put the
            // window where the shell actually granted space, not at the raw monitor edge.
            let want_y = match layout.edge { Edge::Top => granted.top, Edge::Bottom => granted.bottom - win_h };
            if want_y != target.top {
                winstyle::pin(hwnd, RECT { left: target.left, top: want_y, right: target.right, bottom: want_y + win_h });
                unsafe {
                    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), target.left, want_y, target.right - target.left, win_h, SWP_NOACTIVATE);
                }
                if let Some(inst) = self.inner.lock().unwrap().get_mut(id) {
                    inst.placed = Some((RECT { top: want_y, bottom: want_y + win_h, ..target }, thickness, layout.edge));
                }
            }

        }
        if let Some(g) = glass {
            g.geometry_changed();
        }
        let _ = self.app.emit_to(label.as_str(), "dock:layout", layout);
    }

    fn set_appbar(&self, id: &str, on: bool) {
        if let Some(inst) = self.inner.lock().unwrap().get_mut(id) {
            inst.appbar = on;
        }
    }

    pub fn layout_for(&self, label: &str) -> Option<DockLayout> {
        self.inner.lock().unwrap().values().find(|i| i.label == label).map(|i| i.layout.clone())
    }

    pub fn glass_for(&self, label: &str) -> Option<Glass> {
        self.inner.lock().unwrap().values().find(|i| i.label == label).and_then(|i| i.glass.clone())
    }

    pub fn hwnd_for(&self, label: &str) -> Option<HWND> {
        self.inner.lock().unwrap().values().find(|i| i.label == label).map(|i| HWND(i.hwnd as _))
    }

    pub fn for_each_glass(&self, f: impl Fn(&Glass)) {
        for i in self.inner.lock().unwrap().values() {
            if let Some(g) = &i.glass {
                f(g);
            }
        }
    }

    pub fn labels(&self) -> Vec<String> {
        self.inner.lock().unwrap().values().map(|i| i.label.clone()).collect()
    }

    /// Hide docks on monitors with a fullscreen window; show the others (D-011, brief §4.1).
    pub fn set_fullscreen_monitors(&self, monitors: &[isize]) {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};
        for i in self.inner.lock().unwrap().values() {
            let hide = monitors.contains(&i.display.hmonitor);
            unsafe {
                let _ = ShowWindow(HWND(i.hwnd as _), if hide { SW_HIDE } else { SW_SHOWNOACTIVATE });
            }
        }
    }

    /// Explorer restarted: every AppBar registration is gone and must be redone.
    pub fn forget_appbars(&self) {
        for i in self.inner.lock().unwrap().values_mut() {
            i.appbar = false;
            i.placed = None;
        }
    }

    /// Clean exit: give the space back.
    pub fn shutdown(&self) {
        let ids: Vec<String> = self.inner.lock().unwrap().keys().cloned().collect();
        for id in ids {
            self.destroy(&id);
        }
    }

    pub fn update_region(&self, label: &str, rects: Vec<(RECT, i32)>) {
        let h = {
            let mut g = self.inner.lock().unwrap();
            let Some(inst) = g.values_mut().find(|i| i.label == label) else { return };
            if inst.region == rects {
                return;
            }
            inst.region = rects.clone();
            HWND(inst.hwnd as _)
        };
        appbar::set_input_region(h, &rects);
    }

    /// First paint done: show the dock (it was positioned while hidden).
    pub fn mark_ready(&self, label: &str) {
        let h = {
            let mut g = self.inner.lock().unwrap();
            let Some(inst) = g.values_mut().find(|i| i.label == label) else { return };
            if inst.ready {
                return;
            }
            inst.ready = true;
            HWND(inst.hwnd as _)
        };
        // Reveal once no window overlaps the reserved strip any more (komorebi has finished moving
        // them), so the dock never appears on top of windows that are about to move. Checked every
        // 50 ms against real window positions, at most 1.5 s.
        let (strip, mon) = {
            let g = self.inner.lock().unwrap();
            let Some(inst) = g.values().find(|i| i.label == label) else { return };
            let m = inst.display.rect;
            let thick = inst.placed.map(|p| p.1).unwrap_or(0);
            let strip = match inst.layout.edge {
                Edge::Bottom => RECT { left: m[0], top: m[3] - thick, right: m[2], bottom: m[3] },
                Edge::Top => RECT { left: m[0], top: m[1], right: m[2], bottom: m[1] + thick },
            };
            (strip, inst.display.rect())
        };
        let raw = h.0 as isize;
        std::thread::spawn(move || {
            // New threads may not inherit per-monitor DPI awareness; without it window rects come back
            // scaled (virtualised) and never intersect the physical-pixel strip.
            unsafe {
                let _ = windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            }
            let t0 = std::time::Instant::now();
            // komorebi can move windows clear, then briefly back (stale retile), then clear again, so
            // require the strip to stay clear for 300 ms (6 checks), capped at 2.5 s.
            let mut waited = 0;
            let mut clear_run = 0;
            for _ in 0..50 {
                if windows_overlap(strip, mon) { clear_run = 0; } else { clear_run += 1; }
                if clear_run >= 6 {
                    break;
                }
                waited += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            log::info!("dock reveal: strip {:?}, waited {} checks ({} ms)", strip, waited, t0.elapsed().as_millis());
            winstyle::cloak(HWND(raw as _), false);
        });
    }

    pub fn set_glass_rect(&self, label: &str, r: GlassRect) {
        if let Some(g) = self.glass_for(label) {
            g.set_rect(r);
        }
    }
}
