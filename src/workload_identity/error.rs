// @oagen-ignore-file
//! [`WorkloadIdentityError`] and the hint table.

use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

/// SDK code: the token endpoint could not be reached or kept failing.
pub(crate) const CODE_EXCHANGE_FAILED: &str = "exchange_failed";
/// SDK code: the token endpoint answered with something the SDK cannot read.
pub(crate) const CODE_INVALID_RESPONSE: &str = "invalid_response";
/// SDK code: the identity token provider failed.
pub(crate) const CODE_PROVIDER_FAILED: &str = "provider_failed";
/// SDK code: the credentials are missing a required setting.
pub(crate) const CODE_INVALID_CONFIGURATION: &str = "invalid_configuration";

/// Longest `description` kept from a server response, in characters.
const DESCRIPTION_LIMIT: usize = 500;

/// The one place hint wording lives. The wording is pending the server team's
/// final error codes; change it here and nowhere else.
const HINTS: &[(&str, &str)] = &[
    (
        "invalid_grant",
        "The identity token did not match a trust rule for this application, or the token has expired. Check the issuer, audience and subject claims against the trust rule.",
    ),
    (
        "invalid_target",
        "The resource or audience is wrong. The trust rule's audience must match the audience the identity token was requested with, and the resource must be registered for the environment.",
    ),
    (
        "invalid_client",
        "Check that the client ID is the M2M application that has the trust rule.",
    ),
    (
        "unauthorized_client",
        "This application is not allowed to use workload identity federation. Check that the feature is enabled for the environment and that the application has a trust rule.",
    ),
    (
        "invalid_scope",
        "The requested scope is not allowed for this application. Fix the scope or leave it unset.",
    ),
    (
        "invalid_request",
        "The token request was malformed. Check that the token URL points at the `/oauth2/token` endpoint.",
    ),
    (
        CODE_EXCHANGE_FAILED,
        "The token endpoint could not be reached or kept failing. Check the token URL and try again.",
    ),
    (
        CODE_INVALID_RESPONSE,
        "The token endpoint returned a response the SDK could not read. Check that the token URL points at the `/oauth2/token` endpoint.",
    ),
    (
        CODE_PROVIDER_FAILED,
        "The identity token provider failed, so there was no token to exchange.",
    ),
    (
        CODE_INVALID_CONFIGURATION,
        "Workload identity credentials are missing a required setting.",
    ),
];

/// Hint for an OAuth code the table does not list.
pub(crate) const DEFAULT_HINT: &str = "The token endpoint rejected the exchange.";

fn hint_for(code: &str) -> &'static str {
    HINTS
        .iter()
        .find(|(known, _)| *known == code)
        .map(|(_, hint)| *hint)
        .unwrap_or(DEFAULT_HINT)
}

/// A workload identity failure: bad configuration, a failing identity token
/// provider, or a token exchange the endpoint rejected.
///
/// Surfaces from API calls as [`crate::Error::WorkloadIdentity`]. The message
/// is `<code>: <hint>`, followed by the description when there is one. It
/// never contains the identity token or the access token.
#[derive(Clone)]
pub struct WorkloadIdentityError {
    /// The OAuth error code from the token endpoint (`invalid_grant`,
    /// `invalid_client`, ...), or one of the SDK's own codes:
    /// `exchange_failed`, `invalid_response`, `provider_failed`,
    /// `invalid_configuration`.
    pub code: String,
    /// The server's `error_description`, cut to 500 characters. For
    /// `invalid_configuration` it names the missing setting. Empty when there
    /// is none.
    pub description: String,
    /// One sentence on what to check. Never empty.
    pub hint: String,
    source: Option<Arc<dyn StdError + Send + Sync>>,
}

impl WorkloadIdentityError {
    pub(crate) fn new(code: &str, description: &str) -> Self {
        Self {
            code: code.to_string(),
            description: description.chars().take(DESCRIPTION_LIMIT).collect(),
            hint: hint_for(code).to_string(),
            source: None,
        }
    }

    pub(crate) fn with_source(
        mut self,
        source: impl Into<Box<dyn StdError + Send + Sync>>,
    ) -> Self {
        self.source = Some(Arc::from(source.into()));
        self
    }

    pub(crate) fn invalid_configuration(description: &str) -> Self {
        Self::new(CODE_INVALID_CONFIGURATION, description)
    }

    pub(crate) fn exchange_failed() -> Self {
        Self::new(CODE_EXCHANGE_FAILED, "")
    }

    pub(crate) fn invalid_response() -> Self {
        Self::new(CODE_INVALID_RESPONSE, "")
    }

    pub(crate) fn provider_failed(source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::new(CODE_PROVIDER_FAILED, "").with_source(source)
    }
}

impl fmt::Display for WorkloadIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.hint)?;
        if !self.description.is_empty() {
            write!(f, " {}", self.description)?;
        }
        Ok(())
    }
}

impl fmt::Debug for WorkloadIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkloadIdentityError")
            .field("code", &self.code)
            .field("description", &self.description)
            .field("hint", &self.hint)
            .field("source", &self.source)
            .finish()
    }
}

impl StdError for WorkloadIdentityError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}

/// A plain-text cause attached to SDK-originated errors (for example "the
/// token endpoint answered HTTP 503"). Never built from a request or response
/// body.
#[derive(Debug)]
pub(crate) struct Detail(pub(crate) String);

impl fmt::Display for Detail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StdError for Detail {}
