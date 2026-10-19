//! The network source on Windows: the interface `GetBestInterface` gives for
//! the default route (0.0.0.0, the route waybar follows), its address and
//! gateway from `GetAdaptersAddresses`, and on Wi-Fi its network and signal
//! from `WlanQueryInterface` ([`super::win32`] makes the link of them).
//!
//! [`Watch`] reports changes: an interface's, an address's or a route's
//! (`NotifyIpInterfaceChange`, `NotifyUnicastIpAddressChange`,
//! `NotifyRouteChange2`), and the WLAN service's connection and signal
//! (`WlanRegisterNotification`). Its callbacks only signal: the caller
//! reads on a thread of its own, since a WLAN callback must not call back
//! into the WLAN API.

use std::ffi::{CStr, c_void};
use std::io;
use std::mem::ManuallyDrop;
use std::net::Ipv4Addr;
use std::ptr;

use lyr_core::network::Reading;
use windows_sys::Win32::Foundation::{
    ERROR_BUFFER_OVERFLOW, ERROR_NO_DATA, HANDLE, NO_ERROR, WIN32_ERROR,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    CancelMibChangeNotify2, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST,
    GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses, GetBestInterface,
    IF_TYPE_IEEE80211, IP_ADAPTER_ADDRESSES_LH, MIB_IPFORWARD_ROW2, MIB_IPINTERFACE_ROW,
    MIB_NOTIFICATION_TYPE, MIB_UNICASTIPADDRESS_ROW, NotifyIpInterfaceChange, NotifyRouteChange2,
    NotifyUnicastIpAddressChange,
};
use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows_sys::Win32::NetworkManagement::WiFi::{
    L2_NOTIFICATION_DATA, WLAN_API_VERSION_2_0, WLAN_CONNECTION_ATTRIBUTES,
    WLAN_NOTIFICATION_SOURCE_ACM, WLAN_NOTIFICATION_SOURCE_MSM, WLAN_NOTIFICATION_SOURCE_NONE,
    WlanCloseHandle, WlanFreeMemory, WlanOpenHandle, WlanQueryInterface, WlanRegisterNotification,
    wlan_intf_opcode_current_connection,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_UNSPEC, SOCKADDR_IN, SOCKET_ADDRESS};
use windows_sys::core::{GUID, PCSTR, PCWSTR};

use super::win32::{Adapter, Wifi, link, parse_guid};

/// `GetAdaptersAddresses`'s first buffer: the 15 KB Microsoft's reference
/// suggests, which holds most machines' adapters.
const ADAPTERS_BUFFER: u32 = 15 * 1024;

/// The reading now.
///
/// # Errors
///
/// When the adapters cannot be listed.
pub fn read() -> io::Result<Reading> {
    let none = Reading::default();
    // SAFETY: a local the call writes once.
    let mut index = 0;
    if unsafe { GetBestInterface(0, &raw mut index) } != NO_ERROR {
        // No route to anywhere: disconnected.
        return Ok(none);
    }
    let Some((guid, adapter)) = adapters()?
        .into_iter()
        .find(|(at, _, _)| *at == index)
        .map(|(_, guid, adapter)| (guid, adapter))
    else {
        return Ok(none);
    };
    let wifi = guid.filter(|_| adapter.wifi).and_then(wifi);
    Ok(Reading {
        link: link(&adapter, wifi.as_ref()),
        // The radio's state is not read yet.
        radio_blocked: false,
    })
}

/// Every IPv4 adapter: its index, its GUID, and its facts.
fn adapters() -> io::Result<Vec<(u32, Option<u128>, Adapter)>> {
    let flags = GAA_FLAG_INCLUDE_GATEWAYS
        | GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST
        | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size = ADAPTERS_BUFFER;
    loop {
        // u64s, so the buffer is aligned for the structures written into it.
        let mut buffer = vec![0_u64; (size as usize).div_ceil(8)];
        let first = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        // SAFETY: the buffer holds `size` bytes, which the call fills, or
        // says how many it needs in `size`.
        let error = unsafe {
            GetAdaptersAddresses(u32::from(AF_INET), flags, ptr::null(), first, &raw mut size)
        };
        match error {
            // SAFETY: the call filled the buffer, which lives to the end of
            // this arm.
            NO_ERROR => return Ok(unsafe { walk(first) }),
            ERROR_NO_DATA => return Ok(Vec::new()),
            ERROR_BUFFER_OVERFLOW => {}
            error => return Err(io::Error::from_raw_os_error(error as i32)),
        }
    }
}

