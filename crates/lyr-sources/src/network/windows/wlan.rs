//! The Wi-Fi menu's source on Windows, through Native Wifi: the first
//! wireless interface, its radio, the networks its last scan found and the
//! saved profiles among them ([`super::super::win32::wifi`] shapes them), and
//! the menu's actions: scan, switch the radio, join (writing a profile for
//! a new network), leave and forget. [`Client::watch`] reports the WLAN
//! service's scan and connection notifications.
//!
//! Checked on a machine with no wireless interface only
//! (`docs/backlog.md`).

use std::ffi::c_void;
use std::io;
use std::ptr;
use std::time::Duration;

use lyr_core::wifi_menu::{Radio, Wifi};
use windows_sys::Win32::Foundation::NO_ERROR;
use windows_sys::Win32::NetworkManagement::WiFi::{
    L2_NOTIFICATION_DATA, WLAN_AVAILABLE_NETWORK, WLAN_AVAILABLE_NETWORK_LIST,
    WLAN_CONNECTION_NOTIFICATION_DATA, WLAN_CONNECTION_PARAMETERS, WLAN_INTERFACE_INFO,
    WLAN_INTERFACE_INFO_LIST, WLAN_NOTIFICATION_SOURCE_ACM, WLAN_NOTIFICATION_SOURCE_MSM,
    WLAN_PHY_RADIO_STATE, WlanConnect, WlanDeleteProfile, WlanDisconnect, WlanEnumInterfaces,
    WlanFreeMemory, WlanGetAvailableNetworkList, WlanReasonCodeToString, WlanRegisterNotification,
    WlanScan, WlanSetInterface, WlanSetProfile, dot11_BSS_type_infrastructure,
    wlan_connection_mode_profile, wlan_interface_state_not_ready, wlan_intf_opcode_radio_state,
    wlan_notification_acm_connection_attempt_fail, wlan_notification_acm_connection_complete,
    wlan_notification_acm_scan_complete, wlan_notification_acm_scan_fail,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::core::GUID;

use super::super::win32::wifi::{Entry, format_guid, listing};
use super::{Wlan, radio_state};

/// `DOT11_RADIO_STATE`'s on and off, from Microsoft's reference.
const RADIO_ON: i32 = 1;
const RADIO_OFF: i32 = 2;
/// `WlanReasonCodeToString`'s buffer, in UTF-16 units.
const REASON_TEXT: usize = 512;

/// How long the machine has been up, the clock [`Wifi::last_scan`] is
/// kept on.
#[must_use]
pub fn since_boot() -> Duration {
    // SAFETY: no arguments, no failure.
    Duration::from_millis(unsafe { GetTickCount64() })
}

/// What the WLAN service reports to [`Client::watch`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// A scan ended, or failed.
    Scanned,
    /// The interface joined the network with this SSID.
    Joined(Vec<u8>),
    /// Joining the network with this SSID failed, for this
    /// `WLAN_REASON_CODE`.
    Failed(Vec<u8>, u32),
    /// Something else changed: the radio, the connection, a profile.
    Changed,
}

/// What a notice calls.
type OnNotice = Box<dyn Fn(Notice) + Send + Sync>;

/// A WLAN client handle and the callback its notifications call.
pub struct Client {
    // Declared first, so it closes first: closing waits for a callback under
    // way, which reads `on_notice`.
    wlan: Wlan,
    on_notice: Option<Box<OnNotice>>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("watching", &self.on_notice.is_some())
            .finish_non_exhaustive()
    }
}

/// A Win32 error code as an `io::Error`, `Ok` for success.
fn check(error: u32) -> io::Result<()> {
    if error == NO_ERROR {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(error as i32))
    }
}

/// `text` as a terminated UTF-16 string.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// A fixed UTF-16 field up to its terminator.
fn narrow(field: &[u16]) -> String {
    let end = field
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(field.len());
    String::from_utf16_lossy(&field[..end])
}

/// An SSID's bytes from its `DOT11_SSID`.
fn ssid(length: u32, bytes: &[u8; 32]) -> Vec<u8> {
    bytes[..(length as usize).min(bytes.len())].to_vec()
}

