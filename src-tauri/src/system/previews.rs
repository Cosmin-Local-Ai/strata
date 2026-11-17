//! Live window previews (brief §4.3: hover previews, live, with title and close button).
//!
//! DWM thumbnails: Windows composes a live copy of the source window into the card window at the
//! rectangles the page reserved for them. Strata renders nothing for them, and they update live at no
//! cost to us. Registered when a preview card opens, unregistered when it closes.

use serde::Deserialize;
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, RECT, SIZE};
use windows::Win32::Graphics::Dwm::{
    DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties, DWM_THUMBNAIL_PROPERTIES,
    DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
};

#[derive(Debug, Deserialize)]
pub struct Slot {
    pub hwnd: i64,
    /// Placeholder rect in the card window's client area, physical px.
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

static ACTIVE: Mutex<Vec<isize>> = Mutex::new(Vec::new());

pub fn clear() {
    let mut a = ACTIVE.lock().unwrap();
    for t in a.drain(..) {
        unsafe {
            let _ = DwmUnregisterThumbnail(t);
        }
    }
}

/// Show live thumbnails of `slots` inside `dest` (the card window), each fitted into its placeholder
/// keeping the source window's aspect ratio.
pub fn set(dest: HWND, slots: &[Slot]) {
    clear();
    let mut a = ACTIVE.lock().unwrap();
    for s in slots {
        unsafe {
            let Ok(t) = DwmRegisterThumbnail(dest, HWND(s.hwnd as _)) else { continue };
            let src: SIZE = DwmQueryThumbnailSourceSize(t).unwrap_or(SIZE { cx: 16, cy: 9 });
            let (sw, sh) = (src.cx.max(1) as f64, src.cy.max(1) as f64);
            let k = (s.w / sw).min(s.h / sh);
            let (w, h) = (sw * k, sh * k);
            let x = s.x + (s.w - w) / 2.0;
            let y = s.y + (s.h - h) / 2.0;
            let props = DWM_THUMBNAIL_PROPERTIES {
                dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_VISIBLE | DWM_TNP_OPACITY | DWM_TNP_SOURCECLIENTAREAONLY,
                rcDestination: RECT { left: x.round() as i32, top: y.round() as i32, right: (x + w).round() as i32, bottom: (y + h).round() as i32 },
                opacity: 255,
                fVisible: true.into(),
                fSourceClientAreaOnly: false.into(),
                ..Default::default()
            };
            let _ = DwmUpdateThumbnailProperties(t, &props);
            a.push(t);
        }
    }
}
