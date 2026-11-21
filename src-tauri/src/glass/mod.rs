//! Native glass layer (decisions D-003, D-011).
//!
//! The dock's glass body is drawn by Direct3D 11 into a DirectComposition *surface* that sits behind the
//! WebView2 child of the dock window. There is no DXGI swap chain and no `Present`, so game overlays never
//! attach to Strata.
//!
//! The pixels behind the dock come from one of two sources:
//! * `Background::Image` (default): a strip of the wallpaper (Windows or Wallpaper Engine preview) decoded
//!   with WIC by `wallpaper.rs`. No screen capture at all.
//! * `Background::Live` (opt-in): Windows.Graphics.Capture of the dock's monitor only, with the dock
//!   excluded from capture (`WDA_EXCLUDEFROMCAPTURE`). Rate-capped.
//!
//! The D3D device is created on the adapter that drives the dock's monitor, so nothing ever crosses GPUs.

mod eq;
pub mod wallpaper;

use std::sync::{Arc, Mutex};
use windows::core::{s, Interface, Result, PCSTR};
use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{HMODULE, HWND, POINT, RECT};
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{ID3DBlob, D3D_DRIVER_TYPE_UNKNOWN, D3D_DRIVER_TYPE_WARP, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::{DCompositionCreateDevice, IDCompositionDevice, IDCompositionSurface, IDCompositionTarget, IDCompositionVisual};
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, HMONITOR, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::WinRT::Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect, SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE};

#[repr(C)]
#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlassStyle {
    /// rgb tint 0..1 + amount
    pub tint: [f32; 4],
    pub refraction: f32,
    pub blur_lod: f32,
    pub rim: f32,
    pub dispersion: f32,
    /// flat body colour + mix (solid styles)
    pub shade: [f32; 4],
}

