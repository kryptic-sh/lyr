//! The network module's source on macOS: the primary interface, its router
//! and address from the dynamic store (`SCDynamicStore`), and Wi-Fi's name
//! and signal from CoreWLAN, read again whenever the store reports a change
//! to the global or any interface's IPv4, link or AirPort state, on a run
//! loop thread of its own. The signal is read only then: no change
//! notification follows it, and nothing is polled.

use std::ffi::c_void;
use std::io;
use std::ptr::NonNull;

use lyr_core::network::Reading;
use objc2_core_foundation::{
    CFArray, CFDictionary, CFRetained, CFRunLoopSource, CFString, CFType, kCFRunLoopDefaultMode,
};
use objc2_core_wlan::CWWiFiClient;
use objc2_system_configuration::{
    SCDynamicStore, SCDynamicStoreContext, kSCCompAnyRegex, kSCDynamicStoreDomainState,
    kSCDynamicStorePropNetPrimaryInterface, kSCEntNetAirPort, kSCEntNetIPv4, kSCEntNetLink,
    kSCPropNetIPv4Addresses, kSCPropNetIPv4Router, kSCPropNetIPv4SubnetMasks,
};

use super::scdynamic::{Primary, Wifi, link};
use crate::macos_runloop::RunLoopThread;

/// A dictionary from the store, keyed by CFStrings.
type Entity = CFDictionary<CFString, CFType>;

/// The store's dictionary at `key`, or `None` when there is none.
fn entity(store: &SCDynamicStore, key: &CFString) -> Option<CFRetained<Entity>> {
    let value = SCDynamicStore::value(Some(store), key)?;
    let dictionary = value.downcast::<CFDictionary>().ok()?;
    // SAFETY: the store's entities are dictionaries keyed by CFStrings; each
    // read checks the value's type.
    Some(unsafe { CFRetained::cast_unchecked(dictionary) })
}

fn text(entity: &Entity, key: &CFString) -> Option<String> {
    Some(entity.get(key)?.downcast::<CFString>().ok()?.to_string())
}

/// The first string of an array value, such as an interface's first
/// address.
fn first(entity: &Entity, key: &CFString) -> Option<String> {
    let array = entity.get(key)?.downcast::<CFArray>().ok()?;
    // SAFETY: each element is checked to be a string as it is read.
    let array: &CFArray<CFType> = unsafe { array.cast_unchecked() };
    Some(array.get(0)?.downcast::<CFString>().ok()?.to_string())
}

/// The primary interface's IPv4 state, or `None` with no primary interface
/// (no network).
fn primary(store: &SCDynamicStore) -> Option<Primary> {
    // SAFETY: the domain and entity names are the framework's statics.
    let (domain, ipv4) = unsafe { (kSCDynamicStoreDomainState, kSCEntNetIPv4) };
    let global = entity(
        store,
        &SCDynamicStore::key_create_network_global_entity(None, domain, ipv4),
    )?;
    // SAFETY: as above.
    let (primary_key, router_key) =
        unsafe { (kSCDynamicStorePropNetPrimaryInterface, kSCPropNetIPv4Router) };
    let interface = text(&global, primary_key)?;
    let key = SCDynamicStore::key_create_network_interface_entity(
        None,
        domain,
        &CFString::from_str(&interface),
        Some(ipv4),
    );
    // SAFETY: as above.
    let (addresses, masks) = unsafe { (kSCPropNetIPv4Addresses, kSCPropNetIPv4SubnetMasks) };
    let address = entity(store, &key).and_then(|state| {
        Some((
            first(&state, addresses)?,
            first(&state, masks).unwrap_or_default(),
        ))
    });
    Some(Primary {
        interface,
        router: text(&global, router_key),
        address,
    })
}

/// The default Wi-Fi interface, and whether its radio is off; `None` on a
/// Mac with no Wi-Fi.
fn wifi() -> Option<(Wifi, bool)> {
    // SAFETY: the shared client and its default interface are plain
    // Objective-C calls with no preconditions.
    unsafe {
        let interface = CWWiFiClient::sharedWiFiClient().interface()?;
        let wifi = Wifi {
            interface: interface.interfaceName()?.to_string(),
            ssid: interface.ssid().map(|ssid| ssid.to_string()),
            rssi: i64::try_from(interface.rssiValue()).unwrap_or(i64::MIN),
        };
        Some((wifi, !interface.powerOn()))
    }
}

/// The reading from `store` and CoreWLAN: the primary interface's link, and
/// the Wi-Fi radio being off as the blocked radio, which shows as disabled
/// when there is no link.
fn read(store: &SCDynamicStore) -> Reading {
    let wifi = wifi();
    Reading {
        link: primary(store).map(|primary| link(&primary, wifi.as_ref().map(|(wifi, _)| wifi))),
        radio_blocked: wifi.is_some_and(|(_, off)| off),
    }
}

