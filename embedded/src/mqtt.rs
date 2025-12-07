use std::thread;
use std::ffi::CStr;
use esp_idf_svc::mqtt::client::{
    MqttClientConfiguration, 
    QoS,
    EspMqttClient,
};
use esp_idf_svc::tls::X509;
use std::time::Duration;
use crate::ca::CA_CERT;

pub fn setup_mqtt_client(broker_url: &str) -> EspMqttClient<'_> {
    let ca_cstr = CStr::from_bytes_with_nul(CA_CERT.as_bytes())
        .expect("CA_CERT should be a valid C string");

    let conf = MqttClientConfiguration {
        client_id: Some("esp32-device-001".into()),
        server_certificate: Some(X509::pem(ca_cstr)),
        ..Default::default()
    };

    let url = format!("wss://{}", broker_url);
    let (mqtt_client, _connection) = EspMqttClient::new(url.as_str(), &conf).expect("Failed to create MQTT client");

    mqtt_client
}

pub fn publish_telemetry_with_retry(client: &mut EspMqttClient, topic: &str, payload: &str, max_retries: u8) {
    for attempt in 1..=max_retries {
        match client.publish(topic, QoS::AtMostOnce, false, payload.as_bytes()) {
            Ok(_) => {
                log::info!("Payload berhasil dikirim ke topic {}", topic);
                break;
            }
            Err(e) => {
                log::warn!("Gagal mengirim payload (attempt {}): {:?}", attempt + 1, e);
                if attempt < max_retries {
                    thread::sleep(Duration::from_millis(500));
                } else {
                    log::error!("Gagal mengirim payload setelah {} percobaan", max_retries);
                }
            }
        }
    }
}