impl Default for GlassStyle {
    fn default() -> Self {
        Self { tint: [1.0, 1.0, 1.0, 0.06], refraction: 0.6, blur_lod: 1.5, rim: 0.35, dispersion: 1.2, shade: [0.0, 0.0, 0.0, 0.0] }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Params {
    rect: [f32; 4],
    tint: [f32; 4],
    win_size: [f32; 2],
    radius: f32,
    refraction: f32,
    bg_origin: [f32; 2],
    bg_size: [f32; 2],
    blur_lod: f32,
    rim: f32,
    dispersion: f32,
    body_alpha: f32,
    shade: [f32; 4],
}

/// Geometry of the glass body inside the dock window, in physical px.
#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlassRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Background {
    Image,
    Live,
}

struct Inner {
    hwnd: HWND,
    monitor: HMONITOR,
    device: ID3D11Device,
    ctx: ID3D11DeviceContext,
    d3d: IDirect3DDevice,
    dcomp: IDCompositionDevice,
    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
    surface: Option<IDCompositionSurface>,
    /// Region drawn last time, so we can erase it when the body shrinks or moves.
    drawn: Option<RECT>,
    /// DirectComposition requires the first BeginDraw on a new surface to cover all of it.
    surface_fresh: bool,
    vs: ID3D11VertexShader,
    ps: ID3D11PixelShader,
    cb: ID3D11Buffer,
    sampler: ID3D11SamplerState,
    raster: ID3D11RasterizerState,
    /// Background texture covering exactly the dock window, with mips for frost.
    bg: Option<(ID3D11Texture2D, ID3D11ShaderResourceView, u32, u32)>,
    size: (u32, u32),
    rect: GlassRect,
    style: GlassStyle,
    mode: Background,
    capture: Option<(Direct3D11CaptureFramePool, GraphicsCaptureSession)>,
    live_interval_ms: i64,
    /// GPU yield (D-011): no drawing, no capture, no textures.
    yielding: bool,
    frames: u64,
    eq: Option<eq::Eq>,
    renders: u64,
    adapter_name: String,
}

unsafe impl Send for Inner {}

#[derive(Clone)]
pub struct Glass(Arc<Mutex<Inner>>);

fn compile(src: &str, entry: PCSTR, target: PCSTR) -> Result<ID3DBlob> {
    unsafe {
        let mut blob = None;
        let mut err = None;
        let r = D3DCompile(src.as_ptr() as _, src.len(), s!("glass.hlsl"), None, None, entry, target, 0, 0, &mut blob, Some(&mut err));
        if let Err(e) = r {
            if let Some(err) = err {
                let msg = std::slice::from_raw_parts(err.GetBufferPointer() as *const u8, err.GetBufferSize());
                log::error!("glass shader compile failed: {}", String::from_utf8_lossy(msg));
            }
            return Err(e);
        }
        Ok(blob.unwrap())
    }
}

fn blob_bytes(b: &ID3DBlob) -> &[u8] {
    unsafe { std::slice::from_raw_parts(b.GetBufferPointer() as *const u8, b.GetBufferSize()) }
}

/// The DXGI adapter that drives `monitor`. Never falls back to another GPU silently: if no adapter
/// claims the monitor we return None and the caller renders without glass.
fn adapter_for_monitor(monitor: HMONITOR) -> Option<(IDXGIAdapter1, String)> {
    unsafe {
        let f: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
        let mut i = 0;
        while let Ok(a) = f.EnumAdapters1(i) {
            let mut j = 0;
            while let Ok(o) = a.EnumOutputs(j) {
                if let Ok(d) = o.GetDesc() {
                    if d.Monitor == monitor {
                        let desc = a.GetDesc1().ok()?;
                        let name = String::from_utf16_lossy(&desc.Description).trim_end_matches('\0').to_string();
                        return Some((a, name));
                    }
                }
                j += 1;
            }
            i += 1;
        }
        None
    }
}

pub fn monitor_rect(monitor: HMONITOR) -> RECT {
    unsafe {
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(monitor, &mut mi);
        mi.rcMonitor
    }
}

impl Glass {
    /// Image mode renders with WARP (Windows' software rasteriser): the GPU vendor driver is never
    /// loaded into Strata, so vendor overlays (NVIDIA Alt+Z etc.) never attach and games keep the GPU
    /// (D-011). Live mode needs the hardware device of the monitor's own adapter for capture.
    pub fn new(hwnd: HWND, mode: Background) -> Result<Self> {
        unsafe {
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
            let mut device = None;
            let mut ctx = None;
            let adapter_name;
            if mode == Background::Live {
                let Some((adapter, name)) = adapter_for_monitor(monitor) else {
                    log::warn!("glass: no DXGI adapter drives the dock's monitor; live glass unavailable");
                    return Err(windows::core::Error::empty());
                };
                adapter_name = name;
                D3D11CreateDevice(&adapter, D3D_DRIVER_TYPE_UNKNOWN, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut ctx))?;
            } else {
                adapter_name = "WARP (software)".to_string();
                D3D11CreateDevice(None, D3D_DRIVER_TYPE_WARP, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut ctx))?;
            }
            log::info!("glass: device '{adapter_name}', mode {mode:?}");
            let device: ID3D11Device = device.unwrap();
            let ctx: ID3D11DeviceContext = ctx.unwrap();
            // IPC commands and the capture callback reach the context from different threads; all access
            // is serialised by our mutex and D3D is told so.
            if let Ok(mt) = device.cast::<ID3D11Multithread>() {
                let _ = mt.SetMultithreadProtected(true);
            }
            let dxgi: IDXGIDevice = device.cast()?;
            let d3d: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?.cast()?;

            let mut rc = RECT::default();
            GetClientRect(hwnd, &mut rc)?;
            let (w, h) = ((rc.right - rc.left).max(1) as u32, (rc.bottom - rc.top).max(1) as u32);

            let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
            // topmost = false: our visual is composed *behind* the WebView2 child window.
            let target = dcomp.CreateTargetForHwnd(hwnd, false)?;
            let visual = dcomp.CreateVisual()?;
            let surface = dcomp.CreateSurface(w, h, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ALPHA_MODE_PREMULTIPLIED)?;
            visual.SetContent(&surface)?;
            target.SetRoot(&visual)?;
            dcomp.Commit()?;

            let src = include_str!("shader.hlsl");
            let vsb = compile(src, s!("vs_main"), s!("vs_5_0"))?;
            let psb = compile(src, s!("ps_main"), s!("ps_5_0"))?;
            let mut vs = None;
            device.CreateVertexShader(blob_bytes(&vsb), None, Some(&mut vs))?;
            let mut ps = None;
            device.CreatePixelShader(blob_bytes(&psb), None, Some(&mut ps))?;

