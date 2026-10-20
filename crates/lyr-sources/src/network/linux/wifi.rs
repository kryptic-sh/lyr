//! The Wi-Fi menu's side of NetworkManager: the Wi-Fi device, its access
//! points and the saved profiles that join them, and scanning, joining and
//! forgetting, as `docs/plan/10-popups-and-menus.md#over-networkmanagers-d-bus-api`
//! lays them out.

use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use lyr_core::wifi_menu::{AccessPoint, Profile, Radio, Security, Wifi};
use tokio::sync::watch;
use zbus::message::Type;
use zbus::{MatchRule, MessageStream, ObjectPath, OwnedObjectPath, OwnedVariant, Variant};

use super::{
    AccessPointProxy, DEVICE_TYPE_WIFI, DeviceProxy, NONE, NetworkManager, ROOT, WirelessProxy,
};
use crate::bus::Follow;
use crate::network::{Failed, Join};

/// NetworkManager's saved profiles.
const SETTINGS: &str = "/org/freedesktop/NetworkManager/Settings";
/// `NM_DEVICE_STATE_UNMANAGED` and `NM_DEVICE_STATE_UNAVAILABLE`, when the
/// device cannot scan.
const UNMANAGED: u32 = 10;
const UNAVAILABLE: u32 = 20;
/// `NM_DEVICE_STATE_ACTIVATED` and `NM_DEVICE_STATE_FAILED`, which end a
/// join.
const ACTIVATED: u32 = 100;
const FAILED: u32 = 120;

/// A profile's settings, as `GetSettings` gives them.
type Settings = HashMap<String, HashMap<String, OwnedVariant>>;

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Settings",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings"
)]
trait SettingsList {
    fn list_connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Settings.Connection",
    default_service = "org.freedesktop.NetworkManager"
)]
trait SavedProfile {
    fn get_settings(&self) -> zbus::Result<Settings>;
    fn update(&self, properties: HashMap<&str, HashMap<&str, Variant<'_>>>) -> zbus::Result<()>;
    fn delete(&self) -> zbus::Result<()>;
}

/// A setting's value, when it is there and of type `T`.
fn setting<T>(settings: &Settings, section: &str, key: &str) -> Option<T>
where
    T: TryFrom<OwnedVariant>,
{
    T::try_from(settings.get(section)?.get(key)?.clone()).ok()
}

/// The SSID a profile joins, when it is a Wi-Fi client profile: only a
/// Wi-Fi profile has an `802-11-wireless` section, and a hotspot or ad-hoc
/// one would start a network rather than join one.
fn client_ssid(settings: &Settings) -> Option<String> {
    let mode: Option<String> = setting(settings, "802-11-wireless", "mode");
    if mode.is_some_and(|mode| mode != "infrastructure") {
        return None;
    }
    let ssid: Vec<u8> = setting(settings, "802-11-wireless", "ssid")?;
    Some(String::from_utf8_lossy(&ssid).into_owned())
}

/// NetworkManager's `key-mgmt` for `security`, `None` for an open network.
const fn key_management(security: Security) -> Option<&'static str> {
    match security {
        Security::Psk => Some("wpa-psk"),
        Security::Sae => Some("sae"),
        Security::Owe => Some("owe"),
        Security::Open | Security::Wep | Security::Enterprise => None,
    }
}

/// How long the machine has been up, as `CLOCK_BOOTTIME` counts it, which
/// `/proc/uptime` reports (`ktime_get_boottime_ts64` in the kernel's
/// `fs/proc/uptime.c`). `LastScan` is on that clock.
///
/// # Errors
///
/// When `/proc/uptime` cannot be read or parsed.
pub fn since_boot() -> std::io::Result<Duration> {
    let uptime = std::fs::read_to_string("/proc/uptime")?;
    uptime
        .split_whitespace()
        .next()
        .and_then(|seconds| seconds.parse::<f64>().ok())
        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
        .ok_or_else(|| std::io::Error::other(format!("/proc/uptime reads {uptime:?}")))
}

