//! GCP identity token provider for WorkOS workload identity federation.
//!
//! [`gcp_metadata`] returns a provider that asks the GCP metadata server for
//! an ID token for the workload's default service account. It works wherever
//! the metadata server is reachable: Compute Engine, GKE, Cloud Run, Cloud
//! Functions and App Engine. Pass it to `workos::WorkloadIdentityCredentials`:
//!
//! ```no_run
//! use workos::{Client, WorkloadIdentityCredentials};
//! use workos_workload_identity_gcp::gcp_metadata;
//!
//! # fn build() -> Result<(), workos::Error> {
//! let credentials = WorkloadIdentityCredentials::builder()
//!     .client_id("client_123")
//!     .token_url("https://auth.example.com/oauth2/token")
//!     .environment_id("environment_123")
//!     .identity_token_provider(gcp_metadata("https://api.workos.com"))
//!     .build()?;
//!
//! let client = Client::builder().credentials(credentials).try_build()?;
//! # let _ = client;
//! # Ok(()) }
//! ```

use std::error::Error as StdError;
use std::fmt;
use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use workos::workload_identity::{IdentityTokenProvider, ProviderError};

/// The metadata server. Not configurable; tests point the provider elsewhere
/// through a private field.
const METADATA_BASE_URL: &str = "http://metadata.google.internal";
const IDENTITY_PATH: &str = "/computeMetadata/v1/instance/service-accounts/default/identity";
/// How long the one GET may take.
const TIMEOUT: Duration = Duration::from_secs(10);

/// An identity token provider for workloads running on GCP.
///
/// `audience` is the `aud` claim the token is requested with; it must match
/// the audience of the trust rule in WorkOS. The conventional value is
/// `https://api.workos.com`.
///
/// The provider makes one GET to the metadata server with a 10-second
/// timeout. It follows no redirects, keeps no cache and does not retry; the
/// SDK retries a failed call as part of the token exchange.
///
/// The request never goes through a proxy, even when `HTTP_PROXY` is set.
pub fn gcp_metadata(audience: impl Into<String>) -> GcpMetadata {
    GcpMetadata {
        audience: audience.into(),
        base_url: METADATA_BASE_URL.to_string(),
        http: OnceLock::new(),
    }
}

/// The provider [`gcp_metadata`] returns.
pub struct GcpMetadata {
    audience: String,
    base_url: String,
    http: OnceLock<reqwest::Client>,
}

impl GcpMetadata {
    fn http(&self) -> Result<&reqwest::Client, Error> {
        if let Some(client) = self.http.get() {
            return Ok(client);
        }
        // The metadata server is an address inside the workload's own VM or
        // container. A proxy is another machine: it cannot reach that server,
        // and it could answer with its own identity. reqwest takes a proxy
        // from `HTTP_PROXY` unless it is told not to.
        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|e| Error::with_source("could not build the HTTP client", e))?;
        Ok(self.http.get_or_init(|| client))
    }

    async fn fetch(&self) -> Result<String, Error> {
        let url = format!(
            "{}{IDENTITY_PATH}?audience={}&format=full",
            self.base_url,
            percent_encode(&self.audience)
        );
        let response = self
            .http()?
            .get(url)
            .header("Metadata-Flavor", "Google")
            .send()
            .await
            .map_err(|e| {
                Error::with_source(
                    "the request to the GCP metadata server failed",
                    e.without_url(),
                )
            })?;

        let status = response.status();
        if status != reqwest::StatusCode::OK {
            return Err(Error::new(format!(
                "the GCP metadata server answered HTTP {}",
                status.as_u16()
            )));
        }
        // The body is the token, so it never goes into an error.
        let body = response.text().await.map_err(|e| {
            Error::with_source(
                "could not read the GCP metadata server response",
                e.without_url(),
            )
        })?;
        let token = body.trim();
        if token.is_empty() {
            return Err(Error::new(
                "the GCP metadata server returned an empty identity token",
            ));
        }
        Ok(token.to_string())
    }
}

#[async_trait]
impl IdentityTokenProvider for GcpMetadata {
    async fn token(&self) -> Result<String, ProviderError> {
        Ok(self.fetch().await?)
    }
}

impl fmt::Debug for GcpMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GcpMetadata")
            .field("audience", &self.audience)
            .finish_non_exhaustive()
    }
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

