// @oagen-ignore-file
//! Redacted string wrapper for credentials, tokens, and other secret values.
//!
//! Generated request/response structs use [`SecretString`] for fields whose
//! names look like secrets (`password`, `client_secret`, `access_token`,
//! `refresh_token`, `token`, etc.) so the default `Debug` representation does
//! not accidentally leak the underlying value into logs, panics, or error
//! reports. The wire format is a plain string — `SecretString` serializes and
//! deserializes transparently — and the inner value is accessible via
//! [`SecretString::expose`].

use std::fmt;

use serde::{Deserialize, Serialize};

/// A `String` whose [`Debug`] implementation prints `"<redacted>"` instead of
/// the contained value.
///
/// The serialized representation is identical to a `String` — the JSON wire
/// format does not change. Read the underlying value with [`Self::expose`] or
/// [`AsRef::<str>::as_ref`].
#[derive(Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    /// Wrap a `String` so its `Debug` output is redacted.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the wrapped secret. Take care not to log or otherwise expose
    /// the result — the wrapper exists specifically to make accidental
    /// disclosure harder.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper and return the underlying `String`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(\"<redacted>\")")
    }
}

impl fmt::Display for SecretString {
    /// Display also redacts. Construct an explicit log-friendly representation
    /// only via [`Self::expose`] when intentional disclosure is required.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl AsRef<str> for SecretString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<SecretString> for String {
    fn from(value: SecretString) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let s = SecretString::new("hunter2");
        let dbg = format!("{s:?}");
        assert!(!dbg.contains("hunter2"), "debug leaked secret: {dbg}");
        assert_eq!(dbg, "SecretString(\"<redacted>\")");
    }

    #[test]
    fn display_is_redacted() {
        let s = SecretString::new("hunter2");
        assert_eq!(format!("{s}"), "<redacted>");
    }

    #[test]
    fn expose_round_trips() {
        let s = SecretString::new("hunter2");
        assert_eq!(s.expose(), "hunter2");
    }

    #[test]
    fn serde_is_transparent() {
        let s = SecretString::new("hunter2");
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"hunter2\"");
        let parsed: SecretString = serde_json::from_str("\"hunter2\"").unwrap();
        assert_eq!(parsed.expose(), "hunter2");
        // Even when nested inside a struct that derives Debug, the secret
        // should not appear in the printed representation.
        #[derive(Debug)]
        struct Wrap {
            #[allow(dead_code)]
            api_key: SecretString,
        }
        let w = Wrap {
            api_key: SecretString::new("hunter2"),
        };
        let dbg = format!("{w:?}");
        assert!(!dbg.contains("hunter2"), "outer debug leaked secret: {dbg}");
    }
}
