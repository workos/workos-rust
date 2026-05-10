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

## Per-Request Options

Each generated method has a `*_with_options` companion that takes a `RequestOptions`. Use it to pass an idempotency key, additional headers, or any other per-call override:

```rust
use workos::{RequestOptions, organizations::CreateOrganizationParams};

let opts = RequestOptions::new().idempotency_key("ik_create_acme_42");

let org = client
    .organizations()
    .create_organization_with_options(
        CreateOrganizationParams::new(workos::OrganizationInput {
            name: "Acme".into(),
            ..Default::default()
        }),
        Some(&opts),
    )
    .await?;
```

Replaying a mutating request with the same idempotency key is safe; WorkOS recognises the key on the server side.

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

## Retries

The client retries `429` and `5xx` responses, plus retryable transport errors, up to `max_retries` times (default `3`) with exponential backoff. Set `0` to disable:

```rust
let client = workos::Client::builder()
    .api_key(std::env::var("WORKOS_API_KEY").unwrap())
    .max_retries(0)
    .build();
```

Pair retries with an idempotency key when the request mutates state, so a redelivered request is processed exactly once.

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
