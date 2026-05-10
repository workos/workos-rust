// @oagen-ignore-file
//! Shared utilities for the helper layer.

/// Minimal RFC 3986 unreserved-set percent-encode for path segments and query
/// values. Re-exports [`crate::client::path_segment`] under the helpers namespace
/// so hand-written endpoint helpers and generated resource code share a single
/// implementation.
pub(crate) use crate::client::path_segment as percent_encode;

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
}