impl Client {
    /// A client of the WLAN service; `None` on a machine without it.
    #[must_use]
    pub fn open() -> Option<Self> {
        Some(Self {
            wlan: Wlan::open()?,
            on_notice: None,
        })
    }

    /// The first wireless interface, and whether it is ready; `None` with
    /// none.
    fn interface(&self) -> Option<(GUID, bool)> {
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = ptr::null_mut();
        // SAFETY: the call allocates the list into a local.
        if unsafe { WlanEnumInterfaces(self.wlan.0, ptr::null(), &raw mut list) } != NO_ERROR
            || list.is_null()
        {
            return None;
        }
        // SAFETY: the call wrote a list of `dwNumberOfItems` interfaces, the
        // array running past its declared length of one.
        let first = unsafe {
            ((*list).dwNumberOfItems > 0)
                .then(|| *(&raw const (*list).InterfaceInfo).cast::<WLAN_INTERFACE_INFO>())
        };
        // SAFETY: allocated by `WlanEnumInterfaces`, freed once, after its
        // last read.
        unsafe { WlanFreeMemory(list.cast()) };
        first.map(|info| {
            (
                info.InterfaceGuid,
                info.isState != wlan_interface_state_not_ready,
            )
        })
    }

    /// The first wireless interface; `None` with none.
    #[must_use]
    pub fn device(&self) -> Option<GUID> {
        self.interface().map(|(guid, _)| guid)
    }

    /// The networks the interface `guid` last saw.
    #[must_use]
    pub fn entries(&self, guid: &GUID) -> Vec<Entry> {
        let mut list: *mut WLAN_AVAILABLE_NETWORK_LIST = ptr::null_mut();
        // SAFETY: the GUID is a live local; the call allocates the list.
        let error = unsafe {
            WlanGetAvailableNetworkList(self.wlan.0, guid, 0, ptr::null(), &raw mut list)
        };
        if error != NO_ERROR || list.is_null() {
            return Vec::new();
        }
        // SAFETY: the call wrote `dwNumberOfItems` networks, the array
        // running past its declared length of one.
        let networks: &[WLAN_AVAILABLE_NETWORK] = unsafe {
            std::slice::from_raw_parts(
                (&raw const (*list).Network).cast(),
                (*list).dwNumberOfItems as usize,
            )
        };
        let entries = networks
            .iter()
            .map(|network| Entry {
                ssid: ssid(network.dot11Ssid.uSSIDLength, &network.dot11Ssid.ucSSID),
                quality: network.wlanSignalQuality,
                auth: network.dot11DefaultAuthAlgorithm,
                cipher: network.dot11DefaultCipherAlgorithm,
                secured: network.bSecurityEnabled != 0,
                flags: network.dwFlags,
                profile: narrow(&network.strProfileName),
            })
            .collect();
        // SAFETY: allocated by `WlanGetAvailableNetworkList`, freed once,
        // after the entries were copied out.
        unsafe { WlanFreeMemory(list.cast()) };
        entries
    }

    /// What the menu shows now, with the last scan ended `last_scan` after
    /// boot.
    #[must_use]
    pub fn read(&self, last_scan: Option<Duration>) -> Wifi {
        let Some((guid, ready)) = self.interface() else {
            return Wifi::default();
        };
        let phys = radio_state(&self.wlan, &guid).unwrap_or_default();
        let radio = Radio {
            enabled: phys.iter().any(|&(_, software, _)| software == RADIO_ON),
            hardware_enabled: phys.iter().any(|&(_, _, hardware)| hardware == RADIO_ON),
        };
        let (points, active, profiles) = listing(&self.entries(&guid));
        Wifi {
            device: Some(format_guid(to_u128(&guid))),
            radio,
            available: ready,
            last_scan,
            points,
            active,
            profiles,
        }
    }

    /// Asks the interface `guid` for a scan, which [`Notice::Scanned`]
    /// ends.
    ///
    /// # Errors
    ///
    /// When the WLAN service refuses.
    pub fn scan(&self, guid: &GUID) -> io::Result<()> {
        // SAFETY: the GUID is a live local; no SSID or IEs.
        check(unsafe { WlanScan(self.wlan.0, guid, ptr::null(), ptr::null(), ptr::null()) })
    }

