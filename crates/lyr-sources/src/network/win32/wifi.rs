//! What Windows' Native Wifi reports for the Wi-Fi menu, and the profile a
//! join writes, on every platform's tests; [`super::super::windows::wlan`]
//! makes the calls. Windows lists networks, one entry per SSID and saved
//! profile, not access points, so an entry's SSID, in hex, stands for its
//! access point's path.

use lyr_core::wifi_menu::{AccessPoint, Profile, Recovery, Security};

/// `DOT11_AUTH_ALGORITHM` and `DOT11_CIPHER_ALGORITHM` values, from
/// Microsoft's `wlantypes.h`, spelled out as plan 11 asks of Windows ABI
/// values so the mapping is tested everywhere.
const AUTH_OPEN: i32 = 1;
const AUTH_SHARED_KEY: i32 = 2;
const AUTH_WPA_PSK: i32 = 4;
const AUTH_WPA_NONE: i32 = 5;
const AUTH_RSNA_PSK: i32 = 7;
const AUTH_WPA3_SAE: i32 = 9;
const AUTH_OWE: i32 = 10;
const CIPHER_TKIP: i32 = 2;
const CIPHER_CCMP: i32 = 4;
const CIPHER_GCMP: i32 = 8;
/// `WLAN_AVAILABLE_NETWORK`'s flags.
const CONNECTED: u32 = 0x1;
const HAS_PROFILE: u32 = 0x2;
/// `WLAN_REASON_CODE_MSMSEC_PSK_MISMATCH_SUSPECTED`: the password is wrong.
pub const PSK_MISMATCH: u32 = 0x48014;

/// One `WLAN_AVAILABLE_NETWORK`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub ssid: Vec<u8>,
    /// `wlanSignalQuality`, 0 – 100.
    pub quality: u32,
    pub auth: i32,
    pub cipher: i32,
    /// `bSecurityEnabled`.
    pub secured: bool,
    /// `dwFlags`.
    pub flags: u32,
    /// `strProfileName`, empty with no saved profile.
    pub profile: String,
}

/// The access point path an SSID stands for: its bytes in hex.
#[must_use]
pub fn point(ssid: &[u8]) -> String {
    ssid.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The SSID [`point`] stands for.
#[must_use]
pub fn ssid_of(point: &str) -> Option<Vec<u8>> {
    if !point.len().is_multiple_of(2) {
        return None;
    }
    (0..point.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(point.get(at..at + 2)?, 16).ok())
        .collect()
}

/// How joining a network that authenticates with `auth` asks for a
/// password, as the menu sees it.
#[must_use]
pub const fn security(auth: i32, secured: bool) -> Security {
    match auth {
        AUTH_OPEN if secured => Security::Wep,
        AUTH_OPEN => Security::Open,
        AUTH_SHARED_KEY => Security::Wep,
        AUTH_WPA_PSK | AUTH_WPA_NONE | AUTH_RSNA_PSK => Security::Psk,
        AUTH_WPA3_SAE => Security::Sae,
        AUTH_OWE => Security::Owe,
        // WPA, RSNA, WPA3 Enterprise and anything newer: 802.1X.
        _ => Security::Enterprise,
    }
}

/// What the menu lists from `entries`: the access points, the one in use,
/// and the saved profiles in range.
#[must_use]
pub fn listing(entries: &[Entry]) -> (Vec<AccessPoint>, Option<String>, Vec<Profile>) {
    let mut points = Vec::new();
    let mut active = None;
    let mut profiles: Vec<Profile> = Vec::new();
    for entry in entries {
        let path = point(&entry.ssid);
        let ssid = String::from_utf8_lossy(&entry.ssid).into_owned();
        if entry.flags & CONNECTED != 0 {
            active = Some(path.clone());
        }
        if entry.flags & HAS_PROFILE != 0
            && !entry.profile.is_empty()
            && !profiles.iter().any(|known| known.path == entry.profile)
        {
            profiles.push(Profile {
                path: entry.profile.clone(),
                ssid: ssid.clone(),
            });
        }
        points.push(AccessPoint {
            path,
            ssid,
            strength: u8::try_from(entry.quality.min(100)).unwrap_or(100),
            security: security(entry.auth, entry.secured),
        });
    }
    (points, active, profiles)
}

/// The profile XML, named `name`, that joins the network `entry` describes
/// with `password`; `None` for a network the menu refuses (WEP, 802.1X) or
/// one whose cipher a profile cannot name. Windows' own dialog names a new
/// profile after its SSID.
#[must_use]
pub fn profile_xml(name: &str, entry: &Entry, password: Option<&str>) -> Option<String> {
    let authentication = match security(entry.auth, entry.secured) {
        Security::Open => "open",
        Security::Owe => "OWE",
        Security::Sae => "WPA3SAE",
        Security::Psk if entry.auth == AUTH_RSNA_PSK => "WPA2PSK",
        Security::Psk => "WPAPSK",
        Security::Wep | Security::Enterprise => return None,
    };
    let encryption = match entry.cipher {
        _ if authentication == "open" => "none",
        CIPHER_TKIP => "TKIP",
        CIPHER_CCMP => "AES",
        CIPHER_GCMP => "GCMP",
        _ => return None,
    };
    let key = match password.filter(|_| security(entry.auth, entry.secured).needs_password()) {
        Some(password) => format!(
            "<sharedKey><keyType>{}</keyType><protected>false</protected>\
             <keyMaterial>{}</keyMaterial></sharedKey>",
            // A WPA-PSK key of 64 hex digits is the key itself.
            if password.len() == 64 && password.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                "networkKey"
            } else {
                "passPhrase"
            },
            escape(password)
        ),
        None => String::new(),
    };
    let ssid = escape(&String::from_utf8_lossy(&entry.ssid));
    let name = escape(name);
    Some(format!(
        "<?xml version=\"1.0\"?>\
         <WLANProfile xmlns=\"http://www.microsoft.com/networking/WLAN/profile/v1\">\
         <name>{name}</name>\
         <SSIDConfig><SSID><hex>{hex}</hex><name>{ssid}</name></SSID></SSIDConfig>\
         <connectionType>ESS</connectionType><connectionMode>auto</connectionMode>\
         <MSM><security><authEncryption><authentication>{authentication}</authentication>\
         <encryption>{encryption}</encryption><useOneX>false</useOneX></authEncryption>\
         {key}</security></MSM></WLANProfile>",
        hex = point(&entry.ssid).to_uppercase(),
    ))
}

