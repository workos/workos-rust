use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// An access token that may be exchanged for a [`Profile`](crate::sso::Profile).
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AccessToken(String);

impl Display for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AccessToken {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for AccessToken {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}
