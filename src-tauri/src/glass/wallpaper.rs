//! Still-image background for the glass (decision D-003): find what the desktop shows behind the dock
//! and decode just the strip under the dock window, with no screen capture.
//!
//! Sources, in order:
//! 1. Wallpaper Engine, when it is running and has a wallpaper on this monitor: the item's preview
//!    image (read-only from WE's config.json, never written).
//! 2. The Windows wallpaper for this monitor via IDesktopWallpaper, honouring the fit mode.
//! 3. The desktop background colour.

use std::path::{Path, PathBuf};
use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Win32::Foundation::{GENERIC_READ, RECT};
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Shell::{DesktopWallpaper, IDesktopWallpaper, DESKTOP_WALLPAPER_POSITION, DWPOS_CENTER, DWPOS_FILL, DWPOS_FIT, DWPOS_SPAN, DWPOS_STRETCH, DWPOS_TILE};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fit {
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
    Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackgroundSpec {
    pub path: Option<PathBuf>,
    pub fit: Fit,
    /// 0x00BBGGRR like COLORREF
    pub color: u32,
    pub from_wallpaper_engine: bool,
}

/// BGRA (premultiplied) pixels exactly covering the dock window.
pub struct Strip {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

fn pwstr_to_string(p: windows::core::PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    let s = unsafe { p.to_string().unwrap_or_default() };
    unsafe { CoTaskMemFree(Some(p.0 as _)) };
    s
}

fn fit_from(pos: DESKTOP_WALLPAPER_POSITION) -> Fit {
    match pos {
        DWPOS_CENTER => Fit::Center,
        DWPOS_TILE => Fit::Tile,
        DWPOS_STRETCH => Fit::Stretch,
        DWPOS_FIT => Fit::Fit,
        DWPOS_SPAN => Fit::Span,
        DWPOS_FILL => Fit::Fill,
        _ => Fit::Fill,
    }
}

/// Index of `monitor_rect` among IDesktopWallpaper's monitors, plus its wallpaper and fit.
/// Must be called on a thread with COM initialised.
pub fn windows_wallpaper(monitor_rect: RECT) -> Option<(usize, BackgroundSpec)> {
    unsafe {
        let dw: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL).ok()?;
        let n = dw.GetMonitorDevicePathCount().ok()?;
        let fit = fit_from(dw.GetPosition().unwrap_or(DWPOS_FILL));
        let color = dw.GetBackgroundColor().map(|c| c.0).unwrap_or(0);
        for i in 0..n {
            let id = dw.GetMonitorDevicePathAt(i).ok()?;
            let id_s = pwstr_to_string(id);
            let hid = HSTRING::from(id_s.as_str());
            let Ok(r) = dw.GetMonitorRECT(PCWSTR(hid.as_ptr())) else { continue };
            if r == monitor_rect {
                let path = dw.GetWallpaper(PCWSTR(hid.as_ptr())).map(pwstr_to_string).unwrap_or_default();
                let path = if path.is_empty() { None } else { Some(PathBuf::from(path)) };
                return Some((i as usize, BackgroundSpec { path, fit, color, from_wallpaper_engine: false }));
            }
        }
        None
    }
}

/// Wallpaper Engine's current item preview for WE monitor index `index`, if WE is running.
/// The MonitorN ↔ display mapping is INFERRED (Monitor0 = primary on this PC, checked against the
/// visible wallpaper on 17 Nov 2026); we use IDesktopWallpaper's monitor order, which matches it here.
pub fn wallpaper_engine_preview(index: usize) -> Option<PathBuf> {
    if !crate::util::process_running(&["wallpaper64.exe", "wallpaper32.exe"]) {
        return None;
    }
    let cfg_path = crate::wallpaper_engine::config_path()?;
    let text = std::fs::read_to_string(cfg_path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let obj = v.as_object()?;
    for (_, user) in obj {
        let Some(sel) = user.pointer("/general/wallpaperconfig/selectedwallpapers") else { continue };
        let Some(file) = sel.get(format!("Monitor{index}")).and_then(|m| m.get("file")).and_then(|f| f.as_str()) else { continue };
        return crate::wallpaper_engine::preview_for(Path::new(file));
    }
    None
}

pub fn resolve(monitor_rect: RECT) -> BackgroundSpec {
    let win = windows_wallpaper(monitor_rect);
    let index = win.as_ref().map(|w| w.0).unwrap_or(0);
    if let Some(p) = wallpaper_engine_preview(index) {
        return BackgroundSpec { path: Some(p), fit: Fit::Fill, color: 0, from_wallpaper_engine: true };
    }
    win.map(|w| w.1).unwrap_or(BackgroundSpec { path: None, fit: Fit::Fill, color: 0, from_wallpaper_engine: false })
}

/// Decode the part of the background image that sits behind `window` (screen px) on `monitor`.
/// Must be called on a thread with COM initialised.
pub fn load_strip(spec: &BackgroundSpec, monitor: RECT, window: RECT) -> windows::core::Result<Strip> {
    let ww = (window.right - window.left).max(1) as u32;
    let wh = (window.bottom - window.top).max(1) as u32;
    let (r, g, b) = ((spec.color & 0xff) as u8, ((spec.color >> 8) & 0xff) as u8, ((spec.color >> 16) & 0xff) as u8);
    let mut pixels = vec![0u8; (ww * wh * 4) as usize];
    for px in pixels.chunks_exact_mut(4) {
        px.copy_from_slice(&[b, g, r, 255]);
    }
    let Some(path) = &spec.path else { return Ok(Strip { width: ww, height: wh, pixels }) };

    unsafe {
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let decoder = wic.CreateDecoderFromFilename(&HSTRING::from(path.as_os_str()), None, GENERIC_READ, WICDecodeMetadataCacheOnDemand)?;
        let frame = decoder.GetFrame(0)?;
        let (mut iw, mut ih) = (0u32, 0u32);
        frame.GetSize(&mut iw, &mut ih)?;
        if iw == 0 || ih == 0 {
            return Ok(Strip { width: ww, height: wh, pixels });
        }

        // Area the image is laid out in: the monitor, or the whole virtual screen for Span.
        let area = if spec.fit == Fit::Span {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            RECT { left: x, top: y, right: x + GetSystemMetrics(SM_CXVIRTUALSCREEN), bottom: y + GetSystemMetrics(SM_CYVIRTUALSCREEN) }
        } else {
            monitor
        };
        let aw = (area.right - area.left) as f64;
        let ah = (area.bottom - area.top) as f64;
        let (fw, fh) = (iw as f64, ih as f64);
        // Displayed image size and top-left, in screen px.
        let (dw, dh) = match spec.fit {
            Fit::Fill | Fit::Span => { let s = (aw / fw).max(ah / fh); (fw * s, fh * s) }
            Fit::Fit => { let s = (aw / fw).min(ah / fh); (fw * s, fh * s) }
            Fit::Stretch => (aw, ah),
            Fit::Center | Fit::Tile => (fw, fh), // Tile approximated by a single centred copy
        };
        let dx = area.left as f64 + (aw - dw) / 2.0;
        let dy = area.top as f64 + (ah - dh) / 2.0;

        // Intersection of the window with the displayed image, in screen px.
        let ix0 = (window.left as f64).max(dx);
        let iy0 = (window.top as f64).max(dy);
        let ix1 = (window.right as f64).min(dx + dw);
        let iy1 = (window.bottom as f64).min(dy + dh);
        if ix1 - ix0 < 1.0 || iy1 - iy0 < 1.0 {
            return Ok(Strip { width: ww, height: wh, pixels });
        }
        let sx = fw / dw;
        let sy = fh / dh;
        let src = WICRect {
            X: ((ix0 - dx) * sx).floor().max(0.0) as i32,
            Y: ((iy0 - dy) * sy).floor().max(0.0) as i32,
            Width: (((ix1 - ix0) * sx).ceil() as i32).clamp(1, iw as i32),
            Height: (((iy1 - iy0) * sy).ceil() as i32).clamp(1, ih as i32),
        };
        let src = WICRect { Width: src.Width.min(iw as i32 - src.X), Height: src.Height.min(ih as i32 - src.Y), ..src };
        let out_w = (ix1 - ix0).round().max(1.0) as u32;
        let out_h = (iy1 - iy0).round().max(1.0) as u32;

        let clipper = wic.CreateBitmapClipper()?;
        clipper.Initialize(&frame, &src)?;
        let scaler = wic.CreateBitmapScaler()?;
        scaler.Initialize(&clipper, out_w, out_h, WICBitmapInterpolationModeFant)?;
        let conv = wic.CreateFormatConverter()?;
        conv.Initialize(&scaler, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom)?;
        let stride = out_w * 4;
        let mut buf = vec![0u8; (stride * out_h) as usize];
        conv.cast::<IWICBitmapSource>()?.CopyPixels(std::ptr::null(), stride, &mut buf)?;

        let ox = (ix0 - window.left as f64).round() as u32;
        let oy = (iy0 - window.top as f64).round() as u32;
        for y in 0..out_h.min(wh.saturating_sub(oy)) {
            let w = out_w.min(ww.saturating_sub(ox)) as usize * 4;
            let s = (y * stride) as usize;
            let d = (((oy + y) * ww + ox) * 4) as usize;
            pixels[d..d + w].copy_from_slice(&buf[s..s + w]);
        }
    }
    Ok(Strip { width: ww, height: wh, pixels })
}
