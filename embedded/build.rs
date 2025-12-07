use std::env;
use dotenvy::from_path;

fn main() {
    let _ = from_path(".env");

    println!("cargo:rerun-if-changed=.env");
    
    if let Ok(ssid) = env::var("WIFI_SSID") {
        println!("cargo:rustc-env=WIFI_SSID={}", ssid);
    }

    if let Ok(password) = env::var("WIFI_PASSWORD") {
        println!("cargo:rustc-env=WIFI_PASSWORD={}", password);
    }

    if let Ok(server_pubkey) = env::var("SERVER_PUBLIC_KEY") {
        println!("cargo:rustc-env=SERVER_PUBLIC_KEY={}", server_pubkey);
    }

    if let Ok(broker_url) = env::var("MQTT_BROKER") {
        println!("cargo:rustc-env=MQTT_BROKER={}", broker_url);
    }

    if let Ok(mqtt_topic) = env::var("MQTT_TOPIC") {
        println!("cargo:rustc-env=MQTT_TOPIC={}", mqtt_topic);
    }

    embuild::espidf::sysenv::output();
}
