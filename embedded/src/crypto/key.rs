use x25519_dalek::{
    EphemeralSecret,
    PublicKey,
};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};

pub fn generate_ephemeral_keypair() -> (EphemeralSecret, PublicKey) {
    let ephemeral_secret_key = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_public_key = PublicKey::from(&ephemeral_secret_key);

    log::info!("Ephemeral public key (base64): {}", general_purpose::STANDARD.encode(ephemeral_public_key.as_bytes()));

    (ephemeral_secret_key, ephemeral_public_key)
}