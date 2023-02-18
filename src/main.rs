mod animation;
mod convert;
mod http;
mod template;
mod wifi;

use crate::{
    animation::Loader,
    template::WifiSettingsTemplate,
    wifi::{storage::WifiStorage, update_wifi},
};
use core::time::Duration;
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::Point,
    primitives::PrimitiveStyle,
    text::{Baseline, Text},
    Drawable,
};
use embedded_svc::http::Method;
use esp_idf_hal::{i2c::I2cDriver, prelude::Peripherals};
use esp_idf_svc::{eventloop::EspSystemEventLoop, http::server::EspHttpServer, nvs, wifi::EspWifi};
use log::info;
use ssd1306::{
    prelude::DisplayConfig, rotation::DisplayRotation, size::DisplaySize128x64,
    I2CDisplayInterface, Ssd1306,
};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as sys;

static CMD_QUEUE: heapless::mpmc::Q2<Command> = heapless::mpmc::Q2::new();

#[derive(Debug, Clone)]
enum Command {
    UpdateWifi(WifiSettingsTemplate),
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::log::EspLogger::initialize_default();
    log::set_max_level(log::LevelFilter::Trace);
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    sys::link_patches();

    let Peripherals {
        modem, i2c0, pins, ..
    } = Peripherals::take().unwrap();

    info!("Initialize NVS");
    let nvs_partition = nvs::EspDefaultNvsPartition::take()?;

    info!("Load WiFi settings from NVS");
    let mut wifi_storage = WifiStorage::new(nvs_partition.clone())?;
    let wifi_info = wifi_storage.get_info()?;

    let sysloop = EspSystemEventLoop::take()?;
    let mut wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs_partition.clone()))?;

    info!("Wifi capabilities: {:?}", wifi.get_capabilities()?);
    update_wifi(&mut wifi, &sysloop, wifi_info)?;

    // TODO: implement [`Captive Portal`](https://gitlab.com/defcronyke/wifi-captive-portal-esp-idf).
    // This requires that we need a simple DNS server.
    let mut http_server = EspHttpServer::new(&Default::default())?;
    http_server
        .fn_handler("/", Method::Get, {
            let wifi_storage = WifiStorage::new(nvs_partition)?;
            move |request| http::root_handler(request, &wifi_storage)
        })?
        .fn_handler("/json/net", Method::Get, http::scan_handler)?
        .fn_handler("/", Method::Post, http::post_handler)?;

    // Setup SSD1306 Display
    let display_driver = I2cDriver::new(
        i2c0,
        pins.gpio10,
        pins.gpio9,
        &esp_idf_hal::i2c::I2cConfig::default(),
    )?;
    let interface = I2CDisplayInterface::new(display_driver);
    let mut display = Box::new(
        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode(),
    );
    display.init().unwrap();
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();
    let message = Text::with_baseline("Hello Rust!", Point::zero(), text_style, Baseline::Top);

    let raw = ImageRaw::<BinaryColor>::new(include_bytes!("../assets/rust.raw"), 64);
    let img = Image::new(&raw, Point::new(32, 0));
    let mut loader = Loader::new(
        Point::new(128 - 12, 12),
        20,
        PrimitiveStyle::with_stroke(BinaryColor::On, 2),
    );

    let mut last = current_time()?;
    loop {
        if let Some(Command::UpdateWifi(template)) = CMD_QUEUE.dequeue() {
            info!("setting new Wifi configuration");
            let wifi_info = template.try_into()?;
            wifi_storage.set_info(Some(&wifi_info))?;
            update_wifi(&mut wifi, &sysloop, wifi_info)?;
        }
        let now = current_time()?;
        // Clear screen
        display.clear();
        // Update (animation) components
        loader.update(Duration::from_micros(now - last));

        // redraw all component
        message.draw(display.as_mut()).unwrap();
        img.draw(display.as_mut()).unwrap();
        loader.draw(display.as_mut()).unwrap();
        display.flush().unwrap();
        last = now;
        unsafe { sys::usleep(10_000) };
    }
}

fn current_time() -> Result<u64, sys::EspError> {
    let mut tv_now = Default::default();
    unsafe { sys::esp!(sys::gettimeofday(&mut tv_now, core::ptr::null_mut()))? };
    Ok(tv_now.tv_sec as u64 * 1000000 + tv_now.tv_usec as u64)
}
