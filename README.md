# WorkOS Rust Library

The WorkOS Rust SDK provides async access to the WorkOS API from Rust applications. It uses `tokio`, ships with a default `reqwest` HTTP transport, and includes helpers for common WorkOS flows such as AuthKit, SSO, webhooks, sessions, JWKS, PKCE, and Vault local crypto.

## Documentation

- [WorkOS API Reference](https://workos.com/docs/reference)
- [Crate docs on docs.rs](https://docs.rs/workos)
- [Changelog](./CHANGELOG.md)

## Installation

Requires Rust `1.88+` (edition 2024). The repository pins this via `rust-toolchain.toml`, so a fresh checkout will install the matching toolchain automatically when you run any `cargo` or `./script/ci` command under `rustup`.

```bash
cargo add workos
```

By default, the crate enables `reqwest` with `rustls-tls`. You can switch TLS backends or provide a custom HTTP transport; see [HTTP Transport](#http-transport).

## Quick Start

```rust
use workos::{Client, organizations::ListOrganizationsParams};

#[tokio::main]
async fn main() -> Result<(), workos::Error> {
    let client = Client::builder()
        .api_key(std::env::var("WORKOS_API_KEY").unwrap())
        .client_id(std::env::var("WORKOS_CLIENT_ID").unwrap())
        .build();

    let page = client
        .organizations()
        .list_organizations(ListOrganizationsParams::default())
        .await?;

    for org in page.data {
        println!("{}: {}", org.id, org.name);
    }

    Ok(())
}
```

For an API-key-only client with default settings:

```rust
let client = workos::Client::new(std::env::var("WORKOS_API_KEY").unwrap());
```

## Configuration

`Client::builder()` supports:

| Method            | Description                                                         |
| ----------------- | ------------------------------------------------------------------- |
| `.api_key(_)`     | WorkOS secret key (`sk_...`), required for authenticated API calls. |
| `.client_id(_)`   | WorkOS Client ID, required for AuthKit, SSO, and JWKS helpers.      |
| `.base_url(_)`    | Override the API host. Defaults to `https://api.workos.com`.        |
| `.timeout(_)`     | Per-request timeout. Defaults to 30 seconds.                        |
| `.max_retries(_)` | Retry budget for `429` and `5xx` responses. Defaults to 3.          |
| `.user_agent(_)`  | Override the `User-Agent` header.                                   |
| `.transport(_)`   | Plug in a custom `HttpTransport`.                                   |

`build()` panics if the supplied API key or user-agent contains bytes that aren't valid in an HTTP header. Use `.try_build()` to surface those failures as `Err(workos::Error::Builder(_))` instead — useful when the API key comes from untrusted input.

The client is cheap to clone and can be shared across handlers and tasks.

## API Access

API resources are exposed as accessors on `Client`, for example:

```rust
client.organizations();
client.user_management();
client.sso();
client.webhooks();
client.audit_logs();
```

List endpoints return `Page<T>` values with `data` and `list_metadata` cursors. The crate also exports `auto_paginate` for stream-based iteration.

Every API call returns `Result<_, workos::Error>`. The error type includes API errors, transport failures, decode errors, configuration errors, and helper-specific failures. It also provides predicates such as `is_unauthorized()`, `is_not_found()`, `is_rate_limited()`, and `is_server_error()`.

See the [crate docs](https://docs.rs/workos) for the full resource list, request and response types, pagination details, and helper APIs.

### Forward-compatible enums

Generated enums are `#[non_exhaustive]` and include an `Unknown(String)` variant for wire values the SDK doesn't recognize yet — WorkOS can add new enum values server-side without breaking deserialization for older SDK builds. Match defensively:

```rust
use workos::ConnectionType;

match connection.r#type {
    ConnectionType::GoogleOAuth => { /* ... */ }
    ConnectionType::Unknown(raw) => {
        log::warn!("unknown connection type: {raw}");
    }
    _ => { /* ... */ }
}
```

The original wire string is preserved through round-trips: serializing an `Unknown("FooBar")` re-emits `"FooBar"` verbatim. Each enum also implements `Display`, `FromStr` (infallible), and `AsRef<str>` for ergonomic conversions.

## Per-Request Options

Each generated method has a `*_with_options` companion that takes a `RequestOptions`. Use it to pass additional headers, a per-request retry policy, or — on endpoints that support it — an idempotency key:

```rust
use workos::{RequestOptions, audit_logs::CreateEventParams};

let opts = RequestOptions::new().idempotency_key("51343ada-3f0f-4372-9d24-4d0b0dfba384");

let response = client
    .audit_logs()
    .create_event_with_options(
        CreateEventParams::new(workos::AuditLogEventIngestion {
            organization_id: "org_01EHZNVPK3SFK441A1RGBFSHRT".into(),
            event,
        }),
        Some(&opts),
    )
    .await?;
```

> [!NOTE]
> The WorkOS API currently honors `Idempotency-Key` only on the [Create Audit Log Event](https://workos.com/docs/reference/audit-logs/event) endpoint (`POST /audit_logs/events`). Other endpoints accept the header but do not deduplicate requests, so retrying any other mutation with the same key can still create a duplicate.

### Request strategies

For finer-grained control, attach a `RequestStrategy` to override the client's default retry behavior on a single call:

```rust
use workos::{RequestOptions, RequestStrategy};

// Send exactly once, regardless of `max_retries`:
let opts = RequestOptions::new().strategy(RequestStrategy::Once);

// Make a request retry-eligible by attaching an idempotency key (the key
// is also sent as the `Idempotency-Key` header; the server only
// deduplicates on audit log event creation — see the note above):
let opts = RequestOptions::new()
    .strategy(RequestStrategy::Idempotent("ik_42".into()));

// Custom retry budget with jitter:
let opts = RequestOptions::new().strategy(RequestStrategy::ExponentialBackoff {
    max_attempts: 5,
    jitter: true,
});
```

Variants: `Once`, `Idempotent(key)`, `Retry { max_attempts }`, `ExponentialBackoff { max_attempts, jitter }`.

## Errors

API errors carry structured metadata. Always log `request_id()` when reporting bugs to WorkOS:

```rust
match client.organizations().get_organization("org_missing").await {
    Ok(org) => println!("{org:?}"),
    Err(err) if err.is_not_found() => println!("not found"),
    Err(err) => {
        eprintln!(
            "API error {} (code={:?}, request_id={:?}): {}",
            err.status().unwrap_or(0),
            err.code(),
            err.request_id(),
            err.api().map(|a| a.message.as_str()).unwrap_or(""),
        );
        if let Some(after) = err.retry_after() {
            eprintln!("retry after {after:?}");
        }
    }
}
```

`err.api()` returns the full `ApiError` with the raw response headers and body for advanced debugging.

## Sensitive Fields

Fields that hold credentials or tokens — `password`, `client_secret`, `access_token`, `refresh_token`, `token`, `secret`, etc. — are typed as `workos::SecretString`. Their `Debug` representation prints `"<redacted>"`, so secrets don't leak through logs, panic messages, or error reports. Read the underlying value with `.expose()` when you genuinely need it:

```rust
let token: &str = session.access_token.expose();
```

`SecretString` serializes transparently as a JSON string, so the wire format is unchanged. Constructors that accept a sensitive parameter take `impl Into<SecretString>` — passing a `String` or `&str` works without an explicit conversion.

## Retries

The client retries `429` and `5xx` responses (plus retryable transport errors) up to `max_retries` times — default `3` — with exponential backoff and equal-jitter. The `Retry-After` header is honored when present and supersedes the computed backoff.

To preserve at-most-once semantics for state-changing calls, only safe HTTP methods (`GET`/`HEAD`/`OPTIONS`) and requests carrying an `Idempotency-Key` are auto-retried. POST/PUT/PATCH/DELETE without an idempotency key are sent exactly once. Keep in mind that the server only deduplicates on audit log event creation, so a retried mutation on any other endpoint may be applied more than once even when a key is attached.

```rust
// Disable retries entirely for this client:
let client = workos::Client::builder()
    .api_key(std::env::var("WORKOS_API_KEY").unwrap())
    .max_retries(0)
    .build();
```

When creating audit log events, pair the request with an idempotency key (or `RequestStrategy::Idempotent`) so a redelivered request is processed exactly once on the server. For other mutations, prefer `RequestStrategy::Once` (the default for unsafe methods) and handle retries at the application level, since the server does not deduplicate them.

## Auto-Paging

Every list endpoint generates a `*_auto_paging` companion that returns a `futures_util::Stream`, advancing the `after` cursor under the hood:

```rust
use futures_util::TryStreamExt;
use workos::organizations::ListOrganizationsParams;

let all: Vec<workos::Organization> = client
    .organizations()
    .list_organizations_auto_paging(ListOrganizationsParams::default())
    .try_collect()
    .await?;
```

For custom paginated flows the crate also re-exports the lower-level `auto_paginate(fetch)` helper, which drives any `(after) -> Result<Page<T>, _>` closure to exhaustion.

## Helpers

The SDK includes hand-maintained helpers for:

- AuthKit and SSO URL builders, PKCE flows, token exchange, logout, and device authorization.
- Webhook signature verification.
- Sealed session cookies.
- JWKS fetching and URL construction.
- Vault key-value operations and optional local AES-GCM encryption.
- Public PKCE-only clients for browser or mobile flows that must not hold an API key.

## HTTP Transport

The default HTTP transport is `reqwest`, gated behind the default `reqwest` feature. To use another client, share an existing request pipeline, or support environments such as WASM, disable default features and provide a `workos::transport::HttpTransport` implementation:

```toml
# Cargo.toml
workos = { version = "1", default-features = false }
```

```rust
let transport: workos::transport::SharedTransport = std::sync::Arc::new(MyTransport);

let client = workos::Client::builder()
    .api_key("sk_...")
    .transport(transport)
    .build();
```

Supported crate features:

| Feature      | Default | Description                                           |
| ------------ | ------- | ----------------------------------------------------- |
| `reqwest`    | yes     | Enables the bundled `reqwest` transport.              |
| `rustls-tls` | yes     | Uses `rustls` for TLS through `reqwest`.              |
| `native-tls` | no      | Uses the platform native TLS stack through `reqwest`. |

## More Information

- [WorkOS Docs](https://workos.com/docs)
- [API Reference](https://workos.com/docs/reference)
- [Issues](https://github.com/workos/workos-rust/issues)

## License

MIT. See [LICENSE.txt](./LICENSE.txt).
