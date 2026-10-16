// @oagen-ignore-file
//! Shared utilities for the helper layer.

/// Minimal RFC 3986 unreserved-set percent-encode for path segments and query
/// values. Re-exports [`crate::client::path_segment`] under the helpers namespace
/// so hand-written endpoint helpers and generated resource code share a single
/// implementation.
pub(crate) use crate::client::path_segment as percent_encode;

use crate::error::Error;

/// Fills `buf` with bytes from the operating-system CSPRNG (via aws-lc-rs).
///
/// Used for AEAD nonces, PKCE verifiers, OAuth `state` values, and retry
/// jitter. Fails only if the OS random source is unavailable.
pub(crate) fn random_bytes(buf: &mut [u8]) -> Result<(), Error> {
    aws_lc_rs::rand::fill(buf)
        .map_err(|_| Error::Crypto("operating-system CSPRNG unavailable".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encode_basic() {
        assert_eq!(percent_encode("hello"), "hello");
        assert_eq!(percent_encode("a b/c"), "a%20b%2Fc");
        assert_eq!(percent_encode("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn percent_encode_unicode_and_query_chars() {
        assert_eq!(percent_encode("café"), "caf%C3%A9");
        assert_eq!(percent_encode("a?b#c&d=e"), "a%3Fb%23c%26d%3De");
        assert_eq!(percent_encode("already%20encoded"), "already%2520encoded");
    }

    #[test]
    fn random_bytes_fills_and_varies() {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        random_bytes(&mut a).unwrap();
        random_bytes(&mut b).unwrap();
        assert!(a.iter().any(|&x| x != 0));
        assert_ne!(a, b);
    }
}
