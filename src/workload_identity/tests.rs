// @oagen-ignore-file
//! Conformance cases WIF-01 to WIF-24. Each case ID is in the name of the
//! test that covers it.
//!
//! Everything runs against local mocks: one wiremock server for the token
//! endpoint and one for the API. Time-dependent cases use an injected clock
//! and retries use a zero-delay sleeper, so nothing waits on a real clock.
//!
//! The SDK has no logging, so the "no token in a log line" half of WIF-17
//! has nothing to capture; the case asserts on error text instead.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::FutureExt;
use http::Method;
use serde_json::json;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use super::*;
use crate::transport::{HttpRequest, HttpResponse, HttpTransport, TransportError};
use crate::{Client, RequestOptions};

const CLIENT_ID: &str = "client_wif_test";
const ENVIRONMENT_ID: &str = "environment_wif_test";

// ---------------------------------------------------------------------------
// Mock token endpoint
// ---------------------------------------------------------------------------

/// One scripted answer from the token endpoint.
enum Reply {
    /// HTTP 200 with the next `access-N`. `expires_in` is left out of the
    /// body when `None`.
    Success { expires_in: Option<u64> },
    /// A literal response.
    Raw(ResponseTemplate),
}

fn oauth_error(code: &str) -> Reply {
    Reply::Raw(ResponseTemplate::new(400).set_body_json(json!({
        "error": code,
        "error_description": format!("description for {code}"),
    })))
}

fn status(code: u16) -> Reply {
    Reply::Raw(ResponseTemplate::new(code))
}

/// Plays the script, then answers every further POST with a success.
struct TokenResponder {
    script: Mutex<VecDeque<Reply>>,
    default_expires_in: Option<u64>,
    successes: AtomicUsize,
}

impl Respond for TokenResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let reply = self
            .script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Reply::Success {
                expires_in: self.default_expires_in,
            });
        match reply {
            Reply::Raw(template) => template,
            Reply::Success { expires_in } => {
                let n = self.successes.fetch_add(1, Ordering::SeqCst) + 1;
                let mut body =
                    json!({ "access_token": format!("access-{n}"), "token_type": "Bearer" });
                if let Some(seconds) = expires_in {
                    body["expires_in"] = json!(seconds);
                }
                ResponseTemplate::new(200).set_body_json(body)
            }
        }
    }
}

struct TokenEndpoint {
    server: MockServer,
}

impl TokenEndpoint {
    /// Every exchange succeeds with `expires_in: 300`.
    async fn start() -> Self {
        Self::scripted(Vec::new()).await
    }

    /// The script first, then successes with `expires_in: 300`.
    async fn scripted(script: Vec<Reply>) -> Self {
        Self::with(script, Some(300)).await
    }

    async fn with(script: Vec<Reply>, default_expires_in: Option<u64>) -> Self {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(TokenResponder {
                script: Mutex::new(script.into()),
                default_expires_in,
                successes: AtomicUsize::new(0),
            })
            .mount(&server)
            .await;
        Self { server }
    }

    fn url(&self) -> String {
        format!("{}/oauth2/token", self.server.uri())
    }

    async fn requests(&self) -> Vec<Request> {
        self.server.received_requests().await.unwrap()
    }

    /// The exchange count: POSTs the endpoint received.
    async fn count(&self) -> usize {
        self.requests().await.len()
    }

    /// The decoded form of each exchange, in order.
    async fn forms(&self) -> Vec<HashMap<String, String>> {
        self.requests()
            .await
            .iter()
            .map(|request| {
                url::form_urlencoded::parse(&request.body)
                    .into_owned()
                    .collect()
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Mock API
// ---------------------------------------------------------------------------

/// Answers with the scripted statuses, then with `then` forever.
struct ApiResponder {
    script: Mutex<VecDeque<u16>>,
    then: u16,
}

impl Respond for ApiResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let status = self.script.lock().unwrap().pop_front().unwrap_or(self.then);
        if status == 200 {
            return ResponseTemplate::new(200).set_body_json(json!({
                "object": "list",
                "data": [],
                "list_metadata": { "before": null, "after": null },
            }));
        }
        // `Retry-After: 0` keeps the client's own retry loop from sleeping.
        ResponseTemplate::new(status)
            .insert_header("retry-after", "0")
            .set_body_json(json!({ "message": "scripted failure" }))
    }
}

struct Api {
    server: MockServer,
}

impl Api {
    /// Every request succeeds.
    async fn start() -> Self {
        Self::scripted(&[], 200).await
    }

    async fn scripted(script: &[u16], then: u16) -> Self {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ApiResponder {
                script: Mutex::new(script.iter().copied().collect()),
                then,
            })
            .mount(&server)
            .await;
        Self { server }
    }

    async fn requests(&self) -> Vec<Request> {
        self.server.received_requests().await.unwrap()
    }

    /// The `Authorization` header of each attempt, in order.
    async fn bearers(&self) -> Vec<String> {
        self.requests()
            .await
            .iter()
            .map(|request| header(request, "authorization").unwrap_or_default())
            .collect()
    }
}

fn header(request: &Request, name: &str) -> Option<String> {
    request
        .headers
        .get(name)
        .map(|value| value.to_str().unwrap().to_owned())
}

// ---------------------------------------------------------------------------
// Credentials, clients, clock, providers
// ---------------------------------------------------------------------------

/// A provider that returns `assertion` and counts its calls.
fn provider(
    assertion: &'static str,
    calls: &Arc<AtomicUsize>,
) -> impl IdentityTokenProvider + 'static {
    let calls = calls.clone();
    move || {
        calls.fetch_add(1, Ordering::SeqCst);
        async move { Ok::<_, ProviderError>(assertion.to_owned()) }
    }
}

/// The fixture credentials: the case list's client ID and environment ID, a
/// callback returning `assertion-1`, an empty environment, and no real sleep
/// between exchange attempts.
fn credentials(endpoint: &TokenEndpoint) -> WorkloadIdentityCredentialsBuilder {
    WorkloadIdentityCredentials::builder()
        .client_id(CLIENT_ID)
        .token_url(endpoint.url())
        .environment_id(ENVIRONMENT_ID)
        .identity_token_provider(provider("assertion-1", &Arc::default()))
        .env(|_| None)
        .sleeper(|_| async {}.boxed())
}