/// The adapters of the list `GetAdaptersAddresses` wrote from `first`.
///
/// # Safety
///
/// `first` is a list `GetAdaptersAddresses` filled, alive for the call.
unsafe fn walk(first: *const IP_ADAPTER_ADDRESSES_LH) -> Vec<(u32, Option<u128>, Adapter)> {
    let mut adapters = Vec::new();
    let mut at = first;
    // SAFETY, throughout: every pointer followed is one the list holds, null
    // ending each chain, as `GetAdaptersAddresses` documents.
    while let Some(adapter) = unsafe { at.as_ref() } {
        let index = unsafe { adapter.Anonymous1.Anonymous.IfIndex };
        let guid = unsafe { narrow(adapter.AdapterName) }.and_then(|name| parse_guid(&name));
        let mut address = None;
        let mut unicast = adapter.FirstUnicastAddress;
        while let Some(entry) = unsafe { unicast.as_ref() } {
            if let Some(ipv4) = unsafe { ipv4(&entry.Address) } {
                address = Some((ipv4, entry.OnLinkPrefixLength));
                break;
            }
            unicast = entry.Next;
        }
        let mut gateway = None;
        let mut gateways = adapter.FirstGatewayAddress;
        while let Some(entry) = unsafe { gateways.as_ref() } {
            if let Some(ipv4) = unsafe { ipv4(&entry.Address) } {
                gateway = Some(ipv4);
                break;
            }
            gateways = entry.Next;
        }
        adapters.push((
            index,
            guid,
            Adapter {
                name: unsafe { wide(adapter.FriendlyName) },
                wifi: adapter.IfType == IF_TYPE_IEEE80211,
                up: adapter.OperStatus == IfOperStatusUp,
                address,
                gateway,
            },
        ));
        at = adapter.Next;
    }
    adapters
}

/// An IPv4 address a `SOCKET_ADDRESS` holds; `None` for another family.
///
/// # Safety
///
/// `address` points at a socket address of its stated length, or at null.
unsafe fn ipv4(address: &SOCKET_ADDRESS) -> Option<Ipv4Addr> {
    // SAFETY: as the caller promises.
    let family = unsafe { address.lpSockaddr.as_ref() }?.sa_family;
    if family != AF_INET {
        return None;
    }
    // SAFETY: an AF_INET address is a SOCKADDR_IN.
    let inet = unsafe { &*address.lpSockaddr.cast::<SOCKADDR_IN>() };
    // SAFETY: every member of the union is the same four bytes.
    let bytes = unsafe { inet.sin_addr.S_un.S_addr }.to_ne_bytes();
    Some(Ipv4Addr::from(bytes))
}

/// A wide string up to its terminating zero; empty for null.
///
/// # Safety
///
/// `text` is a zero-terminated wide string, or null.
unsafe fn wide(text: PCWSTR) -> String {
    if text.is_null() {
        return String::new();
    }
    let mut length = 0;
    // SAFETY: as the caller promises, there is a zero to find.
    while unsafe { *text.add(length) } != 0 {
        length += 1;
    }
    // SAFETY: the `length` units before the zero.
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length) })
}

/// A narrow string, as `AdapterName` is; `None` for null.
///
/// # Safety
///
/// `text` is a zero-terminated string, or null.
unsafe fn narrow(text: PCSTR) -> Option<String> {
    if text.is_null() {
        return None;
    }
    // SAFETY: as the caller promises.
    Some(
        unsafe { CStr::from_ptr(text.cast()) }
            .to_string_lossy()
            .into_owned(),
    )
}

/// A WLAN client handle, closed when dropped.
struct Wlan(HANDLE);

