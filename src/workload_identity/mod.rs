// @oagen-ignore-file
//! Workload identity federation: authenticate with the identity a platform
//! already gives the workload, instead of a stored API key.
//!
//! A workload running in GitHub Actions, GCP, AWS or anywhere else that issues
//! it a JWT hands that token to an [`IdentityTokenProvider`].
//! [`WorkloadIdentityCredentials`] exchanges it at the token endpoint for a
//! short-lived WorkOS access token, caches the access token, and refreshes it
//! shortly before it expires.
//!
//! ```no_run
//! use workos::{Client, WorkloadIdentityCredentials, token_file};
//!
//! # fn build() -> Result<(), workos::Error> {
//! let credentials = WorkloadIdentityCredentials::builder()
//!     .client_id("client_123")
//!     .token_url("https://auth.example.com/oauth2/token")
//!     .environment_id("environment_123")
//!     .identity_token_provider(token_file("/var/run/secrets/workos/token"))
//!     .build()?;
//!
//! let client = Client::builder().credentials(credentials).try_build()?;
//! # let _ = client;
//! # Ok(()) }
//! ```
//!
//! The platform providers are separate crates, one per platform:
//! `workos-workload-identity-github-actions`, `workos-workload-identity-gcp`
//! and `workos-workload-identity-aws`. Each depends on this crate for the
//! [`IdentityTokenProvider`] trait, which is why they are not Cargo features
//! of `workos`.
//!
//! The exchange runs on a spawned task, so the credentials must be used from
//! inside a Tokio runtime. Outside one, the first request fails with code
//! `exchange_failed`. The task belongs to the runtime of the caller that
//! started it, so use one instance from one long-lived runtime.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::FutureExt;
use futures_util::future::{BoxFuture, Shared};
use http::{HeaderMap, HeaderName, HeaderValue, header::AUTHORIZATION};

use crate::credentials::{CredentialSnapshot, Credentials};
use crate::error::Error;
use crate::transport::SharedTransport;

mod error;
mod exchange;
mod provider;
#[cfg(test)]
mod tests;

pub use error::WorkloadIdentityError;
pub use provider::{
    IdentityTokenProvider, ProviderError, TokenEnv, TokenFile, token_env, token_file,
};

use error::Detail;
use exchange::{DEFAULT_LIFETIME, ExchangeConfig, Sleeper};

/// Header that tells the API which environment a workload identity token is
/// for. Sent on every request a [`WorkloadIdentityCredentials`] client makes.
pub const ENVIRONMENT_ID_HEADER: &str = "x-url-environment-id";

/// Default for the `resource` form field of the exchange.
pub const DEFAULT_RESOURCE: &str = "https://api.workos.com";

/// Environment variable that supplies `client_id` when it is not set.
pub const CLIENT_ID_ENV: &str = "WORKOS_CLIENT_ID";
/// Environment variable that supplies `token_url` when it is not set.
pub const TOKEN_URL_ENV: &str = "WORKOS_WIF_TOKEN_URL";
/// Environment variable that supplies `scope` when it is not set.
pub const SCOPE_ENV: &str = "WORKOS_WIF_SCOPE";
/// Environment variable that supplies `environment_id` when it is not set.
pub const ENVIRONMENT_ID_ENV: &str = "WORKOS_ENVIRONMENT_ID";

/// A token is refreshed once less than this much of its life remains...
const REFRESH_THRESHOLD: Duration = Duration::from_secs(60);
/// ...or less than this fraction of its lifetime, whichever is shorter.
const REFRESH_FRACTION: f64 = 0.2;
/// Longest one shared exchange may run, retries and backoff included.
const EXCHANGE_DEADLINE: Duration = Duration::from_secs(30);

type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;
type EnvLookup = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// What every caller waiting on one exchange receives: the `Authorization`
/// value and the generation it was stored under.
type ExchangeOutcome = Result<(HeaderValue, u64), WorkloadIdentityError>;
type SharedExchange = Shared<BoxFuture<'static, ExchangeOutcome>>;

/// Credentials that exchange a workload's identity token for a short-lived
/// WorkOS access token.
///
/// Build one with [`WorkloadIdentityCredentials::builder`] and pass it to
/// [`crate::ClientBuilder::credentials`]. Configuration is immutable; build a
/// new instance to change it.
///
/// The access token and the exchange in progress belong to the instance.
/// Cloning hands out another handle to the same instance, so clones share
/// one token; two instances built separately never do, even with identical
/// configuration.
#[derive(Clone)]
pub struct WorkloadIdentityCredentials {
    inner: Arc<Inner>,
}