fn client(api: &Api, credentials: &WorkloadIdentityCredentials) -> Client {
    Client::builder()
        .base_url(api.server.uri())
        .credentials(credentials.clone())
        .build()
}

/// One API request: `GET /organizations`.
async fn list(client: &Client) -> Result<(), Error> {
    client
        .organizations()
        .list_organizations(Default::default())
        .await
        .map(|_| ())
}

fn wif_error(result: Result<(), Error>) -> WorkloadIdentityError {
    match result {
        Err(Error::WorkloadIdentity(error)) => error,
        other => panic!("expected a WorkloadIdentityError, got {other:?}"),
    }
}

/// A clock the test moves by hand.
#[derive(Clone)]
struct TestClock {
    origin: Instant,
    elapsed: Arc<Mutex<Duration>>,
}

impl TestClock {
    fn new() -> Self {
        Self {
            origin: Instant::now(),
            elapsed: Arc::default(),
        }
    }

    fn advance_to(&self, seconds: u64) {
        *self.elapsed.lock().unwrap() = Duration::from_secs(seconds);
    }

    fn reader(&self) -> impl Fn() -> Instant + Send + Sync + 'static {
        let clock = self.clone();
        move || clock.origin + *clock.elapsed.lock().unwrap()
    }
}

/// Makes a request at t=0, `fresh_at` and `stale_at`, and checks that only
/// the last one triggers a second exchange.
async fn assert_refreshes_between(endpoint: TokenEndpoint, fresh_at: u64, stale_at: u64) {
    let api = Api::start().await;
    let clock = TestClock::new();
    let credentials = credentials(&endpoint)
        .clock(clock.reader())
        .build()
        .unwrap();
    let client = client(&api, &credentials);

    list(&client).await.unwrap();
    clock.advance_to(fresh_at);
    list(&client).await.unwrap();
    assert_eq!(
        endpoint.count().await,
        1,
        "refreshed too early, at t={fresh_at}s"
    );

    clock.advance_to(stale_at);
    list(&client).await.unwrap();
    assert_eq!(
        endpoint.count().await,
        2,
        "did not refresh at t={stale_at}s"
    );
    assert_eq!(
        api.bearers().await,
        ["Bearer access-1", "Bearer access-1", "Bearer access-2"]
    );
}

// ---------------------------------------------------------------------------
// Exchange transports for the concurrency and connection-failure cases
// ---------------------------------------------------------------------------

/// Sends the exchange for real, then holds the response until released.
struct HeldTransport {
    inner: SharedTransport,
    gate: tokio::sync::Semaphore,
}

impl HeldTransport {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: default_transport().unwrap(),
            gate: tokio::sync::Semaphore::new(0),
        })
    }

    fn release(&self) {
        self.gate.close();
    }
}

#[async_trait]
impl HttpTransport for HeldTransport {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        let response = self.inner.execute(request).await;
        // Resolves when the gate is closed.
        let _ = self.gate.acquire().await;
        response
    }
}

/// Fails the first call as a refused connection, then sends for real.
struct FailsToConnectOnce {
    inner: SharedTransport,
    calls: AtomicUsize,
}

#[async_trait]
impl HttpTransport for FailsToConnectOnce {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(TransportError::connect("connection refused"));
        }
        self.inner.execute(request).await
    }
}

/// Callers currently waiting on the shared exchange.
fn waiting(credentials: &WorkloadIdentityCredentials) -> usize {
    let state = credentials.inner.lock();
    state
        .inflight
        .as_ref()
        .and_then(|inflight| inflight.call.strong_count())
        // One handle is the state's own.
        .map_or(0, |handles| handles.saturating_sub(1))
}

async fn wait_until_waiting(credentials: &WorkloadIdentityCredentials, callers: usize) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while waiting(credentials) != callers {
        assert!(
            Instant::now() < deadline,
            "never saw {callers} waiting callers"
        );
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}

// ---------------------------------------------------------------------------
// Running a case in a child process with a controlled environment
// ---------------------------------------------------------------------------

/// `std::env::set_var` is unsafe in edition 2024 and this crate forbids
/// unsafe code, so the cases that depend on real environment variables run
/// themselves again in a child process whose environment is set up front.
const CHILD_MARKER: &str = "WORKOS_WIF_TEST_CHILD";
const TEST_MODULE: &str = "workload_identity::tests";
const WIF_VARIABLES: [&str; 4] = [CLIENT_ID_ENV, TOKEN_URL_ENV, SCOPE_ENV, ENVIRONMENT_ID_ENV];

fn is_child(test: &str) -> bool {
    std::env::var(CHILD_MARKER).as_deref() == Ok(test)
}

/// Run `test` alone in a child process with `set` added to its environment
/// and every WIF variable not in `set` removed. Fails if the child fails.
async fn run_in_child(test: &'static str, set: Vec<(&'static str, String)>) {
    let output = tokio::task::spawn_blocking(move || {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg(format!("{TEST_MODULE}::{test}"))
            .args(["--exact", "--nocapture", "--test-threads=1"])
            .env(CHILD_MARKER, test);
        for name in WIF_VARIABLES {
            command.env_remove(name);
        }
        for (name, value) in set {
            command.env(name, value);
        }
        command.output().unwrap()
    })
    .await
    .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "the child run of {test} failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// WIF-01 to WIF-08: exchange, cache and refresh
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wif_01_first_request_sends_one_exchange_post() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let unscoped = credentials(&endpoint).build().unwrap();

    list(&client(&api, &unscoped)).await.unwrap();

    let requests = endpoint.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, Method::POST);
    assert_eq!(requests[0].url.path(), "/oauth2/token");
    assert_eq!(
        header(&requests[0], "content-type").as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(
        header(&requests[0], "accept").as_deref(),
        Some("application/json")
    );
    assert_eq!(header(&requests[0], "authorization"), None);

    let form = &endpoint.forms().await[0];
    let expected = HashMap::from([
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", "assertion-1"),
        ("client_id", "client_wif_test"),
        ("resource", "https://api.workos.com"),
    ]);
    assert_eq!(
        form.len(),
        expected.len(),
        "unexpected form fields: {form:?}"
    );
    for (field, value) in expected {
        assert_eq!(
            form.get(field).map(String::as_str),
            Some(value),
            "field {field}"
        );
    }
    assert!(!form.contains_key("scope"));
    assert!(!form.contains_key("client_secret"));

    // Again on a new instance, with a scope.
    let endpoint = TokenEndpoint::start().await;
    let scoped = credentials(&endpoint)
        .scope("organizations:write")
        .build()
        .unwrap();
    list(&client(&api, &scoped)).await.unwrap();

    let forms = endpoint.forms().await;
    assert_eq!(forms.len(), 1);
    assert_eq!(
        forms[0].get("scope").map(String::as_str),
        Some("organizations:write")
    );
    assert!(!forms[0].contains_key("client_secret"));
}

