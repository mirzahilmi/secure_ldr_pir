use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use hex;
use hkdf::Hkdf;
use serde::Deserialize;
use sha2::Sha256;
use std::str;

use x25519_dalek::{PublicKey, StaticSecret};
use ascon_aead::{AsconAead128, AsconAead128Key, AsconAead128Nonce};
use ascon_aead::aead::{Aead, KeyInit};

#[derive(Deserialize, Debug)]
struct EncryptedHeader {
    // algorithm: String,
    ephemeral_public_key: String,
}

#[derive(Deserialize, Debug)]
struct EncryptedMessage {
    header: EncryptedHeader,
    nonce: String,
    ciphertext: String,
}

fn hex_to_32_bytes(hex_str: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(hex_str)?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Expected 32 bytes for key"))?;
    Ok(arr)
}

fn base64_to_vec(s: &str) -> Result<Vec<u8>> {
    let v = general_purpose::STANDARD.decode(s)?;
    Ok(v)
}


fn main() -> Result<()> {
    // ---------- SAMPLE ENCRYPTED JSON (replace with real input) ----------
    let sample_json = r#"
    {
    "ciphertext": "+cF4V6z9OnbolyT+5Z2DrHuox+PRuqwX/TaUPIVryUqjRTm0Pc9/GXlbNJG0XwnyRsXr1fUsx6XlwKDwGsTettEGie2ld7U58EI4/pFzq20IIO6w4jRxgkDkn9iix8lfw1J9Ew==",
    "header": {
        "algorithm": "X25519+HKDF-SHA256+ASCON128",
        "ephemeral_public_key": "25ru/9JtiE5b0cUHHDHoRqNlnxRBlLhQKoQ5vxX3tE8="
    },
    "nonce": "6wg6L5GxAAABmrpvISEADg=="
}
    "#;

    // Parse JSON
    let msg: EncryptedMessage = serde_json::from_str(sample_json)?;
    // println!("Parsed message header: {:?}", msg.header);

    // ---------- Server static private key ----------
    let server_private_key_hex = "4174bee44869f6672f32daed3ca7dd10b8a8141813df58ebfc00dda0563cfbc1";

    let server_sk_bytes = hex_to_32_bytes(server_private_key_hex)?;
    let server_secret = StaticSecret::from(server_sk_bytes);

    // ---------- decode ephemeral public key ----------
    let eph_pub_b64 = msg.header.ephemeral_public_key;
    let eph_pub_bytes = base64_to_vec(&eph_pub_b64)?;
    let eph_pub_arr: [u8; 32] = eph_pub_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("ephemeral public key must be 32 bytes"))?;

    let eph_pub = PublicKey::from(eph_pub_arr);

    // ---------- Perform ECDH: server_secret (static) x eph_pub (device ephemeral) ----------
    let shared = server_secret.diffie_hellman(&eph_pub);
    let shared_bytes = shared.as_bytes();

    // ---------- HKDF derive same okm (32 bytes) ----------
    let hk = Hkdf::<Sha256>::new(None, shared_bytes);
    let mut okm = [0u8; 32];
    hk.expand(b"ascon-derive-v1", &mut okm)
        .map_err(|_| anyhow::anyhow!("HKDF expand failed"))?;

    // K_ascon is first 16 bytes
    let k_ascon = &okm[0..16];

    // ---------- create Ascon cipher instance ----------
    let key = AsconAead128Key::from_slice(k_ascon);
    let cipher = AsconAead128::new(key);

    // println!("Derived Ascon key : {:?}", key);
    println!("Ascon cipher key (hex): {}", hex::encode(key.as_slice()));

    // ---------- decode nonce and ciphertext from base64 ----------
    let nonce_bytes = base64_to_vec(&msg.nonce)?;
    if nonce_bytes.len() != 16 {
        return Err(anyhow::anyhow!("nonce must be 16 bytes"));
    }
    let nonce_arr: [u8; 16] = nonce_bytes.as_slice().try_into().unwrap();
    let ascon_nonce = AsconAead128Nonce::from_slice(&nonce_arr);

    let ciphertext_bytes = base64_to_vec(&msg.ciphertext)?;

    // println!("Ciphertext bytes: {:?}", ciphertext_bytes);

    // println!("Ciphertext (hex): {}", hex::encode(&ciphertext_bytes));

    // println!("Ascon nonce (hex): {}", hex::encode(ascon_nonce.as_slice()));

    // println!("Nonce bytes: {:?}", nonce_bytes);

    // ---------- Decrypt ----------
    let plaintext_bytes = cipher.decrypt(ascon_nonce, ciphertext_bytes.as_ref())
        .map_err(|_| anyhow::anyhow!("Decryption failed"))?;

    let plaintext_str = str::from_utf8(&plaintext_bytes)?;
    println!("Decrypted plaintext JSON: {}", plaintext_str);

    Ok(())
}
