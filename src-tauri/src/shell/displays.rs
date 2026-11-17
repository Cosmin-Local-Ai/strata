//! Display enumeration with stable identities (D-008).
//!
//! Displays are keyed by their device-interface path (e.g. `\\?\DISPLAY#SAM7179#...`), which survives
//! re-plugging and re-ordering, unlike `\\.\DISPLAYn` or an index. A display gets a dock only if the
//! user enabled it; by default only the primary display does, so small auxiliary screens (case displays, sensor panels) never do.

use crate::config::Settings;
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, DISPLAY_DEVICEW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{EDD_GET_DEVICE_INTERFACE_NAME, MONITORINFOF_PRIMARY};

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    #[serde(skip)]
    pub hmonitor: isize,
    /// Stable key (device-interface path), falls back to the GDI device name.
    pub id: String,
    /// `\\.\DISPLAYn`, for logs only.
    pub gdi_name: String,
    pub name: String,
    pub primary: bool,
    /// Physical px.
    pub rect: [i32; 4],
    pub work: [i32; 4],
    pub dpi: u32,
}

impl Display {
    pub fn monitor(&self) -> HMONITOR {
        HMONITOR(self.hmonitor as _)
    }
    pub fn scale(&self) -> f64 {
        self.dpi as f64 / 96.0
    }
    pub fn rect(&self) -> RECT {
        RECT { left: self.rect[0], top: self.rect[1], right: self.rect[2], bottom: self.rect[3] }
    }
    /// Should this display get a dock, given the user's settings? Primary: yes unless disabled. Others:
    /// only if explicitly enabled.
    pub fn dock_enabled(&self, s: &Settings) -> bool {
        match s.displays.get(&self.id) {
            Some(p) => p.enabled,
            None => self.primary,
        }
    }
}

fn wide_to_string(w: &[u16]) -> String {
    let len = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..len])
}

unsafe extern "system" fn enum_proc(hm: HMONITOR, _hdc: HDC, _rc: *mut RECT, data: LPARAM) -> BOOL {
    let out = unsafe { &mut *(data.0 as *mut Vec<HMONITOR>) };
    out.push(hm);
    true.into()
}

pub fn enumerate() -> Vec<Display> {
    let mut hms: Vec<HMONITOR> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(enum_proc), LPARAM(&mut hms as *mut _ as isize));
    }
    let mut out = Vec::new();
    for hm in hms {
        unsafe {
            let mut mi = MONITORINFOEXW::default();
            mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            if !GetMonitorInfoW(hm, &mut mi as *mut _ as *mut MONITORINFO).as_bool() {
                continue;
            }
            let gdi_name = wide_to_string(&mi.szDevice);
            let mut dd = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
            let gdi_w: Vec<u16> = gdi_name.encode_utf16().chain(std::iter::once(0)).collect();
            let (id, name) = if EnumDisplayDevicesW(windows::core::PCWSTR(gdi_w.as_ptr()), 0, &mut dd, EDD_GET_DEVICE_INTERFACE_NAME).as_bool() {
                (wide_to_string(&dd.DeviceID), wide_to_string(&dd.DeviceString))
            } else {
                (gdi_name.clone(), gdi_name.clone())
            };
            let (mut dx, mut dy) = (96u32, 96u32);
            let _ = GetDpiForMonitor(hm, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
            let r = mi.monitorInfo.rcMonitor;
            let w = mi.monitorInfo.rcWork;
            out.push(Display {
                hmonitor: hm.0 as isize,
                id: if id.is_empty() { gdi_name.clone() } else { id },
                gdi_name,
                name,
                primary: mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                rect: [r.left, r.top, r.right, r.bottom],
                work: [w.left, w.top, w.right, w.bottom],
                dpi: dx,
            });
        }
    }
    out
}