            let cbd = D3D11_BUFFER_DESC {
                ByteWidth: std::mem::size_of::<Params>().div_ceil(16) as u32 * 16,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                ..Default::default()
            };
            let mut cb = None;
            device.CreateBuffer(&cbd, None, Some(&mut cb))?;
            let sd = D3D11_SAMPLER_DESC {
                Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                MaxLOD: f32::MAX,
                ..Default::default()
            };
            let mut sampler = None;
            device.CreateSamplerState(&sd, Some(&mut sampler))?;
            let rd = D3D11_RASTERIZER_DESC { FillMode: D3D11_FILL_SOLID, CullMode: D3D11_CULL_NONE, ScissorEnable: true.into(), DepthClipEnable: true.into(), ..Default::default() };
            let mut raster = None;
            device.CreateRasterizerState(&rd, Some(&mut raster))?;

            Ok(Glass(Arc::new(Mutex::new(Inner {
                hwnd, monitor, device, ctx, d3d, dcomp, _target: target, visual, surface: Some(surface), drawn: None, surface_fresh: true,
                vs: vs.unwrap(), ps: ps.unwrap(), cb: cb.unwrap(), sampler: sampler.unwrap(), raster: raster.unwrap(),
                bg: None, size: (w, h), rect: GlassRect::default(), style: GlassStyle::default(), mode,
                capture: None, live_interval_ms: 33, yielding: false, frames: 0, eq: None, renders: 0, adapter_name,
            }))))
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.0.lock().unwrap().hwnd
    }

    /// (Re)load the background for the current mode. Image mode decodes on a worker thread.
    pub fn refresh_background(&self) {
        let (mode, yielding) = {
            let g = self.0.lock().unwrap();
            (g.mode, g.yielding)
        };
        if yielding {
            return;
        }
        match mode {
            Background::Live => {
                if let Err(e) = self.start_live() {
                    // The owner recreates Glass in Image mode (different device), see lib.rs.
                    log::warn!("live glass failed: {e}");
                }
            }
            Background::Image => {
                self.stop_live();
                let (hwnd_raw, monitor_raw) = {
                    let g = self.0.lock().unwrap();
                    (g.hwnd.0 as isize, g.monitor.0 as isize)
                };
                let me = self.clone();
                std::thread::spawn(move || unsafe {
                    let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
                    let hwnd = HWND(hwnd_raw as _);
                    let mrect = monitor_rect(HMONITOR(monitor_raw as _));
                    let mut wr = RECT::default();
                    let _ = GetWindowRect(hwnd, &mut wr);
                    let spec = wallpaper::resolve(mrect);
                    log::info!("glass background: {:?}", spec);
                    match wallpaper::load_strip(&spec, mrect, wr) {
                        Ok(strip) => me.set_strip(strip),
                        Err(e) => log::warn!("glass background decode failed: {e}"),
                    }
                });
            }
        }
    }

    pub fn mode(&self) -> Background {
        self.0.lock().unwrap().mode
    }

    fn set_strip(&self, strip: wallpaper::Strip) {
        let mut g = self.0.lock().unwrap();
        if g.yielding || g.mode != Background::Image {
            return;
        }
        if let Err(e) = g.upload(&strip) {
            log::warn!("glass upload failed: {e}");
            return;
        }
        if let Err(e) = g.render() { log::warn!("glass render failed: {e}"); }
    }

