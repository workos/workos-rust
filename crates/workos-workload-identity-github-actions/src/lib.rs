//! GitHub Actions identity token provider for WorkOS workload identity
//! federation.
//!
//! [`github_actions`] returns a provider that asks the GitHub Actions runner
//! for an OIDC token on behalf of the running job. Pass it to
//! `workos::WorkloadIdentityCredentials`:
//!
//! ```no_run
//! use workos::{Client, WorkloadIdentityCredentials};
//! use workos_workload_identity_github_actions::github_actions;
//!
//! # fn build() -> Result<(), workos::Error> {
//! let credentials = WorkloadIdentityCredentials::builder()
//!     .client_id("client_123")
//!     .token_url("https://auth.example.com/oauth2/token")
//!     .environment_id("environment_123")
//!     .identity_token_provider(github_actions("https://api.workos.com"))
//!     .build()?;
//!
//! let client = Client::builder().credentials(credentials).try_build()?;
//! # let _ = client;
//! # Ok(()) }
//! ```
//!
//! The workflow or job needs `permissions: id-token: write`.

use std::error::Error as StdError;
use std::fmt;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use workos::workload_identity::{IdentityTokenProvider, ProviderError};

/// URL of the runner's OIDC token endpoint. Set by GitHub Actions.
const REQUEST_URL_ENV: &str = "ACTIONS_ID_TOKEN_REQUEST_URL";
/// Bearer token for that endpoint. Set by GitHub Actions.
const REQUEST_TOKEN_ENV: &str = "ACTIONS_ID_TOKEN_REQUEST_TOKEN";
/// How long the one GET may take.
const TIMEOUT: Duration = Duration::from_secs(10);

const MISSING_ENVIRONMENT: &str = "GitHub Actions did not provide an OIDC token endpoint. Add `permissions: id-token: write` to the workflow or job.";

type EnvLookup = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// An identity token provider for jobs running in GitHub Actions.
///
/// `audience` is the `aud` claim the token is requested with; it must match
/// the audience of the trust rule in WorkOS. The conventional value is
/// `https://api.workos.com`.
///
/// The provider reads `ACTIONS_ID_TOKEN_REQUEST_URL` and
/// `ACTIONS_ID_TOKEN_REQUEST_TOKEN` on every call and makes one GET with a
/// 10-second timeout. It follows no redirects, keeps no cache and does not
/// retry; the SDK retries a failed call as part of the token exchange.
pub fn github_actions(audience: impl Into<String>) -> GithubActions {
    GithubActions {
        audience: audience.into(),
        env: Arc::new(|name| std::env::var(name).ok()),
        http: OnceLock::new(),
    }
}

/// The provider [`github_actions`] returns.
pub struct GithubActions {
    audience: String,
    env: EnvLookup,
    http: OnceLock<reqwest::Client>,
}

impl GithubActions {
    fn http(&self) -> Result<&reqwest::Client, Error> {
        if let Some(client) = self.http.get() {
            return Ok(client);
        }
        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| Error::with_source("could not build the HTTP client", e))?;
        Ok(self.http.get_or_init(|| client))
    }

    async fn fetch(&self) -> Result<String, Error> {
        let variable = |name: &str| (self.env)(name).filter(|value| !value.is_empty());
        let (Some(url), Some(request_token)) =
            (variable(REQUEST_URL_ENV), variable(REQUEST_TOKEN_ENV))
        else {
            return Err(Error::new(MISSING_ENVIRONMENT));
        };

        let response = self
            .http()?
            .get(with_audience(&url, &self.audience))
            .bearer_auth(request_token)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| {
                Error::with_source(
                    "the request to the GitHub Actions OIDC token endpoint failed",
                    e.without_url(),
                )
            })?;

        let status = response.status();
        if status != reqwest::StatusCode::OK {
            return Err(Error::new(format!(
                "the GitHub Actions OIDC token endpoint answered HTTP {}",
                status.as_u16()
            )));
        }
        let body = response.bytes().await.map_err(|e| {
            Error::with_source(
                "could not read the GitHub Actions OIDC token response",
                e.without_url(),
            )
        })?;

        // The body holds the token, so it never goes into an error.
        serde_json::from_slice::<serde_json::Value>(&body)
            .ok()
            .and_then(|json| json.get("value")?.as_str().map(str::to_owned))
            .filter(|token| !token.is_empty())
            .ok_or_else(|| Error::new("the GitHub Actions OIDC token response has no `value`"))
    }
}

#[async_trait]
impl IdentityTokenProvider for GithubActions {
    async fn token(&self) -> Result<String, ProviderError> {
        Ok(self.fetch().await?)
    }
}