/// `text` escaped for XML character data.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// What to do with a profile after joining with it failed for `reason`
/// (a `WLAN_REASON_CODE`): as `lyr_core::wifi_menu::recovery` decides on
/// Linux, with Windows' wrong-password reason standing for missing secrets.
#[must_use]
pub fn recovery(saved: bool, security: Security, reason: u32) -> Recovery {
    let secrets = reason == PSK_MISMATCH;
    if saved {
        if secrets && security.needs_password() {
            Recovery::NewPassword
        } else {
            Recovery::Keep
        }
    } else if secrets || security == Security::Open {
        Recovery::Delete
    } else {
        Recovery::Keep
    }
}

/// A GUID as `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`, which
/// [`super::parse_guid`] reads back.
#[must_use]
pub fn format_guid(guid: u128) -> String {
    let digits = format!("{guid:032x}");
    format!(
        "{{{}-{}-{}-{}-{}}}",
        &digits[..8],
        &digits[8..12],
        &digits[12..16],
        &digits[16..20],
        &digits[20..]
    )
}

#[cfg(test)]
mod tests {
    use lyr_core::wifi_menu::{Recovery, Security};

    use super::super::parse_guid;
    use super::{
        CONNECTED, Entry, HAS_PROFILE, PSK_MISMATCH, format_guid, listing, point, profile_xml,
        recovery, security, ssid_of,
    };

    fn entry(ssid: &str, auth: i32, cipher: i32, secured: bool) -> Entry {
        Entry {
            ssid: ssid.as_bytes().to_vec(),
            quality: 70,
            auth,
            cipher,
            secured,
            flags: 0,
            profile: String::new(),
        }
    }

    #[test]
    fn an_ssid_stands_for_its_point_and_back() {
        assert_eq!(point(b"Home"), "486f6d65");
        assert_eq!(ssid_of("486f6d65").as_deref(), Some(&b"Home"[..]));
        assert_eq!(ssid_of("486"), None, "odd length");
        assert_eq!(ssid_of("zz"), None);
    }

    #[test]
    fn the_auth_algorithm_decides_the_security() {
        assert_eq!(security(1, false), Security::Open);
        assert_eq!(security(1, true), Security::Wep, "open auth with a key");
        assert_eq!(security(2, true), Security::Wep);
        assert_eq!(security(4, true), Security::Psk, "WPA-PSK");
        assert_eq!(security(7, true), Security::Psk, "WPA2-PSK");
        assert_eq!(security(9, true), Security::Sae);
        assert_eq!(security(10, true), Security::Owe);
        assert_eq!(security(3, true), Security::Enterprise, "WPA");
        assert_eq!(security(6, true), Security::Enterprise, "WPA2");
        assert_eq!(security(11, true), Security::Enterprise, "WPA3");
    }

