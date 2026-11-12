// @oagen-ignore-file
//! The credentials contract: how a [`crate::Client`] authenticates each HTTP
//! attempt.
//!
//! Every client authenticates through [`Credentials`]. A static API key is the
//! trivial implementation; [`crate::WorkloadIdentityCredentials`] exchanges a
//! workload's identity token for a short-lived access token.

use std::fmt;

use async_trait::async_trait;
use http::{HeaderMap, HeaderValue, header::AUTHORIZATION};

use crate::error::Error;

/// The authorization headers for one HTTP attempt, and the credential they
/// came from.
///
/// The client resolves a snapshot before every attempt and keeps it for that
/// attempt, so a `401` invalidates the credential that was actually sent and
/// never a newer one.
#[derive(Clone)]
pub struct CredentialSnapshot {
    /// Headers to set on the attempt. They overwrite headers of the same name
    /// already on the request.
    pub headers: HeaderMap,
    /// Identifies the credential these headers came from. Passed back to
    /// [`Credentials::invalidate`] when the API answers `401`.
    pub generation: u64,
    /// `true` when a `401` on this snapshot is worth one re-resolve and one
    /// resend. Always `false` for a static API key.
    pub recoverable: bool,
}

impl fmt::Debug for CredentialSnapshot {
    /// Prints the header names only, so a token never reaches a log line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.headers.keys().map(|name| name.as_str()).collect();
        f.debug_struct("CredentialSnapshot")
            .field("headers", &names)
            .field("generation", &self.generation)
            .field("recoverable", &self.recoverable)
            .finish()
    }
}

/// Supplies the authorization headers for API requests.
///
/// Pass an implementation to [`crate::ClientBuilder::credentials`]. Most
/// callers use [`crate::WorkloadIdentityCredentials`] and never implement
/// this trait.
#[async_trait]
pub trait Credentials: Send + Sync {
    /// Return the headers for the next HTTP attempt.
    ///
    /// Called before every attempt, including transport retries. It may wait
    /// on the network. Dropping the returned future is how a caller gives up;
    /// an implementation must tolerate that.
    async fn resolve(&self) -> Result<CredentialSnapshot, Error>;

    /// Mark the credential with this generation as rejected. A no-op when a
    /// newer credential is already held. Never waits on the network.
    fn invalidate(&self, generation: u64);
}

/// A static API key: the same header on every attempt, and no recovery.
pub(crate) struct StaticKeyCredentials {
    headers: HeaderMap,
}

impl StaticKeyCredentials {
    /// `authorization` is `None` for a client built without an API key
    /// (public PKCE flows), which sends no `Authorization` header.
    pub(crate) fn new(authorization: Option<HeaderValue>) -> Self {
        let mut headers = HeaderMap::new();
        if let Some(value) = authorization {
            headers.insert(AUTHORIZATION, value);
        }
        Self { headers }
    }
}

#[async_trait]
impl Credentials for StaticKeyCredentials {
    async fn resolve(&self) -> Result<CredentialSnapshot, Error> {
        Ok(CredentialSnapshot {
            headers: self.headers.clone(),
            generation: 0,
            recoverable: false,
        })
    }

    fn invalidate(&self, _generation: u64) {}
}
