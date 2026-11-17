//! Screenshot Capture widget: screen, window (with Windows 11 rounded corners kept as transparency)
//! or region, saved to Pictures\Screenshots and copied to the clipboard; optional text recognition
//! (Windows.Media.Ocr, on-device). Plain GDI capture only: no GPU capture APIs (D-003).

use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct Shot { pub w: u32, pub h: u32, pub bgra: Vec<u8> }

/// The last full-screen grab, kept while the region selector is open.
static PENDING: Mutex<Option<Shot>> = Mutex::new(None);

fn grab(x: i32, y: i32, w: i32, h: i32, from: Option<HWND>) -> Result<Shot, String> {
    unsafe {
        let src = GetDC(from);
        let mem = CreateCompatibleDC(Some(src));
        let bmp = CreateCompatibleBitmap(src, w, h);
        let old = SelectObject(mem, bmp.into());
        let ok = if let Some(hw) = from {
            // PW_RENDERFULLCONTENT (2) renders DirectComposition/WebView content too.
            windows::Win32::Storage::Xps::PrintWindow(hw, mem, windows::Win32::Storage::Xps::PRINT_WINDOW_FLAGS(2)).as_bool()
        } else {
            BitBlt(mem, 0, 0, w, h, Some(src), x, y, SRCCOPY | CAPTUREBLT).is_ok()
        };
        let mut bi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, ..Default::default() }, ..Default::default() };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        GetDIBits(mem, bmp, 0, h as u32, Some(buf.as_mut_ptr() as _), &mut bi, DIB_RGB_COLORS);
        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(from, src);
        if !ok { return Err("capture failed".into()) }
        for px in buf.chunks_exact_mut(4) { px[3] = 255 }
        Ok(Shot { w: w as u32, h: h as u32, bgra: buf })
    }
}

fn main_monitor() -> RECT {
    unsafe {
        let hm = MonitorFromPoint(windows::Win32::Foundation::POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(hm, &mut mi);
        mi.rcMonitor
    }
}

fn crop(s: &Shot, x: u32, y: u32, w: u32, h: u32) -> Shot {
    let (x, y) = (x.min(s.w.saturating_sub(1)), y.min(s.h.saturating_sub(1)));
    let (w, h) = (w.min(s.w - x).max(1), h.min(s.h - y).max(1));
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for row in y..y + h {
        let start = ((row * s.w + x) * 4) as usize;
        out.extend_from_slice(&s.bgra[start..start + (w * 4) as usize]);
    }
    Shot { w, h, bgra: out }
}

fn save_png(s: &Shot, path: &std::path::Path) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), s.w, s.h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    let rgba: Vec<u8> = s.bgra.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
    wr.write_image_data(&rgba).map_err(|e| e.to_string())
}

fn clipboard_image(s: &Shot) {
    use windows::Win32::System::DataExchange::*;
    use windows::Win32::System::Memory::*;
    unsafe {
        let hdr = BITMAPINFOHEADER { biSize: 40, biWidth: s.w as i32, biHeight: s.h as i32, biPlanes: 1, biBitCount: 32, ..Default::default() };
        let size = 40 + s.bgra.len();
        let Ok(hg) = GlobalAlloc(GMEM_MOVEABLE, size) else { return };
        let p = GlobalLock(hg) as *mut u8;
        std::ptr::copy_nonoverlapping(&hdr as *const _ as *const u8, p, 40);
        // DIBs are bottom-up.
        let row = (s.w * 4) as usize;
        for y in 0..s.h as usize {
            std::ptr::copy_nonoverlapping(s.bgra.as_ptr().add((s.h as usize - 1 - y) * row), p.add(40 + y * row), row);
        }
        let _ = GlobalUnlock(hg);
        if OpenClipboard(None).is_ok() {
            let _ = EmptyClipboard();
            let _ = SetClipboardData(8 /* CF_DIB */, Some(windows::Win32::Foundation::HANDLE(hg.0)));
            let _ = CloseClipboard();
        }
    }
}

pub fn clipboard_text(t: &str) {
    use windows::Win32::System::DataExchange::*;
    use windows::Win32::System::Memory::*;
    let w: Vec<u16> = t.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let Ok(hg) = GlobalAlloc(GMEM_MOVEABLE, w.len() * 2) else { return };
        let p = GlobalLock(hg) as *mut u16;
        std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
        let _ = GlobalUnlock(hg);
        if OpenClipboard(None).is_ok() {
            let _ = EmptyClipboard();
            let _ = SetClipboardData(13 /* CF_UNICODETEXT */, Some(windows::Win32::Foundation::HANDLE(hg.0)));
            let _ = CloseClipboard();
        }
    }
}