#[tokio::test]
async fn wif_02_api_request_carries_access_token_and_environment_header() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let credentials = credentials(&endpoint).build().unwrap();

    list(&client(&api, &credentials)).await.unwrap();

    let requests = api.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        header(&requests[0], "authorization").as_deref(),
        Some("Bearer access-1")
    );
    assert_eq!(
        header(&requests[0], "x-url-environment-id").as_deref(),
        Some("environment_wif_test")
    );
}

#[tokio::test]
async fn wif_03_second_request_reuses_the_cached_token() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let clock = TestClock::new();
    let credentials = credentials(&endpoint)
        .clock(clock.reader())
        .build()
        .unwrap();
    let client = client(&api, &credentials);

    list(&client).await.unwrap();
    list(&client).await.unwrap();

    assert_eq!(endpoint.count().await, 1);
    assert_eq!(api.bearers().await, ["Bearer access-1", "Bearer access-1"]);
}

#[tokio::test]
async fn wif_04_refreshes_when_less_than_60_seconds_remain() {
    assert_refreshes_between(TokenEndpoint::with(Vec::new(), Some(300)).await, 239, 241).await;
}

#[tokio::test]
async fn wif_05_refresh_threshold_is_20_percent_for_a_short_lifetime() {
    assert_refreshes_between(TokenEndpoint::with(Vec::new(), Some(100)).await, 79, 81).await;
}

#[tokio::test]
async fn wif_06_missing_expires_in_is_treated_as_300_seconds() {
    assert_refreshes_between(TokenEndpoint::with(Vec::new(), None).await, 239, 241).await;
}

#[tokio::test]
async fn wif_07_two_instances_never_share_a_token() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let first = credentials(&endpoint).build().unwrap();
    let second = credentials(&endpoint).build().unwrap();

    list(&client(&api, &first)).await.unwrap();
    list(&client(&api, &second)).await.unwrap();

    assert_eq!(endpoint.count().await, 2);
    assert_eq!(api.bearers().await, ["Bearer access-1", "Bearer access-2"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wif_08_concurrent_first_requests_share_one_exchange() {
    const CALLERS: usize = 20;
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let held = HeldTransport::new();
    let provider_calls = Arc::new(AtomicUsize::new(0));
    let credentials = credentials(&endpoint)
        .identity_token_provider(provider("assertion-1", &provider_calls))
        .transport(held.clone())
        .build()
        .unwrap();
    let client = client(&api, &credentials);

    let requests: Vec<_> = (0..CALLERS)
        .map(|_| {
            let client = client.clone();
            tokio::spawn(async move { list(&client).await })
        })
        .collect();

    // The endpoint's response is held until every caller is waiting on it.
    wait_until_waiting(&credentials, CALLERS).await;
    held.release();

    for request in requests {
        request.await.unwrap().unwrap();
    }
    assert_eq!(endpoint.count().await, 1);
    assert_eq!(provider_calls.load(Ordering::SeqCst), 1);
    assert_eq!(api.bearers().await, vec!["Bearer access-1"; CALLERS]);
}

// ---------------------------------------------------------------------------
// WIF-09 to WIF-13: 401 recovery
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wif_09_a_401_exchanges_again_and_retries_once() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::scripted(&[401], 200).await;
    let credentials = credentials(&endpoint).build().unwrap();

    list(&client(&api, &credentials)).await.unwrap();

    assert_eq!(endpoint.count().await, 2);
    assert_eq!(api.bearers().await, ["Bearer access-1", "Bearer access-2"]);
}

#[tokio::test]
async fn wif_10_a_second_401_is_returned_to_the_caller() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::scripted(&[], 401).await;
    let credentials = credentials(&endpoint).build().unwrap();

    let error = list(&client(&api, &credentials)).await.unwrap_err();

    assert!(
        matches!(error, Error::Api(_)),
        "unexpected error: {error:?}"
    );
    assert!(error.is_unauthorized());
    assert_eq!(endpoint.count().await, 2);
    assert_eq!(api.bearers().await, ["Bearer access-1", "Bearer access-2"]);
}

#[tokio::test]
async fn wif_11_a_late_401_does_not_discard_a_newer_token() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let credentials = credentials(&endpoint).build().unwrap();

    let first = credentials.resolve().await.unwrap();
    assert_eq!(first.generation, 1);
    assert_eq!(first.headers[AUTHORIZATION], "Bearer access-1");
    assert!(first.recoverable);

    credentials.invalidate(first.generation);
    let second = credentials.resolve().await.unwrap();
    assert_eq!(second.generation, 2);
    assert_eq!(second.headers[AUTHORIZATION], "Bearer access-2");

    // The late 401 for generation 1.
    credentials.invalidate(first.generation);
    list(&client(&api, &credentials)).await.unwrap();

    assert_eq!(credentials.resolve().await.unwrap().generation, 2);
    assert_eq!(endpoint.count().await, 2);
    assert_eq!(api.bearers().await, ["Bearer access-2"]);
}

#[tokio::test]
async fn wif_12_transport_retries_do_not_reset_the_recovery_allowance() {
    let endpoint = TokenEndpoint::start().await;
    // 401, then 503, then 401; a fourth attempt would get a 200.
    let api = Api::scripted(&[401, 503, 401], 200).await;
    let credentials = credentials(&endpoint).build().unwrap();

    let error = list(&client(&api, &credentials)).await.unwrap_err();

    assert!(error.is_unauthorized(), "unexpected error: {error:?}");
    assert_eq!(endpoint.count().await, 2);
    assert_eq!(
        api.bearers().await,
        ["Bearer access-1", "Bearer access-2", "Bearer access-2"]
    );
}

