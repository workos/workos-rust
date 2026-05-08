// @oagen-ignore-file
//! Vault — KV operations + local AES-256-GCM crypto (Cat 3, H18).
//!
//! Wire format for `LocalEncrypt` (matches workos-go):
//! `LEB128(len(encryptedKeys)) || encryptedKeys || nonce(12) || ciphertext+tag`
//! the whole thing then base64-encoded.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64_STANDARD;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Error;
use crate::helpers::util::percent_encode;

/// Encryption context for a vault key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyContext {
    #[serde(rename = "type")]
    pub kind: String,
    pub environment_id: String,
}

/// Metadata about a vault object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub context: KeyContext,
    pub environment_id: String,
    pub id: String,
    pub key_id: String,
    pub updated_at: String,
    pub updated_by: String,
    pub version_id: String,
}

/// A vault KV object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultObject {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<ObjectMetadata>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// Summary of a vault object returned by listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultObjectDigest {
    pub id: String,
    pub name: String,
    pub environment_id: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version_id: Option<String>,
}

/// A specific version of a vault object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultObjectVersion {
    pub version_id: String,
    pub updated_at: String,
    pub updated_by: String,
}

/// Plaintext data key + its encrypted counterpart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataKeyPair {
    pub context: KeyContext,
    pub data_key: DataKey,
    pub encrypted_keys: String,
}

