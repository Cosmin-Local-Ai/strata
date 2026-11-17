//! App/file icons for the web UI, served as PNG at `http://icon.localhost/<url-encoded path>?s=<px>`.
//!
//! Extraction uses IShellItemImageFactory, which understands exes, shortcuts (.lnk → target icon),
//! files, folders and `shell:AppsFolder\<AUMID>` Store apps, the same way Explorer does. Work runs on
//! one STA thread (shell COM objects want STA); results are cached in memory.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{DeleteObject, HPALETTE};
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STREAM_SEEK_CUR};
use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY};

type Job = (String, u32, Sender<Option<Vec<u8>>>);

fn worker() -> &'static Mutex<Sender<Job>> {
    static W: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
    W.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new()
            .name("strata-icons".into())
            .spawn(move || {
                unsafe {
                    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                }
                let mut cache: HashMap<(String, u32), Option<Vec<u8>>> = HashMap::new();
                for (path, size, reply) in rx {
                    let key = (path.to_lowercase(), size);
                    let png = cache.entry(key).or_insert_with(|| extract(&path, size).map_err(|e| log::debug!("icon {path}: {e}")).ok()).clone();
                    let _ = reply.send(png);
                }
            })
            .expect("icon thread");
        Mutex::new(tx)
    })
}

/// PNG bytes for `path` at `size` px (blocking; call from a worker / async context).
pub fn get(path: &str, size: u32) -> Option<Vec<u8>> {
    let (tx, rx) = channel();
    worker().lock().unwrap().send((path.to_string(), size.clamp(16, 256), tx)).ok()?;
    rx.recv().ok().flatten()
}

fn extract(path: &str, size: u32) -> windows::core::Result<Vec<u8>> {
    unsafe {
        let item: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(super::expand_env(path)), None)?;
        let hbm = item.GetImage(SIZE { cx: size as i32, cy: size as i32 }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)?;
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let bmp = wic.CreateBitmapFromHBITMAP(hbm, HPALETTE::default(), WICBitmapUseAlpha);
        let _ = DeleteObject(hbm.into());
        let bmp = bmp?;
        let (mut w, mut h) = (0u32, 0u32);
        bmp.GetSize(&mut w, &mut h)?;
        // Shell bitmaps are bottom-up DIBs; WIC handles orientation. Encode PNG into memory.
        let mut buf = vec![0u8; (w * h * 4 + 8192) as usize];
        let stream = wic.CreateStream()?;
        stream.InitializeFromMemory(&mut buf)?;
        let enc = wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        enc.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let mut frame = None;
        enc.CreateNewFrame(&mut frame, std::ptr::null_mut())?;
        let frame = frame.unwrap();
        frame.Initialize(None)?;
        frame.SetSize(w, h)?;
        let mut fmt = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut fmt)?;
        frame.WriteSource(&bmp.cast::<IWICBitmapSource>()?, std::ptr::null())?;
        frame.Commit()?;
        enc.Commit()?;
        let mut pos = 0u64;
        stream.Seek(0, STREAM_SEEK_CUR, Some(&mut pos))?;
        buf.truncate(pos as usize);
        Ok(buf)
    }
}
