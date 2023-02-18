use embedded_svc::{storage::RawStorage};
use esp_idf_svc::nvs;
use esp_idf_sys as sys;
use super::{WifiInfo, DEDAULT_IP_INFO, ApMode};

pub struct WifiStorage<T: nvs::NvsPartitionId> {
    nvs: nvs::EspNvs<T>,
}

impl<P: nvs::NvsPartitionId> WifiStorage<P> {
    const SETTINGS_KEY: &str = "settings";

    pub fn new(nvs_partition: nvs::EspNvsPartition<P>) -> Result<Self, sys::EspError> {
        let nvs = nvs::EspNvs::new(nvs_partition, "wifi", true)?;
        Ok(Self { nvs })
    }

    pub fn get_info(&self) -> Result<WifiInfo, anyhow::Error> {
        let mut buf = heapless::Vec::<_, 240>::new();
        let Some(len) = self.nvs.len(Self::SETTINGS_KEY)? else {
            return Ok(
                WifiInfo { 
                    ip_info: DEDAULT_IP_INFO, 
                    sta_config: None, 
                    ap_config: None, 
                    ap_mode: ApMode::NoConnOnBoot
                }
            )
        };
        buf.resize_default(len)
            .expect("value is less than 120 bytes");
        self.nvs.get_raw(Self::SETTINGS_KEY, &mut buf[..len])?;
        Ok(postcard::from_bytes(&buf)?)
    }

    pub fn set_info(&mut self, config: Option<&WifiInfo>) -> Result<(), anyhow::Error> {
        if let Some(config) = config {
            let buf = postcard::to_vec::<_, 240>(&config)?;
            self.nvs.set_raw(Self::SETTINGS_KEY, &buf)?;
        } else {
            self.nvs.remove(Self::SETTINGS_KEY)?;
        }
        Ok(())
    }
}