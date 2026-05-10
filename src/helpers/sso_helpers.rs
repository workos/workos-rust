// @oagen-ignore-file
//! SSO helpers (H14, H15, H16, H17).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use serde::Serialize;

use crate::client::Client;
use crate::error::Error;
use crate::helpers::pkce::generate_pkce_pair;

/// Parameters for an SSO authorization URL.
#[derive(Debug, Clone, Default)]
pub struct SsoAuthorizationUrlParams {
    pub redirect_uri: String,
    /// If empty, the client's configured `client_id` is used.
    pub client_id: Option<String>,
    pub provider: Option<String>,
    pub connection_id: Option<String>,
    pub organization_id: Option<String>,
    pub domain_hint: Option<String>,
    pub login_hint: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// Result of [`SsoHelper::pkce_authorization_url`].
#[derive(Debug, Clone)]
pub struct SsoPkceAuthorizationUrl {
    pub url: String,
    pub code_verifier: String,
    pub state: String,
}

/// Parameters for [`SsoHelper::pkce_code_exchange`].
#[derive(Debug, Clone)]
pub struct SsoPkceCodeExchangeParams {
    pub code: String,
    pub code_verifier: String,
}

/// Parameters for [`SsoHelper::logout_url`].
#[derive(Debug, Clone)]
pub struct SsoLogoutUrlParams {
    /// Profile / session identifier passed to the logout authorize endpoint.
    pub session_id: String,
    pub return_to: Option<String>,
}

#[derive(Debug, Serialize)]
struct SsoTokenBody<'a> {
    grant_type: &'static str,
    client_id: &'a str,
    client_secret: &'a str,
    code: String,
    code_verifier: String,
}

/// SSO helper bundle. Obtain via [`crate::Client::sso_helpers`].
pub struct SsoHelper<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> SsoHelper<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Builds an SSO authorization URL. No HTTP call is made.
    pub fn authorization_url(&self, params: SsoAuthorizationUrlParams) -> Result<String, Error> {
        let client_id = params
            .client_id
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.client.client_id().to_string());
        if client_id.is_empty() {
            return Err(Error::Builder(
                "client_id is required for SSO authorization URL".to_string(),
            ));
        }
        if params.redirect_uri.is_empty() {
            return Err(Error::Builder(
                "redirect_uri is required for SSO authorization URL".to_string(),
            ));
        }
        let mut url = url::Url::parse(&format!("{}/sso/authorize", self.client.base_url()))
            .map_err(|e| Error::Builder(format!("invalid base URL: {e}")))?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("client_id", &client_id);
            q.append_pair("redirect_uri", &params.redirect_uri);
            q.append_pair("response_type", "code");
            if let Some(v) = &params.provider {
                q.append_pair("provider", v);
            }
            if let Some(v) = &params.connection_id {
                q.append_pair("connection_id", v);
            }
            if let Some(v) = &params.organization_id {
                q.append_pair("organization_id", v);
            }
            if let Some(v) = &params.domain_hint {
                q.append_pair("domain_hint", v);
            }
            if let Some(v) = &params.login_hint {
                q.append_pair("login_hint", v);
            }
            if let Some(v) = &params.state {
                q.append_pair("state", v);
            }
            if let Some(v) = &params.code_challenge {
                q.append_pair("code_challenge", v);
            }
            if let Some(v) = &params.code_challenge_method {
                q.append_pair("code_challenge_method", v);
            }
        }
        Ok(url.into())
    }

    /// Generates PKCE parameters + state and builds an SSO authorization URL.
    pub fn pkce_authorization_url(
        &self,
        mut params: SsoAuthorizationUrlParams,
    ) -> Result<SsoPkceAuthorizationUrl, Error> {
        let pair = generate_pkce_pair()?;
        params.code_challenge = Some(pair.code_challenge.clone());
        params.code_challenge_method = Some(pair.code_challenge_method.to_string());
        let state = match params.state.clone() {
            Some(s) => s,
            None => {
                let mut buf = [0u8; 32];
                rand::rng().fill_bytes(&mut buf);
                URL_SAFE_NO_PAD.encode(buf)
            }
        };
        params.state = Some(state.clone());
        let url = self.authorization_url(params)?;
        Ok(SsoPkceAuthorizationUrl {
            url,
            code_verifier: pair.code_verifier,
            state,
        })
    }

    /// Exchanges an SSO authorization code with PKCE — `POST /sso/token`.
    pub async fn pkce_code_exchange(
        &self,
        params: SsoPkceCodeExchangeParams,
    ) -> Result<crate::models::SSOTokenResponse, Error> {
        let body = SsoTokenBody {
            grant_type: "authorization_code",
            client_id: self.client.client_id(),
            client_secret: self.client.api_key(),
            code: params.code,
            code_verifier: params.code_verifier,
        };
        self.client
            .request_json(http::Method::POST, "/sso/token", &body)
            .await
    }

    /// End-to-end SSO logout: obtains a logout token, then builds a logout
    /// redirect URL.
    pub async fn logout_url(&self, params: SsoLogoutUrlParams) -> Result<String, Error> {
        let body = serde_json::json!({ "profile_id": params.session_id });
        let resp: crate::models::SSOLogoutAuthorizeResponse = self
            .client
            .request_json(http::Method::POST, "/sso/logout/authorize", &body)
            .await?;

        let mut url = url::Url::parse(&format!("{}/sso/logout", self.client.base_url()))
            .map_err(|e| Error::Builder(format!("invalid base URL: {e}")))?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("token", resp.logout_token.expose());
            if let Some(v) = &params.return_to {
                q.append_pair("return_to", v);
            }
        }
        Ok(url.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;

    fn client_with(client_id: &str, base: &str) -> Client {
        Client::builder()
            .client_id(client_id)
            .base_url(base)
            .build()
    }

    #[test]
    fn builds_basic_url() {
        let c = client_with("client_123", "https://api.workos.com");
        let helper = SsoHelper::new(&c);
        let url = helper
            .authorization_url(SsoAuthorizationUrlParams {
                redirect_uri: "https://app.example/cb".to_string(),
                ..Default::default()
            })
            .unwrap();
        assert!(url.starts_with("https://api.workos.com/sso/authorize?"));
        assert!(url.contains("client_id=client_123"));
        assert!(url.contains("response_type=code"));
    }

    #[test]
    fn requires_redirect() {
        let c = client_with("client_123", "https://api.workos.com");
        assert!(
            SsoHelper::new(&c)
                .authorization_url(SsoAuthorizationUrlParams::default())
                .is_err()
        );
    }

    #[test]
    fn pkce_url_has_challenge() {
        let c = client_with("client_123", "https://api.workos.com");
        let result = SsoHelper::new(&c)
            .pkce_authorization_url(SsoAuthorizationUrlParams {
                redirect_uri: "https://app.example/cb".to_string(),
                ..Default::default()
            })
            .unwrap();
        assert!(result.url.contains("code_challenge_method=S256"));
        assert!(!result.code_verifier.is_empty());
    }
}