impl Wlan {
    fn open() -> Option<Self> {
        let mut version = 0;
        let mut handle: HANDLE = ptr::null_mut();
        // SAFETY: locals the call writes.
        let error = unsafe {
            WlanOpenHandle(
                WLAN_API_VERSION_2_0,
                ptr::null(),
                &raw mut version,
                &raw mut handle,
            )
        };
        // A machine without the WLAN service has no Wi-Fi to read.
        (error == NO_ERROR).then_some(Self(handle))
    }
}

impl Drop for Wlan {
    fn drop(&mut self) {
        // SAFETY: the handle came from `WlanOpenHandle` and is closed once; it
        // waits for its callbacks to end. A failure leaks a handle in a
        // process that keeps running.
        unsafe { WlanCloseHandle(self.0, ptr::null()) };
    }
}

/// The Wi-Fi interface `guid`'s current connection; `None` while it has
/// none.
fn wifi(guid: u128) -> Option<Wifi> {
    let wlan = Wlan::open()?;
    let guid = GUID::from_u128(guid);
    let mut size = 0;
    let mut data: *mut c_void = ptr::null_mut();
    // SAFETY: the GUID and the outputs are locals; the call allocates `data`.
    let error = unsafe {
        WlanQueryInterface(
            wlan.0,
            &raw const guid,
            wlan_intf_opcode_current_connection,
            ptr::null(),
            &raw mut size,
            &raw mut data,
            ptr::null_mut(),
        )
    };
    if error != NO_ERROR || data.is_null() {
        return None;
    }
    // SAFETY: the call wrote a WLAN_CONNECTION_ATTRIBUTES at `data`.
    let attributes = unsafe { &*data.cast::<WLAN_CONNECTION_ATTRIBUTES>() };
    let association = &attributes.wlanAssociationAttributes;
    let length =
        (association.dot11Ssid.uSSIDLength as usize).min(association.dot11Ssid.ucSSID.len());
    let wifi = Wifi {
        ssid: association.dot11Ssid.ucSSID[..length].to_vec(),
        quality: association.wlanSignalQuality,
    };
    // SAFETY: allocated by `WlanQueryInterface`, freed once, after its last
    // read.
    unsafe { WlanFreeMemory(data) };
    Some(wifi)
}

/// What a change calls.
type OnChange = Box<dyn Fn() + Send + Sync>;

/// Calls the [`OnChange`] at `context`.
///
/// # Safety
///
/// `context` is the `OnChange` a live [`Watch`] keeps.
unsafe fn call(context: *const c_void) {
    // SAFETY: as the caller promises.
    (unsafe { &*context.cast::<OnChange>() })();
}

unsafe extern "system" fn interface_changed(
    context: *const c_void,
    _row: *const MIB_IPINTERFACE_ROW,
    _kind: MIB_NOTIFICATION_TYPE,
) {
    // SAFETY: the context a `Watch` registered and keeps until it cancels.
    unsafe { call(context) };
}

unsafe extern "system" fn address_changed(
    context: *const c_void,
    _row: *const MIB_UNICASTIPADDRESS_ROW,
    _kind: MIB_NOTIFICATION_TYPE,
) {
    // SAFETY: as for `interface_changed`.
    unsafe { call(context) };
}

unsafe extern "system" fn route_changed(
    context: *const c_void,
    _row: *const MIB_IPFORWARD_ROW2,
    _kind: MIB_NOTIFICATION_TYPE,
) {
    // SAFETY: as for `interface_changed`.
    unsafe { call(context) };
}

unsafe extern "system" fn wlan_changed(_data: *mut L2_NOTIFICATION_DATA, context: *mut c_void) {
    // SAFETY: as for `interface_changed`.
    unsafe { call(context) };
}

/// The registrations calling `on_change` while it lives.
pub struct Watch {
    /// IP Helper's notifications.
    handles: Vec<HANDLE>,
    /// The WLAN service's, where there is one.
    wlan: Option<Wlan>,
    /// The callbacks' context, boxed twice so its address stays put; freed
    /// only once every registration is gone ([`Drop`]).
    on_change: ManuallyDrop<Box<OnChange>>,
}

