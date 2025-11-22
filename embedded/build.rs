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

    embuild::espidf::sysenv::output();
}
