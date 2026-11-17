//! Network card data: the active connection (Ethernet / Wi-Fi, adapter, IPv4, link speed, real internet
//! connectivity) and nearby Wi-Fi networks with connect-to-saved-network.

use super::net::{wifi, WifiState};
use serde::Serialize;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::NetworkManagement::WiFi::*;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetInfo {
    /// "ethernet" | "wifi" | "other" | "none"
    pub kind: String,
    pub adapter: String,
    pub ipv4: String,
    pub link_mbps: u64,
    /// Windows' own judgement (Network List Manager): real internet, not just a link.
    pub internet: bool,
    pub wifi: WifiState,
}

pub fn info() -> NetInfo {
    use windows::Win32::NetworkManagement::IpHelper::*;
    use windows::Win32::Networking::WinSock::{AF_INET, AF_UNSPEC, SOCKADDR_IN};
    let mut out = NetInfo { kind: "none".into(), wifi: wifi(), ..Default::default() };
    unsafe {
        let mut size = 16 * 1024u32;
        let mut buf = vec![0u8; size as usize];
        let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
        let mut r = GetAdaptersAddresses(AF_UNSPEC.0 as u32, flags, None, Some(buf.as_mut_ptr() as _), &mut size);
        if r == 111 {
            buf = vec![0u8; size as usize];
            r = GetAdaptersAddresses(AF_UNSPEC.0 as u32, flags, None, Some(buf.as_mut_ptr() as _), &mut size);
        }
        if r == 0 {
            let mut p = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
            while !p.is_null() {
                let a = &*p;
                // Up and with a default gateway: the adapter actually carrying traffic.
                if a.OperStatus == windows::Win32::NetworkManagement::Ndis::IfOperStatusUp && !a.FirstGatewayAddress.is_null() {
                    out.kind = match a.IfType { 6 => "ethernet", 71 => "wifi", _ => "other" }.into();
                    out.adapter = a.FriendlyName.to_string().unwrap_or_default();
                    out.link_mbps = a.TransmitLinkSpeed / 1_000_000;
                    let mut u = a.FirstUnicastAddress;
                    while !u.is_null() {
                        let sa = (*u).Address.lpSockaddr;
                        if !sa.is_null() && (*sa).sa_family == AF_INET {
                            let v4 = &*(sa as *const SOCKADDR_IN);
                            let b = v4.sin_addr.S_un.S_un_b;
                            out.ipv4 = format!("{}.{}.{}.{}", b.s_b1, b.s_b2, b.s_b3, b.s_b4);
                            break;
                        }
                        u = (*u).Next;
                    }
                    break;
                }
                p = a.Next;
            }
        }
        use windows::Win32::Networking::NetworkListManager::{INetworkListManager, NetworkListManager, NLM_CONNECTIVITY_IPV4_INTERNET, NLM_CONNECTIVITY_IPV6_INTERNET};
        use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(nlm) = CoCreateInstance::<_, INetworkListManager>(&NetworkListManager, None, CLSCTX_ALL) {
            if let Ok(c) = nlm.GetConnectivity() {
                out.internet = (c.0 & (NLM_CONNECTIVITY_IPV4_INTERNET.0 | NLM_CONNECTIVITY_IPV6_INTERNET.0)) != 0;
            }
        }
    }
    out
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u32,
    pub secured: bool,
    pub connected: bool,
    /// Windows has a saved profile, so we can connect without asking for a password.
    pub saved: bool,
}

/// Nearby Wi-Fi networks (asks the adapter to scan; results are from the latest scan).
pub fn networks() -> Vec<WifiNetwork> {
    let mut out: Vec<WifiNetwork> = Vec::new();
    unsafe {
        let mut ver = 0u32;
        let mut h = HANDLE::default();
        if WlanOpenHandle(2, None, &mut ver, &mut h) != 0 {
            return out;
        }
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        if WlanEnumInterfaces(h, None, &mut list) == 0 && !list.is_null() {
            let l = &*list;
            for info in std::slice::from_raw_parts(l.InterfaceInfo.as_ptr(), l.dwNumberOfItems as usize) {
                let _ = WlanScan(h, &info.InterfaceGuid, None, None, None);
                let mut nets: *mut WLAN_AVAILABLE_NETWORK_LIST = std::ptr::null_mut();
                if WlanGetAvailableNetworkList(h, &info.InterfaceGuid, 0, None, &mut nets) == 0 && !nets.is_null() {
                    let n = &*nets;
                    for net in std::slice::from_raw_parts(n.Network.as_ptr(), n.dwNumberOfItems as usize) {
                        let ssid = String::from_utf8_lossy(&net.dot11Ssid.ucSSID[..net.dot11Ssid.uSSIDLength as usize]).to_string();
                        if ssid.is_empty() {
                            continue;
                        }
                        let connected = net.dwFlags & WLAN_AVAILABLE_NETWORK_CONNECTED != 0;
                        let saved = net.dwFlags & WLAN_AVAILABLE_NETWORK_HAS_PROFILE != 0;
                        match out.iter_mut().find(|x| x.ssid == ssid) {
                            Some(x) => {
                                x.signal = x.signal.max(net.wlanSignalQuality);
                                x.connected |= connected;
                                x.saved |= saved;
                            }
                            None => out.push(WifiNetwork { ssid, signal: net.wlanSignalQuality, secured: net.bSecurityEnabled.as_bool(), connected, saved }),
                        }
                    }
                    WlanFreeMemory(nets as _);
                }
            }
            WlanFreeMemory(list as _);
        }
        WlanCloseHandle(h, None);
    }
    out.sort_by(|a, b| b.connected.cmp(&a.connected).then(b.signal.cmp(&a.signal)));
    out
}

/// Connect to a network with a saved Windows profile. New networks need a password; the card opens
/// Windows' own network flyout for those rather than handling Wi-Fi passwords itself.
pub fn connect(ssid: &str) -> Result<(), String> {
    use windows::core::HSTRING;
    unsafe {
        let mut ver = 0u32;
        let mut h = HANDLE::default();
        if WlanOpenHandle(2, None, &mut ver, &mut h) != 0 {
            return Err("No Wi-Fi adapter".into());
        }
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        let mut result = Err("No Wi-Fi adapter".to_string());
        if WlanEnumInterfaces(h, None, &mut list) == 0 && !list.is_null() {
            let l = &*list;
            if let Some(info) = std::slice::from_raw_parts(l.InterfaceInfo.as_ptr(), l.dwNumberOfItems as usize).first() {
                let profile = HSTRING::from(ssid);
                let params = WLAN_CONNECTION_PARAMETERS {
                    wlanConnectionMode: wlan_connection_mode_profile,
                    strProfile: windows::core::PCWSTR(profile.as_ptr()),
                    dot11BssType: dot11_BSS_type_any,
                    ..Default::default()
                };
                let r = WlanConnect(h, &info.InterfaceGuid, &params, None);
                result = if r == 0 { Ok(()) } else { Err(format!("Windows couldn't connect to {ssid} (error {r})")) };
            }
            WlanFreeMemory(list as _);
        }
        WlanCloseHandle(h, None);
        result
    }
}
