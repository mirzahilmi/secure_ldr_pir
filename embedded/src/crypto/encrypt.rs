use x25519_dalek::{
    EphemeralSecret,
    PublicKey,
};
use ascon_aead::{
    AsconAead128, 
    AsconAead128Key, 
    AsconAead128Nonce,
};
use ascon_aead::aead::{
    Aead,
    KeyInit,
};
use base64::{engine::general_purpose, Engine as _};
use hkdf::Hkdf;
use sha2::Sha256;
use crate::model::Telemetry;
use core::sync::atomic::{
    AtomicU16, 
    Ordering,
};
use crate::mqtt::publish_telemetry_with_retry;
use esp_idf_svc::mqtt::client::EspMqttClient;
use esp_idf_svc::sys::esp_timer_get_time;

static NONCE_COUNTER: AtomicU16 = AtomicU16::new(0);

// digunakan untuk derive symmetric key dan nonce seed dari ECDH shared secret menggunakan HKDF-SHA256
pub fn derive_symmetric_params(ephemeral_secret_key: EphemeralSecret, server_public_key: &PublicKey) -> (AsconAead128, [u8; 16]) {
    let shared_secret = ephemeral_secret_key.diffie_hellman(server_public_key);
    let shared_secret_bytes = shared_secret.as_bytes();

    let hkdf = Hkdf::<Sha256>::new(None, shared_secret_bytes);
    let mut output_keying_material = [0u8; 32];
    hkdf.expand(b"ascon-derive-v1", &mut output_keying_material).unwrap();

    let ascon_key_bytes = &output_keying_material[0..16];
    let ascon_cipher_key = AsconAead128Key::from_slice(ascon_key_bytes); // adl raw key bytes
    let ascon_cipher = AsconAead128::new(ascon_cipher_key);

    let mut nonce_seed_bytes = [0u8; 16];
    nonce_seed_bytes.copy_from_slice(&output_keying_material[16..32]);

    (ascon_cipher, nonce_seed_bytes)
}

// digunakan untuk mengenkripsi payload telemetry menggunakan ASCON-AEAD128
fn encrypt_payload(ascon_cipher: &AsconAead128, nonce_seed_bytes: &[u8; 16], plaintext_json: &str, timestamp_unix_millis: u64) -> (String, String) {
    let nonce_bytes = generate_nonce(nonce_seed_bytes, timestamp_unix_millis);

    let nonce = AsconAead128Nonce::from_slice(&nonce_bytes);
    let ciphertext = ascon_cipher.encrypt(nonce, plaintext_json.as_bytes()).unwrap();

    (
        general_purpose::STANDARD.encode(ciphertext),
        general_purpose::STANDARD.encode(nonce_bytes),
    )
}

fn generate_nonce(nonce_seed_bytes: &[u8; 16], timestamp_unix_millis: u64) -> [u8; 16] {
    let mut nonce = [0u8; 16]; // size nonce ascon adalah 16 bytes
    
    let selector = ((timestamp_unix_millis & 0xFF) as u8) ^ ((timestamp_unix_millis >> 8 & 0xFF) as u8);

    // 6 bytes dari nonce diambil dari nonce_seed_bytes berdasarkan selector
    let selected_seed_part = pick_6_bytes_from_nonce_seed(nonce_seed_bytes, selector);
    nonce[0..6].copy_from_slice(&selected_seed_part);

    // 8 bytes berikutnya adalah timestamp dalam millis (big-endian)
    nonce[6..14].copy_from_slice(&timestamp_unix_millis.to_be_bytes());

    // 2 bytes terakhir adalah counter yang diincrement setiap pemanggilan
    let counter = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    nonce[14..16].copy_from_slice(&counter.to_be_bytes());

    nonce
}

fn pick_6_bytes_from_nonce_seed(seed: &[u8; 16], selector: u8) -> [u8; 6] {
    let start_index = selector as usize % 11; // 16 - 6 + 1 = 11

    let mut result = [0u8; 6];
    result.copy_from_slice(&seed[start_index..start_index + 6]);
    result
}

// digunakan untuk mengenkripsi data telemetry pada saat terjadi perubahan reading dari sensor
// pub fn encrypt_on_sensor_event(ascon_cipher: &AsconAead128, nonce_seed_bytes: &[u8; 16], ephemeral_public_key: &PublicKey, payload: &Telemetry) {
//     let plaintext_json = serde_json::to_string(payload).unwrap();
//     let (ciphertext_base64, nonce_base64) = encrypt_payload(ascon_cipher, nonce_seed_bytes, &plaintext_json, payload.timestamp_ms);

//     let output = serde_json::json!({
//         "header": {
//             "algorithm": "X25519+HKDF-SHA256+ASCON128",
//             "ephemeral_public_key": general_purpose::STANDARD.encode(ephemeral_public_key.as_bytes())
//         },
//         "nonce": nonce_base64,
//         "ciphertext": ciphertext_base64,
//     });

//     log::info!("Encrypted message -> {}", output.to_string());
// }

// digunakan untuk mengenkripsi data telemetry pada saat terjadi perubahan reading dari sensor dan sending via mqtt
pub fn encrypt_on_sensor_event_and_send(ascon_cipher: &AsconAead128, nonce_seed_bytes: &[u8; 16], ephemeral_public_key: &PublicKey, payload: &Telemetry, mqtt_client: &mut EspMqttClient<'_>, topic: &str) {
    let plaintext_json = serde_json::to_string(payload).unwrap();

    let start = unsafe { esp_timer_get_time() };
    
    let (ciphertext_base64, nonce_base64) = encrypt_payload(ascon_cipher, nonce_seed_bytes, &plaintext_json, payload.timestamp_ms);

    let end = unsafe { esp_timer_get_time() };
    let encryption_duration_ns = ((end - start) as u64) * 1000;

    let output = serde_json::json!({
        "header": {
            "algorithm": "X25519+HKDF-SHA256+ASCON128",
            "ephemeral_public_key": general_purpose::STANDARD.encode(ephemeral_public_key.as_bytes())
        },
        "nonce": nonce_base64,
        "ciphertext": ciphertext_base64,
        "metrics": {
            "encrypt_time_ns": encryption_duration_ns
        }
    });

    let payload_str = output.to_string();
    
    publish_telemetry_with_retry(mqtt_client, topic, &payload_str, 3);
}