    /// Switches every phy of the interface `guid` on or off in software.
    ///
    /// # Errors
    ///
    /// When its radio cannot be read or a phy refuses.
    pub fn set_radio(&self, guid: &GUID, on: bool) -> io::Result<()> {
        let phys = radio_state(&self.wlan, guid)
            .ok_or_else(|| io::Error::other("the radio cannot be read"))?;
        for (index, _, _) in phys {
            let state = WLAN_PHY_RADIO_STATE {
                dwPhyIndex: index,
                dot11SoftwareRadioState: if on { RADIO_ON } else { RADIO_OFF },
                // Read-only; ignored when set.
                dot11HardwareRadioState: 0,
            };
            // SAFETY: the GUID and the state are live locals of the size
            // given.
            check(unsafe {
                WlanSetInterface(
                    self.wlan.0,
                    guid,
                    wlan_intf_opcode_radio_state,
                    size_of::<WLAN_PHY_RADIO_STATE>() as u32,
                    (&raw const state).cast(),
                    ptr::null(),
                )
            })?;
        }
        Ok(())
    }

    /// Leaves the network the interface `guid` is on.
    ///
    /// # Errors
    ///
    /// When the WLAN service refuses.
    pub fn disconnect(&self, guid: &GUID) -> io::Result<()> {
        // SAFETY: the GUID is a live local.
        check(unsafe { WlanDisconnect(self.wlan.0, guid, ptr::null()) })
    }

    /// Deletes the saved profile `name`.
    ///
    /// # Errors
    ///
    /// When the WLAN service refuses.
    pub fn forget(&self, guid: &GUID, name: &str) -> io::Result<()> {
        let name = wide(name);
        // SAFETY: the GUID and the terminated name are live locals.
        check(unsafe { WlanDeleteProfile(self.wlan.0, guid, name.as_ptr(), ptr::null()) })
    }

    /// Saves the profile `xml` for every user, replacing one of its name.
    ///
    /// # Errors
    ///
    /// When the WLAN service refuses it, saying why where it can.
    pub fn set_profile(&self, guid: &GUID, xml: &str) -> io::Result<()> {
        let xml = wide(xml);
        let mut reason = 0;
        // SAFETY: the GUID and the terminated XML are live locals; no
        // security descriptor; `reason` is a local the call writes.
        let error = unsafe {
            WlanSetProfile(
                self.wlan.0,
                guid,
                0,
                xml.as_ptr(),
                ptr::null(),
                1,
                ptr::null(),
                &raw mut reason,
            )
        };
        check(error).map_err(|error| io::Error::other(format!("{error}: {}", reason_text(reason))))
    }

    /// Joins the network the saved profile `name` describes, which a
    /// [`Notice::Joined`] or [`Notice::Failed`] ends.
    ///
    /// # Errors
    ///
    /// When the WLAN service refuses to start.
    pub fn connect(&self, guid: &GUID, name: &str) -> io::Result<()> {
        let name = wide(name);
        let parameters = WLAN_CONNECTION_PARAMETERS {
            wlanConnectionMode: wlan_connection_mode_profile,
            strProfile: name.as_ptr(),
            // The profile names its SSID.
            pDot11Ssid: ptr::null_mut(),
            pDesiredBssidList: ptr::null_mut(),
            dot11BssType: dot11_BSS_type_infrastructure,
            dwFlags: 0,
        };
        // SAFETY: the GUID, the parameters and the name they point at are
        // live locals for the call.
        check(unsafe { WlanConnect(self.wlan.0, guid, &raw const parameters, ptr::null()) })
    }

    /// Calls `on_notice` for the WLAN service's scan and connection
    /// notifications, and the radio's, while the client lives. A second
    /// call replaces the first's callback.
    ///
    /// # Errors
    ///
    /// When the WLAN service refuses the registration.
    pub fn watch(&mut self, on_notice: impl Fn(Notice) + Send + Sync + 'static) -> io::Result<()> {
        let on_notice: Box<OnNotice> = Box::new(Box::new(on_notice));
        let context: *const OnNotice = &raw const *on_notice;
        // SAFETY: the context is the boxed callback, kept in `self` until
        // the handle closes, which waits for a callback under way.
        check(unsafe {
            WlanRegisterNotification(
                self.wlan.0,
                WLAN_NOTIFICATION_SOURCE_ACM | WLAN_NOTIFICATION_SOURCE_MSM,
                1,
                Some(notified),
                context.cast(),
                ptr::null(),
                ptr::null_mut(),
            )
        })?;
        self.on_notice = Some(on_notice);
        Ok(())
    }
}

