//! Audio extras for the Audio Output card: switching the default output device and the per-app mixer.
//!
//! * Default device: Windows has no documented API for this. We use `IPolicyConfig` (undocumented,
//!   stable since Vista; the interface used by EarTrumpet, SoundSwitch and others). Marked UNDOCUMENTED
//!   in docs/parity.md. All three roles (console, multimedia, communications) are switched together,
//!   as Windows' own Sound settings does.
//! * Mixer: IAudioSessionManager2 sessions on the default output, with ISimpleAudioVolume per app.

use serde::Serialize;
use std::ffi::c_void;
use windows::core::{interface, Interface, GUID, HRESULT, HSTRING, IUnknown, IUnknown_Vtbl, PCWSTR};
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

#[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    fn GetMixFormat(&self, id: PCWSTR, fmt: *mut *mut c_void) -> HRESULT;
    fn GetDeviceFormat(&self, id: PCWSTR, default: i32, fmt: *mut *mut c_void) -> HRESULT;
    fn ResetDeviceFormat(&self, id: PCWSTR) -> HRESULT;
    fn SetDeviceFormat(&self, id: PCWSTR, endpoint: *mut c_void, mix: *mut c_void) -> HRESULT;
    fn GetProcessingPeriod(&self, id: PCWSTR, default: i32, a: *mut i64, b: *mut i64) -> HRESULT;
    fn SetProcessingPeriod(&self, id: PCWSTR, a: *mut i64) -> HRESULT;
    fn GetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
    fn SetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
    fn GetPropertyValue(&self, id: PCWSTR, key: *const c_void, v: *mut c_void) -> HRESULT;
    fn SetPropertyValue(&self, id: PCWSTR, key: *const c_void, v: *mut c_void) -> HRESULT;
    fn SetDefaultEndpoint(&self, id: PCWSTR, role: u32) -> HRESULT;
    fn SetEndpointVisibility(&self, id: PCWSTR, visible: i32) -> HRESULT;
}

const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

fn com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

pub fn set_default_output(device_id: &str) -> Result<(), String> {
    com();
    unsafe {
        let pc: IPolicyConfig = CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let id = HSTRING::from(device_id);
        for role in 0..3u32 {
            pc.SetDefaultEndpoint(PCWSTR(id.as_ptr()), role).ok().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub pid: u32,
    pub name: String,
    pub exe: String,
    pub volume: u32,
    pub muted: bool,
}

/// Every active output device: (id, name, endpoint volume).
fn devices() -> Vec<(String, String, windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume, windows::Win32::Media::Audio::IMMDevice)> {
    use windows::Win32::Media::Audio::{DEVICE_STATE_ACTIVE};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::System::Com::STGM_READ;
    com();
    let mut out = vec![];
    unsafe {
        let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return out };
        let Ok(col) = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else { return out };
        for i in 0..col.GetCount().unwrap_or(0) {
            let Ok(d) = col.Item(i) else { continue };
            let id = d.GetId().map(|p| { let s = p.to_string().unwrap_or_default(); windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as _)); s }).unwrap_or_default();
            let name = d.OpenPropertyStore(STGM_READ).and_then(|ps| ps.GetValue(&PKEY_Device_FriendlyName)).map(|v| v.to_string()).unwrap_or_default();
            if let Ok(v) = d.Activate(CLSCTX_ALL, None) { out.push((id, name, v, d)); }
        }
    }
    out
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutDevice {
    pub id: String,
    pub name: String,
    pub default: bool,
    pub volume: u32,
    pub muted: bool,
    /// Something is playing through it right now (peak meter > 0).
    pub active: bool,
}