struct Inner {
    environment_id: HeaderValue,
    exchange: ExchangeConfig,
    clock: Clock,
    state: Mutex<State>,
}

struct State {
    /// `Bearer <access token>`. `None` before the first exchange and after
    /// the current generation is invalidated.
    authorization: Option<HeaderValue>,
    expires_at: Instant,
    lifetime: Duration,
    /// Starts at 0, which has no token. Only ever increases, and only when a
    /// new token is stored.
    generation: u64,
    inflight: Option<Inflight>,
    next_call_id: u64,
}

/// The one exchange in progress. Callers hold their own clone of `call` and
/// never read it back from the state.
struct Inflight {
    id: u64,
    call: SharedExchange,
}

impl WorkloadIdentityCredentials {
    /// Begin building workload identity credentials.
    pub fn builder() -> WorkloadIdentityCredentialsBuilder {
        WorkloadIdentityCredentialsBuilder::default()
    }

    /// The cached token when it is still fresh, otherwise the result of the
    /// one shared exchange.
    async fn current_token(&self) -> ExchangeOutcome {
        let now = (self.inner.clock)();
        let call = {
            let mut state = self.inner.lock();
            if let Some(authorization) = state.fresh(now) {
                return Ok((authorization, state.generation));
            }
            match &state.inflight {
                Some(inflight) => inflight.call.clone(),
                None => self.register_exchange(&mut state),
            }
        };
        // Dropping this future is how a caller gives up. The exchange itself
        // runs on its own task and carries on for the other callers.
        call.await
    }

    /// Put a new shared exchange in the in-flight slot. Called with the state
    /// lock held, so it only builds the call. The exchange task is spawned on
    /// the call's first poll, which the caller makes right after it releases
    /// the lock.
    fn register_exchange(&self, state: &mut State) -> SharedExchange {
        let id = state.next_call_id;
        state.next_call_id += 1;

        // Dropping `slot` frees the in-flight slot. It travels with the
        // exchange, so the slot is freed however the exchange ends: it
        // finishes, it panics, or its runtime drops the task, even a task
        // that never got to run.
        let slot = ClearInflight {
            inner: Arc::clone(&self.inner),
            id,
        };
        let call: SharedExchange = async move {
            let Ok(runtime) = tokio::runtime::Handle::try_current() else {
                // In the description, so that it is part of the message.
                return Err(WorkloadIdentityError::new(
                    error::CODE_EXCHANGE_FAILED,
                    "workload identity credentials must be used inside a Tokio runtime",
                ));
            };
            // Detached: the exchange is bound by its own deadline, not by the
            // caller who happened to arrive first.
            match runtime.spawn(run_exchange(slot)).await {
                Ok(outcome) => outcome,
                // The task panicked or its runtime shut down.
                Err(_) => Err(WorkloadIdentityError::exchange_failed().with_source(Detail(
                    "the token exchange task stopped before it finished".to_string(),
                ))),
            }
        }
        .boxed()
        .shared();

        state.inflight = Some(Inflight {
            id,
            call: call.clone(),
        });
        call
    }
}

impl Inner {
    fn lock(&self) -> MutexGuard<'_, State> {
        // No caller-supplied code runs while the lock is held, and every
        // critical section leaves the state consistent, so a poisoned lock is
        // still usable.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl State {
    fn fresh(&self, now: Instant) -> Option<HeaderValue> {
        let authorization = self.authorization.as_ref()?;
        let remaining = self.expires_at.saturating_duration_since(now);
        (remaining >= refresh_threshold(self.lifetime)).then(|| authorization.clone())
    }

    fn clear_inflight(&mut self, id: u64) {
        if self
            .inflight
            .as_ref()
            .is_some_and(|inflight| inflight.id == id)
        {
            self.inflight = None;
        }
    }
}

/// `min(60 seconds, 20% of the lifetime)`.
fn refresh_threshold(lifetime: Duration) -> Duration {
    REFRESH_THRESHOLD.min(lifetime.mul_f64(REFRESH_FRACTION))
}

/// Owns one exchange's claim on the in-flight slot and frees the slot when
/// dropped. Never dropped while the state lock is held.
struct ClearInflight {
    inner: Arc<Inner>,
    id: u64,
}

impl Drop for ClearInflight {
    fn drop(&mut self) {
        self.inner.lock().clear_inflight(self.id);
    }
}