/// `guid` as the number `GUID::from_u128` takes.
const fn to_u128(guid: &GUID) -> u128 {
    (guid.data1 as u128) << 96
        | (guid.data2 as u128) << 80
        | (guid.data3 as u128) << 64
        | u64::from_be_bytes(guid.data4) as u128
}

/// The interface the GUID text `device` names.
#[must_use]
pub fn guid(device: &str) -> Option<GUID> {
    super::super::win32::parse_guid(device).map(GUID::from_u128)
}

/// `reason` (a `WLAN_REASON_CODE`) in Windows' words, or its number when
/// Windows has none.
#[must_use]
pub fn reason_text(reason: u32) -> String {
    let mut buffer = [0_u16; REASON_TEXT];
    // SAFETY: the buffer holds the length given, which the call writes
    // within, terminated.
    let error = unsafe {
        WlanReasonCodeToString(reason, REASON_TEXT as u32, buffer.as_mut_ptr(), ptr::null())
    };
    if error == NO_ERROR && buffer[0] != 0 {
        narrow(&buffer)
    } else {
        format!("reason {reason}")
    }
}

/// The WLAN service's callback.
///
/// # Safety
///
/// `data` is the notification the service passes, and `context` the
/// [`OnNotice`] a live [`Client`] registered.
unsafe extern "system" fn notified(data: *mut L2_NOTIFICATION_DATA, context: *mut c_void) {
    // SAFETY: as the caller promises.
    let (data, on_notice) = unsafe { (&*data, &*context.cast::<OnNotice>()) };
    let connection = || {
        (data.NotificationSource == WLAN_NOTIFICATION_SOURCE_ACM
            && data.dwDataSize as usize >= size_of::<WLAN_CONNECTION_NOTIFICATION_DATA>()
            && !data.pData.is_null())
        .then(|| {
            // SAFETY: an ACM connection notification carries this structure,
            // of the size checked above.
            let connection = unsafe { &*data.pData.cast::<WLAN_CONNECTION_NOTIFICATION_DATA>() };
            (
                ssid(
                    connection.dot11Ssid.uSSIDLength,
                    &connection.dot11Ssid.ucSSID,
                ),
                connection.wlanReasonCode,
            )
        })
    };
    let code = data.NotificationCode as i32;
    let acm = data.NotificationSource == WLAN_NOTIFICATION_SOURCE_ACM;
    let notice = if acm
        && (code == wlan_notification_acm_scan_complete || code == wlan_notification_acm_scan_fail)
    {
        Notice::Scanned
    } else if acm
        && (code == wlan_notification_acm_connection_complete
            || code == wlan_notification_acm_connection_attempt_fail)
    {
        match connection() {
            Some((ssid, 0)) => Notice::Joined(ssid),
            Some((ssid, reason)) => Notice::Failed(ssid, reason),
            None => Notice::Changed,
        }
    } else {
        Notice::Changed
    };
    on_notice(notice);
}

#[cfg(test)]
mod tests {
    use windows_sys::core::GUID;

    use super::{Client, since_boot, to_u128};

    #[test]
    fn a_guid_is_the_number_it_was_made_from() {
        let number = 0x0123_4567_89ab_cdef_0011_2233_4455_6677_u128;
        assert_eq!(to_u128(&GUID::from_u128(number)), number);
    }

    #[test]
    fn this_machine_s_wifi_reads_and_a_watch_registers() {
        assert!(!since_boot().is_zero());
        // A machine without the WLAN service has no client to read.
        let Some(mut client) = Client::open() else {
            return;
        };
        let wifi = client.read(None);
        if wifi.device.is_none() {
            assert!(wifi.points.is_empty(), "no interface, nothing seen");
        }
        client.watch(|_| {}).expect("the notifications register");
        assert!(format!("{client:?}").contains("watching: true"));
    }
}
