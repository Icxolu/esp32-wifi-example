pub mod storage;

use crate::convert::Newtype;
use core::time::Duration;
use embedded_svc::{
    ipv4,
    wifi::{
        AccessPointConfiguration, AccessPointInfo, AuthMethod, ClientConfiguration, Configuration,
        Wifi,
    },
};
use enumset::EnumSet;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    handle::RawHandle,
    wifi::{EspWifi, WifiWait},
};
use esp_idf_sys as sys;

/// This holds a WiFi configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WifiInfo {
    pub ip_info: ipv4::IpInfo,
    pub sta_config: Option<ClientConfiguration>,
    pub ap_config: Option<AccessPointConfiguration>,
    pub ap_mode: ApMode,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    serde_repr::Serialize_repr,
    serde_repr::Deserialize_repr,
)]
#[repr(u8)]
pub enum ApMode {
    #[default]
    NoConnOnBoot = 0,
    Always = 2,
    Never = 3,
}

const DEDAULT_IP_INFO: ipv4::IpInfo = ipv4::IpInfo {
    ip: ipv4::Ipv4Addr::UNSPECIFIED,
    subnet: ipv4::Subnet {
        gateway: ipv4::Ipv4Addr::UNSPECIFIED,
        mask: ipv4::Mask(24),
    },
    dns: None,
    secondary_dns: None,
};

pub fn default_ap_config() -> AccessPointConfiguration {
    AccessPointConfiguration {
        ssid: heapless::String::from("ESP32"),
        ssid_hidden: false,
        channel: 1,
        secondary_channel: Some(2),
        protocols: EnumSet::empty(),
        auth_method: AuthMethod::WPA2Personal,
        password: heapless::String::from("test1234"),
        max_connections: 10,
    }
}

pub fn scan_aps() -> Result<Vec<AccessPointInfo>, sys::EspError> {
    unsafe { sys::esp!(sys::esp_wifi_scan_start(core::ptr::null(), true))? };
    let mut num = 0;
    unsafe { sys::esp!(sys::esp_wifi_scan_get_ap_num(&mut num))? };
    let mut buf = vec![sys::wifi_ap_record_t::default(); num as usize];
    unsafe {
        sys::esp!(sys::esp_wifi_scan_get_ap_records(
            &mut num,
            buf.as_mut_ptr()
        ))?
    };
    Ok(buf
        .into_iter()
        .map(|record| AccessPointInfo::from(Newtype(record)))
        .collect())
}

pub fn update_wifi(
    wifi: &mut EspWifi,
    sysloop: &EspSystemEventLoop,
    WifiInfo {
        ip_info,
        sta_config,
        ap_config,
        ap_mode,
    }: WifiInfo,
) -> Result<(), sys::EspError> {
    set_ip_info(wifi, ip_info)?;
    match ap_mode {
        ApMode::NoConnOnBoot => {
            if let Some(config) = sta_config.as_ref() {
                // We should try client only first and setup an ap only if
                // connection failed
                wifi.set_configuration(&Configuration::Client(config.clone()))?;
            } else if let Some(current) = wifi.get_configuration()?.as_ap_conf_ref() {
                let next = ap_config.unwrap_or_else(default_ap_config);

                if current != &next {
                    wifi.set_configuration(&Configuration::Mixed(Default::default(), next))?;
                    wifi.start()?;
                }

                return Ok(());
            }
        }
        ApMode::Always => {
            // We should _ALWYAS_ setup an AP
            if let Some(config) = sta_config.as_ref() {
                wifi.set_configuration(&Configuration::Mixed(
                    config.clone(),
                    ap_config.clone().unwrap_or_else(default_ap_config),
                ))?;
            } else {
                wifi.set_configuration(&Configuration::Mixed(
                    Default::default(),
                    ap_config.unwrap_or_else(default_ap_config),
                ))?;
                wifi.start()?;
                return Ok(());
            }
        }
        ApMode::Never => {
            // We should _NEVER_ setup an AP so we have a client config, we use
            // that otherwise we turn off the network
            if let Some(config) = sta_config.as_ref() {
                wifi.set_configuration(&Configuration::Client(config.clone()))?;
            } else {
                wifi.set_configuration(&Configuration::None)?;
                return Ok(());
            }
        }
    }

    wifi.start()?;
    wifi.connect()?;

    if matches!(ap_mode, ApMode::Always | ApMode::Never) {
        return Ok(());
    }

    if !WifiWait::new(sysloop)?.wait_with_timeout(Duration::from_secs(15), || {
        wifi.is_started().unwrap() && wifi.is_connected().unwrap()
    }) {
        wifi.set_configuration(&Configuration::Mixed(
            Default::default(),
            ap_config.unwrap_or_else(default_ap_config),
        ))?;
        wifi.start()?;
    };

    Ok(())
}

fn set_ip_info(wifi: &mut EspWifi, ip_info: ipv4::IpInfo) -> Result<(), sys::EspError> {
    unsafe {
        let handle = wifi.sta_netif_mut().handle();
        sys::esp!(sys::esp_netif_dhcpc_stop(handle))?;
        sys::esp!(sys::esp_netif_set_ip_info(
            handle,
            &Newtype::from(ip_info).0
        ))?;
        sys::esp!(sys::esp_netif_dhcpc_start(handle))?;
    };

    Ok(())
}