fn ocr(s: &Shot) -> Result<String, String> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    let buf = windows::Security::Cryptography::CryptographicBuffer::CreateFromByteArray(&s.bgra).map_err(|e| e.to_string())?;
    let bmp = SoftwareBitmap::CreateCopyFromBuffer(&buf, BitmapPixelFormat::Bgra8, s.w as i32, s.h as i32).map_err(|e| e.to_string())?;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| "No text-recognition language is installed")?;
    let res = engine.RecognizeAsync(&bmp).map_err(|e| e.to_string())?.join().map_err(|e| e.to_string())?;
    let lines = res.Lines().map_err(|e| e.to_string())?;
    Ok((0..lines.Size().unwrap_or(0)).filter_map(|i| lines.GetAt(i).ok()?.Text().ok().map(|t| t.to_string())).collect::<Vec<_>>().join("\n"))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved { pub path: String, pub text: Option<String> }

fn finish(s: Shot, want_text: bool) -> Result<Saved, String> {
    finish_opt(s, want_text, SAVE.load(std::sync::atomic::Ordering::Relaxed))
}

/// Whether captures are also saved as files (the widget's "Save to Screenshots" switch).
pub static SAVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn finish_opt(s: Shot, want_text: bool, save: bool) -> Result<Saved, String> {
    if !save {
        let text = if want_text { let t = ocr(&s)?; clipboard_text(&t); Some(t) } else { clipboard_image(&s); None };
        return Ok(Saved { path: String::new(), text });
    }
    let dir = crate::local::known_folder("screenshots").or_else(|_| crate::local::known_folder("pictures").map(|p| p.join("Screenshots")))?;
    let _ = std::fs::create_dir_all(&dir);
    let st = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    let path = dir.join(format!("Strata {:04}-{:02}-{:02} {:02}{:02}{:02}.png", st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond));
    save_png(&s, &path)?;
    let text = if want_text {
        let t = ocr(&s)?;
        clipboard_text(&t);
        Some(t)
    } else {
        clipboard_image(&s);
        None
    };
    Ok(Saved { path: path.to_string_lossy().into(), text })
}

pub fn screen(want_text: bool) -> Result<Saved, String> {
    let r = main_monitor();
    finish(grab(r.left, r.top, r.right - r.left, r.bottom - r.top, None)?, want_text)
}

/// The window the user was working in (the dock never takes focus).
pub fn window(want_text: bool) -> Result<Saved, String> {
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    unsafe {
        let hw = GetForegroundWindow();
        if hw.0.is_null() { return Err("no window in front".into()) }
        let mut wr = RECT::default();
        let _ = GetWindowRect(hw, &mut wr);
        let mut fr = RECT::default();
        let _ = DwmGetWindowAttribute(hw, DWMWA_EXTENDED_FRAME_BOUNDS, &mut fr as *mut _ as _, std::mem::size_of::<RECT>() as u32);
        let full = grab(0, 0, wr.right - wr.left, wr.bottom - wr.top, Some(hw))?;
        let mut s = crop(&full, (fr.left - wr.left).max(0) as u32, (fr.top - wr.top).max(0) as u32, (fr.right - fr.left) as u32, (fr.bottom - fr.top) as u32);
        // Windows 11 rounds unmaximised windows (8 px at 100 %): keep the corners transparent.
        if !IsZoomed(hw).as_bool() {
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hw).max(96);
            let r = (8 * dpi / 96) as i32;
            let (w, h) = (s.w as i32, s.h as i32);
            for y in 0..h {
                for x in 0..w {
                    let cx = if x < r { r - x } else if x >= w - r { x - (w - r - 1) } else { 0 };
                    let cy = if y < r { r - y } else if y >= h - r { y - (h - r - 1) } else { 0 };
                    if cx > 0 && cy > 0 {
                        let d = ((cx * cx + cy * cy) as f32).sqrt() - r as f32;
                        let a = (1.0 - d).clamp(0.0, 1.0);
                        s.bgra[((y * w + x) * 4 + 3) as usize] = (a * 255.0) as u8;
                    }
                }
            }
        }
        finish(s, want_text)
    }
}

/// Region: grab the screen now, the overlay lets the user drag a rectangle over this still image.
pub fn region_begin() -> Result<String, String> {
    let r = main_monitor();
    let s = grab(r.left, r.top, r.right - r.left, r.bottom - r.top, None)?;
    let tmp = std::env::temp_dir().join("strata-capture.png");
    save_png(&s, &tmp)?;
    *PENDING.lock().unwrap() = Some(s);
    Ok(tmp.to_string_lossy().into())
}

pub fn region_finish(x: u32, y: u32, w: u32, h: u32, want_text: bool) -> Result<Saved, String> {
    let s = PENDING.lock().unwrap().take().ok_or("nothing captured")?;
    let _ = std::fs::remove_file(std::env::temp_dir().join("strata-capture.png"));
    finish(crop(&s, x, y, w, h), want_text)
}

pub fn region_cancel() {
    *PENDING.lock().unwrap() = None;
    let _ = std::fs::remove_file(std::env::temp_dir().join("strata-capture.png"));
}
