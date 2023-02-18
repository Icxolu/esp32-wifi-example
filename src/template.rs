use crate::wifi::{self, default_ap_config, ApMode, WifiInfo};
use askama::Template;
use embedded_svc::{
    ipv4,
    wifi::{AccessPointConfiguration, AuthMethod, ClientConfiguration},
};
use enumset::EnumSet;

#[derive(Debug, Clone, Template, serde::Serialize, serde::Deserialize)]
#[template(path = "wifi.html")]
pub struct WifiSettingsTemplate {
    pub client: WifiClientSettings,
    pub ap: WifiApSettings,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WifiClientSettings {
    pub ssid: heapless::String<32>,
    pub password: heapless::String<64>,
    pub ip: [u8; 4],
    pub gateway: [u8; 4],
    pub subnet_mask: [u8; 4],
    pub mdns: heapless::String<32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WifiApSettings {
    pub ssid: heapless::String<32>,
    pub hidden: bool,
    pub password: heapless::String<64>,
    pub channel: u8,
    pub mode: ApMode,
}

impl From<WifiInfo> for WifiSettingsTemplate {
    fn from(
        WifiInfo {
            ip_info,
            sta_config,
            ap_config,
            ap_mode,
        }: WifiInfo,
    ) -> Self {
        let sta_config = sta_config.unwrap_or_default();
        let ap_config = ap_config.unwrap_or_else(default_ap_config);
        Self {
            client: WifiClientSettings {
                ssid: sta_config.ssid,
                password: sta_config.password,
                ip: ip_info.ip.octets(),
                gateway: ip_info.subnet.gateway.octets(),
                subnet_mask: (!(u32::MAX >> u32::from(ip_info.subnet.mask.0))).to_be_bytes(),
                mdns: Default::default(),
            },
            ap: WifiApSettings {
                ssid: ap_config.ssid,
                hidden: ap_config.ssid_hidden,
                password: ap_config.password,
                channel: ap_config.channel,
                mode: ap_mode,
            },
        }
    }
}

impl TryFrom<WifiSettingsTemplate> for WifiInfo {
    type Error = esp_idf_sys::EspError;

    fn try_from(
        WifiSettingsTemplate { client, ap }: WifiSettingsTemplate,
    ) -> Result<Self, Self::Error> {
        let ap_info = wifi::scan_aps()?
            .into_iter()
            .find(|ap| (ap.ssid == client.ssid));

        let ip_info = ipv4::IpInfo {
            ip: ipv4::Ipv4Addr::from(client.ip),
            subnet: ipv4::Subnet {
                gateway: ipv4::Ipv4Addr::from(client.gateway),
                mask: ipv4::Mask(u32::from_be_bytes(client.subnet_mask).count_ones() as _),
            },
            dns: None,
            secondary_dns: None,
        };
        let sta_config = (!client.ssid.is_empty()).then_some(ClientConfiguration {
            ssid: client.ssid,
            bssid: None,
            auth_method: ap_info
                .as_ref()
                .map(|ap| ap.auth_method)
                .unwrap_or_else(|| {
                    if client.password.is_empty() {
                        AuthMethod::None
                    } else {
                        AuthMethod::WPA2Personal
                    }
                }),
            password: client.password,
            channel: ap_info.map(|ap| ap.channel),
        });

        let ap_config = (!ap.ssid.is_empty()).then_some(AccessPointConfiguration {
            ssid: ap.ssid,
            ssid_hidden: ap.hidden,
            channel: ap.channel,
            secondary_channel: None,
            protocols: EnumSet::empty(),
            auth_method: AuthMethod::WPA2Personal,
            password: ap.password,
            max_connections: 10,
        });

        Ok(Self {
            ip_info,
            sta_config,
            ap_config,
            ap_mode: ap.mode,
        })
    }
}
