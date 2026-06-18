// @oagen-ignore-file
//! AuthKit helpers (H09, H10, H11, H12).

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use serde::Serialize;

use crate::client::Client;
use crate::error::Error;
use crate::helpers::pkce::generate_pkce_pair;

/// Parameters for an AuthKit authorization URL.
#[derive(Debug, Clone, Default)]
pub struct AuthKitAuthorizationUrlParams {
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
    pub screen_hint: Option<String>,
}

/// AuthKit URL helper bundle. Obtain via [`crate::Client::authkit`].
pub struct AuthKitHelper<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> AuthKitHelper<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Builds an AuthKit authorization URL. No HTTP call is made.
    pub fn authorization_url(
        &self,
        params: AuthKitAuthorizationUrlParams,
    ) -> Result<String, Error> {
        let client_id = params
            .client_id
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.client.client_id().to_string());
        if client_id.is_empty() {
            return Err(Error::Builder(
                "client_id is required for AuthKit authorization URL".to_string(),
            ));
        }
        if params.redirect_uri.is_empty() {
            return Err(Error::Builder(
                "redirect_uri is required for AuthKit authorization URL".to_string(),
            ));
        }

        let mut url = url::Url::parse(&format!(
            "{}/user_management/authorize",
            self.client.base_url()
        ))
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
            if let Some(v) = &params.screen_hint {
                q.append_pair("screen_hint", v);
            }
        }
        Ok(url.into())
    }

    /// Generates PKCE parameters + state and builds an authorization URL.
    pub fn pkce_authorization_url(
        &self,
        mut params: AuthKitAuthorizationUrlParams,
    ) -> Result<AuthKitPkceAuthorizationUrl, Error> {
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
        Ok(AuthKitPkceAuthorizationUrl {
            url,
            code_verifier: pair.code_verifier,
            state,
        })
    }

    /// Exchanges an authorization code with PKCE.
    pub async fn pkce_code_exchange(
        &self,
        params: AuthKitPkceCodeExchangeParams,
    ) -> Result<crate::models::AuthenticateResponse, Error> {
        let body = AuthKitCodeExchangeBody {
            grant_type: "authorization_code",
            client_id: self.client.client_id(),
            client_secret: self.client.api_key(),
            code: params.code,
            code_verifier: params.code_verifier,
        };
        self.client
            .request_json(http::Method::POST, "/user_management/authenticate", &body)
            .await
    }

    /// Initiates a device authorization flow. Returns the device code response.
    pub async fn start_device_authorization(
        &self,
    ) -> Result<crate::models::DeviceAuthorizationResponse, Error> {
        let body = serde_json::json!({ "client_id": self.client.client_id() });
        self.client
            .request_json(
                http::Method::POST,
                "/user_management/authorize/device",
                &body,
            )
            .await
    }

    /// Polls the device-code endpoint until the user authorizes or an error occurs.
    pub async fn poll_device_code(
        &self,
        device_code: &str,
        interval: Duration,
    ) -> Result<crate::models::AuthenticateResponse, Error> {
        let mut interval = if interval.is_zero() {
            Duration::from_secs(5)
        } else {
            interval
        };

        loop {
            tokio::time::sleep(interval).await;
            let body = AuthKitDeviceCodeBody {
                grant_type: "urn:ietf:params:oauth:grant-type:device_code",
                client_id: self.client.client_id(),
                device_code,
            };
            let result: Result<crate::models::AuthenticateResponse, Error> = self
                .client
                .request_json(http::Method::POST, "/user_management/authenticate", &body)
                .await;
            match result {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    // The device-code grant signals its state through the OAuth
                    // `error` code (RFC 8628), which the SDK surfaces as
                    // `ApiError::code` — not the human-readable `message`.
                    match e.api().and_then(|a| a.code.as_deref()) {
                        // Authorization is still pending: keep polling.
                        Some("authorization_pending") => continue,
                        // The server asked us to back off: widen the interval
                        // by 5s (per RFC 8628), then keep polling.
                        Some("slow_down") => {
                            interval += Duration::from_secs(5);
                            continue;
                        }
                        _ => return Err(e),
                    }
                }
            }
        }
    }
}

/// Result of [`AuthKitHelper::pkce_authorization_url`].
#[derive(Debug, Clone)]
pub struct AuthKitPkceAuthorizationUrl {
    pub url: String,
    pub code_verifier: String,
    pub state: String,
}

/// Parameters for [`AuthKitHelper::pkce_code_exchange`].
#[derive(Debug, Clone)]
pub struct AuthKitPkceCodeExchangeParams {
    pub code: String,
    pub code_verifier: String,
}

#[derive(Debug, Serialize)]
struct AuthKitCodeExchangeBody<'a> {
    grant_type: &'static str,
    client_id: &'a str,
    client_secret: &'a str,
    code: String,
    code_verifier: String,
}

#[derive(Debug, Serialize)]
struct AuthKitDeviceCodeBody<'a> {
    grant_type: &'static str,
    client_id: &'a str,
    device_code: &'a str,
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
    fn requires_client_id() {
        let c = client_with("", "https://api.workos.com");
        let helper = AuthKitHelper::new(&c);
        let err = helper
            .authorization_url(AuthKitAuthorizationUrlParams {
                redirect_uri: "https://app.example/cb".to_string(),
                ..Default::default()
            })
            .unwrap_err();
        assert!(matches!(err, Error::Builder(_)));
    }

    #[test]
    fn requires_redirect_uri() {
        let c = client_with("client_123", "https://api.workos.com");
        let helper = AuthKitHelper::new(&c);
        assert!(
            helper
                .authorization_url(AuthKitAuthorizationUrlParams::default())
                .is_err()
        );
    }

    #[test]
    fn builds_basic_url() {
        let c = client_with("client_123", "https://api.workos.com");
        let helper = AuthKitHelper::new(&c);
        let url = helper
            .authorization_url(AuthKitAuthorizationUrlParams {
                redirect_uri: "https://app.example/cb".to_string(),
                provider: Some("authkit".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert!(url.starts_with("https://api.workos.com/user_management/authorize?"));
        assert!(url.contains("client_id=client_123"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("provider=authkit"));
    }

    #[test]
    fn pkce_url_includes_challenge_and_state() {
        let c = client_with("client_123", "https://api.workos.com");
        let helper = AuthKitHelper::new(&c);
        let result = helper
            .pkce_authorization_url(AuthKitAuthorizationUrlParams {
                redirect_uri: "https://app.example/cb".to_string(),
                ..Default::default()
            })
            .unwrap();
        assert!(result.url.contains("code_challenge="));
        assert!(result.url.contains("code_challenge_method=S256"));
        assert!(!result.code_verifier.is_empty());
        assert!(!result.state.is_empty());
    }
}
