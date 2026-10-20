//! What Windows reports about the interface with the default route, as the
//! network module's link, on every platform's tests; [`super::windows`]
//! reads it through IP Helper and the WLAN API.

use std::net::Ipv4Addr;

use lyr_core::network::Link;

/// An adapter's facts from `GetAdaptersAddresses`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Adapter {
    /// `FriendlyName`, such as "Wi-Fi" or "Ethernet": what waybar's `ifname`
    /// is on Linux.
    pub name: String,
    /// `IfType` is `IF_TYPE_IEEE80211`.
    pub wifi: bool,
    /// `OperStatus` is `IfOperStatusUp`.
    pub up: bool,
    /// The first IPv4 address and its on-link prefix length.
    pub address: Option<(Ipv4Addr, u8)>,
    /// The first IPv4 gateway.
    pub gateway: Option<Ipv4Addr>,
}

/// `DOT11_RADIO_STATE`'s off, from Microsoft's reference, spelled out as
/// plan 11 asks of Windows ABI values.
const RADIO_OFF: i32 = 2;

/// Whether a Wi-Fi interface's radio is off: every one of its phys has its
/// software or hardware switch off (`WLAN_PHY_RADIO_STATE`'s
/// `dot11SoftwareRadioState` and `dot11HardwareRadioState`, as pairs). An
/// interface that reports no phys is not called off.
#[must_use]
pub fn radio_off(phys: &[(i32, i32)]) -> bool {
    !phys.is_empty()
        && phys
            .iter()
            .all(|&(software, hardware)| software == RADIO_OFF || hardware == RADIO_OFF)
}

/// A Wi-Fi interface's current connection, from `WlanQueryInterface`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wifi {
    pub ssid: Vec<u8>,
    /// `wlanSignalQuality`, 0 – 100.
    pub quality: u32,
}

/// The link the module shows for `adapter`, with `wifi`'s network when it
/// is a Wi-Fi adapter; `None` while the adapter is not up, which the module
/// shows as disconnected.
#[must_use]
pub fn link(adapter: &Adapter, wifi: Option<&Wifi>) -> Option<Link> {
    if !adapter.up {
        return None;
    }
    let (ipaddr, cidr) = adapter
        .address
        .map(|(address, prefix)| (address.to_string(), prefix.to_string()))
        .unwrap_or_default();
    let wifi = wifi.filter(|_| adapter.wifi);
    Some(Link {
        ifname: adapter.name.clone(),
        ipaddr,
        cidr,
        gwaddr: adapter
            .gateway
            .map(|gateway| gateway.to_string())
            .unwrap_or_default(),
        essid: wifi
            .map(|wifi| String::from_utf8_lossy(&wifi.ssid).into_owned())
            .unwrap_or_default(),
        signal_strength: wifi.map_or(0, |wifi| wifi.quality.min(100) as u8),
        wireless: false,
    })
}

/// A GUID as `GetAdaptersAddresses` names an adapter, `{xxxxxxxx-xxxx-xxxx-
/// xxxx-xxxxxxxxxxxx}`, as the 128-bit number its digits spell, which is what
/// `GUID::from_u128` takes. `None` for anything else.
#[must_use]
pub fn parse_guid(text: &str) -> Option<u128> {
    let inner = text.strip_prefix('{')?.strip_suffix('}')?;
    let groups: Vec<&str> = inner.split('-').collect();
    let lengths: Vec<usize> = groups.iter().map(|group| group.len()).collect();
    if lengths != [8, 4, 4, 4, 12] {
        return None;
    }
    let digits: String = groups.concat();
    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u128::from_str_radix(&digits, 16).ok()
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::{Adapter, Wifi, link, parse_guid};

    #[test]
    fn a_radio_is_off_when_every_phy_is_switched_off() {
        use super::radio_off;
        // DOT11_RADIO_STATE: 1 on, 2 off, 0 unknown.
        assert!(!radio_off(&[]), "no phys reported");
        assert!(!radio_off(&[(1, 1)]));
        assert!(radio_off(&[(2, 1)]), "switched off in Windows");
        assert!(radio_off(&[(1, 2)]), "a hardware switch");
        assert!(!radio_off(&[(2, 1), (1, 1)]), "one phy still on");
        assert!(!radio_off(&[(0, 0)]), "unknown is not off");
    }

    fn wifi_adapter() -> Adapter {
        Adapter {
            name: "Wi-Fi".to_owned(),
            wifi: true,
            up: true,
            address: Some((Ipv4Addr::new(192, 168, 1, 20), 24)),
            gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
        }
    }

    #[test]
    fn a_wifi_adapter_carries_its_network() {
        let network = Wifi {
            ssid: b"home".to_vec(),
            quality: 71,
        };
        let link = link(&wifi_adapter(), Some(&network)).expect("up");
        assert_eq!(link.ifname, "Wi-Fi");
        assert_eq!(
            (link.ipaddr.as_str(), link.cidr.as_str()),
            ("192.168.1.20", "24")
        );
        assert_eq!(link.gwaddr, "192.168.1.1");
        assert_eq!((link.essid.as_str(), link.signal_strength), ("home", 71));
    }

    #[test]
    fn a_wired_or_waiting_or_down_adapter() {
        let wired = Adapter {
            wifi: false,
            name: "Ethernet".to_owned(),
            ..wifi_adapter()
        };
        let stray = Wifi {
            ssid: b"x".to_vec(),
            quality: 200,
        };
        let link_of = |adapter: &Adapter| link(adapter, Some(&stray));
        assert_eq!(
            link_of(&wired).expect("up").essid,
            "",
            "no network on a wire"
        );
        assert_eq!(link_of(&wifi_adapter()).expect("up").signal_strength, 100);
        let waiting = Adapter {
            address: None,
            gateway: None,
            ..wired.clone()
        };
        let waiting = link_of(&waiting).expect("up");
        assert_eq!((waiting.ipaddr.as_str(), waiting.gwaddr.as_str()), ("", ""));
        let down = Adapter { up: false, ..wired };
        assert_eq!(link_of(&down), None);
    }

    #[test]
    fn an_adapter_guid_reads_as_its_digits() {
        assert_eq!(
            parse_guid("{4D36E972-E325-11CE-BFC1-08002BE10318}"),
            Some(0x4D36_E972_E325_11CE_BFC1_0800_2BE1_0318)
        );
        assert_eq!(
            parse_guid("4D36E972-E325-11CE-BFC1-08002BE10318"),
            None,
            "no braces"
        );
        assert_eq!(
            parse_guid("{4D36E972-E325-11CE-BFC1-08002BE1031}"),
            None,
            "short"
        );
        assert_eq!(
            parse_guid("{4D36E972-E325-11CE-BFC1-08002BE1031G}"),
            None,
            "not hex"
        );
        assert_eq!(
            parse_guid("{+D36E972-E325-11CE-BFC1-08002BE10318}"),
            None,
            "a sign, which from_str_radix would take"
        );
        assert_eq!(
            parse_guid("{4D36E972E325-11CE-BFC1-08002BE10318-}"),
            None,
            "grouped wrong"
        );
    }
}
