use crate::{
    template::WifiSettingsTemplate,
    wifi::{self, storage::WifiStorage},
    Command, CMD_QUEUE,
};
use askama::Template as _;
use embedded_svc::{
    http::{
        server::{HandlerError, Request},
        Headers,
    },
    io::{Read, Write},
};
use esp_idf_svc::http::server::EspHttpConnection;

pub fn root_handler<T>(
    request: Request<&mut EspHttpConnection>,
    wifi_storage: &WifiStorage<T>,
) -> Result<(), HandlerError>
where
    T: esp_idf_svc::nvs::NvsPartitionId,
{
    let info = wifi_storage.get_info()?;
    let mut response = request.into_ok_response()?;
    response.write_all(WifiSettingsTemplate::from(info).render()?.as_bytes())?;
    Ok(())
}

pub fn post_handler(mut request: Request<&mut EspHttpConnection>) -> Result<(), HandlerError> {
    let len = request.content_len().unwrap_or_default();
    let mut buf = vec![0; len as usize];
    request.read_exact(&mut buf)?;
    let settings: WifiSettingsTemplate = serde_json::from_slice(&buf)?;
    CMD_QUEUE.enqueue(Command::UpdateWifi(settings))?;
    Ok(())
}

pub fn scan_handler(request: Request<&mut EspHttpConnection>) -> Result<(), HandlerError> {
    use embedded_svc::wifi::AccessPointInfo;

    let buf = wifi::scan_aps()?;
    let aps: Vec<_> = buf.into_iter().map(APInfo).collect();

    #[derive(serde::Serialize)]
    struct NetApi {
        networks: Vec<APInfo>,
    }

    #[derive(serde::Serialize)]
    #[serde(transparent)]
    struct APInfo(#[serde(serialize_with = "serialize_ap_info")] AccessPointInfo);

    pub fn serialize_ap_info<S>(data: &AccessPointInfo, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AccessPointInfo", 5)?;
        state.serialize_field("ssid", &data.ssid)?;
        let [a, b, c, d, e, f] = data.bssid;
        state.serialize_field(
            "bssid",
            &format!("{a:02X}:{b:02X}:{c:02X}:{d:02X}:{e:02X}:{f:02X}"),
        )?;

        state.serialize_field("channel", &data.channel)?;
        state.serialize_field("rssi", &data.signal_strength)?;
        state.serialize_field("enc", &(data.auth_method as u8))?;
        state.end()
    }

    // let ap_info = receiver.recv()?;
    let json = serde_json::to_vec(&NetApi { networks: aps })?;
    let mut response =
        request.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?;
    response.write_all(&json)?;
    Ok(())
}
