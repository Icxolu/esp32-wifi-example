use embedded_svc::{ipv4, wifi};
use enumset::EnumSet;
use esp_idf_sys as sys;

pub struct Newtype<T>(pub T);

impl From<Newtype<sys::wifi_ap_record_t>> for wifi::AccessPointInfo {
    fn from(Newtype(record): Newtype<sys::wifi_ap_record_t>) -> Self {
        wifi::AccessPointInfo {
            ssid: {
                let len = record.ssid.iter().position(|e| *e == 0).unwrap() + 1;
                unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(&record.ssid[..len]) }
                    .to_str()
                    .unwrap()
                    .into()
            },
            bssid: record.bssid,
            channel: record.primary,
            secondary_channel: match record.second {
                sys::wifi_second_chan_t_WIFI_SECOND_CHAN_NONE => wifi::SecondaryChannel::None,
                sys::wifi_second_chan_t_WIFI_SECOND_CHAN_ABOVE => wifi::SecondaryChannel::Above,
                sys::wifi_second_chan_t_WIFI_SECOND_CHAN_BELOW => wifi::SecondaryChannel::Below,
                _ => panic!(),
            },
            signal_strength: record.rssi,
            protocols: EnumSet::empty(),
            auth_method: wifi::AuthMethod::from(Newtype(record.authmode)),
        }
    }
}

impl From<Newtype<sys::wifi_auth_mode_t>> for wifi::AuthMethod {
    fn from(Newtype(mode): Newtype<sys::wifi_auth_mode_t>) -> Self {
        match mode {
            sys::wifi_auth_mode_t_WIFI_AUTH_OPEN => wifi::AuthMethod::None,
            sys::wifi_auth_mode_t_WIFI_AUTH_WEP => wifi::AuthMethod::WEP,
            sys::wifi_auth_mode_t_WIFI_AUTH_WPA_PSK => wifi::AuthMethod::WPA,
            sys::wifi_auth_mode_t_WIFI_AUTH_WPA2_PSK => wifi::AuthMethod::WPA2Personal,
            sys::wifi_auth_mode_t_WIFI_AUTH_WPA_WPA2_PSK => wifi::AuthMethod::WPAWPA2Personal,
            sys::wifi_auth_mode_t_WIFI_AUTH_WPA2_ENTERPRISE => wifi::AuthMethod::WPA2Enterprise,
            sys::wifi_auth_mode_t_WIFI_AUTH_WPA3_PSK => wifi::AuthMethod::WPA3Personal,
            sys::wifi_auth_mode_t_WIFI_AUTH_WPA2_WPA3_PSK => wifi::AuthMethod::WPA2WPA3Personal,
            sys::wifi_auth_mode_t_WIFI_AUTH_WAPI_PSK => wifi::AuthMethod::WAPIPersonal,
            _ => panic!(),
        }
    }
}

impl From<ipv4::IpInfo> for Newtype<sys::esp_netif_ip_info_t> {
    fn from(value: ipv4::IpInfo) -> Self {
        Self(sys::esp_netif_ip_info_t {
            ip: sys::esp_ip4_addr_t {
                addr: value.ip.into(),
            },
            netmask: sys::esp_ip4_addr_t {
                addr: !(u32::MAX >> u32::from(value.subnet.mask.0)),
            },
            gw: sys::esp_ip4_addr_t {
                addr: value.subnet.gateway.into(),
            },
        })
    }
}