/// `request_with_body_opts` takes query parameters; this is "none".
#[derive(serde::Serialize)]
struct NoQuery {}

#[tokio::test]
async fn wif_13_the_401_retry_resends_the_same_body_and_idempotency_key() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::scripted(&[401], 200).await;
    let credentials = credentials(&endpoint).build().unwrap();
    // The Rust SDK generates no idempotency key, so the case passes one.
    let options = RequestOptions::new().idempotency_key("idempotency-key-1");

    let _: serde_json::Value = client(&api, &credentials)
        .request_with_body_opts(
            Method::POST,
            "/organizations",
            &NoQuery {},
            Some(&json!({ "name": "Acme", "domains": ["acme.test"] })),
            Some(&options),
        )
        .await
        .unwrap();

    let requests = api.requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, Method::POST);
    assert!(!requests[0].body.is_empty());
    assert_eq!(requests[0].body, requests[1].body);
    assert_eq!(
        header(&requests[0], "idempotency-key").as_deref(),
        Some("idempotency-key-1")
    );
    assert_eq!(
        header(&requests[0], "idempotency-key"),
        header(&requests[1], "idempotency-key")
    );
    assert_eq!(api.bearers().await, ["Bearer access-1", "Bearer access-2"]);
}

#[tokio::test]
async fn recovery_applies_to_a_post_that_is_not_retry_eligible() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::scripted(&[401], 200).await;
    let credentials = credentials(&endpoint).build().unwrap();

    // No idempotency key: the retry budget for this POST is one attempt, and
    // the 401 recovery is still made on top of it.
    let _: serde_json::Value = client(&api, &credentials)
        .request_json(Method::POST, "/organizations", &json!({ "name": "Acme" }))
        .await
        .unwrap();

    assert_eq!(api.bearers().await, ["Bearer access-1", "Bearer access-2"]);
}

#[tokio::test]
async fn a_per_request_authorization_header_bypasses_the_credentials() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::scripted(&[], 401).await;
    let credentials = credentials(&endpoint).build().unwrap();
    let options = RequestOptions::new().header(
        AUTHORIZATION,
        HeaderValue::from_static("Bearer end-user-token"),
    );

    let error = client(&api, &credentials)
        .organizations()
        .list_organizations_with_options(Default::default(), Some(&options))
        .await
        .unwrap_err();

    // Sent as built, with no exchange, no environment header and no recovery.
    assert!(error.is_unauthorized());
    assert_eq!(endpoint.count().await, 0);
    let requests = api.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        header(&requests[0], "authorization").as_deref(),
        Some("Bearer end-user-token")
    );
    assert_eq!(header(&requests[0], ENVIRONMENT_ID_HEADER), None);
}

// ---------------------------------------------------------------------------
// WIF-14 to WIF-18: exchange failures
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wif_14_exchange_retries_on_connection_failure_429_and_5xx() {
    let api = Api::start().await;

    // (a) 503, then 429 with `Retry-After: 0`, then 200.
    let endpoint = TokenEndpoint::scripted(vec![
        status(503),
        Reply::Raw(ResponseTemplate::new(429).insert_header("retry-after", "0")),
    ])
    .await;
    let calls = Arc::new(AtomicUsize::new(0));
    let credentials_a = credentials(&endpoint)
        .identity_token_provider(provider("assertion-1", &calls))
        .build()
        .unwrap();
    list(&client(&api, &credentials_a)).await.unwrap();
    assert_eq!(endpoint.count().await, 3);
    assert_eq!(calls.load(Ordering::SeqCst), 3);

    // (b) The first attempt fails to connect and the second returns 200.
    let endpoint = TokenEndpoint::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let credentials_b = credentials(&endpoint)
        .identity_token_provider(provider("assertion-1", &calls))
        .transport(Arc::new(FailsToConnectOnce {
            inner: default_transport().unwrap(),
            calls: AtomicUsize::new(0),
        }))
        .build()
        .unwrap();
    list(&client(&api, &credentials_b)).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(endpoint.count().await, 1);

    // (c) The provider fails once, then returns `assertion-2`.
    let endpoint = TokenEndpoint::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let flaky = {
        let calls = calls.clone();
        move || {
            let call = calls.fetch_add(1, Ordering::SeqCst);
            async move {
                if call == 0 {
                    Err("the platform was not ready")
                } else {
                    Ok("assertion-2".to_owned())
                }
            }
        }
    };
    let credentials_c = credentials(&endpoint)
        .identity_token_provider(flaky)
        .build()
        .unwrap();
    list(&client(&api, &credentials_c)).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let forms = endpoint.forms().await;
    assert_eq!(forms.len(), 1);
    assert_eq!(
        forms[0].get("assertion").map(String::as_str),
        Some("assertion-2")
    );

    // Fourth run: 429 with `Retry-After: 1`, then 200, with a sleeper that
    // records what it was asked to wait.
    let endpoint = TokenEndpoint::scripted(vec![Reply::Raw(
        ResponseTemplate::new(429).insert_header("retry-after", "1"),
    )])
    .await;
    let waits = Arc::new(Mutex::new(Vec::new()));
    let recorded = waits.clone();
    let credentials_d = credentials(&endpoint)
        .sleeper(move |delay| {
            recorded.lock().unwrap().push(delay);
            async {}.boxed()
        })
        .build()
        .unwrap();
    list(&client(&api, &credentials_d)).await.unwrap();
    assert_eq!(endpoint.count().await, 2);
    let waits = waits.lock().unwrap();
    assert_eq!(waits.len(), 1);
    assert!(
        waits[0] >= Duration::from_secs(1),
        "waited only {:?}",
        waits[0]
    );
}

#[tokio::test]
async fn a_refused_connection_is_retried_until_the_budget_runs_out() {
    // Nothing listens on this port once the listener is dropped.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let credentials = credentials(&endpoint)
        .token_url(format!("http://127.0.0.1:{port}/oauth2/token"))
        .identity_token_provider(provider("assertion-1", &calls))
        .build()
        .unwrap();

    let error = wif_error(list(&client(&api, &credentials)).await);

    assert_eq!(error.code, "exchange_failed");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        4,
        "one attempt plus the SDK's three retries"
    );
    assert!(api.requests().await.is_empty());
}

