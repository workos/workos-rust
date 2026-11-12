// @oagen-ignore-file
//! The jwt-bearer token exchange: one POST to the token endpoint, retried on
//! transport failures, 429 and 5xx with the client's own backoff.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::future::BoxFuture;
use http::{
    HeaderMap, HeaderValue, Method,
    header::{ACCEPT, CONTENT_TYPE, USER_AGENT},
};

use crate::client::{DEFAULT_MAX_RETRIES, DEFAULT_USER_AGENT, backoff_delay, retry_after};
use crate::transport::{HttpRequest, HttpResponse, SharedTransport};

use super::error::{Detail, WorkloadIdentityError};
use super::provider::IdentityTokenProvider;

/// The `grant_type` form field.
pub(crate) const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";

/// Token lifetime used when the response has no usable `expires_in`.
pub(crate) const DEFAULT_LIFETIME: Duration = Duration::from_secs(300);

/// Waits between exchange attempts. Swapped out in tests.
pub(crate) type Sleeper = Arc<dyn Fn(Duration) -> BoxFuture<'static, ()> + Send + Sync>;

/// Everything one exchange needs. Immutable once the credentials are built.
pub(crate) struct ExchangeConfig {
    pub(crate) client_id: String,
    pub(crate) token_url: String,
    pub(crate) resource: String,
    pub(crate) scope: Option<String>,
    pub(crate) provider: Arc<dyn IdentityTokenProvider>,
    /// Never the API client's transport: this one does not follow redirects
    /// and carries no caller-installed hooks.
    pub(crate) transport: SharedTransport,
    pub(crate) sleeper: Sleeper,
}

/// A successful exchange.
pub(crate) struct ExchangedToken {
    /// `Bearer <access token>`, marked sensitive.
    pub(crate) authorization: HeaderValue,
    pub(crate) lifetime: Duration,
}

/// Run the exchange, retrying up to the SDK's default retry count. The
/// provider is called again and the form rebuilt on every attempt.
pub(crate) async fn exchange(
    config: &ExchangeConfig,
) -> Result<ExchangedToken, WorkloadIdentityError> {
    let mut last_failure: Option<WorkloadIdentityError> = None;
    let mut server_delay: Option<Duration> = None;

    for attempt in 0..=DEFAULT_MAX_RETRIES {
        if attempt > 0 {
            // Same rule as API requests: `Retry-After` wins over the backoff.
            let delay = server_delay
                .take()
                .unwrap_or_else(|| backoff_delay(attempt, true));
            if !delay.is_zero() {
                (config.sleeper)(delay).await;
            }
        }

        let assertion = match config.provider.token().await {
            Ok(token) if !token.trim().is_empty() => token,
            Ok(_) => {
                last_failure = Some(WorkloadIdentityError::provider_failed(Detail(
                    "the identity token provider returned an empty token".to_string(),
                )));
                continue;
            }
            Err(e) => {
                last_failure = Some(WorkloadIdentityError::provider_failed(e));
                continue;
            }
        };

        let response = match config.transport.execute(request(config, &assertion)).await {
            Ok(response) => response,
            Err(e) if e.is_retryable() => {
                last_failure = Some(WorkloadIdentityError::exchange_failed().with_source(e));
                continue;
            }
            Err(e) => return Err(WorkloadIdentityError::exchange_failed().with_source(e)),
        };

        // The status is checked before the body: a 429 or 5xx is retried even
        // when its body carries an OAuth error.
        let status = response.status.as_u16();
        let body = serde_json::from_slice::<serde_json::Value>(&response.body).ok();
        let oauth_error = body.as_ref().and_then(oauth_error);

        if status == 429 || (500..=599).contains(&status) {
            server_delay = retry_after(&response.headers);
            last_failure = Some(oauth_error.unwrap_or_else(|| {
                WorkloadIdentityError::exchange_failed().with_source(status_detail(status))
            }));
            continue;
        }
        if let Some(error) = oauth_error {
            return Err(error);
        }
        if (300..=399).contains(&status) {
            return Err(
                WorkloadIdentityError::exchange_failed().with_source(Detail(format!(
                    "the token endpoint answered HTTP {status}; redirects are not followed"
                ))),
            );
        }
        if status != 200 {
            return Err(
                WorkloadIdentityError::invalid_response().with_source(status_detail(status))
            );
        }
        return parse_success(&response, body.as_ref());
    }

    Err(last_failure.unwrap_or_else(WorkloadIdentityError::exchange_failed))
}

/// The exchange POST. No client secret, no `Authorization` header, no API key.
fn request(config: &ExchangeConfig, assertion: &str) -> HttpRequest {
    let mut form = url::form_urlencoded::Serializer::new(String::new());
    form.append_pair("grant_type", GRANT_TYPE)
        .append_pair("assertion", assertion)
        .append_pair("client_id", &config.client_id)
        .append_pair("resource", &config.resource);
    if let Some(scope) = config.scope.as_deref() {
        form.append_pair("scope", scope);
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));

    HttpRequest {
        method: Method::POST,
        url: config.token_url.clone(),
        headers,
        body: Some(Bytes::from(form.finish())),
    }
}

/// An OAuth error body: a JSON object with a non-empty string `error`.
fn oauth_error(body: &serde_json::Value) -> Option<WorkloadIdentityError> {
    let code = body
        .get("error")?
        .as_str()
        .filter(|code| !code.is_empty())?;
    let description = body
        .get("error_description")
        .and_then(|d| d.as_str())
        .unwrap_or_default();
    Some(WorkloadIdentityError::new(code, description))
}

fn parse_success(
    response: &HttpResponse,
    body: Option<&serde_json::Value>,
) -> Result<ExchangedToken, WorkloadIdentityError> {
    let Some(body) = body else {
        return Err(
            WorkloadIdentityError::invalid_response().with_source(Detail(format!(
                "the token endpoint answered HTTP {} with a body that is not JSON",
                response.status.as_u16()
            ))),
        );
    };
    let access_token = body
        .get("access_token")
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            WorkloadIdentityError::invalid_response()
                .with_source(Detail("the token response has no access_token".to_string()))
        })?;
    // The cause is dropped on purpose: it could describe the token's bytes.
    let mut authorization =
        HeaderValue::from_str(&format!("Bearer {access_token}")).map_err(|_| {
            WorkloadIdentityError::invalid_response().with_source(Detail(
                "the access_token cannot be sent as an HTTP header".to_string(),
            ))
        })?;
    authorization.set_sensitive(true);

    // Absent, not a number, not positive, or too small to count: fall back to
    // the default.
    let lifetime = body
        .get("expires_in")
        .and_then(|e| e.as_f64())
        .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
        .filter(|lifetime| !lifetime.is_zero())
        .unwrap_or(DEFAULT_LIFETIME);

    Ok(ExchangedToken {
        authorization,
        lifetime,
    })
}

fn status_detail(status: u16) -> Detail {
    Detail(format!("the token endpoint answered HTTP {status}"))
}
