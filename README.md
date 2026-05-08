# WorkOS Rust Library

The WorkOS Rust SDK provides async access to the WorkOS API from Rust applications. It uses `tokio`, ships with a default `reqwest` HTTP transport, and includes helpers for common WorkOS flows such as AuthKit, SSO, webhooks, sessions, JWKS, PKCE, and Vault local crypto.

## Documentation

- [WorkOS API Reference](https://workos.com/docs/reference)
- [Crate docs on docs.rs](https://docs.rs/workos)
- [Changelog](./CHANGELOG.md)

## Installation

Requires Rust `1.85+` (edition 2024).

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