#[tokio::test]
async fn wif_15_oauth_errors_are_typed_and_not_retried() {
    let api = Api::start().await;
    for code in [
        "invalid_grant",
        "invalid_target",
        "invalid_client",
        "unauthorized_client",
        "invalid_scope",
        "invalid_request",
    ] {
        let endpoint = TokenEndpoint::scripted(vec![oauth_error(code)]).await;
        let credentials = credentials(&endpoint).build().unwrap();

        let error = wif_error(list(&client(&api, &credentials)).await);

        assert_eq!(error.code, code);
        assert_eq!(error.description, format!("description for {code}"));
        assert!(!error.hint.is_empty());
        assert_ne!(
            error.hint,
            error::DEFAULT_HINT,
            "{code} should have its own hint"
        );
        assert_eq!(
            error.to_string(),
            format!("{code}: {} description for {code}", error.hint)
        );
        assert_eq!(endpoint.count().await, 1, "{code} must not be retried");
    }
    assert!(api.requests().await.is_empty());
}

#[tokio::test]
async fn wif_16_a_failed_exchange_is_not_cached() {
    let endpoint = TokenEndpoint::scripted(vec![oauth_error("invalid_grant")]).await;
    let api = Api::start().await;
    let credentials = credentials(&endpoint).build().unwrap();
    let client = client(&api, &credentials);

    let error = wif_error(list(&client).await);
    assert_eq!(error.code, "invalid_grant");

    list(&client).await.unwrap();
    assert_eq!(endpoint.count().await, 2);
    assert_eq!(api.bearers().await, ["Bearer access-1"]);
    assert_eq!(waiting(&credentials), 0);
}

/// `Display` and `Debug` of the error and of every error in its source chain.
fn every_rendering(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = String::new();
    let mut current = Some(error);
    while let Some(error) = current {
        text.push_str(&format!("{error}\n{error:?}\n"));
        current = error.source();
    }
    text
}

#[tokio::test]
async fn wif_17_tokens_never_appear_in_errors() {
    const ASSERTION: &str = "SECRET-ASSERTION-VALUE";
    const ACCESS_TOKEN: &str = "SECRET-ACCESS-VALUE";
    let secret_success = || {
        Reply::Raw(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": ACCESS_TOKEN,
            "token_type": "Bearer",
            "expires_in": 300,
        })))
    };
    let secret_provider = || async { Ok::<_, ProviderError>(ASSERTION.to_owned()) };
    let mut rendered = String::new();

    // An `invalid_grant` exchange.
    let endpoint = TokenEndpoint::scripted(vec![oauth_error("invalid_grant")]).await;
    let api = Api::start().await;
    let credentials_a = credentials(&endpoint)
        .identity_token_provider(secret_provider)
        .build()
        .unwrap();
    let error = list(&client(&api, &credentials_a)).await.unwrap_err();
    assert_eq!(error.workload_identity().unwrap().code, "invalid_grant");
    rendered.push_str(&every_rendering(&error));

    // A 500 from the token endpoint until the retries run out.
    let endpoint = TokenEndpoint::scripted((0..10).map(|_| status(500)).collect()).await;
    let credentials_b = credentials(&endpoint)
        .identity_token_provider(secret_provider)
        .build()
        .unwrap();
    let error = list(&client(&api, &credentials_b)).await.unwrap_err();
    assert_eq!(error.workload_identity().unwrap().code, "exchange_failed");
    assert_eq!(endpoint.count().await, 4);
    rendered.push_str(&every_rendering(&error));

    // A provider that raises.
    let endpoint = TokenEndpoint::start().await;
    let credentials_c = credentials(&endpoint)
        .identity_token_provider(|| async { Err::<String, _>("the platform refused") })
        .build()
        .unwrap();
    let error = list(&client(&api, &credentials_c)).await.unwrap_err();
    assert_eq!(error.workload_identity().unwrap().code, "provider_failed");
    assert_eq!(endpoint.count().await, 0);
    rendered.push_str(&every_rendering(&error));

    // A second 401 from the API, with a real access token in play.
    let endpoint = TokenEndpoint::scripted(vec![secret_success(), secret_success()]).await;
    let api = Api::scripted(&[], 401).await;
    let credentials_d = credentials(&endpoint)
        .identity_token_provider(secret_provider)
        .build()
        .unwrap();
    let error = list(&client(&api, &credentials_d)).await.unwrap_err();
    assert!(error.is_unauthorized());
    assert_eq!(
        api.bearers().await,
        vec![format!("Bearer {ACCESS_TOKEN}"); 2]
    );
    rendered.push_str(&every_rendering(&error));

    // The credentials and a snapshot holding the live token.
    let endpoint = TokenEndpoint::scripted(vec![secret_success()]).await;
    let credentials_e = credentials(&endpoint)
        .identity_token_provider(secret_provider)
        .build()
        .unwrap();
    let snapshot = credentials_e.resolve().await.unwrap();
    assert_eq!(
        snapshot.headers[AUTHORIZATION],
        format!("Bearer {ACCESS_TOKEN}").as_str()
    );
    rendered.push_str(&format!(
        "{snapshot:?}\n{credentials_e:?}\n{:?}\n",
        snapshot.headers
    ));

    assert!(!rendered.is_empty());
    assert!(
        !rendered.contains(ASSERTION),
        "the assertion leaked:\n{rendered}"
    );
    assert!(
        !rendered.contains(ACCESS_TOKEN),
        "the access token leaked:\n{rendered}"
    );
}

#[tokio::test]
async fn wif_18_the_exchange_does_not_follow_redirects() {
    let elsewhere = TokenEndpoint::start().await;
    let endpoint = TokenEndpoint::scripted(vec![Reply::Raw(
        ResponseTemplate::new(307).insert_header("location", elsewhere.url().as_str()),
    )])
    .await;
    let api = Api::start().await;
    let credentials = credentials(&endpoint).build().unwrap();

    let error = wif_error(list(&client(&api, &credentials)).await);

    assert_eq!(error.code, "exchange_failed");
    assert_eq!(elsewhere.count().await, 0);
    let requests = endpoint.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, Method::POST);
}

