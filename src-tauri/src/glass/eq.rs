//! Native "now playing" equaliser bars.
//!
//! Drawn as DirectComposition visuals on a second composition target that sits *above* the WebView
//! (topmost = true), over the cover art the web UI points us at. Heights are animated by DWM itself:
//! every update hands DWM a short ramp (IDCompositionAnimation) from the current height to the new
//! level, so motion is smooth at the display's refresh rate while Strata and Chromium do no per-frame
//! work. A CSS animation of the same bars cost ~13% of a core on the 7680 px dock; this costs ~0.
//! Levels come from the real audio output (see audio::peak), so the bars react to the music.

use super::{Glass, Inner};
use windows::core::Result;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::Win32::Graphics::DirectComposition::{IDCompositionScaleTransform, IDCompositionTarget, IDCompositionVisual};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM};

pub(super) struct Eq {
    _target: IDCompositionTarget,
    root: IDCompositionVisual,
    bars: Vec<(IDCompositionVisual, IDCompositionScaleTransform, f32)>,
    rect: Option<[f32; 4]>,
    shown: bool,
}

const BARS: usize = 3;

impl Inner {
    fn ensure_eq(&mut self) -> Result<()> {
        if self.eq.is_some() {
            return Ok(());
        }
        unsafe {
            // Second target on the same window, composed above child windows (the WebView).
            let target = self.dcomp.CreateTargetForHwnd(self.hwnd, true)?;
            let root = self.dcomp.CreateVisual()?;
            target.SetRoot(&root)?;
            let mut bars = Vec::new();
            for _ in 0..BARS {
                let v = self.dcomp.CreateVisual()?;
                let t = self.dcomp.CreateScaleTransform()?;
                v.SetTransform(&t)?;
                root.AddVisual(&v, true, None)?;
                bars.push((v, t, 0.3));
            }
            self.eq = Some(Eq { _target: target, root, bars, rect: None, shown: false });
        }
        Ok(())
    }

    /// Bar geometry for `rect` (cover art, window px): three white bars in its bottom-right corner.
    fn layout_eq(&mut self, rect: [f32; 4]) -> Result<()> {
        self.ensure_eq()?;
        // Three bars across the slot's width, full slot height, bottom-aligned.
        let gap = (rect[2] * 0.12).round().max(1.0);
        let bw = ((rect[2] - 2.0 * gap) / 3.0).floor().max(2.0);
        let bh = rect[3].round().max(4.0);
        let (bw_i, bh_i) = (bw as u32, bh as u32);
        let dcomp = self.dcomp.clone();
        let device = self.device.clone();
        let ctx = self.ctx.clone();
        let eq = self.eq.as_mut().unwrap();
        unsafe {
            let total_w = bw * BARS as f32 + gap * (BARS as f32 - 1.0);
            let x0 = (rect[0] + (rect[2] - total_w) / 2.0).round();
            let y0 = (rect[1] + rect[3] - bh).round();
            for (i, (v, t, _)) in eq.bars.iter().enumerate() {
                // White bar with a faint dark edge for contrast on light covers (no box behind it).
                let s = dcomp.CreateSurface(bw_i, bh_i, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ALPHA_MODE_PREMULTIPLIED)?;
                let mut off = POINT::default();
                let tex: ID3D11Texture2D = s.BeginDraw(None, &mut off)?;
                // Fill the bar's rectangle in the (possibly shared) atlas texture with near-white,
                // premultiplied BGRA. A plain upload works on every device (WARP included).
                let px = vec![0xF4u8; (bw_i * bh_i * 4) as usize];
                let bx = windows::Win32::Graphics::Direct3D11::D3D11_BOX {
                    left: off.x as u32, top: off.y as u32, front: 0, right: off.x as u32 + bw_i, bottom: off.y as u32 + bh_i, back: 1,
                };
                ctx.UpdateSubresource(&tex, 0, Some(&bx), px.as_ptr() as _, bw_i * 4, 0);
                let _ = &device;
                s.EndDraw()?;
                v.SetContent(&s)?;
                v.SetOffsetX2(x0 + i as f32 * (bw + gap))?;
                v.SetOffsetY2(y0)?;
                t.SetCenterY2(bh)?;
            }
            eq.rect = Some(rect);
        }
        log::info!("equaliser placed at {:?} (bar {}x{} px)", rect, bw_i, bh_i);
        Ok(())
    }

    fn show_eq(&mut self, on: bool) -> Result<()> {
        let Some(eq) = self.eq.as_mut() else { return Ok(()) };
        if eq.shown == on {
            return Ok(());
        }
        // IDCompositionVisual (v1) has no opacity; hidden = moved off-screen.
        unsafe {
            eq.root.SetOffsetY2(if on { 0.0 } else { -10_000.0 })?;
        }
        eq.shown = on;
        self.dcomp_commit()
    }

    fn dcomp_commit(&self) -> Result<()> {
        unsafe { self.dcomp.Commit() }
    }
}

impl Glass {
    /// Where the cover art is (window px), or None to hide the bars.
    pub fn set_eq_rect(&self, rect: Option<[f32; 4]>) {
        let mut g = self.0.lock().unwrap();
        let r = match rect {
            Some(r) => {
                let same = g.eq.as_ref().and_then(|e| e.rect) == Some(r);
                (if same { Ok(()) } else { g.layout_eq(r) }).and_then(|_| g.show_eq(true))
            }
            None => g.show_eq(false),
        };
        if let Err(e) = r {
            log::warn!("equaliser: {e}");
        }
    }

    /// New levels (0..1 per bar). DWM ramps each bar to its level over `ms` milliseconds.
    pub fn set_eq_levels(&self, levels: [f32; BARS], ms: f32) {
        let mut g = self.0.lock().unwrap();
        if g.yielding {
            return;
        }
        let dcomp = g.dcomp.clone();
        let Some(eq) = g.eq.as_mut() else { return };
        if !eq.shown {
            return;
        }
        let d = (ms / 1000.0) as f64;
        {
            use std::sync::atomic::{AtomicU32, Ordering};
            static N: AtomicU32 = AtomicU32::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            if n % 200 == 0 {
                log::info!("equaliser levels {:?} shown={} (update #{n})", levels, eq.shown);
            }
        }
        unsafe {
            for (i, (_, t, cur)) in eq.bars.iter_mut().enumerate() {
                // Never below a third: idle bars stay visible as a small "playing" mark.
                let target = levels[i].clamp(0.33, 1.0);
                if let Ok(a) = dcomp.CreateAnimation() {
                    let _ = a.AddCubic(0.0, *cur, (target - *cur) / d as f32, 0.0, 0.0);
                    let _ = a.End(d, target);
                    let _ = t.SetScaleY(&a);
                }
                *cur = target;
            }
        }
        let _ = g.dcomp_commit();
    }
}
