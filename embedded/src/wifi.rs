use std::thread;
use std::time::Duration;

use anyhow::Result;
use esp_idf_svc::wifi::{
    EspWifi, 
    ClientConfiguration, 
    AuthMethod, 
    Configuration
};
use esp_idf_svc::hal::gpio::{
    self as gpio,
    PinDriver,
    Output,
};

pub fn connect_wifi(wifi: &mut EspWifi, led: &mut PinDriver<'_, gpio::Gpio2, Output>, ssid: &str, password: &str) -> Result<()> {
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