/// Body of the detached exchange task. On every path the in-flight slot is
/// cleared before the task returns, which is what wakes the waiting callers:
/// a caller that gets an error and retries at once starts a new exchange.
async fn run_exchange(slot: ClearInflight) -> ExchangeOutcome {
    let inner = &slot.inner;

    let result =
        match tokio::time::timeout(EXCHANGE_DEADLINE, exchange::exchange(&inner.exchange)).await {
            Ok(result) => result,
            Err(_) => Err(
                WorkloadIdentityError::exchange_failed().with_source(Detail(format!(
                    "the token exchange did not finish within {} seconds",
                    EXCHANGE_DEADLINE.as_secs()
                ))),
            ),
        };

    let now = (inner.clock)();
    let mut state = inner.lock();
    match result {
        Ok(token) => {
            // An `expires_in` too large to add to the clock is as unusable as
            // a missing one.
            let (lifetime, expires_at) = match now.checked_add(token.lifetime) {
                Some(expires_at) => (token.lifetime, expires_at),
                None => (DEFAULT_LIFETIME, now + DEFAULT_LIFETIME),
            };
            // Store the token and clear the slot in one locked section.
            state.authorization = Some(token.authorization.clone());
            state.lifetime = lifetime;
            state.expires_at = expires_at;
            state.generation += 1;
            state.clear_inflight(slot.id);
            Ok((token.authorization, state.generation))
        }
        Err(error) => {
            // A failed exchange is not cached.
            state.clear_inflight(slot.id);
            Err(error)
        }
    }
}

#[async_trait]
impl Credentials for WorkloadIdentityCredentials {
    async fn resolve(&self) -> Result<CredentialSnapshot, Error> {
        let (authorization, generation) = self.current_token().await?;
        let mut headers = HeaderMap::with_capacity(2);
        headers.insert(AUTHORIZATION, authorization);
        headers.insert(
            HeaderName::from_static(ENVIRONMENT_ID_HEADER),
            self.inner.environment_id.clone(),
        );
        Ok(CredentialSnapshot {
            headers,
            generation,
            recoverable: true,
        })
    }

    fn invalidate(&self, generation: u64) {
        let mut state = self.inner.lock();
        // Conditional on the generation: a late 401 for an old token must not
        // discard a newer one that another request already fetched.
        if state.generation == generation {
            state.authorization = None;
        }
    }
}

impl fmt::Debug for WorkloadIdentityCredentials {
    /// Prints the configuration. The access token is never included.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let config = &self.inner.exchange;
        f.debug_struct("WorkloadIdentityCredentials")
            .field("client_id", &config.client_id)
            .field("token_url", &config.token_url)
            .field("environment_id", &self.inner.environment_id)
            .field("resource", &config.resource)
            .field("scope", &config.scope)
            .finish_non_exhaustive()
    }
}

/// Builder for [`WorkloadIdentityCredentials`].
///
/// `client_id`, `token_url`, `environment_id` and `scope` fall back to the
/// environment variables `WORKOS_CLIENT_ID`, `WORKOS_WIF_TOKEN_URL`,
/// `WORKOS_ENVIRONMENT_ID` and `WORKOS_WIF_SCOPE` when they are not set here.
/// A value set here always wins. The variables only fill in credentials you
/// construct; nothing in the environment turns workload identity on.
#[derive(Default)]
pub struct WorkloadIdentityCredentialsBuilder {
    client_id: Option<String>,
    token_url: Option<String>,
    environment_id: Option<String>,
    resource: Option<String>,
    scope: Option<String>,
    provider: Option<Arc<dyn IdentityTokenProvider>>,
    transport: Option<SharedTransport>,
    clock: Option<Clock>,
    sleeper: Option<Sleeper>,
    env: Option<EnvLookup>,
}

impl WorkloadIdentityCredentialsBuilder {
    /// The M2M application's client ID. Required; defaults to
    /// `WORKOS_CLIENT_ID`.
    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// The token exchange endpoint. Required; defaults to
    /// `WORKOS_WIF_TOKEN_URL`. There is no built-in default.
    pub fn token_url(mut self, token_url: impl Into<String>) -> Self {
        self.token_url = Some(token_url.into());
        self
    }

    /// The environment the access token is for, sent as the
    /// `x-url-environment-id` header on every API request. Required; defaults
    /// to `WORKOS_ENVIRONMENT_ID`.
    pub fn environment_id(mut self, environment_id: impl Into<String>) -> Self {
        self.environment_id = Some(environment_id.into());
        self
    }

    /// The `resource` form field. Defaults to [`DEFAULT_RESOURCE`].
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// The `scope` form field. Defaults to `WORKOS_WIF_SCOPE`; left out of
    /// the exchange when empty.
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Where the workload's identity token comes from. Required.
    ///
    /// Takes [`token_file`], [`token_env`], a provider from one of the
    /// platform crates, or a closure returning a future.
    pub fn identity_token_provider(
        mut self,
        provider: impl IdentityTokenProvider + 'static,
    ) -> Self {
        self.provider = Some(Arc::new(provider));
        self
    }