impl fmt::Debug for GithubActions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GithubActions")
            .field("audience", &self.audience)
            .finish_non_exhaustive()
    }
}

/// Add `audience=<percent-encoded audience>` to a URL that may already have a
/// query string.
fn with_audience(url: &str, audience: &str) -> String {
    let separator = if url.ends_with(['?', '&']) {
        ""
    } else if url.contains('?') {
        "&"
    } else {
        "?"
    };
    format!("{url}{separator}audience={}", percent_encode(audience))
}

/// Percent-encode everything outside the RFC 3986 unreserved set.
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for &byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Why the GitHub Actions provider could not return a token. The message
/// never contains a token.
#[derive(Debug)]
pub struct Error {
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    fn with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn StdError + Send + Sync>>,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(source.into()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const AUDIENCE: &str = "https://api.workos.com";

    /// A provider that reads a fixed environment instead of the process's.
    fn provider(env: &[(&str, &str)]) -> GithubActions {
        let env: HashMap<String, String> = env
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        GithubActions {
            env: Arc::new(move |name| env.get(name).cloned()),
            ..github_actions(AUDIENCE)
        }
    }

    async fn token_endpoint(response: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/token"))
            .respond_with(response)
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn wif_gha_01_requests_token_with_audience_and_bearer() {
        let server = token_endpoint(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "value": "gha-jwt" })),
        )
        .await;
        let url = format!("{}/token?api-version=2.0", server.uri());
        let provider = provider(&[
            (REQUEST_URL_ENV, &url),
            (REQUEST_TOKEN_ENV, "gha-request-token"),
        ]);

        let token = provider.token().await.unwrap();

        assert_eq!(token, "gha-jwt");
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "GET");
        assert_eq!(
            requests[0].url.query(),
            Some("api-version=2.0&audience=https%3A%2F%2Fapi.workos.com")
        );
        assert_eq!(
            requests[0].headers.get("authorization").unwrap(),
            "Bearer gha-request-token"
        );
    }

    #[tokio::test]
    async fn wif_gha_02_missing_environment_names_the_permission() {
        let server = token_endpoint(ResponseTemplate::new(200)).await;
        let url = format!("{}/token?api-version=2.0", server.uri());

        let without_url = provider(&[(REQUEST_TOKEN_ENV, "gha-request-token")]);
        let without_token = provider(&[(REQUEST_URL_ENV, &url)]);
        let empty_token = provider(&[(REQUEST_URL_ENV, &url), (REQUEST_TOKEN_ENV, "")]);

        for provider in [without_url, without_token, empty_token] {
            let error = provider.token().await.unwrap_err();
            assert!(
                error.to_string().contains("permissions: id-token: write"),
                "unexpected message: {error}"
            );
        }
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn fails_on_a_status_other_than_200_and_names_it() {
        let server = token_endpoint(ResponseTemplate::new(404)).await;
        let provider = provider(&[
            (REQUEST_URL_ENV, &format!("{}/token", server.uri())),
            (REQUEST_TOKEN_ENV, "gha-request-token"),
        ]);

        let error = provider.token().await.unwrap_err();

        assert!(
            error.to_string().contains("404"),
            "unexpected message: {error}"
        );
    }

    #[tokio::test]
    async fn fails_on_a_missing_value_without_echoing_the_body() {
        let server = token_endpoint(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "other": "SECRET-BODY" })),
        )
        .await;
        let provider = provider(&[
            (REQUEST_URL_ENV, &format!("{}/token", server.uri())),
            (REQUEST_TOKEN_ENV, "gha-request-token"),
        ]);

        let error = provider.token().await.unwrap_err();

        assert!(!format!("{error} {error:?}").contains("SECRET-BODY"));
    }

    #[tokio::test]
    async fn does_not_follow_a_redirect() {
        let elsewhere = token_endpoint(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "value": "x" })),
        )
        .await;
        let server = token_endpoint(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/token", elsewhere.uri()).as_str()),
        )
        .await;
        let provider = provider(&[
            (REQUEST_URL_ENV, &format!("{}/token", server.uri())),
            (REQUEST_TOKEN_ENV, "gha-request-token"),
        ]);

        assert!(provider.token().await.is_err());
        assert!(elsewhere.received_requests().await.unwrap().is_empty());
    }

    #[test]
    fn appends_the_audience_to_any_url_shape() {
        assert_eq!(
            with_audience("https://h/t", "a:b/c"),
            "https://h/t?audience=a%3Ab%2Fc"
        );
        assert_eq!(
            with_audience("https://h/t?x=1", "a b"),
            "https://h/t?x=1&audience=a%20b"
        );
        assert_eq!(
            with_audience("https://h/t?x=1&", "a"),
            "https://h/t?x=1&audience=a"
        );
    }
}