// ---------------------------------------------------------------------------
// WIF-19 to WIF-24: precedence, providers, cancellation, static keys
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wif_19_api_key_with_credentials_is_a_configuration_error() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let credentials = credentials(&endpoint).build().unwrap();

    let result = Client::builder()
        .api_key("sk_test_explicit")
        .credentials(credentials)
        .base_url(api.server.uri())
        .try_build();

    match result {
        Err(Error::Builder(message)) => {
            assert!(
                message.contains("api_key") && message.contains("credentials"),
                "{message}"
            );
        }
        Err(other) => panic!("expected a configuration error, got {other:?}"),
        Ok(_) => panic!("expected a configuration error, got a client"),
    }
    assert_eq!(endpoint.count().await, 0);
    assert!(api.requests().await.is_empty());
}

#[tokio::test]
async fn wif_20_credentials_win_over_the_environment_api_key() {
    const TEST: &str = "wif_20_credentials_win_over_the_environment_api_key";
    const ENV_KEY: &str = "sk_test_env";
    if !is_child(TEST) {
        return run_in_child(TEST, vec![("WORKOS_API_KEY", ENV_KEY.to_owned())]).await;
    }
    assert_eq!(std::env::var("WORKOS_API_KEY").as_deref(), Ok(ENV_KEY));
    let sent_env_key = |requests: &[Request]| {
        requests
            .iter()
            .any(|request| format!("{:?}", request.headers).contains(ENV_KEY))
    };

    // (a) Valid credentials: the API sees the access token, never the key.
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let valid = WorkloadIdentityCredentials::builder()
        .client_id(CLIENT_ID)
        .token_url(endpoint.url())
        .environment_id(ENVIRONMENT_ID)
        .identity_token_provider(provider("assertion-1", &Arc::default()))
        .build()
        .unwrap();
    list(&client(&api, &valid)).await.unwrap();
    assert_eq!(api.bearers().await, ["Bearer access-1"]);
    assert!(!sent_env_key(&api.requests().await));
    assert!(!sent_env_key(&endpoint.requests().await));

    // (b) No token URL, explicitly or in the environment: construction fails.
    let error = WorkloadIdentityCredentials::builder()
        .client_id(CLIENT_ID)
        .environment_id(ENVIRONMENT_ID)
        .identity_token_provider(provider("assertion-1", &Arc::default()))
        .build()
        .unwrap_err();
    assert_eq!(error.code, "invalid_configuration");
    assert_eq!(
        error.description,
        "token_url is required (or set WORKOS_WIF_TOKEN_URL)"
    );

    // (c) A failing exchange is an error, not a fallback to the key.
    let endpoint = TokenEndpoint::scripted(vec![oauth_error("invalid_client")]).await;
    let api = Api::start().await;
    let failing = WorkloadIdentityCredentials::builder()
        .client_id(CLIENT_ID)
        .token_url(endpoint.url())
        .environment_id(ENVIRONMENT_ID)
        .identity_token_provider(provider("assertion-1", &Arc::default()))
        .build()
        .unwrap();
    let error = wif_error(list(&client(&api, &failing)).await);
    assert_eq!(error.code, "invalid_client");
    assert!(api.requests().await.is_empty());
    assert!(!sent_env_key(&endpoint.requests().await));
}

#[tokio::test]
async fn wif_21_token_file_is_read_again_on_every_exchange() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let clock = TestClock::new();
    let path = std::env::temp_dir().join(format!("workos-wif-21-{}.jwt", std::process::id()));
    std::fs::write(&path, "assertion-1\n").unwrap();
    let credentials = credentials(&endpoint)
        .identity_token_provider(token_file(&path))
        .clock(clock.reader())
        .build()
        .unwrap();
    let client = client(&api, &credentials);

    list(&client).await.unwrap();
    std::fs::write(&path, "  assertion-2\n\n").unwrap();
    clock.advance_to(300);
    list(&client).await.unwrap();
    std::fs::remove_file(&path).unwrap();

    let forms = endpoint.forms().await;
    assert_eq!(forms.len(), 2);
    assert_eq!(
        forms[0].get("assertion").map(String::as_str),
        Some("assertion-1")
    );
    assert_eq!(
        forms[1].get("assertion").map(String::as_str),
        Some("assertion-2")
    );

    // A missing file is a provider failure, and names the file, not a token.
    clock.advance_to(600);
    let error = wif_error(list(&client).await);
    assert_eq!(error.code, "provider_failed");
}

