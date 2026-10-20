//! What macOS's dynamic store and CoreWLAN report, as the module's link, on
//! every platform's tests; [`super::macos`] reads them.

use std::net::Ipv4Addr;

use lyr_core::network::Link;

/// The default Wi-Fi interface as CoreWLAN reports it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wifi {
    /// Its BSD name, such as `en0`.
    pub interface: String,
    /// The network's name; `None` when macOS withholds it, as it does from a
    /// process without Location permission.
    pub ssid: Option<String>,
    /// The signal, in dBm.
    pub rssi: i64,
}

/// The primary interface's IPv4 state, from the dynamic store.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Primary {
    /// Its BSD name, the global IPv4 entity's `PrimaryInterface`.
    pub interface: String,
    /// The global IPv4 entity's `Router`.
    pub router: Option<String>,
    /// The interface's first address and the subnet mask beside it.
    pub address: Option<(String, String)>,
}

/// waybar's signal strength from dBm (`Network::parseSignal` in waybar
/// 0.15.0's `src/modules/network.cpp`): 100 at −45 dBm, falling by the
/// distance from it over the 45 dB to −90, held to 0 – 100 and cut to a
/// whole number as its `int` does.
#[must_use]
pub fn signal(dbm: i64) -> u8 {
    const OPTIMUM: i64 = -45;
    const MINIMUM: i64 = -90;
    let strength = 100.0 - (dbm - OPTIMUM).abs() as f64 / (OPTIMUM - MINIMUM) as f64 * 100.0;
    // Held to 0 – 100 first, so the cast cannot wrap.
    strength.clamp(0.0, 100.0) as u8
}

/// A dotted subnet mask's prefix length, or `None` for one that is not a
/// mask (its set bits not all leading).
#[must_use]
pub fn prefix(mask: &str) -> Option<u32> {
    let bits = u32::from(mask.parse::<Ipv4Addr>().ok()?);
    let length = bits.leading_ones();
    (bits.checked_shl(length).unwrap_or(0) == 0).then_some(length)
}

/// The link the module shows for `primary`, with Wi-Fi's name and signal
/// when the primary interface is the Wi-Fi one.
#[must_use]
pub fn link(primary: &Primary, wifi: Option<&Wifi>) -> Link {
    let wifi = wifi.filter(|wifi| wifi.interface == primary.interface);
    let (ipaddr, cidr) = primary
        .address
        .as_ref()
        .map(|(address, mask)| {
            let cidr = prefix(mask).map(|length| length.to_string());
            (address.clone(), cidr.unwrap_or_default())
        })
        .unwrap_or_default();
    Link {
        ifname: primary.interface.clone(),
        ipaddr,
        cidr,
        gwaddr: primary.router.clone().unwrap_or_default(),
        essid: wifi.and_then(|wifi| wifi.ssid.clone()).unwrap_or_default(),
        signal_strength: wifi.map_or(0, |wifi| signal(wifi.rssi)),
        wireless: wifi.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Primary, Wifi, link, prefix, signal};

    #[test]
    fn the_signal_is_waybars() {
        assert_eq!(signal(-45), 100, "the optimum");
        assert_eq!(signal(-90), 0);
        assert_eq!(signal(-100), 0, "held at 0");
        assert_eq!(signal(-60), 66, "100 - 15/45*100 = 66.6, cut");
        assert_eq!(signal(-30), 66, "too strong is marked down too");
        assert_eq!(signal(-67), 51);
    }

    #[test]
    fn a_mask_is_its_prefix_length() {
        assert_eq!(prefix("255.255.255.0"), Some(24));
        assert_eq!(prefix("255.255.240.0"), Some(20));
        assert_eq!(prefix("255.255.255.255"), Some(32));
        assert_eq!(prefix("0.0.0.0"), Some(0));
        assert_eq!(prefix("255.0.255.0"), None, "not a mask");
        assert_eq!(prefix("nope"), None);
    }

    fn primary(interface: &str) -> Primary {
        Primary {
            interface: interface.to_owned(),
            router: Some("192.168.1.1".to_owned()),
            address: Some(("192.168.1.20".to_owned(), "255.255.255.0".to_owned())),
        }
    }

    #[test]
    fn a_wired_primary_has_no_wifi_fields() {
        let wifi = Wifi {
            interface: "en1".to_owned(),
            ssid: Some("home".to_owned()),
            rssi: -50,
        };
        let link = link(&primary("en0"), Some(&wifi));
        assert_eq!(
            (
                link.ifname.as_str(),
                link.ipaddr.as_str(),
                link.cidr.as_str()
            ),
            ("en0", "192.168.1.20", "24")
        );
        assert_eq!(link.gwaddr, "192.168.1.1");
        assert_eq!(link.essid, "", "Wi-Fi is another interface");
        assert!(!link.wireless);
    }

    #[test]
    fn a_wifi_primary_has_its_name_or_is_marked_wireless() {
        let mut wifi = Wifi {
            interface: "en0".to_owned(),
            ssid: Some("home".to_owned()),
            rssi: -60,
        };
        let named = link(&primary("en0"), Some(&wifi));
        assert_eq!((named.essid.as_str(), named.signal_strength), ("home", 66));
        assert!(named.wireless);

        wifi.ssid = None;
        let withheld = link(&primary("en0"), Some(&wifi));
        assert_eq!(withheld.essid, "");
        assert!(withheld.wireless, "still Wi-Fi without its name");
    }

    #[test]
    fn no_address_leaves_the_link_waiting() {
        let waiting = Primary {
            address: None,
            ..primary("en0")
        };
        let link = link(&waiting, None);
        assert_eq!((link.ipaddr.as_str(), link.cidr.as_str()), ("", ""));
    }
}
