// @oagen-ignore-file
//! Vault — local AES-256-GCM crypto helpers.
//!
//! Wire format for `LocalEncrypt` (matches workos-go):
//! `LEB128(len(encryptedKeys)) || encryptedKeys || nonce(12) || ciphertext+tag`
//! the whole thing then base64-encoded.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64_STANDARD;
use rand::RngCore;

use crate::error::Error;

/// Result of [`crate::resources::VaultApi::encrypt`].
#[derive(Debug, Clone)]
pub struct VaultEncryptResult {
    pub encrypted_data: String,
    pub context: std::collections::HashMap<String, String>,
    pub encrypted_keys: String,
}

/// Locally encrypt `data` with AES-256-GCM.
///
/// * `data_key_b64` — base64-encoded 32-byte AES key (from `CreateDataKeyResponse.data_key`).
/// * `encrypted_keys_b64` — base64-encoded encrypted key blob (from `CreateDataKeyResponse.encrypted_keys`).
/// * `associated_data` — AAD bound into the AEAD tag.
pub fn local_encrypt(
    data: &str,
    data_key_b64: &str,
    encrypted_keys_b64: &str,
    associated_data: &str,
) -> Result<String, Error> {
    let raw_key = B64_STANDARD
        .decode(data_key_b64)
        .map_err(|e| Error::VaultCrypto(format!("decode data key: {e}")))?;
    if raw_key.len() != 32 {
        return Err(Error::VaultCrypto(format!(
            "data key must be 32 bytes; got {}",
            raw_key.len()
        )));
    }
    let encrypted_keys = B64_STANDARD
        .decode(encrypted_keys_b64)
        .map_err(|e| Error::VaultCrypto(format!("decode encrypted keys: {e}")))?;

    let cipher = Aes256Gcm::new_from_slice(&raw_key)
        .map_err(|e| Error::VaultCrypto(format!("init AES-GCM: {e}")))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: data.as_bytes(),
                aad: associated_data.as_bytes(),
            },
        )
        .map_err(|e| Error::VaultCrypto(format!("encrypt: {e}")))?;

    let prefix = encode_leb128(encrypted_keys.len() as u32);
    let mut buf = Vec::with_capacity(
        prefix.len() + encrypted_keys.len() + nonce_bytes.len() + ciphertext.len(),
    );
    buf.extend_from_slice(&prefix);
    buf.extend_from_slice(&encrypted_keys);
    buf.extend_from_slice(&nonce_bytes);
    buf.extend_from_slice(&ciphertext);
    Ok(B64_STANDARD.encode(buf))
}

/// Locally decrypt `encrypted_data` with AES-256-GCM.
///
/// * `data_key_b64` — base64-encoded AES key (from `DecryptResponse.data_key`).
/// * `associated_data` — AAD that was passed to [`local_encrypt`].
pub fn local_decrypt(
    encrypted_data: &str,
    data_key_b64: &str,
    associated_data: &str,
) -> Result<String, Error> {
    let raw = B64_STANDARD
        .decode(encrypted_data)
        .map_err(|e| Error::VaultCrypto(format!("base64 decode: {e}")))?;
    let (keys_len, bytes_read) = decode_leb128(&raw)?;
    let offset = bytes_read + keys_len as usize;
    if offset + 12 > raw.len() {
        return Err(Error::VaultCrypto(
            "encrypted data too short: missing nonce".to_string(),
        ));
    }
    let nonce = &raw[offset..offset + 12];
    let ciphertext = &raw[offset + 12..];
    if ciphertext.is_empty() {
        return Err(Error::VaultCrypto(
            "encrypted data too short: missing ciphertext".to_string(),
        ));
    }
    let raw_key = B64_STANDARD
        .decode(data_key_b64)
        .map_err(|e| Error::VaultCrypto(format!("decode data key: {e}")))?;
    let cipher = Aes256Gcm::new_from_slice(&raw_key)
        .map_err(|e| Error::VaultCrypto(format!("init AES-GCM: {e}")))?;
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad: associated_data.as_bytes(),
            },
        )
        .map_err(|e| Error::VaultCrypto(format!("decrypt: {e}")))?;
    String::from_utf8(plaintext).map_err(|e| Error::VaultCrypto(format!("utf-8: {e}")))
}