pub fn out_devices() -> Vec<OutDevice> {
    com();
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    let def = unsafe {
        CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()
            .and_then(|en| en.GetDefaultAudioEndpoint(eRender, eConsole).ok())
            .and_then(|d| d.GetId().ok().map(|p| { let s = p.to_string().unwrap_or_default(); windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as _)); s }))
            .unwrap_or_default()
    };
    devices().into_iter().map(|(id, name, v, d)| unsafe {
        let peak = d.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None).ok().and_then(|m| m.GetPeakValue().ok()).unwrap_or(0.0);
        OutDevice { default: id == def, volume: v.GetMasterVolumeLevelScalar().map(|x| (x * 100.0).round() as u32).unwrap_or(0), muted: v.GetMute().map(|b| b.as_bool()).unwrap_or(false), active: peak > 0.0001, id, name }
    }).collect()
}

pub fn set_device(id: &str, volume: Option<u32>, muted: Option<bool>) -> Result<(), String> {
    let (_, _, v, _) = devices().into_iter().find(|d| d.0 == id).ok_or("device not found")?;
    unsafe {
        if let Some(x) = volume { v.SetMasterVolumeLevelScalar(x.min(100) as f32 / 100.0, std::ptr::null()).map_err(|e| e.to_string())?; }
        if let Some(m) = muted { v.SetMute(m, std::ptr::null()).map_err(|e| e.to_string())?; }
    }
    Ok(())
}

fn sessions_raw() -> Vec<(u32, ISimpleAudioVolume, String)> {
    com();
    let mut out = vec![];
    for (_, _, _, dev) in devices() { unsafe {
        let Ok(mgr) = dev.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { continue };
        let Ok(list) = mgr.GetSessionEnumerator() else { continue };
        for i in 0..list.GetCount().unwrap_or(0) {
            let Ok(ctl) = list.GetSession(i) else { continue };
            let Ok(c2) = ctl.cast::<IAudioSessionControl2>() else { continue };
            let pid = c2.GetProcessId().unwrap_or(0);
            let system = c2.IsSystemSoundsSession() == windows::Win32::Foundation::S_OK;
            let Ok(vol) = ctl.cast::<ISimpleAudioVolume>() else { continue };
            let name = if system { "System sounds".to_string() } else {
                c2.GetDisplayName().ok().map(|p| { let s = p.to_string().unwrap_or_default(); windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as _)); s }).unwrap_or_default()
            };
            out.push((if system { 0 } else { pid }, vol, name));
        }
    } }
    out
}

pub fn sessions() -> Vec<Session> {
    let mut v: Vec<Session> = sessions_raw()
        .into_iter()
        .map(|(pid, vol, name)| unsafe {
            let exe = if pid == 0 { String::new() } else { crate::shell::gamemode::process_path(pid) };
            let pretty = if !name.is_empty() && !name.starts_with('@') { name } else {
                exe.rsplit('\\').next().unwrap_or("App").trim_end_matches(".exe").to_string()
            };
            Session { pid, name: pretty, exe, volume: vol.GetMasterVolume().map(|x| (x * 100.0).round() as u32).unwrap_or(0), muted: vol.GetMute().map(|b| b.as_bool()).unwrap_or(false) }
        })
        .collect();
    // One row per app (an app may have sessions on several devices; setting one sets them all).
    let mut seen = std::collections::HashSet::new();
    v.retain(|s| seen.insert(s.pid));
    v
}

pub fn set_session(pid: u32, volume: Option<u32>, muted: Option<bool>) -> Result<(), String> {
    for (p, vol, _) in sessions_raw() {
        if p != pid {
            continue;
        }
        unsafe {
            if let Some(v) = volume {
                vol.SetMasterVolume(v.min(100) as f32 / 100.0, std::ptr::null()).map_err(|e| e.to_string())?;
            }
            if let Some(m) = muted {
                vol.SetMute(m, std::ptr::null()).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore] // read-only listing of the real devices
    fn live_list() {
        for d in super::out_devices() { println!("DEV {} | default={} vol={} active={}", d.name, d.default, d.volume, d.active); }
        for s in super::sessions() { println!("APP {} pid={} vol={}", s.name, s.pid, s.volume); }
    }
}
