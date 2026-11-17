//! Audio output: default playback device volume and mute through Core Audio (IAudioEndpointVolume),
//! plus the device list. Change-driven: Windows calls us back when the volume changes anywhere, and
//! when the default device changes. Runs on its own MTA thread (Core Audio callbacks are free-threaded).

use serde::Serialize;
use std::sync::{Arc, Mutex};
use windows::core::{implement, PCWSTR};
use windows::Win32::Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl, IAudioMeterInformation};
use windows::Win32::Media::Audio::{
    eConsole, eRender, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator,
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::PROPERTYKEY;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub default: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    pub available: bool,
    /// 0..=100
    pub volume: u32,
    pub muted: bool,
    pub device: String,
    pub devices: Vec<AudioDevice>,
}

type Emit = Arc<dyn Fn(&AudioState) + Send + Sync>;

struct Inner {
    enumerator: Option<IMMDeviceEnumerator>,
    endpoint: Option<IAudioEndpointVolume>,
    callback: Option<IAudioEndpointVolumeCallback>,
    meter: Option<IAudioMeterInformation>,
    last: AudioState,
}

#[derive(Clone)]
pub struct Audio(Arc<Mutex<Inner>>, Emit);
unsafe impl Send for Inner {}

#[implement(IAudioEndpointVolumeCallback)]
struct VolCb(Audio);
impl IAudioEndpointVolumeCallback_Impl for VolCb_Impl {
    fn OnNotify(&self, _data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        let a = self.0.clone();
        // Never call back into Core Audio from inside its own callback.
        std::thread::spawn(move || a.refresh());
        Ok(())
    }
}

/// Bumped on every audio device change; cached meters are re-resolved only then.
static DEVICE_GEN: std::sync::Mutex<u64> = std::sync::Mutex::new(0);

#[implement(IMMNotificationClient)]
struct DevCb(Audio);
impl IMMNotificationClient_Impl for DevCb_Impl {
    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _s: DEVICE_STATE) -> windows::core::Result<()> { self.kick(); Ok(()) }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> windows::core::Result<()> { self.kick(); Ok(()) }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> windows::core::Result<()> { self.kick(); Ok(()) }
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _id: &PCWSTR) -> windows::core::Result<()> {
        if flow == eRender && role == eConsole { self.kick(); }
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> windows::core::Result<()> { Ok(()) }
}
impl DevCb_Impl {
    fn kick(&self) {
        *DEVICE_GEN.lock().unwrap() += 1;
        let a = self.0.clone();
        std::thread::spawn(move || {
            unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
            a.bind_default();
            a.refresh();
        });
    }
}

fn device_name(d: &IMMDevice) -> String {
    unsafe {
        d.OpenPropertyStore(STGM_READ)
            .and_then(|ps| ps.GetValue(&PKEY_Device_FriendlyName))
            .map(|v| v.to_string())
            .unwrap_or_default()
    }
}

fn device_id(d: &IMMDevice) -> String {
    unsafe { d.GetId().map(|p| { let s = p.to_string().unwrap_or_default(); windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as _)); s }).unwrap_or_default() }
}

