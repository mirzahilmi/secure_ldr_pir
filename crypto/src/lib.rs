use base64::Engine;
use hkdf::Hkdf;
use sha2::Sha256;
use std::str;

use ascon_aead::aead::{Aead, KeyInit};
use ascon_aead::{AsconAead128, AsconAead128Key, AsconAead128Nonce};
use x25519_dalek::{PublicKey, StaticSecret};

use std::ffi::{CStr, c_char};

#[repr(C)]
pub enum Status {
    Ok = 0,
    Error = -1,
    TooSmall = -2,
}

#[repr(C)]
#[derive(Debug)]
pub struct Cipher {
    pub ciphertext: *const c_char,
    pub public_key: *const c_char,
    pub nonce: *const c_char,
}

#[unsafe(no_mangle)]
pub extern "C" fn hello_world() {
    println!("Hello, World from Rust!");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hello_name(name: *const c_char) {
    let name = unsafe { CStr::from_ptr(name).to_str().unwrap() };
    println!("Hello, {name}");
}

/// # Safety
/// It is infact unsafe
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decrypt(
    cipher: Cipher,
    out: *mut u8,
    size: usize,
    actual_size: *mut usize,
) -> Status {
    let private_key = match std::env::var("PRIVATE_KEY") {
        Ok(key) => key,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };

    let public_key = match unsafe { CStr::from_ptr(cipher.public_key).to_str() } {
        Ok(key) => key,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };

    let nonce = match unsafe { CStr::from_ptr(cipher.nonce).to_str() } {
        Ok(nonce) => nonce,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };
    let nonce = match base64::engine::general_purpose::STANDARD.decode(nonce) {
        Ok(nonce) => nonce,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };

    let ciphertext = match unsafe { CStr::from_ptr(cipher.ciphertext).to_str() } {
        Ok(ciphertext) => ciphertext,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };
    let ciphertext = match base64::engine::general_purpose::STANDARD.decode(ciphertext) {
        Ok(ciphertext) => ciphertext,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };

    let private_key = match hex::decode(private_key) {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };
    let private_key: [u8; 32] = match private_key.as_slice().try_into() {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };
    let secret_key = StaticSecret::from(private_key);

    let public_key = match base64::engine::general_purpose::STANDARD.decode(public_key) {
        Ok(key) => key,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };
    let public_key: [u8; 32] = match public_key.as_slice().try_into() {
        Ok(key) => key,
        Err(e) => {
            println!("{e}");
            return Status::Error;
        }
    };
    let eph_pub = PublicKey::from(public_key);

    let shared = secret_key.diffie_hellman(&eph_pub);
    let shared_bytes = shared.as_bytes();

    let hk = Hkdf::<Sha256>::new(None, shared_bytes);
    let mut okm = [0u8; 32];
    if let Err(e) = hk.expand(b"ascon-derive-v1", &mut okm) {
        println!("{e}");
        return Status::Error;
    }

    let k_ascon = &okm[0..16];

    let key = AsconAead128Key::from_slice(k_ascon);
    let cipher = AsconAead128::new(key);

    if nonce.len() != 16 {
        println!("nonce size is not equal to 16");
        return Status::Error;
    }
    let nonce_arr: [u8; 16] = nonce.as_slice().try_into().unwrap();
    let ascon_nonce = AsconAead128Nonce::from_slice(&nonce_arr);

    let plaintext = cipher
        .decrypt(ascon_nonce, ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Decryption failed"))
        .unwrap();

    println!(
        "Decrypted plaintext JSON: {}",
        str::from_utf8(&plaintext).unwrap()
    );

    if size < plaintext.len() {
        unsafe { *actual_size = plaintext.len() }
        return Status::TooSmall;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(plaintext.as_ptr(), out, plaintext.len());
        *actual_size = plaintext.len()
    }

    Status::Ok
}
