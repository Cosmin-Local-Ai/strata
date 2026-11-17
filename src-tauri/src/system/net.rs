//! Wi-Fi status through the native WLAN API (wlanapi.dll): SSID and signal quality of the connected
//! interface. Called on demand by the tile (which polls slowly and pauses when hidden). Reports
//! "no Wi-Fi adapter" on PCs without one, so the tile can explain itself (Wi-Fi only on PCs
//! with Wi-Fi).

use serde::Serialize;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::NetworkManagement::WiFi::*;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiState {
    pub adapter: bool,
    pub connected: bool,
    pub ssid: String,
    /// 0..=100 (Windows' signal quality).
    pub signal: u32,
}

pub fn wifi() -> WifiState {
    let mut st = WifiState::default();
    unsafe {
        let mut ver = 0u32;
        let mut h = HANDLE::default();
        if WlanOpenHandle(2, None, &mut ver, &mut h) != 0 {
            return st;
        }
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        if WlanEnumInterfaces(h, None, &mut list) == 0 && !list.is_null() {
            let l = &*list;
            let infos = std::slice::from_raw_parts(l.InterfaceInfo.as_ptr(), l.dwNumberOfItems as usize);
            st.adapter = !infos.is_empty();
            for info in infos {
                if info.isState != wlan_interface_state_connected {
                    continue;
                }
                let mut size = 0u32;
                let mut data: *mut std::ffi::c_void = std::ptr::null_mut();
                if WlanQueryInterface(h, &info.InterfaceGuid, wlan_intf_opcode_current_connection, None, &mut size, &mut data, None) == 0 && !data.is_null() {
                    let c = &*(data as *const WLAN_CONNECTION_ATTRIBUTES);
                    let ss = &c.wlanAssociationAttributes.dot11Ssid;
                    st.ssid = String::from_utf8_lossy(&ss.ucSSID[..ss.uSSIDLength as usize]).to_string();
                    st.signal = c.wlanAssociationAttributes.wlanSignalQuality;
                    st.connected = true;
                    WlanFreeMemory(data);
                    break;
                }
            }
            WlanFreeMemory(list as _);
        }
        WlanCloseHandle(h, None);
    }
    st
}