/// Why the GCP provider could not return a token. The message never contains
/// a token.
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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const AUDIENCE: &str = "https://api.workos.com";

    /// Tells `child_requests_a_token` where its metadata server is.
    const CHILD_METADATA_URL: &str = "WORKOS_TEST_GCP_METADATA_URL";

    /// A provider pointed at a mock in place of the metadata server.
    fn provider(server: &MockServer) -> GcpMetadata {
        GcpMetadata {
            base_url: server.uri(),
            ..gcp_metadata(AUDIENCE)
        }
    }

    async fn metadata_server(response: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(IDENTITY_PATH))
            .respond_with(response)
            .mount(&server)
            .await;
        server
    }

    #[test]
    fn the_factory_targets_the_real_metadata_server() {
        assert_eq!(
            gcp_metadata(AUDIENCE).base_url,
            "http://metadata.google.internal"
        );
    }

    #[tokio::test]
    async fn wif_gcp_01_requests_identity_with_audience_format_and_flavor() {
        let server = metadata_server(ResponseTemplate::new(200).set_body_string("gcp-jwt\n")).await;

        let token = provider(&server).token().await.unwrap();

        assert_eq!(token, "gcp-jwt");
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "GET");
        assert_eq!(
            requests[0].url.path(),
            "/computeMetadata/v1/instance/service-accounts/default/identity"
        );
        assert_eq!(
            requests[0].url.query(),
            Some("audience=https%3A%2F%2Fapi.workos.com&format=full")
        );
        assert_eq!(
            requests[0].headers.get("metadata-flavor").unwrap(),
            "Google"
        );
    }

    #[tokio::test]
    async fn wif_gcp_02_rejects_non_200_empty_body_and_redirect() {
        let not_found = metadata_server(ResponseTemplate::new(404)).await;
        let error = provider(&not_found).token().await.unwrap_err();
        assert!(
            error.to_string().contains("404"),
            "unexpected message: {error}"
        );

        let empty = metadata_server(ResponseTemplate::new(200).set_body_string(" \n")).await;
        assert!(provider(&empty).token().await.is_err());

        let elsewhere =
            metadata_server(ResponseTemplate::new(200).set_body_string("other-jwt")).await;
        let location = format!("{}{IDENTITY_PATH}", elsewhere.uri());
        let redirect = metadata_server(
            ResponseTemplate::new(302).insert_header("location", location.as_str()),
        )
        .await;
        assert!(provider(&redirect).token().await.is_err());
        assert_eq!(redirect.received_requests().await.unwrap().len(), 1);
        assert!(elsewhere.received_requests().await.unwrap().is_empty());
    }

    /// reqwest reads the proxy variables once per process, and this crate
    /// forbids the `unsafe` that setting them here would take. So the request
    /// is made by a child process that starts with them set: this test binary
    /// again, running `child_requests_a_token` alone.
    #[tokio::test]
    async fn the_request_never_goes_through_a_configured_proxy() {
        let metadata = metadata_server(ResponseTemplate::new(200).set_body_string("gcp-jwt")).await;
        let proxy = MockServer::start().await;

        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args(["--exact", "tests::child_requests_a_token", "--ignored"])
            .env(CHILD_METADATA_URL, metadata.uri())
            .env_remove("NO_PROXY")
            .env_remove("no_proxy");
        for name in ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
            child.env(name, proxy.uri());
        }
        let output = tokio::task::spawn_blocking(move || child.output())
            .await
            .unwrap()
            .unwrap();

        assert!(
            output.status.success(),
            "the child failed: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(metadata.received_requests().await.unwrap().len(), 1);
        assert!(proxy.received_requests().await.unwrap().is_empty());
    }

    /// The child half of `the_request_never_goes_through_a_configured_proxy`.
    /// It does nothing when it is run on its own.
    #[tokio::test]
    #[ignore = "run by the_request_never_goes_through_a_configured_proxy, in a child process"]
    async fn child_requests_a_token() {
        let Ok(base_url) = std::env::var(CHILD_METADATA_URL) else {
            return;
        };
        let provider = GcpMetadata {
            base_url,
            ..gcp_metadata(AUDIENCE)
        };

        assert_eq!(provider.token().await.unwrap(), "gcp-jwt");
    }
}