impl Audio {
    pub fn start(emit: impl Fn(&AudioState) + Send + Sync + 'static) -> Audio {
        let a = Audio(Arc::new(Mutex::new(Inner { enumerator: None, endpoint: None, callback: None, meter: None, last: AudioState::default() })), Arc::new(emit));
        let me = a.clone();
        std::thread::Builder::new().name("strata-audio".into()).spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else {
                log::warn!("Core Audio unavailable");
                return;
            };
            let dev_cb: IMMNotificationClient = DevCb(me.clone()).into();
            let _ = en.RegisterEndpointNotificationCallback(&dev_cb);
            me.0.lock().unwrap().enumerator = Some(en);
            me.bind_default();
            me.refresh();
            // Keep the notification client alive for the life of the process.
            std::mem::forget(dev_cb);
        }).expect("audio thread");
        a
    }

    fn bind_default(&self) {
        let mut g = self.0.lock().unwrap();
        unsafe {
            if let (Some(ep), Some(cb)) = (g.endpoint.take(), g.callback.take()) {
                let _ = ep.UnregisterControlChangeNotify(&cb);
            }
            let Some(en) = g.enumerator.clone() else { return };
            let Ok(dev) = en.GetDefaultAudioEndpoint(eRender, eConsole) else { return };
            let Ok(ep) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) else { return };
            let cb: IAudioEndpointVolumeCallback = VolCb(self.clone()).into();
            let _ = ep.RegisterControlChangeNotify(&cb);
            g.endpoint = Some(ep);
            g.callback = Some(cb);
            g.meter = dev.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None).ok();
        }
    }

    fn refresh(&self) {
        let st = {
            let g = self.0.lock().unwrap();
            let mut st = AudioState::default();
            unsafe {
                if let Some(ep) = &g.endpoint {
                    st.available = true;
                    st.volume = ep.GetMasterVolumeLevelScalar().map(|v| (v * 100.0).round() as u32).unwrap_or(0);
                    st.muted = ep.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                }
                if let Some(en) = &g.enumerator {
                    let def = en.GetDefaultAudioEndpoint(eRender, eConsole).ok().map(|d| device_id(&d)).unwrap_or_default();
                    if let Ok(col) = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
                        for i in 0..col.GetCount().unwrap_or(0) {
                            if let Ok(d) = col.Item(i) {
                                let id = device_id(&d);
                                let name = device_name(&d);
                                if id == def { st.device = name.clone(); }
                                st.devices.push(AudioDevice { default: id == def, id, name });
                            }
                        }
                    }
                }
            }
            st
        };
        let changed = {
            let mut g = self.0.lock().unwrap();
            let c = g.last != st;
            g.last = st.clone();
            c
        };
        if changed {
            (self.1)(&st);
        }
    }

    /// Peak (0..1) of the audio session belonging to the app whose name contains `app_hint` (e.g.
    /// "spotify"), on whichever output device it plays to. Falls back to the default device's meter.
    /// The session meter is cached and re-resolved every few seconds.
    pub fn peak_for(&self, app_hint: &str) -> Option<(f32, f32)> {
        use windows::Win32::Media::Audio::{IAudioSessionControl2, IAudioSessionManager2};
        use windows::core::Interface;
        thread_local! {
            static CACHE: std::cell::RefCell<(Option<IAudioMeterInformation>, u64, String)> =
                std::cell::RefCell::new((None, u64::MAX, String::new()));
        }
        let hint = app_hint.to_lowercase();
        let cached = CACHE.with(|c| {
            let c = c.borrow();
            // Re-resolve only when the app changes, or after a device change (generation bump).
            if c.2 == hint && c.1 == *DEVICE_GEN.lock().unwrap() { c.0.clone() } else { None }
        });
        let meter = cached.or_else(|| {
            let en = self.0.lock().unwrap().enumerator.clone()?;
            let mut found: Option<IAudioMeterInformation> = None;
            unsafe {
                let col = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE).ok()?;
                'dev: for i in 0..col.GetCount().unwrap_or(0) {
                    let Ok(d) = col.Item(i) else { continue };
                    let Ok(mgr) = d.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { continue };
                    let Ok(sessions) = mgr.GetSessionEnumerator() else { continue };
                    for j in 0..sessions.GetCount().unwrap_or(0) {
                        let Ok(ctl) = sessions.GetSession(j) else { continue };
                        let Ok(ctl2) = ctl.cast::<IAudioSessionControl2>() else { continue };
                        let pid = ctl2.GetProcessId().unwrap_or(0);
                        if pid == 0 { continue; }
                        let name = crate::shell::gamemode::process_name(pid).to_lowercase();
                        log::debug!("audio session pid {pid} {name} on {}", device_name(&d));
                        if !hint.is_empty() && name.contains(&hint) {
                            found = ctl.cast::<IAudioMeterInformation>().ok();
                            if found.is_some() { break 'dev; }
                        }
                    }
                }
            }
            log::info!("equaliser source for '{hint}': {}", if found.is_some() { "app session meter" } else { "default device meter (app session not found)" });
            CACHE.with(|c| *c.borrow_mut() = (found.clone(), *DEVICE_GEN.lock().unwrap(), hint.clone()));
            found
        });
        // The app's own session first. Virtual mixers (SteelSeries Sonar, Voicemeeter) report silence
        // on their virtual endpoints, so fall back to the loudest physical output: the final mix.
        let app = meter.and_then(|m| unsafe { m.GetPeakValue().ok() }).unwrap_or(0.0);
        if app > 1e-4 {
            return Some((app, app));
        }
        self.loudest_output().or_else(|| self.peak())
    }

    /// Peak of the loudest active output device (meters cached, refreshed every 5 s).
    fn loudest_output(&self) -> Option<(f32, f32)> {
        thread_local! {
            static METERS: std::cell::RefCell<(Vec<IAudioMeterInformation>, u64)> =
                std::cell::RefCell::new((Vec::new(), u64::MAX));
        }
        let generation = *DEVICE_GEN.lock().unwrap();
        let stale = METERS.with(|m| m.borrow().1 != generation);
        if stale {
            let en = self.0.lock().unwrap().enumerator.clone()?;
            let mut v = Vec::new();
            unsafe {
                if let Ok(col) = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
                    for i in 0..col.GetCount().unwrap_or(0) {
                        if let Ok(d) = col.Item(i) {
                            if let Ok(m) = d.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None) {
                                v.push(m);
                            }
                        }
                    }
                }
            }
            METERS.with(|m| *m.borrow_mut() = (v, generation));
        }
        METERS.with(|m| {
            let best = m.borrow().0.iter().filter_map(|x| unsafe { x.GetPeakValue().ok() }).fold(0f32, f32::max);
            Some((best, best))
        })
    }

    /// Current output peak per channel (0..1), from Windows' audio meter. Cheap (no capture).
    pub fn peak(&self) -> Option<(f32, f32)> {
        let m = self.0.lock().unwrap().meter.clone()?;
        unsafe {
            let n = m.GetMeteringChannelCount().unwrap_or(0);
            if n >= 2 {
                let mut v = [0f32; 2];
                if m.GetChannelsPeakValues(&mut v).is_ok() {
                    return Some((v[0], v[1]));
                }
            }
            m.GetPeakValue().ok().map(|p| (p, p))
        }
    }

    pub fn state(&self) -> AudioState {
        self.0.lock().unwrap().last.clone()
    }

    pub fn set_volume(&self, v: u32) -> Result<(), String> {
        unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
        let ep = self.0.lock().unwrap().endpoint.clone().ok_or("no audio device")?;
        unsafe { ep.SetMasterVolumeLevelScalar(v.min(100) as f32 / 100.0, std::ptr::null()).map_err(|e| e.to_string()) }
    }

    pub fn set_mute(&self, m: bool) -> Result<(), String> {
        unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
        let ep = self.0.lock().unwrap().endpoint.clone().ok_or("no audio device")?;
        unsafe { ep.SetMute(m, std::ptr::null()).map_err(|e| e.to_string()) }
    }
}