/// Plaintext data key (base64-encoded AES key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataKey {
    pub key: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct VaultListObjectsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_values: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaultListObjectsResponse {
    pub data: Vec<VaultObjectDigest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultCreateObjectParams {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_context: Option<KeyContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultUpdateObjectParams {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_context: Option<KeyContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultCreateDataKeyParams {
    pub context: KeyContext,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultDecryptDataKeyParams {
    pub context: KeyContext,
    pub encrypted_keys: String,
}

#[derive(Debug, Clone, Deserialize)]
struct VaultListObjectVersionsResponse {
    data: Vec<VaultObjectVersion>,
}

/// Result of [`VaultApi::encrypt`].
#[derive(Debug, Clone)]
pub struct VaultEncryptResult {
    pub encrypted_data: String,
    pub key_context: KeyContext,
    pub encrypted_keys: String,
}

/// Vault API handle. Obtain via [`crate::Client::vault`].
pub struct VaultApi<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> VaultApi<'a> {
    pub async fn list_objects(
        &self,
        params: VaultListObjectsParams,
    ) -> Result<VaultListObjectsResponse, Error> {
        self.client
            .request_with_query(http::Method::GET, "/vault/v1/kv", &params)
            .await
    }

    pub async fn create_object(
        &self,
        params: VaultCreateObjectParams,
    ) -> Result<ObjectMetadata, Error> {
        self.client
            .request_json(http::Method::POST, "/vault/v1/kv", &params)
            .await
    }

    pub async fn read_object(&self, object_id: &str) -> Result<VaultObject, Error> {
        let path = format!("/vault/v1/kv/{}", percent_encode(object_id));
        self.client
            .request_with_query::<(), _>(http::Method::GET, &path, &())
            .await
    }

    pub async fn read_object_by_name(&self, name: &str) -> Result<VaultObject, Error> {
        let path = format!("/vault/v1/kv/name/{}", percent_encode(name));
        self.client
            .request_with_query::<(), _>(http::Method::GET, &path, &())
            .await
    }

    pub async fn describe_object(&self, object_id: &str) -> Result<VaultObject, Error> {
        let path = format!("/vault/v1/kv/{}/metadata", percent_encode(object_id));
        self.client
            .request_with_query::<(), _>(http::Method::GET, &path, &())
            .await
    }

    pub async fn update_object(
        &self,
        object_id: &str,
        params: VaultUpdateObjectParams,
    ) -> Result<VaultObject, Error> {
        let path = format!("/vault/v1/kv/{}", percent_encode(object_id));
        self.client
            .request_json(http::Method::PUT, &path, &params)
            .await
    }

    pub async fn delete_object(&self, object_id: &str) -> Result<(), Error> {
        let path = format!("/vault/v1/kv/{}", percent_encode(object_id));
        self.client
            .request_empty::<()>(http::Method::DELETE, &path, None)
            .await
    }

    pub async fn list_object_versions(
        &self,
        object_id: &str,
    ) -> Result<Vec<VaultObjectVersion>, Error> {
        let path = format!("/vault/v1/kv/{}/versions", percent_encode(object_id));
        let resp: VaultListObjectVersionsResponse = self
            .client
            .request_with_query::<(), _>(http::Method::GET, &path, &())
            .await?;
        Ok(resp.data)
    }

    pub async fn create_data_key(
        &self,
        params: VaultCreateDataKeyParams,
    ) -> Result<DataKeyPair, Error> {
        self.client
            .request_json(http::Method::POST, "/vault/v1/keys/data-key", &params)
            .await
    }

    pub async fn decrypt_data_key(
        &self,
        params: VaultDecryptDataKeyParams,
    ) -> Result<DataKey, Error> {
        self.client
            .request_json(http::Method::POST, "/vault/v1/keys/decrypt", &params)
            .await
    }

    /// Generates a fresh data key and locally encrypts `data` with AES-256-GCM.
    pub async fn encrypt(
        &self,
        data: &str,
        key_context: KeyContext,
        associated_data: &str,
    ) -> Result<VaultEncryptResult, Error> {
        let pair = self
            .create_data_key(VaultCreateDataKeyParams {
                context: key_context,
            })
            .await?;
        let encrypted = local_encrypt(data, &pair, associated_data)?;
        Ok(VaultEncryptResult {
            encrypted_data: encrypted,
            key_context: pair.context,
            encrypted_keys: pair.encrypted_keys,
        })
    }

    /// Decrypts data previously encrypted with [`encrypt`]. Calls the API to
    /// decrypt the data key, then performs local AES-GCM decryption.
    pub async fn decrypt(
        &self,
        encrypted_data: &str,
        associated_data: &str,
    ) -> Result<String, Error> {
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
        let encrypted_keys_bytes = &raw[bytes_read..total];
        let encrypted_keys_b64 = B64_STANDARD.encode(encrypted_keys_bytes);

        let dk = self
            .decrypt_data_key(VaultDecryptDataKeyParams {
                context: KeyContext {
                    kind: String::new(),
                    environment_id: String::new(),
                },
                encrypted_keys: encrypted_keys_b64,
            })
            .await?;
        local_decrypt(encrypted_data, &dk, associated_data)
    }
}

/// Locally encrypt `data` with AES-256-GCM using the data key in `pair`.
pub fn local_encrypt(
    data: &str,
    pair: &DataKeyPair,
    associated_data: &str,
) -> Result<String, Error> {
    let raw_key = B64_STANDARD
        .decode(&pair.data_key.key)
        .map_err(|e| Error::VaultCrypto(format!("decode data key: {e}")))?;
    if raw_key.len() != 32 {
        return Err(Error::VaultCrypto(format!(
            "data key must be 32 bytes; got {}",
            raw_key.len()
        )));
    }
    let encrypted_keys = B64_STANDARD
        .decode(&pair.encrypted_keys)
        .map_err(|e| Error::VaultCrypto(format!("decode encrypted keys: {e}")))?;

    let cipher = Aes256Gcm::new_from_slice(&raw_key)
        .map_err(|e| Error::VaultCrypto(format!("init AES-GCM: {e}")))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
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

/// Locally decrypt `encrypted_data` with AES-256-GCM using `data_key`.
pub fn local_decrypt(
    encrypted_data: &str,
    data_key: &DataKey,
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
        .decode(&data_key.key)
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
        result |= ((b & 0x7f) as u32) << shift;
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

    fn make_pair() -> DataKeyPair {
        // Use a deterministic 32-byte key for tests.
        let key = [9u8; 32];
        DataKeyPair {
            context: KeyContext {
                kind: "environment".to_string(),
                environment_id: "env_1".to_string(),
            },
            data_key: DataKey {
                key: B64_STANDARD.encode(key),
            },
            encrypted_keys: B64_STANDARD.encode([1u8, 2, 3, 4, 5]),
        }
    }

    #[test]
    fn local_encrypt_decrypt_round_trip() {
        let pair = make_pair();
        let plaintext = "hello world";
        let aad = "ctx:env_1";
        let sealed = local_encrypt(plaintext, &pair, aad).unwrap();
        let opened = local_decrypt(&sealed, &pair.data_key, aad).unwrap();
        assert_eq!(opened, plaintext);
    }

    #[test]
    fn local_decrypt_rejects_wrong_aad() {
        let pair = make_pair();
        let sealed = local_encrypt("secret", &pair, "ctx-a").unwrap();
        assert!(local_decrypt(&sealed, &pair.data_key, "ctx-b").is_err());
    }
}