impl Watch {
    /// Calls `on_change`, on a system thread, at each change to the
    /// interfaces, their addresses, the routes or the Wi-Fi connection,
    /// until the watch is dropped.
    ///
    /// # Errors
    ///
    /// When IP Helper refuses a registration; those made before it are undone.
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let on_change: Box<OnChange> = Box::new(Box::new(on_change));
        let context = ptr::from_ref::<OnChange>(&on_change).cast::<c_void>();
        let mut watch = Self {
            handles: Vec::new(),
            wlan: None,
            on_change: ManuallyDrop::new(on_change),
        };
        let registered = |error: WIN32_ERROR, handle: HANDLE, watch: &mut Self| {
            if error != NO_ERROR {
                // Dropping `watch` undoes the registrations made so far.
                return Err(io::Error::from_raw_os_error(error as i32));
            }
            watch.handles.push(handle);
            Ok(())
        };
        let mut handle: HANDLE = ptr::null_mut();
        // SAFETY, for each: the callback matches the type, the context lives
        // in the watch until its registration is cancelled, and `handle` is
        // a local the call writes.
        let error = unsafe {
            NotifyIpInterfaceChange(
                AF_UNSPEC,
                Some(interface_changed),
                context,
                false,
                &raw mut handle,
            )
        };
        registered(error, handle, &mut watch)?;
        let error = unsafe {
            NotifyUnicastIpAddressChange(
                AF_UNSPEC,
                Some(address_changed),
                context,
                false,
                &raw mut handle,
            )
        };
        registered(error, handle, &mut watch)?;
        let error = unsafe {
            NotifyRouteChange2(
                AF_UNSPEC,
                Some(route_changed),
                context,
                false,
                &raw mut handle,
            )
        };
        registered(error, handle, &mut watch)?;
        // Wi-Fi changes, where the WLAN service runs; without it there is no
        // Wi-Fi to follow.
        if let Some(wlan) = Wlan::open() {
            // SAFETY: as above; the handle is the watch's until it closes it.
            let error = unsafe {
                WlanRegisterNotification(
                    wlan.0,
                    WLAN_NOTIFICATION_SOURCE_ACM | WLAN_NOTIFICATION_SOURCE_MSM,
                    1,
                    Some(wlan_changed),
                    context,
                    ptr::null(),
                    ptr::null_mut(),
                )
            };
            if error != NO_ERROR {
                return Err(io::Error::from_raw_os_error(error as i32));
            }
            watch.wlan = Some(wlan);
        }
        Ok(watch)
    }
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch")
            .field("handles", &self.handles.len())
            .field("wlan", &self.wlan.is_some())
            .finish_non_exhaustive()
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let mut all_gone = true;
        for &handle in &self.handles {
            // SAFETY: each handle came from a `Notify…` call and is cancelled
            // once; the call waits for a callback under way.
            all_gone &= unsafe { CancelMibChangeNotify2(handle) } == NO_ERROR;
        }
        if let Some(wlan) = self.wlan.take() {
            // SAFETY: stops the notifications before the handle closes, which
            // waits for a callback under way.
            all_gone &= unsafe {
                WlanRegisterNotification(
                    wlan.0,
                    WLAN_NOTIFICATION_SOURCE_NONE,
                    1,
                    None,
                    ptr::null(),
                    ptr::null(),
                    ptr::null_mut(),
                )
            } == NO_ERROR;
            drop(wlan);
        }
        // A registration that would not go can still call back, so its
        // context is leaked rather than freed under it.
        if all_gone {
            // SAFETY: nothing holds the context any more, and it is dropped
            // once.
            unsafe { ManuallyDrop::drop(&mut self.on_change) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Watch, adapters, read};

    #[test]
    fn this_machine_s_network_reads_and_a_watch_registers() {
        // Whatever this machine's network is, the calls answer, and a loopback
        // interface is always among the adapters.
        let adapters = adapters().expect("the adapters");
        assert!(
            adapters.iter().any(|(_, _, adapter)| adapter
                .address
                .is_some_and(|(address, _)| address.is_loopback())),
            "{adapters:?}"
        );
        read().expect("a reading");
        let watch = Watch::new(|| {}).expect("registered");
        assert_eq!(watch.handles.len(), 3);
    }
}