/// Extract the base64-encoded encrypted-keys blob from a sealed envelope.
pub fn extract_encrypted_keys(encrypted_data: &str) -> Result<String, Error> {
    let raw = B64_STANDARD
        .decode(encrypted_data)
        .map_err(|e| Error::VaultCrypto(format!("base64 decode: {e}")))?;
    let (keys_len, bytes_read) = decode_leb128(&raw)?;
    let total = bytes_read + keys_len as usize;
    if raw.len() < total {
        return Err(Error::VaultCrypto(
            "encrypted data too short for declared key length".to_string(),
        ));
    }
    Ok(B64_STANDARD.encode(&raw[bytes_read..total]))
}

fn encode_leb128(mut n: u32) -> Vec<u8> {
    if n == 0 {
        return vec![0];
    }
    let mut out = Vec::new();
    while n > 0 {
        let mut b = (n & 0x7f) as u8;
        n >>= 7;
        if n > 0 {
            b |= 0x80;
        }
        out.push(b);
    }
    out
}

fn decode_leb128(buf: &[u8]) -> Result<(u32, usize), Error> {
    let mut result: u32 = 0;
    let mut shift: u32 = 0;
    for (i, &b) in buf.iter().enumerate() {
        let chunk = (b & 0x7f) as u32;
        if shift == 28 && (chunk >> 4) != 0 {
            return Err(Error::VaultCrypto(
                "LEB128 value too large for uint32".to_string(),
            ));
        }
        result |= chunk << shift;
        if b & 0x80 == 0 {
            return Ok((result, i + 1));
        }
        shift += 7;
        if shift >= 35 {
            return Err(Error::VaultCrypto(
                "LEB128 value too large for uint32".to_string(),
            ));
        }
    }
    Err(Error::VaultCrypto(
        "unexpected end of LEB128 data".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leb128_round_trip() {
        for n in [0u32, 1, 127, 128, 16383, 16384, 1_234_567] {
            let bytes = encode_leb128(n);
            let (decoded, consumed) = decode_leb128(&bytes).unwrap();
            assert_eq!(decoded, n);
            assert_eq!(consumed, bytes.len());
        }
    }

    fn make_key_material() -> (String, String) {
        let key = [9u8; 32];
        (
            B64_STANDARD.encode(key),
            B64_STANDARD.encode([1u8, 2, 3, 4, 5]),
        )
    }

    #[test]
    fn local_encrypt_decrypt_round_trip() {
        let (data_key, encrypted_keys) = make_key_material();
        let plaintext = "hello world";
        let aad = "ctx:env_1";
        let sealed = local_encrypt(plaintext, &data_key, &encrypted_keys, aad).unwrap();
        let opened = local_decrypt(&sealed, &data_key, aad).unwrap();
        assert_eq!(opened, plaintext);
    }

    #[test]
    fn local_decrypt_rejects_wrong_aad() {
        let (data_key, encrypted_keys) = make_key_material();
        let sealed = local_encrypt("secret", &data_key, &encrypted_keys, "ctx-a").unwrap();
        assert!(local_decrypt(&sealed, &data_key, "ctx-b").is_err());
    }

    #[test]
    fn extract_encrypted_keys_round_trip() {
        let (data_key, encrypted_keys) = make_key_material();
        let sealed = local_encrypt("data", &data_key, &encrypted_keys, "aad").unwrap();
        let extracted = extract_encrypted_keys(&sealed).unwrap();
        assert_eq!(extracted, encrypted_keys);
    }
}
