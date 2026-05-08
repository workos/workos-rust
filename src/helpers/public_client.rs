// @oagen-ignore-file
//! Public-client factory (H19) — PKCE-only client without an API key.

use crate::client::{Client, DEFAULT_BASE_URL};
use crate::error::Error;
use crate::helpers::authkit::{
    AuthKitAuthorizationUrlParams, AuthKitHelper, AuthKitPkceAuthorizationUrl,
};
use crate::helpers::sso_helpers::{SsoAuthorizationUrlParams, SsoHelper, SsoPkceAuthorizationUrl};

/// PKCE-only / browser-style client. Wraps an internal [`Client`] configured
/// with no API key — only the helper surface suitable for public clients is
/// exposed here.
pub struct PublicClient {
    client: Client,
}

impl PublicClient {
    /// Construct a public client with the given `client_id`. Uses the default
    /// base URL.
    pub fn new(client_id: impl Into<String>) -> Self {
        Self::with_base_url(client_id, DEFAULT_BASE_URL)
    }

    /// Construct a public client with a custom base URL.
    pub fn with_base_url(client_id: impl Into<String>, base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .client_id(client_id)
            .base_url(base_url)
            .build();
        Self { client }
    }

    /// Build an AuthKit PKCE authorization URL. Always generates fresh PKCE
    /// + state.
    pub fn authkit_authorization_url(
        &self,
        params: AuthKitAuthorizationUrlParams,
    ) -> Result<AuthKitPkceAuthorizationUrl, Error> {
        AuthKitHelper::new(&self.client).pkce_authorization_url(params)
    }

    /// Build an SSO PKCE authorization URL. Always generates fresh PKCE
    /// + state.
    pub fn sso_authorization_url(
        &self,
        params: SsoAuthorizationUrlParams,
    ) -> Result<SsoPkceAuthorizationUrl, Error> {
        SsoHelper::new(&self.client).pkce_authorization_url(params)
    }

    /// Underlying client (no API key configured).
    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_authkit_url_with_pkce() {
        let pc = PublicClient::with_base_url("client_123", "https://api.workos.com");
        let result = pc
            .authkit_authorization_url(AuthKitAuthorizationUrlParams {
                redirect_uri: "https://app.example/cb".to_string(),
                ..Default::default()
            })
            .unwrap();
        assert!(result.url.contains("code_challenge_method=S256"));
        assert!(result.url.contains("client_id=client_123"));
    }

    #[test]
    fn builds_sso_url_with_pkce() {
        let pc = PublicClient::with_base_url("client_123", "https://api.workos.com");
        let result = pc
            .sso_authorization_url(SsoAuthorizationUrlParams {
                redirect_uri: "https://app.example/cb".to_string(),
                ..Default::default()
            })
            .unwrap();
        assert!(result.url.contains("code_challenge_method=S256"));
    }
}
