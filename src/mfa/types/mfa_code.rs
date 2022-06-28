use std::fmt::Display;

use serde::Serialize;

/// The 6 digit code to be verified in Verify Factor.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct MfaCode(String);

impl Display for MfaCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MfaCode {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for MfaCode {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}