impl NetworkManager {
    /// The first Wi-Fi device, in NetworkManager's order.
    async fn wifi_device(&self) -> zbus::Result<Option<OwnedObjectPath>> {
        for path in self.manager.get_devices().await? {
            if self
                .proxy::<DeviceProxy>(&path)
                .await?
                .device_type()
                .await?
                == DEVICE_TYPE_WIFI
            {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    /// What the Wi-Fi menu shows now.
    ///
    /// # Errors
    ///
    /// When NetworkManager does not answer, or answers with the wrong types.
    pub async fn wifi(&self) -> zbus::Result<Wifi> {
        let radio = Radio {
            enabled: self.manager.wireless_enabled().await?,
            hardware_enabled: self.manager.wireless_hardware_enabled().await?,
        };
        let profiles = self.profiles().await?;
        let Some(device) = self.wifi_device().await? else {
            return Ok(Wifi {
                radio,
                profiles,
                ..Wifi::default()
            });
        };
        let state = self.proxy::<DeviceProxy>(&device).await?.state().await?;
        let wireless = self.proxy::<WirelessProxy>(&device).await?;
        let last_scan = u64::try_from(wireless.last_scan().await?)
            .ok()
            .map(Duration::from_millis);
        let active = wireless.active_access_point().await?;
        let mut points = Vec::new();
        for path in wireless.get_all_access_points().await? {
            let point = self.proxy::<AccessPointProxy>(&path).await?;
            points.push(AccessPoint {
                ssid: String::from_utf8_lossy(&point.ssid().await?).into_owned(),
                strength: point.strength().await?,
                security: Security::of(
                    point.flags().await?,
                    point.wpa_flags().await?,
                    point.rsn_flags().await?,
                ),
                path: path.to_string(),
            });
        }
        Ok(Wifi {
            device: Some(device.to_string()),
            radio,
            available: state != UNMANAGED && state != UNAVAILABLE,
            last_scan,
            points,
            active: (active.as_str() != NONE).then(|| active.to_string()),
            profiles,
        })
    }

    /// The saved Wi-Fi client profiles.
    async fn profiles(&self) -> zbus::Result<Vec<Profile>> {
        let list = SettingsListProxy::builder(&self.connection)
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?;
        let mut profiles = Vec::new();
        for path in list.list_connections().await? {
            let settings = self
                .proxy::<SavedProfileProxy>(&path)
                .await?
                .get_settings()
                .await?;
            if let Some(ssid) = client_ssid(&settings) {
                profiles.push(Profile {
                    path: path.to_string(),
                    ssid,
                });
            }
        }
        Ok(profiles)
    }

    /// Turns the Wi-Fi radio on or off.
    ///
    /// # Errors
    ///
    /// When NetworkManager refuses.
    pub async fn set_radio(&self, on: bool) -> zbus::Result<()> {
        self.manager.set_wireless_enabled(on).await
    }

    /// Asks the device at `device` to scan. The scan's end shows as a new
    /// `LastScan`.
    ///
    /// # Errors
    ///
    /// When NetworkManager refuses, as it does while a scan runs.
    pub async fn scan(&self, device: &str) -> zbus::Result<()> {
        let device = OwnedObjectPath::try_from(device)?;
        self.proxy::<WirelessProxy>(&device)
            .await?
            .request_scan(HashMap::new())
            .await
    }

    /// Joins a network on the device at `device`, and waits for the device
    /// to be up or to have failed. A failure comes back as [`Failed`] with
    /// the device's reason, and the profile made for the attempt, which is
    /// left for the caller to keep or delete
    /// ([`lyr_core::wifi_menu::recovery`]).
    ///
    /// # Errors
    ///
    /// When NetworkManager refuses the request or stops answering.
    pub async fn join(&self, device: &str, join: &Join) -> zbus::Result<Result<(), Failed>> {
        let device = OwnedObjectPath::try_from(device)?;
        // Subscribed first, so the end cannot come before the wait.
        let mut changes = self
            .proxy::<DeviceProxy>(&device)
            .await?
            .receive_device_state_changed()
            .await?;
        let made = match join {
            Join::Saved { profile, point } => {
                self.manager
                    .activate_connection(
                        &ObjectPath::try_from(profile.as_str())?,
                        &device,
                        &ObjectPath::try_from(point.as_str())?,
                    )
                    .await?;
                None
            }
            Join::New {
                ssid,
                security,
                password,
                point,
            } => {
                let mut settings = HashMap::from([
                    (
                        "802-11-wireless",
                        HashMap::from([
                            ("ssid", Variant::from(ssid.as_bytes().to_vec())),
                            ("mode", Variant::from("infrastructure")),
                        ]),
                    ),
                    (
                        "connection",
                        HashMap::from([
                            ("type", Variant::from("802-11-wireless")),
                            ("id", Variant::from(ssid.as_str())),
                        ]),
                    ),
                ]);
                if let Some(key_management) = key_management(*security) {
                    let mut secured = HashMap::from([("key-mgmt", Variant::from(key_management))]);
                    if let Some(password) = password {
                        secured.insert("psk", Variant::from(password.as_str()));
                    }
                    settings.insert("802-11-wireless-security", secured);
                }
                let (profile, _active) = self
                    .manager
                    .add_and_activate_connection(
                        settings,
                        &device,
                        &ObjectPath::try_from(point.as_str())?,
                    )
                    .await?;
                Some(profile.to_string())
            }
        };
        while let Some(change) = changes.next().await {
            let args = change.args()?;
            match args.new_state {
                ACTIVATED => return Ok(Ok(())),
                FAILED => {
                    return Ok(Err(Failed {
                        reason: args.reason,
                        profile: made,
                    }));
                }
                _ => {}
            }
        }
        Err(zbus::Error::Failure(
            "NetworkManager stopped sending the device's state".to_owned(),
        ))
    }

    /// Deletes the saved profiles at `profiles`.
    ///
    /// # Errors
    ///
    /// When NetworkManager refuses one; those before it are gone.
    pub async fn forget(&self, profiles: &[String]) -> zbus::Result<()> {
        for profile in profiles {
            let path = OwnedObjectPath::try_from(profile.as_str())?;
            self.proxy::<SavedProfileProxy>(&path)
                .await?
                .delete()
                .await?;
        }
        Ok(())
    }

    /// Gives the saved profile at `profile` a new `password`, keeping the
    /// rest of its settings.
    ///
    /// # Errors
    ///
    /// When NetworkManager refuses.
    pub async fn replace_password(&self, profile: &str, password: &str) -> zbus::Result<()> {
        let path = OwnedObjectPath::try_from(profile)?;
        let saved = self.proxy::<SavedProfileProxy>(&path).await?;
        let settings = saved.get_settings().await?;
        let mut update: HashMap<&str, HashMap<&str, Variant<'_>>> = settings
            .iter()
            .map(|(section, values)| {
                (
                    section.as_str(),
                    values
                        .iter()
                        .map(|(key, value)| (key.as_str(), Variant::from(value.clone())))
                        .collect(),
                )
            })
            .collect();
        update
            .entry("802-11-wireless-security")
            .or_default()
            .insert("psk", Variant::from(password));
        saved.update(update).await
    }

    /// Sends what the Wi-Fi menu shows to `states` now and after every
    /// change to the radio, the Wi-Fi device or the saved profiles, until
    /// NetworkManager goes away or every receiver is dropped. An access
    /// point's own changes (its strength) wait for the next of those.
    ///
    /// # Errors
    ///
    /// When subscribing or reading fails.
    pub async fn watch_wifi(&self, states: &watch::Sender<Wifi>) -> zbus::Result<()> {
        let properties = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .member("PropertiesChanged")
            .path_namespace(ROOT)
            .build()?;
        let profiles = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.NetworkManager.Settings")
            .path(SETTINGS)
            .build()?;
        // Subscribed before the first read, so a change in between is not lost.
        let mut changes = Follow::new(futures_util::stream::select(
            MessageStream::for_match_rule(properties, &self.connection, None).await?,
            MessageStream::for_match_rule(profiles, &self.connection, None).await?,
        ));
        // Any change during the first read; then the manager's, the
        // profiles' and the Wi-Fi device's.
        let followed = |device: Option<&str>, change: zbus::Result<zbus::Message>| {
            let change = change?;
            let header = change.header();
            let path = header.path().map(|path| path.as_str());
            Ok(path.is_some_and(|path| path == ROOT || path == SETTINGS || device == Some(path)))
        };
        let mut wifi = changes.during(self.wifi(), |_| Ok(true)).await??;
        states.send_replace(wifi.clone());
        while changes
            .changed(|change| followed(wifi.device.as_deref(), change))
            .await?
        {
            wifi = changes
                .during(self.wifi(), |change| {
                    followed(wifi.device.as_deref(), change)
                })
                .await??;
            if states.send(wifi.clone()).is_err() {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