    #[test]
    fn the_listing_keeps_the_connected_point_and_saved_profiles() {
        let mut home = entry("Home", 7, 4, true);
        home.flags = CONNECTED | HAS_PROFILE;
        home.profile = "Home".to_owned();
        let mut cafe = entry("Cafe", 1, 0, false);
        cafe.quality = 140;
        let (points, active, profiles) = listing(&[home.clone(), cafe, home]);
        assert_eq!(points.len(), 3, "the menu merges by SSID itself");
        assert_eq!(active.as_deref(), Some("486f6d65"));
        assert_eq!(profiles.len(), 1, "one profile, listed twice");
        assert_eq!(profiles[0].path, "Home");
        assert_eq!(profiles[0].ssid, "Home");
        assert_eq!(points[1].strength, 100, "quality is held to 100");
        assert_eq!(points[1].security, Security::Open);
        let unsaved = entry("Other", 7, 4, true);
        let (_, active, profiles) = listing(&[unsaved]);
        assert_eq!(active, None);
        assert!(profiles.is_empty());
        let mut saved = entry("Saved", 7, 4, true);
        saved.flags = HAS_PROFILE;
        saved.profile = "Saved".to_owned();
        let (_, active, profiles) = listing(&[saved]);
        assert_eq!(active, None, "saved is not in use");
        assert_eq!(profiles.len(), 1);
    }

    #[test]
    fn a_profile_names_the_network_s_auth_cipher_and_key() {
        let xml = profile_xml(
            "Home & <Co>",
            &entry("Home & <Co>", 7, 4, true),
            Some("pass\"word"),
        )
        .expect("PSK");
        assert_eq!(
            xml.matches("<name>Home &amp; &lt;Co&gt;</name>").count(),
            2,
            "the profile's and the SSID's: {xml}"
        );
        let renamed =
            profile_xml("Mine", &entry("Home", 7, 4, true), Some("password")).expect("PSK");
        assert!(renamed.contains("<name>Mine</name>"), "{renamed}");
        assert!(renamed.contains("<name>Home</name>"), "{renamed}");
        assert!(xml.contains("<hex>486F6D652026203C436F3E</hex>"), "{xml}");
        assert!(xml.contains("<authentication>WPA2PSK</authentication>"));
        assert!(xml.contains("<encryption>AES</encryption>"));
        assert!(xml.contains("<keyType>passPhrase</keyType>"));
        assert!(xml.contains("<keyMaterial>pass&quot;word</keyMaterial>"));
        let wpa = profile_xml("Old", &entry("Old", 4, 2, true), Some("password")).expect("WPA");
        assert!(wpa.contains("<authentication>WPAPSK</authentication>"));
        assert!(wpa.contains("<encryption>TKIP</encryption>"));
        let hex = "a".repeat(64);
        let raw = profile_xml("Raw", &entry("Raw", 7, 4, true), Some(&hex)).expect("PSK");
        assert!(raw.contains("<keyType>networkKey</keyType>"));
        let sae = profile_xml("New", &entry("New", 9, 8, true), Some("x")).expect("SAE");
        assert!(sae.contains("<authentication>WPA3SAE</authentication>"));
        assert!(sae.contains("<encryption>GCMP</encryption>"));
        let open = profile_xml("Cafe", &entry("Cafe", 1, 0, false), Some("ignored")).expect("open");
        assert!(open.contains("<authentication>open</authentication>"));
        assert!(open.contains("<encryption>none</encryption>"));
        assert!(!open.contains("sharedKey"), "an open network takes no key");
        assert!(
            profile_xml("OWE", &entry("OWE", 10, 4, true), None)
                .expect("OWE")
                .contains("<authentication>OWE</authentication>")
        );
    }

    #[test]
    fn refused_networks_and_unknown_ciphers_make_no_profile() {
        assert_eq!(
            profile_xml("WEP", &entry("WEP", 1, 1, true), Some("k")),
            None
        );
        assert_eq!(profile_xml("Corp", &entry("Corp", 6, 4, true), None), None);
        assert_eq!(
            profile_xml("Odd", &entry("Odd", 7, 99, true), Some("password")),
            None
        );
    }

    #[test]
    fn a_wrong_password_offers_a_new_one_or_drops_the_new_profile() {
        assert_eq!(
            recovery(true, Security::Psk, PSK_MISMATCH),
            Recovery::NewPassword
        );
        assert_eq!(recovery(true, Security::Psk, 1), Recovery::Keep);
        assert_eq!(recovery(true, Security::Open, PSK_MISMATCH), Recovery::Keep);
        assert_eq!(
            recovery(false, Security::Psk, PSK_MISMATCH),
            Recovery::Delete
        );
        assert_eq!(recovery(false, Security::Open, 1), Recovery::Delete);
        assert_eq!(recovery(false, Security::Psk, 1), Recovery::Keep);
    }

    #[test]
    fn a_guid_round_trips_through_its_text() {
        let guid = 0x0123_4567_89ab_cdef_0011_2233_4455_6677_u128;
        let text = format_guid(guid);
        assert_eq!(text, "{01234567-89ab-cdef-0011-223344556677}");
        assert_eq!(parse_guid(&text), Some(guid));
    }
}
