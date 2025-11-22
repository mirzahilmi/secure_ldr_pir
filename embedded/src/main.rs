use std::thread;
use std::env;
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};
use chrono::{
    DateTime,
    Utc,
    FixedOffset,
};

use anyhow::Result;
use serde::Serialize;

use esp_idf_svc::log::EspLogger;
// use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::gpio::{
    self as gpio,
    PinDriver,
    Output,
};

use esp_idf_svc::wifi::{
    EspWifi,
    ClientConfiguration,
    AuthMethod,
    Configuration
};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::sntp::{
    EspSntp,
    SyncStatus
};

#[derive(Serialize)]
struct Telemetry {
    device_id: &'static str,
    timestamp: u64,
    ldr: bool,
    pir: bool,
}

fn get_timestamp_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn sync_time_via_ntp() -> Result<()> {
    let ntp = EspSntp::new_default().unwrap();

    log::info!("Starting NTP time sync...");

    while ntp.get_sync_status() != SyncStatus::Completed {
        log::info!("Waiting for NTP time sync...");
        thread::sleep(Duration::from_secs(1));
    }

    log::info!("NTP time sync completed.");

    let system_time_now = SystemTime::now();
    let utc: DateTime<Utc> = system_time_now.into();

    let utc_plus_7 = utc.with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap());

    log::info!("Current time (UTC+7): {}", utc_plus_7.format("%d/%m/%Y %H:%M:%S"));

    Ok(())
}

fn connect_wifi(wifi: &mut EspWifi, led: &mut PinDriver<'_, gpio::Gpio2, Output>, ssid: &str, password: &str) -> Result<()> {
    let wifi_config = Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        password: password.try_into().unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_config)?;

    wifi.start()?;
    wifi.connect()?;

    log::info!("Connecting to WiFi...");

    while !wifi.is_connected()? {
        led.toggle()?;
        thread::sleep(Duration::from_millis(300));
    }

    led.set_low()?;
    log::info!("Connected to WiFi network: {}", ssid);

    Ok(())
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // setup wifi
    // while connecting to wifi, builtin led shoould blink
    let mut builtin_led = PinDriver::output(peripherals.pins.gpio2)?;

    let mut wifi = EspWifi::new(peripherals.modem, sysloop, Some(nvs))?;

    connect_wifi(&mut wifi, &mut builtin_led, env!("WIFI_SSID"), env!("WIFI_PASSWORD"))?;

    thread::sleep(Duration::from_secs(1));

    sync_time_via_ntp()?;

    //setup sensor pins
    let ldr_pin = PinDriver::input(peripherals.pins.gpio5)?;
    let pir_pin = PinDriver::input(peripherals.pins.gpio23)?;

    let mut prev_ldr_value = ldr_pin.is_high();
    let mut prev_pir_value = pir_pin.is_high();

    loop {
        let ldr_value = ldr_pin.is_high();
        let pir_value = pir_pin.is_high();

        if ldr_value != prev_ldr_value || pir_value != prev_pir_value {
            let payload = Telemetry {
                device_id: "esp32-device-001",
                timestamp: get_timestamp_unix(),
                ldr: ldr_value,
                pir: pir_value,
            };

            let payload_json = serde_json::to_string(&payload)?;
            log::info!("Payload: {}", payload_json);

            prev_ldr_value = ldr_value;
            prev_pir_value = pir_value;
        }

        thread::sleep(Duration::from_millis(200));
    }
}