/// The reading now.
///
/// # Errors
///
/// When the dynamic store opens no session.
pub fn reading() -> io::Result<Reading> {
    // SAFETY: no callback and no context: a session only to read through.
    let store = unsafe {
        SCDynamicStore::new(None, &CFString::from_str("lyr"), None, std::ptr::null_mut())
    }
    .ok_or_else(|| io::Error::other("the dynamic store opened no session"))?;
    Ok(read(&store))
}

/// What a change calls, with the reading after it.
type OnChange = Box<dyn Fn(Reading) + Send>;

/// Reads again through the store the change came from and calls the
/// [`OnChange`] the store was made with.
unsafe extern "C-unwind" fn changed(
    store: NonNull<SCDynamicStore>,
    _keys: NonNull<CFArray>,
    context: *mut c_void,
) {
    // SAFETY: `context` is a `Registration`'s `OnChange`, which it frees only
    // after invalidating the store's source, the only caller; `store` is the
    // live store the call is about.
    let (on_change, store) = unsafe { (&*context.cast::<OnChange>(), store.as_ref()) };
    on_change(read(store));
}

/// The store, its source on the watch's run loop and the callback it calls,
/// freed source first.
struct Registration {
    _store: CFRetained<SCDynamicStore>,
    source: CFRetained<CFRunLoopSource>,
    context: *mut OnChange,
}

impl Drop for Registration {
    fn drop(&mut self) {
        self.source.invalidate();
        // SAFETY: the box came from `Box::into_raw`, and the source that
        // called into it is invalidated, so nothing reads it again.
        drop(unsafe { Box::from_raw(self.context) });
    }
}

/// The thread watching the dynamic store, calling its callback while it
/// lives.
#[derive(Debug)]
pub struct Watch {
    /// Kept for its drop, which stops the thread.
    _thread: RunLoopThread,
}

impl Watch {
    /// Calls `on_change` with the reading whenever the global or an
    /// interface's IPv4, link or AirPort state changes, on a thread of the
    /// watch's own, until dropped.
    ///
    /// # Errors
    ///
    /// When the thread cannot start or the dynamic store refuses the watch.
    pub fn new(on_change: impl Fn(Reading) + Send + 'static) -> io::Result<Self> {
        RunLoopThread::spawn("lyr network", move |run_loop| {
            let context: *mut OnChange = Box::into_raw(Box::new(Box::new(on_change)));
            let registration = register(context);
            if registration.is_none() {
                // SAFETY: nothing else holds the box.
                drop(unsafe { Box::from_raw(context) });
            }
            let registration = registration
                .ok_or_else(|| io::Error::other("the dynamic store refused the watch"))?;
            // SAFETY: the default mode is a static CF string.
            run_loop.add_source(Some(&*registration.source), unsafe {
                kCFRunLoopDefaultMode
            });
            Ok(registration)
        })
        .map(|thread| Self { _thread: thread })
    }
}

/// A store whose changes call `context`'s callback, watching the global
/// IPv4 entity and every interface's IPv4, link and AirPort entities.
fn register(context: *mut OnChange) -> Option<Registration> {
    let mut store_context = SCDynamicStoreContext {
        version: 0,
        info: context.cast(),
        retain: None,
        release: None,
        copyDescription: None,
    };
    // SAFETY: `changed` reads the info pointer as the `OnChange` it is; the
    // store copies the context.
    let store = unsafe {
        SCDynamicStore::new(
            None,
            &CFString::from_str("lyr network"),
            Some(changed),
            &raw mut store_context,
        )
    }?;
    // SAFETY: the domain, entity and regex names are the framework's statics.
    let (domain, any, ipv4, link, airport) = unsafe {
        (
            kSCDynamicStoreDomainState,
            kSCCompAnyRegex,
            kSCEntNetIPv4,
            kSCEntNetLink,
            kSCEntNetAirPort,
        )
    };
    let global = SCDynamicStore::key_create_network_global_entity(None, domain, ipv4);
    let patterns = [ipv4, link, airport].map(|entity| {
        SCDynamicStore::key_create_network_interface_entity(None, domain, any, Some(entity))
    });
    let keys = CFArray::from_retained_objects(&[global]);
    let patterns = CFArray::from_retained_objects(&patterns);
    // SAFETY: both arrays hold CFStrings, as the call expects.
    let watched =
        unsafe { store.set_notification_keys(Some(keys.as_opaque()), Some(patterns.as_opaque())) };
    if !watched {
        return None;
    }
    let source = SCDynamicStore::new_run_loop_source(None, &store, 0)?;
    Some(Registration {
        _store: store,
        source,
        context,
    })
}

#[cfg(test)]
mod tests {
    use super::{Watch, reading};

    #[test]
    fn the_network_reads_and_a_watch_starts_and_stops() {
        // A CI Mac is wired or has no network; either reads.
        let reading = reading().expect("a store session");
        if let Some(link) = &reading.link {
            assert!(!link.ifname.is_empty());
        }
        drop(Watch::new(|_| {}).expect("watching the store"));
    }
}