#[tokio::test]
async fn wif_22_env_and_callback_providers_and_environment_defaults() {
    const TEST: &str = "wif_22_env_and_callback_providers_and_environment_defaults";
    const TOKEN_URL: &str = "WORKOS_WIF_TEST_TOKEN_URL";
    const API_URL: &str = "WORKOS_WIF_TEST_API_URL";

    if !is_child(TEST) {
        // The parent owns the mocks, because their URLs have to be in the
        // child's environment before it starts. The child makes the requests.
        let endpoint = TokenEndpoint::start().await;
        let api = Api::start().await;
        run_in_child(
            TEST,
            vec![
                ("MY_JWT", "assertion-env".to_owned()),
                (CLIENT_ID_ENV, "client_env".to_owned()),
                (TOKEN_URL_ENV, endpoint.url()),
                (SCOPE_ENV, "scope_env".to_owned()),
                (ENVIRONMENT_ID_ENV, "environment_env".to_owned()),
                (TOKEN_URL, endpoint.url()),
                (API_URL, api.server.uri()),
            ],
        )
        .await;

        let forms = endpoint.forms().await;
        let field = |run: usize, name: &str| forms[run].get(name).cloned();
        let environments: Vec<_> = api
            .requests()
            .await
            .iter()
            .map(|request| header(request, ENVIRONMENT_ID_HEADER).unwrap_or_default())
            .collect();
        assert_eq!(forms.len(), 4);
        // (a) token_env
        assert_eq!(field(0, "assertion").as_deref(), Some("assertion-env"));
        // (b) callback
        assert_eq!(field(1, "assertion").as_deref(), Some("assertion-cb"));
        // (c) every omitted field came from the environment
        assert_eq!(field(2, "client_id").as_deref(), Some("client_env"));
        assert_eq!(field(2, "scope").as_deref(), Some("scope_env"));
        assert_eq!(environments[2], "environment_env");
        // (d) explicit values win
        assert_eq!(field(3, "client_id").as_deref(), Some("client_explicit"));
        assert_eq!(field(3, "scope").as_deref(), Some("scope_explicit"));
        assert_eq!(environments[3], "environment_explicit");
        return;
    }

    let callback = || async { Ok::<_, ProviderError>("assertion-cb".to_owned()) };
    let client = |credentials: WorkloadIdentityCredentials| {
        Client::builder()
            .base_url(std::env::var(API_URL).unwrap())
            .credentials(credentials)
            .build()
    };
    let explicit = || {
        WorkloadIdentityCredentials::builder()
            .client_id(CLIENT_ID)
            .token_url(std::env::var(TOKEN_URL).unwrap())
            .environment_id(ENVIRONMENT_ID)
            .scope("")
    };

    // (a)
    let from_env = explicit()
        .identity_token_provider(token_env("MY_JWT"))
        .build()
        .unwrap();
    list(&client(from_env)).await.unwrap();
    // (b)
    let from_callback = explicit()
        .identity_token_provider(callback)
        .build()
        .unwrap();
    list(&client(from_callback)).await.unwrap();
    // (c) Only a provider; the POST reaching the mock proves the token URL.
    let defaults = WorkloadIdentityCredentials::builder()
        .identity_token_provider(callback)
        .build()
        .unwrap();
    list(&client(defaults)).await.unwrap();
    // (d)
    let overridden = WorkloadIdentityCredentials::builder()
        .client_id("client_explicit")
        .scope("scope_explicit")
        .environment_id("environment_explicit")
        .identity_token_provider(callback)
        .build()
        .unwrap();
    list(&client(overridden)).await.unwrap();

    // An unset variable is a provider failure.
    let unset = explicit()
        .identity_token_provider(token_env("WORKOS_WIF_TEST_UNSET"))
        .sleeper(|_| async {}.boxed())
        .build()
        .unwrap();
    assert_eq!(
        wif_error(list(&client(unset)).await).code,
        "provider_failed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wif_23_cancelling_one_caller_does_not_cancel_the_exchange() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let held = HeldTransport::new();
    let credentials = credentials(&endpoint)
        .transport(held.clone())
        .build()
        .unwrap();
    let client = client(&api, &credentials);

    let caller_a = tokio::spawn({
        let client = client.clone();
        async move { list(&client).await }
    });
    let caller_b = tokio::spawn({
        let client = client.clone();
        async move { list(&client).await }
    });
    wait_until_waiting(&credentials, 2).await;

    // A gives up while the endpoint is still holding its response.
    caller_a.abort();
    assert!(caller_a.await.unwrap_err().is_cancelled());
    wait_until_waiting(&credentials, 1).await;
    held.release();

    caller_b.await.unwrap().unwrap();
    assert_eq!(endpoint.count().await, 1);
    assert_eq!(api.bearers().await, ["Bearer access-1"]);
}

#[tokio::test]
async fn the_exchange_finishes_even_when_every_caller_gives_up() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let held = HeldTransport::new();
    let credentials = credentials(&endpoint)
        .transport(held.clone())
        .build()
        .unwrap();
    let client = client(&api, &credentials);

    let only_caller = tokio::spawn({
        let client = client.clone();
        async move { list(&client).await }
    });
    wait_until_waiting(&credentials, 1).await;
    only_caller.abort();
    assert!(only_caller.await.unwrap_err().is_cancelled());
    held.release();

    // The detached exchange stores its token; the next request reuses it.
    list(&client).await.unwrap();
    assert_eq!(endpoint.count().await, 1);
    assert_eq!(api.bearers().await, ["Bearer access-1"]);
}

#[tokio::test]
async fn wif_24_static_api_key_clients_are_unchanged() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::scripted(&[200], 401).await;
    let client = Client::builder()
        .api_key("sk_test_static")
        .base_url(api.server.uri())
        .build();
    assert_eq!(
        client.default_headers()[AUTHORIZATION],
        "Bearer sk_test_static"
    );
    assert_eq!(client.api_key(), "sk_test_static");

    list(&client).await.unwrap();
    let error = list(&client).await.unwrap_err();

    assert!(error.is_unauthorized());
    let requests = api.requests().await;
    assert_eq!(
        requests.len(),
        2,
        "the 401 is returned after exactly one attempt"
    );
    for request in &requests {
        assert_eq!(
            header(request, "authorization").as_deref(),
            Some("Bearer sk_test_static")
        );
        assert_eq!(header(request, ENVIRONMENT_ID_HEADER), None);
        assert_eq!(
            header(request, "user-agent").as_deref(),
            Some(concat!("workos-rust/", env!("CARGO_PKG_VERSION")))
        );
    }
    assert_eq!(endpoint.count().await, 0);

    // A client with no key at all still sends no `Authorization` header.
    let api = Api::start().await;
    let keyless = Client::builder().base_url(api.server.uri()).build();
    list(&keyless).await.unwrap();
    assert_eq!(header(&api.requests().await[0], "authorization"), None);
}

// ---------------------------------------------------------------------------
// The in-flight slot is freed however an exchange ends
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_panicking_provider_does_not_block_the_next_exchange() {
    let endpoint = TokenEndpoint::start().await;
    let api = Api::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let panics_once = {
        let calls = calls.clone();
        move || {
            let call = calls.fetch_add(1, Ordering::SeqCst);
            async move {
                assert!(call > 0, "a bug in the provider");
                Ok::<_, ProviderError>("assertion-1".to_owned())
            }
        }
    };
    let credentials = credentials(&endpoint)
        .identity_token_provider(panics_once)
        .build()
        .unwrap();
    let client = client(&api, &credentials);

    let error = wif_error(list(&client).await);
    assert_eq!(error.code, "exchange_failed");
    assert_eq!(waiting(&credentials), 0);

    list(&client).await.unwrap();
    assert_eq!(endpoint.count().await, 1);
    assert_eq!(api.bearers().await, ["Bearer access-1"]);
}