    /// Live glass: capture only the dock's own monitor on the same adapter, with Strata's windows
    /// excluded from capture. Opt-in (D-003): the dock is then invisible to OBS and screenshots.
    fn start_live(&self) -> Result<()> {
        let mut g = self.0.lock().unwrap();
        g.stop_capture();
        unsafe {
            SetWindowDisplayAffinity(g.hwnd, WDA_EXCLUDEFROMCAPTURE)?;
            let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem = interop.CreateForMonitor(g.monitor)?;
            let item_size = item.Size()?;
            // WGC clips content to the pool size from the top-left, so we only allocate up to the dock
            // window's bottom-right corner (a top dock needs only a thin strip).
            let mr = monitor_rect(g.monitor);
            let mut wr = RECT::default();
            GetWindowRect(g.hwnd, &mut wr)?;
            let need_w = (wr.right - mr.left).clamp(1, item_size.Width);
            let need_h = (wr.bottom - mr.top).clamp(1, item_size.Height);
            let size = windows::Graphics::SizeInt32 { Width: need_w, Height: need_h };
            let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&g.d3d, DirectXPixelFormat::B8G8R8A8UIntNormalized, 1, size)?;
            let session = pool.CreateCaptureSession(&item)?;
            let _ = session.SetIsBorderRequired(false);
            let _ = session.SetIsCursorCaptureEnabled(false);
            let _ = session.SetMinUpdateInterval(TimeSpan { Duration: g.live_interval_ms * 10_000 });
            log::info!("live glass: monitor {}x{}, pool {}x{}, every {} ms", item_size.Width, item_size.Height, need_w, need_h, g.live_interval_ms);
            let weak = Arc::downgrade(&self.0);
            let (ml, mt) = (mr.left, mr.top);
            pool.FrameArrived(&TypedEventHandler::new(move |p: windows_core::Ref<Direct3D11CaptureFramePool>, _| {
                let Some(p) = p.as_ref() else { return Ok(()) };
                let frame = p.TryGetNextFrame()?;
                if let Some(inner) = weak.upgrade() {
                    let mut g = inner.lock().unwrap();
                    if !g.yielding {
                        g.frames += 1;
                        let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
                        let tex: ID3D11Texture2D = access.GetInterface()?;
                        if let Err(e) = g.ingest_capture(&tex, ml, mt) {
                            log::warn!("live glass ingest failed: {e}");
                        }
                        if let Err(e) = g.render() { log::warn!("glass render failed: {e}"); }
                    }
                }
                frame.Close()?;
                Ok(())
            }))?;
            session.StartCapture()?;
            g.capture = Some((pool, session));
        }
        Ok(())
    }

    fn stop_live(&self) {
        let mut g = self.0.lock().unwrap();
        g.stop_capture();
        unsafe {
            let _ = SetWindowDisplayAffinity(g.hwnd, WDA_NONE);
        }
    }

    /// GPU yield (D-011). Entering: stop capture, keep the last drawn glass on screen (DWM keeps the
    /// surface), drop our textures and trim the device. Leaving: reload the background and redraw.
    pub fn set_yield(&self, on: bool) {
        {
            let mut g = self.0.lock().unwrap();
            if g.yielding == on {
                return;
            }
            g.yielding = on;
            if on {
                g.stop_capture();
                g.bg = None;
                unsafe {
                    g.ctx.ClearState();
                    g.ctx.Flush();
                    if let Ok(d) = g.device.cast::<IDXGIDevice3>() {
                        d.Trim();
                    }
                }
                // Clear to transparent: while yielding the dock paints a plain CSS background that
                // always follows its real size. A frozen glass frame would keep the old size and
                // position (tiles outside the glass, a stretched bar after toggling magnification).
                if let Err(e) = g.clear() { log::warn!("glass clear failed: {e}"); }
                log::info!("glass: yielding GPU");
                return;
            }
        }
        log::info!("glass: resuming");
        {
            let mut g = self.0.lock().unwrap();
            unsafe { g.monitor = MonitorFromWindow(g.hwnd, MONITOR_DEFAULTTOPRIMARY); }
            if let Err(e) = g.resize() { log::warn!("glass resize failed: {e}"); }
        }
        self.refresh_background();
    }

    pub fn set_rect(&self, rect: GlassRect) {
        let mut g = self.0.lock().unwrap();
        g.rect = rect;
        if !g.yielding {
            if let Err(e) = g.render() { log::warn!("glass render failed: {e}"); }
        }
    }

    pub fn set_style(&self, style: GlassStyle) {
        let mut g = self.0.lock().unwrap();
        g.style = style;
        if !g.yielding {
            if let Err(e) = g.render() { log::warn!("glass render failed: {e}"); }
        }
    }

    /// Window was resized or moved: rebuild the surface and the background strip.
    pub fn geometry_changed(&self) {
        {
            let mut g = self.0.lock().unwrap();
            if g.yielding {
                return; // keep the last image; re-done when yield ends (refresh_background)
            }
            unsafe {
                g.monitor = MonitorFromWindow(g.hwnd, MONITOR_DEFAULTTOPRIMARY);
            }
            if let Err(e) = g.resize() {
                log::warn!("glass resize failed: {e}");
            }
        }
        self.refresh_background();
    }

    pub fn debug(&self) -> String {
        let g = self.0.lock().unwrap();
        format!("adapter='{}' mode={:?} yield={} size={:?} rect={:?} bg={:?} frames={} renders={}",
            g.adapter_name, g.mode, g.yielding, g.size, g.rect, g.bg.as_ref().map(|b| (b.2, b.3)), g.frames, g.renders)
    }
}

