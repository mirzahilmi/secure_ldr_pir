mod model;
mod wifi;
mod util;
mod sensor;
mod crypto;
mod mqtt;
mod ca;

use model::Telemetry;
use util::{
    sync_time_via_ntp,
    get_timestamp_unix,
};
use wifi::connect_wifi;
use sensor::Sensors;
use crypto::{
    key::generate_ephemeral_keypair,
    encrypt::derive_symmetric_params,
    encrypt::encrypt_on_sensor_event_and_send,
};
use mqtt::setup_mqtt_client;

use std::thread;
use std::env;
use std::time::Duration;
use anyhow::Result;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::gpio::{
    PinDriver,
};
use esp_idf_svc::hal::adc::attenuation::DB_11;
use esp_idf_svc::hal::adc::oneshot::AdcDriver;
use esp_idf_svc::hal::adc::oneshot::AdcChannelDriver;
use esp_idf_svc::hal::adc::oneshot::config::AdcChannelConfig;

use esp_idf_svc::wifi::EspWifi;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::sntp::EspSntp;
use x25519_dalek::PublicKey;
use base64::{engine::general_purpose, Engine as _};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let Peripherals { pins, modem, adc1, .. } = peripherals;

    //setup cryptos
    let server_public_key_base64 = env!("SERVER_PUBLIC_KEY");
    let server_public_key_vec = general_purpose::STANDARD.decode(server_public_key_base64).expect("Valid base64 pubkey");

    let server_public_key_bytes: [u8; 32] = server_public_key_vec
        .try_into()
        .expect("Server public key should be 32 bytes");

    let server_public_key = PublicKey::from(server_public_key_bytes);

    let (eph_secret, eph_public) = generate_ephemeral_keypair();
    let (cipher, nonce_seed) = derive_symmetric_params(eph_secret, &server_public_key);

    //pins
    let gpio2 = pins.gpio2; // builtin led
    // let gpio5 = pins.gpio5; // ldr sensor
    let gpio23 = pins.gpio23; // pir sensor
    let gpio35 = pins.gpio35; // ldr sensor (analog)

    // setup wifi
    // while connecting to wifi, builtin led shoould blink
    let mut builtin_led = PinDriver::output(gpio2)?;
    let mut wifi = EspWifi::new(modem, sysloop, Some(nvs))?;
    connect_wifi(&mut wifi, &mut builtin_led, env!("WIFI_SSID"), env!("WIFI_PASSWORD"))?;

    thread::sleep(Duration::from_secs(1));

    let sntp = EspSntp::new_default().unwrap();
    sync_time_via_ntp(&sntp, &mut builtin_led)?;

    // MQTT setup
    let mut mqtt_client = setup_mqtt_client(env!("MQTT_BROKER"));
    let topic = env!("MQTT_TOPIC");

    //setup sensor pins
    let sensors = Sensors::new(gpio23)?;
    // let mut prev_ldr_value = sensors.ldr.is_high();
    let mut prev_pir_value = sensors.pir.is_high();

    // LDR sensor analog readings
    let adc = AdcDriver::new(adc1)?;
    let config = AdcChannelConfig {
        attenuation: DB_11,
        ..Default::default()
    };
    let mut adc_pin_35 = AdcChannelDriver::new(&adc, gpio35, &config)?;

    let mut last_send_time = get_timestamp_unix();
    let send_interval_ms: u64 = 10000; 
    let threshold = 50;

    let mut prev_ldr_value: u16 = adc.read(&mut adc_pin_35)?;
    let mut first_send = true;

    loop {
        // let ldr_value = sensors.ldr.is_high();
        let pir_value = sensors.pir.is_high();
        
        let ldr_value: u16 = adc.read(&mut adc_pin_35)?;
        let delta = (ldr_value as i32 - prev_ldr_value as i32).abs();
        let time_elapsed = get_timestamp_unix() - last_send_time;

        if first_send || delta > threshold || time_elapsed >= send_interval_ms || pir_value != prev_pir_value {
            let payload = Telemetry {
                device_id: "esp32-device-001",
                timestamp_ms: get_timestamp_unix(),
                ldr: ldr_value,
                pir: pir_value,
            };

            // log::info!("\nLDR Sensor Readings: {}", ldr_value);
            // log::info!("PIR Sensor State: {}", pir_value);
            encrypt_on_sensor_event_and_send(&cipher, &nonce_seed, &eph_public, &payload, &mut mqtt_client, &topic);

            // let payload_json = serde_json::to_string(&payload)?;
            // log::info!("Payload: {}", payload_json);

            prev_ldr_value = ldr_value;
            prev_pir_value = pir_value;

            last_send_time = get_timestamp_unix();

            first_send = false;
        }

        thread::sleep(Duration::from_millis(200));
    }
}