//! Image thumbnails for the gallery, served at `http://thumb.localhost/<url-encoded path>?w=<px>`.
//! WIC decodes the first frame (also of Wallpaper Engine's animated preview.gif), scales it to the
//! requested width and encodes a JPEG. Cached in memory (bounded), so the WebView never loads full-size
//! 4K/8K wallpapers just to draw a 300 px tile.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::GENERIC_READ;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, STREAM_SEEK_CUR};

fn cache() -> &'static Mutex<HashMap<(String, u32), Vec<u8>>> {
    static C: OnceLock<Mutex<HashMap<(String, u32), Vec<u8>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Only images; never arbitrary files.
fn allowed(path: &str) -> bool {
    let p = path.to_lowercase();
    [".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp", ".jxr", ".tif", ".tiff", ".ico"].iter().any(|x| p.ends_with(x))
}

pub fn get(path: &str, width: u32) -> Option<Vec<u8>> {
    if !allowed(path) {
        return None;
    }
    let w = width.clamp(32, 1024);
    let key = (path.to_lowercase(), w);
    if let Some(b) = cache().lock().unwrap().get(&key) {
        return Some(b.clone());
    }
    let bytes = make(path, w).map_err(|e| log::debug!("thumb {path}: {e}")).ok()?;
    let mut c = cache().lock().unwrap();
    if c.len() > 600 {
        c.clear(); // simple bound; thumbnails are cheap to rebuild
    }
    c.insert(key, bytes.clone());
    Some(bytes)
}

fn make(path: &str, width: u32) -> windows::core::Result<Vec<u8>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let dec = wic.CreateDecoderFromFilename(&HSTRING::from(path), None, GENERIC_READ, WICDecodeMetadataCacheOnDemand)?;
        let frame = dec.GetFrame(0)?;
        let (mut iw, mut ih) = (0u32, 0u32);
        frame.GetSize(&mut iw, &mut ih)?;
        let w = width.min(iw.max(1));
        let h = ((ih as f64) * (w as f64) / (iw.max(1) as f64)).round().max(1.0) as u32;
        let scaler = wic.CreateBitmapScaler()?;
        scaler.Initialize(&frame, w, h, WICBitmapInterpolationModeFant)?;
        let conv = wic.CreateFormatConverter()?;
        // PNG keeps transparency (icons); photos are JPEG.
        let png = path.to_lowercase().ends_with(".png") || path.to_lowercase().ends_with(".ico");
        let pf = if png { GUID_WICPixelFormat32bppBGRA } else { GUID_WICPixelFormat24bppBGR };
        conv.Initialize(&scaler, &pf, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom)?;
        let mut buf = vec![0u8; (w * h * 4 + 65536) as usize];
        let stream = wic.CreateStream()?;
        stream.InitializeFromMemory(&mut buf)?;
        let enc = wic.CreateEncoder(if png { &GUID_ContainerFormatPng } else { &GUID_ContainerFormatJpeg }, std::ptr::null())?;
        enc.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let mut fr = None;
        enc.CreateNewFrame(&mut fr, std::ptr::null_mut())?;
        let fr = fr.unwrap();
        fr.Initialize(None)?;
        fr.SetSize(w, h)?;
        let mut fmt = pf;
        fr.SetPixelFormat(&mut fmt)?;
        fr.WriteSource(&conv.cast::<IWICBitmapSource>()?, std::ptr::null())?;
        fr.Commit()?;
        enc.Commit()?;
        let mut pos = 0u64;
        stream.Seek(0, STREAM_SEEK_CUR, Some(&mut pos))?;
        buf.truncate(pos as usize);
        Ok(buf)
    }
}