#[test]
fn a_runtime_dropped_mid_exchange_does_not_block_the_next_exchange() {
    // The mocks live on a runtime that outlasts the one being dropped.
    let host = tokio::runtime::Runtime::new().unwrap();
    let endpoint = host.block_on(TokenEndpoint::start());
    let credentials = credentials(&endpoint).build().unwrap();

    // Poll `resolve` once, which spawns the exchange, then drop the runtime
    // before the exchange task has run at all.
    let doomed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    doomed.block_on(async {
        assert!(credentials.resolve().now_or_never().is_none());
    });
    drop(doomed);
    assert!(credentials.inner.lock().inflight.is_none());

    let snapshot = host.block_on(credentials.resolve()).unwrap();
    assert_eq!(snapshot.generation, 1);
    assert_eq!(snapshot.headers[AUTHORIZATION], "Bearer access-1");
    assert_eq!(host.block_on(endpoint.count()), 1);
}

#[test]
fn resolving_outside_a_tokio_runtime_is_an_error_and_leaves_nothing_behind() {
    let host = tokio::runtime::Runtime::new().unwrap();
    let endpoint = host.block_on(TokenEndpoint::start());
    let credentials = credentials(&endpoint).build().unwrap();

    let outcome = credentials.resolve().now_or_never();

    match outcome {
        Some(Err(Error::WorkloadIdentity(error))) => {
            assert_eq!(error.code, "exchange_failed");
            assert!(error.to_string().contains("Tokio runtime"), "{error}");
        }
        other => panic!("expected an exchange_failed error, got {other:?}"),
    }
    assert!(credentials.inner.lock().inflight.is_none());
    assert_eq!(host.block_on(credentials.resolve()).unwrap().generation, 1);
}

#[tokio::test]
async fn an_unusable_expires_in_is_treated_as_300_seconds() {
    // Not a number, not positive, too small to count, too large for a
    // `Duration`, and too large to add to the clock.
    for expires_in in [
        json!("300"),
        json!(0),
        json!(-5),
        json!(1e-12),
        json!(1e30),
        json!(1.8e19),
    ] {
        let success = |n: u32| {
            Reply::Raw(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": format!("access-{n}"),
                "token_type": "Bearer",
                "expires_in": expires_in,
            })))
        };
        let endpoint = TokenEndpoint::scripted(vec![success(1), success(2)]).await;
        assert_refreshes_between(endpoint, 239, 241).await;
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[test]
fn missing_settings_are_errors_at_construction_and_name_the_variable() {
    let provider = || async { Ok::<_, ProviderError>("assertion-1".to_owned()) };
    let complete = || {
        WorkloadIdentityCredentials::builder()
            .client_id(CLIENT_ID)
            .token_url("https://auth.example.test/oauth2/token")
            .environment_id(ENVIRONMENT_ID)
            .identity_token_provider(provider)
            .env(|_| None)
    };
    assert!(complete().build().is_ok());

    let cases: [(WorkloadIdentityCredentialsBuilder, &str); 6] = [
        (
            complete().client_id(""),
            "client_id is required (or set WORKOS_CLIENT_ID)",
        ),
        (
            complete().token_url(""),
            "token_url is required (or set WORKOS_WIF_TOKEN_URL)",
        ),
        (
            complete().environment_id(""),
            "environment_id is required (or set WORKOS_ENVIRONMENT_ID)",
        ),
        (
            complete().token_url("auth.example.test/oauth2/token"),
            "token_url must be an absolute http or https URL",
        ),
        (
            complete().environment_id("bad\nvalue"),
            "environment_id cannot be sent as an HTTP header",
        ),
        (
            WorkloadIdentityCredentials::builder()
                .client_id(CLIENT_ID)
                .token_url("https://auth.example.test/oauth2/token")
                .environment_id(ENVIRONMENT_ID)
                .env(|_| None),
            "identity_token_provider is required",
        ),
    ];
    for (builder, description) in cases {
        let error = builder.build().unwrap_err();
        assert_eq!(error.code, "invalid_configuration");
        assert_eq!(error.description, description);
        assert!(!error.hint.is_empty());
    }
}

#[test]
fn refresh_threshold_is_the_shorter_of_60_seconds_and_20_percent() {
    assert_eq!(
        refresh_threshold(Duration::from_secs(300)),
        Duration::from_secs(60)
    );
    assert_eq!(
        refresh_threshold(Duration::from_secs(3600)),
        Duration::from_secs(60)
    );
    assert_eq!(
        refresh_threshold(Duration::from_secs(100)),
        Duration::from_secs(20)
    );
}

#[tokio::test]
async fn unreadable_success_responses_are_invalid_response() {
    let api = Api::start().await;
    for reply in [
        ResponseTemplate::new(200).set_body_string("not json"),
        ResponseTemplate::new(200).set_body_json(json!({ "token_type": "Bearer" })),
        ResponseTemplate::new(200).set_body_json(json!({ "access_token": "" })),
        ResponseTemplate::new(404).set_body_string("not found"),
    ] {
        let endpoint = TokenEndpoint::scripted(vec![Reply::Raw(reply)]).await;
        let credentials = credentials(&endpoint).build().unwrap();
        let error = wif_error(list(&client(&api, &credentials)).await);
        assert_eq!(error.code, "invalid_response");
        assert_eq!(endpoint.count().await, 1);
    }
}

#[tokio::test]
async fn a_retried_status_reports_the_oauth_code_from_its_body() {
    let api = Api::start().await;
    let busy = || {
        Reply::Raw(ResponseTemplate::new(503).set_body_json(json!({
            "error": "temporarily_unavailable",
            "error_description": "x".repeat(600),
        })))
    };
    let endpoint = TokenEndpoint::scripted((0..10).map(|_| busy()).collect()).await;
    let credentials = credentials(&endpoint).build().unwrap();

    let error = wif_error(list(&client(&api, &credentials)).await);

    // The status is checked before the body, so the 503 is retried; the code
    // comes from the last body, the hint is the generic one, and the
    // description is cut to 500 characters.
    assert_eq!(endpoint.count().await, 4);
    assert_eq!(error.code, "temporarily_unavailable");
    assert_eq!(error.hint, error::DEFAULT_HINT);
    assert_eq!(error.description.chars().count(), 500);
}