impl Inner {
    fn stop_capture(&mut self) {
        if let Some((pool, session)) = self.capture.take() {
            let _ = session.Close();
            let _ = pool.Close();
        }
    }

    fn resize(&mut self) -> Result<()> {
        unsafe {
            let mut rc = RECT::default();
            GetClientRect(self.hwnd, &mut rc)?;
            let (w, h) = ((rc.right - rc.left).max(1) as u32, (rc.bottom - rc.top).max(1) as u32);
            if (w, h) == self.size && self.surface.is_some() {
                return Ok(());
            }
            let surface = self.dcomp.CreateSurface(w, h, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ALPHA_MODE_PREMULTIPLIED)?;
            self.visual.SetContent(&surface)?;
            self.dcomp.Commit()?;
            self.surface = Some(surface);
            self.drawn = None;
            self.surface_fresh = true;
            self.size = (w, h);
            self.bg = None;
        }
        Ok(())
    }

    fn ensure_bg(&mut self, w: u32, h: u32) -> Result<()> {
        if self.bg.as_ref().map(|b| (b.2, b.3)) == Some((w, h)) {
            return Ok(());
        }
        unsafe {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: w, Height: h, MipLevels: 0, ArraySize: 1, Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 }, Usage: D3D11_USAGE_DEFAULT,
                BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
                CPUAccessFlags: 0, MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
            };
            let mut t = None;
            self.device.CreateTexture2D(&desc, None, Some(&mut t))?;
            let t = t.unwrap();
            let mut srv = None;
            self.device.CreateShaderResourceView(&t, None, Some(&mut srv))?;
            self.bg = Some((t, srv.unwrap(), w, h));
        }
        Ok(())
    }

    fn upload(&mut self, strip: &wallpaper::Strip) -> Result<()> {
        self.ensure_bg(strip.width, strip.height)?;
        unsafe {
            let (t, srv, _, _) = self.bg.as_ref().unwrap();
            self.ctx.UpdateSubresource(t, 0, None, strip.pixels.as_ptr() as _, strip.width * 4, 0);
            self.ctx.GenerateMips(srv);
        }
        Ok(())
    }

    /// Copy the part of the captured monitor behind the dock window into our mip-mapped texture.
    fn ingest_capture(&mut self, frame: &ID3D11Texture2D, mon_left: i32, mon_top: i32) -> Result<()> {
        let (w, h) = self.size;
        self.ensure_bg(w, h)?;
        unsafe {
            let mut wr = RECT::default();
            GetWindowRect(self.hwnd, &mut wr)?;
            let origin = POINT { x: wr.left - mon_left, y: wr.top - mon_top };
            let mut fd = D3D11_TEXTURE2D_DESC::default();
            frame.GetDesc(&mut fd);
            let left = origin.x.max(0);
            let top = origin.y.max(0);
            let right = (origin.x + w as i32).min(fd.Width as i32);
            let bottom = (origin.y + h as i32).min(fd.Height as i32);
            if right <= left || bottom <= top {
                return Ok(());
            }
            let bx = D3D11_BOX { left: left as u32, top: top as u32, front: 0, right: right as u32, bottom: bottom as u32, back: 1 };
            let (bgt, srv, _, _) = self.bg.as_ref().unwrap();
            self.ctx.CopySubresourceRegion(bgt, 0, (left - origin.x) as u32, (top - origin.y) as u32, 0, frame, 0, Some(&bx));
            self.ctx.GenerateMips(srv);
        }
        Ok(())
    }

    /// Make the whole surface transparent (one cheap draw).
    fn clear(&mut self) -> Result<()> {
        let Some(surface) = self.surface.clone() else { return Ok(()) };
        let (w, h) = self.size;
        let region = RECT { left: 0, top: 0, right: w as i32, bottom: h as i32 };
        unsafe {
            let mut offset = POINT::default();
            let tex: ID3D11Texture2D = surface.BeginDraw(Some(&region), &mut offset)?;
            let mut rtv = None;
            self.device.CreateRenderTargetView(&tex, None, Some(&mut rtv))?;
            if let Some(rtv) = rtv {
                // The surface may live in a shared atlas: clear only our rectangle.
                let r = RECT { left: offset.x, top: offset.y, right: offset.x + w as i32, bottom: offset.y + h as i32 };
                if let Ok(ctx1) = self.ctx.cast::<ID3D11DeviceContext1>() {
                    ctx1.ClearView(&rtv, &[0.0, 0.0, 0.0, 0.0], Some(&[r]));
                }
            }
            surface.EndDraw()?;
            self.dcomp.Commit()?;
        }
        self.drawn = None;
        self.surface_fresh = true;
        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        let Some(surface) = self.surface.clone() else { return Ok(()) };
        let (w, h) = self.size;
        // Dirty region = previous body bounds ∪ current body bounds, padded for anti-aliasing.
        let pad = 3.0;
        let cur = if self.rect.w > 0.0 && self.rect.h > 0.0 {
            Some(RECT {
                left: (self.rect.x - pad).floor().max(0.0) as i32,
                top: (self.rect.y - pad).floor().max(0.0) as i32,
                right: ((self.rect.x + self.rect.w + pad).ceil() as i32).min(w as i32),
                bottom: ((self.rect.y + self.rect.h + pad).ceil() as i32).min(h as i32),
            })
        } else {
            None
        };
        let region = if self.surface_fresh {
            RECT { left: 0, top: 0, right: w as i32, bottom: h as i32 }
        } else { match (self.drawn, cur) {
            (Some(a), Some(b)) => RECT { left: a.left.min(b.left), top: a.top.min(b.top), right: a.right.max(b.right), bottom: a.bottom.max(b.bottom) },
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => return Ok(()),
        } };
        if region.right <= region.left || region.bottom <= region.top {
            return Ok(());
        }
        unsafe {
            let mut offset = POINT::default();
            let tex: ID3D11Texture2D = surface.BeginDraw(Some(&region), &mut offset)?;
            let mut rtv = None;
            self.device.CreateRenderTargetView(&tex, None, Some(&mut rtv))?;
            let rtv = rtv.unwrap();
            let s = self.style;
            let (bw, bh, srv) = match &self.bg {
                Some((_, srv, bw, bh)) => (*bw, *bh, Some(srv.clone())),
                None => (1, 1, None),
            };
            let p = Params {
                rect: [self.rect.x, self.rect.y, self.rect.w, self.rect.h],
                tint: s.tint,
                win_size: [w as f32, h as f32],
                radius: self.rect.radius,
                refraction: s.refraction,
                bg_origin: [0.0, 0.0],
                bg_size: [bw as f32, bh as f32],
                blur_lod: s.blur_lod,
                rim: s.rim,
                dispersion: s.dispersion,
                body_alpha: if srv.is_some() { self.rect.alpha } else { 0.0 },
                shade: s.shade,
            };
            self.ctx.UpdateSubresource(&self.cb, 0, None, &p as *const _ as _, 0, 0);
            self.ctx.OMSetRenderTargets(Some(&[Some(rtv)]), None);
            // Map window px onto the atlas: window (region.left, region.top) lands on `offset`.
            let vp = D3D11_VIEWPORT {
                TopLeftX: (offset.x - region.left) as f32,
                TopLeftY: (offset.y - region.top) as f32,
                Width: w as f32,
                Height: h as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            };
            self.ctx.RSSetViewports(Some(&[vp]));
            // The surface may live in a shared atlas: never touch pixels outside our update rect.
            let sc = RECT { left: offset.x, top: offset.y, right: offset.x + (region.right - region.left), bottom: offset.y + (region.bottom - region.top) };
            self.ctx.RSSetState(&self.raster);
            self.ctx.RSSetScissorRects(Some(&[sc]));
            self.ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.ctx.VSSetShader(&self.vs, None);
            self.ctx.PSSetShader(&self.ps, None);
            self.ctx.PSSetConstantBuffers(0, Some(&[Some(self.cb.clone())]));
            self.ctx.PSSetShaderResources(0, Some(&[srv]));
            self.ctx.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            self.ctx.Draw(3, 0);
            self.ctx.PSSetShaderResources(0, Some(&[None]));
            self.ctx.OMSetRenderTargets(None, None);
            surface.EndDraw()?;
            self.dcomp.Commit()?;
            self.drawn = cur;
            self.surface_fresh = false;
            self.renders += 1;
        }
        Ok(())
    }
}
