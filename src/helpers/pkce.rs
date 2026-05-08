// @oagen-ignore-file
//! PKCE utilities (H08).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::Error;

/// PKCE code verifier + challenge pair.
#[derive(Debug, Clone)]
pub struct PkcePair {
    pub code_verifier: String,
    pub code_challenge: String,
    /// Always `"S256"`.
    pub code_challenge_method: &'static str,
}

/// Generates a cryptographically random PKCE code verifier.
///
/// `length` must be 43..=128. Defaults to 43 when `None`.
pub fn generate_code_verifier(length: Option<usize>) -> Result<String, Error> {
    let n = length.unwrap_or(43);
    if !(43..=128).contains(&n) {
        return Err(Error::Crypto(format!(
            "PKCE code verifier length must be between 43 and 128, got {n}"
        )));
    }

    let byte_len = (3 * n).div_ceil(4);
    let mut buf = vec![0u8; byte_len];
    rand::rng().fill_bytes(&mut buf);
    let encoded = URL_SAFE_NO_PAD.encode(&buf);
    Ok(encoded[..n].to_string())
}

/// Computes the S256 code challenge for a verifier.
pub fn generate_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// Generates a complete PKCE pair (default verifier length 43).
pub fn generate_pkce_pair() -> Result<PkcePair, Error> {
    let verifier = generate_code_verifier(None)?;
    let challenge = generate_code_challenge(&verifier);
    Ok(PkcePair {
        code_verifier: verifier,
        code_challenge: challenge,
        code_challenge_method: "S256",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_default_length() {
        let v = generate_code_verifier(None).unwrap();
        assert_eq!(v.len(), 43);
    }

    #[test]
    fn verifier_custom_length() {
        let v = generate_code_verifier(Some(64)).unwrap();
        assert_eq!(v.len(), 64);
    }

    #[test]
    fn verifier_rejects_too_short() {
        assert!(generate_code_verifier(Some(10)).is_err());
    }

    #[test]
    fn verifier_rejects_too_long() {
        assert!(generate_code_verifier(Some(200)).is_err());
    }

    #[test]
    fn challenge_is_deterministic_for_known_verifier() {
        // RFC 7636 example.
        let v = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(generate_code_challenge(v), expected);
    }

    #[test]
    fn pair_round_trip() {
        let pair = generate_pkce_pair().unwrap();
        assert_eq!(pair.code_challenge_method, "S256");
        assert_eq!(pair.code_verifier.len(), 43);
        // challenge must equal challenge(verifier).
        assert_eq!(
            pair.code_challenge,
            generate_code_challenge(&pair.code_verifier)
        );
    }
}