    /// Replace the transport the exchange is sent through.
    #[cfg(test)]
    pub(crate) fn transport(mut self, transport: SharedTransport) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Replace the clock that decides when a token needs a refresh.
    #[cfg(test)]
    pub(crate) fn clock(mut self, clock: impl Fn() -> Instant + Send + Sync + 'static) -> Self {
        self.clock = Some(Arc::new(clock));
        self
    }

    /// Replace the wait between exchange attempts.
    #[cfg(test)]
    pub(crate) fn sleeper(
        mut self,
        sleeper: impl Fn(Duration) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    ) -> Self {
        self.sleeper = Some(Arc::new(sleeper));
        self
    }

    /// Replace the process environment the defaults are read from.
    #[cfg(test)]
    pub(crate) fn env(
        mut self,
        env: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        self.env = Some(Arc::new(env));
        self
    }

    /// Validate the configuration and build the credentials.
    ///
    /// Fails with code `invalid_configuration` when a required setting is
    /// missing after the environment is consulted. No request is sent; the
    /// first exchange happens on the first API call.
    pub fn build(self) -> Result<WorkloadIdentityCredentials, WorkloadIdentityError> {
        let env = self
            .env
            .unwrap_or_else(|| Arc::new(|name: &str| std::env::var(name).ok()));

        let client_id = required(self.client_id, &env, "client_id", CLIENT_ID_ENV)?;
        let token_url = required(self.token_url, &env, "token_url", TOKEN_URL_ENV)?;
        let environment_id = required(
            self.environment_id,
            &env,
            "environment_id",
            ENVIRONMENT_ID_ENV,
        )?;
        let scope = self
            .scope
            .or_else(|| env(SCOPE_ENV))
            .filter(|scope| !scope.is_empty());
        let resource = self
            .resource
            .filter(|resource| !resource.is_empty())
            .unwrap_or_else(|| DEFAULT_RESOURCE.to_string());
        let provider = self.provider.ok_or_else(|| {
            WorkloadIdentityError::invalid_configuration("identity_token_provider is required")
        })?;

        let is_http_url =
            url::Url::parse(&token_url).is_ok_and(|url| matches!(url.scheme(), "http" | "https"));
        if !is_http_url {
            return Err(WorkloadIdentityError::invalid_configuration(
                "token_url must be an absolute http or https URL",
            ));
        }
        let environment_id = HeaderValue::from_str(&environment_id).map_err(|_| {
            WorkloadIdentityError::invalid_configuration(
                "environment_id cannot be sent as an HTTP header",
            )
        })?;

        let transport = match self.transport {
            Some(transport) => transport,
            None => default_transport()?,
        };

        Ok(WorkloadIdentityCredentials {
            inner: Arc::new(Inner {
                environment_id,
                exchange: ExchangeConfig {
                    client_id,
                    token_url,
                    resource,
                    scope,
                    provider,
                    transport,
                    sleeper: self
                        .sleeper
                        .unwrap_or_else(|| Arc::new(|delay| tokio::time::sleep(delay).boxed())),
                },
                clock: self.clock.unwrap_or_else(|| Arc::new(Instant::now)),
                state: Mutex::new(State {
                    authorization: None,
                    expires_at: Instant::now(),
                    lifetime: Duration::ZERO,
                    generation: 0,
                    inflight: None,
                    next_call_id: 0,
                }),
            }),
        })
    }
}

/// An explicit value wins; otherwise the environment variable. Empty counts
/// as missing.
fn required(
    explicit: Option<String>,
    env: &EnvLookup,
    field: &str,
    variable: &str,
) -> Result<String, WorkloadIdentityError> {
    explicit
        .or_else(|| env(variable))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            WorkloadIdentityError::invalid_configuration(&format!(
                "{field} is required (or set {variable})"
            ))
        })
}

/// The exchange's own transport: the bundled `reqwest` client with redirects
/// switched off.
#[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
fn default_transport() -> Result<SharedTransport, WorkloadIdentityError> {
    let transport =
        crate::transport::ReqwestTransport::without_redirects(crate::client::DEFAULT_TIMEOUT)
            .map_err(|e| {
                WorkloadIdentityError::invalid_configuration(
                    "the HTTP client for the token exchange could not be built",
                )
                .with_source(e)
            })?;
    Ok(Arc::new(transport))
}

#[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
fn default_transport() -> Result<SharedTransport, WorkloadIdentityError> {
    Err(WorkloadIdentityError::invalid_configuration(
        "workload identity needs the bundled HTTP client: enable the `rustls-tls` or `native-tls` feature",
    ))
}
