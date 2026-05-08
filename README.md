# WorkOS Rust Library

The WorkOS library for Rust provides convenient access to the WorkOS API from applications written in Rust. The crate is `async`-first (built on `tokio`), uses `reqwest` by default, and ships with a hand-maintained helper layer for webhooks, sessions, AuthKit, SSO, JWKS, PKCE, and Vault local crypto.

## Documentation

See the [WorkOS API Reference](https://workos.com/docs/reference) for usage examples and the [crate docs on docs.rs](https://docs.rs/workos) for type-level details.

## Installation

Requires Rust `1.85+` (edition 2024).

```bash
cargo add workos
```

`reqwest` with `rustls-tls` is enabled by default. To swap TLS backends or supply your own transport, see [Crate Features](#crate-features) below.

## Quick Start

```rust
use workos::{Client, ListOrganizationsParams};

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

For a quick API-key-only client, use `Client::new("sk_...")`.

## Configuration

`Client::builder()` returns a fluent builder:

| Method            | Description                                                     |
| ----------------- | --------------------------------------------------------------- |
| `.api_key(_)`     | WorkOS secret key (`sk_...`); required for authenticated calls. |
| `.client_id(_)`   | WorkOS Client ID; required for AuthKit/SSO/JWKS helpers.        |
| `.base_url(_)`    | Override the API host. Defaults to `https://api.workos.com`.    |
| `.timeout(_)`     | Per-request timeout. Defaults to 30 seconds.                    |
| `.max_retries(_)` | Retry budget for `429`/`5xx` responses. Defaults to 3.          |
| `.user_agent(_)`  | Override the `User-Agent` header.                               |
| `.transport(_)`   | Plug in a custom `HttpTransport` (disables `reqwest`).          |

The client is `Clone`-cheap (it wraps an `Arc`) — share one instance across handlers and tasks.

## Available Services

All API resources are accessed through accessor methods on the `Client`:

| Accessor                                    | Description                                  |
| ------------------------------------------- | -------------------------------------------- |
| `client.admin_portal()`                     | Admin Portal link generation                 |
| `client.api_keys()`                         | Organization API key management              |
| `client.audit_logs()`                       | Audit log events, exports, and retention     |
| `client.authorization()`                    | Fine-grained authorization (FGA) and RBAC    |
| `client.connect()`                          | Connect (OAuth and M2M) application registry |
| `client.directory_sync()`                   | Directory Sync (directories, users, groups)  |
| `client.events()`                           | Event stream                                 |
| `client.feature_flags()`                    | Feature flag management and evaluation       |
| `client.groups()`                           | Organization-scoped groups and membership    |
| `client.multi_factor_auth()`                | MFA factors and challenges                   |
| `client.organization_domains()`             | Organization domain verification             |
| `client.organizations()`                    | Organization CRUD                            |
| `client.pipes()`                            | Data integrations and connected accounts     |
| `client.radar()`                            | Radar attempts and lists                     |
| `client.sso()`                              | Single Sign-On connections and profiles      |
| `client.user_management()`                  | Users, invitations, sessions, auth methods   |
| `client.webhooks()`                         | Webhook endpoint management                  |
| `client.widgets()`                          | Widget session tokens                        |
| `client.passwordless()` _(non-spec helper)_ | Passwordless (magic-link) sessions           |
| `client.vault()` _(non-spec helper)_        | Vault KV + client-side AES-GCM crypto        |

### Authentication flows

`POST /user_management/authenticate` is exposed as eight strongly-typed wrappers — pick the one that matches your grant type so you only pass the params it actually needs:

| Method                                                                 | Grant type                          |
| ---------------------------------------------------------------------- | ----------------------------------- |
| `client.user_management().authenticate_with_password(_)`               | Email + password                    |
| `client.user_management().authenticate_with_code(_)`                   | Authorization code                  |
| `client.user_management().authenticate_with_refresh_token(_)`          | Refresh token                       |
| `client.user_management().authenticate_with_magic_auth(_)`             | Magic Auth code                     |
| `client.user_management().authenticate_with_email_verification(_)`     | Email verification code             |
| `client.user_management().authenticate_with_totp(_)`                   | MFA / TOTP                          |
| `client.user_management().authenticate_with_organization_selection(_)` | Org selection during multi-org auth |
| `client.user_management().authenticate_with_device_code(_)`            | Device code (CLI)                   |

Each wrapper sets `grant_type` automatically and pulls `client_id` / `client_secret` from the client config.

## Pagination

List endpoints return a `Page<T>` with `data` and `list_metadata.{before,after}`. For full-collection iteration, drive the cursor yourself or use the `auto_paginate` helper, which yields a `futures::Stream` of items:

```rust
use futures::TryStreamExt;
use workos::{Client, ListOrganizationsParams, auto_paginate};

let stream = auto_paginate(|after| {
    let client = client.clone();
    async move {
        client
            .organizations()
            .list_organizations(ListOrganizationsParams { after, ..Default::default() })
            .await
    }
});

let all: Vec<_> = stream.try_collect().await?;
```

## Error Handling

Every API call returns `Result<_, workos::Error>`. The error type carries enough detail to branch on HTTP status, transport failures, decode errors, and helper-specific failure modes:

| Variant                                | When                                                                |
| -------------------------------------- | ------------------------------------------------------------------- |
| `Error::Api { status, code, message }` | The API responded with a non-2xx status.                            |
| `Error::Network(_)`                    | The configured `HttpTransport` failed (DNS, TLS, connect timeout…). |
| `Error::Decode(_)`                     | Response body could not be deserialized.                            |
| `Error::Builder(_)`                    | Caller supplied an invalid configuration or parameter.              |
| `Error::Webhook(_)`                    | Webhook signature verification failed.                              |
| `Error::Session(_)`                    | Sealed-session encrypt/decrypt failed.                              |
| `Error::VaultCrypto(_)`                | Vault local AES-GCM failed.                                         |
| `Error::Jwt(_)`                        | JWT or JWKS verification failed.                                    |
| `Error::Crypto(_)`                     | A primitive (HMAC/AES/PKCE) failed.                                 |

Convenience predicates: `err.is_unauthorized()`, `err.is_not_found()`, `err.is_rate_limited()`, `err.is_server_error()`.

```rust
match client.organizations().get_organization("org_doesnotexist", Default::default()).await {
    Ok(org) => println!("{}", org.name),
    Err(e) if e.is_not_found() => println!("organization not found"),
    Err(e) => return Err(e),
}
```

`429` and `5xx` responses, as well as retryable transport errors, are retried automatically with exponential backoff up to `max_retries`.

## Webhooks

Manage webhook endpoints with `client.webhooks()`. Verify incoming webhook payloads with `WebhookVerifier`:

```rust
use workos::WebhookVerifier;

let verifier = WebhookVerifier::new(std::env::var("WORKOS_WEBHOOK_SECRET").unwrap());

let body = verifier.verify_payload(&sig_header, &raw_body)?;

// Or, deserialize directly into a typed event envelope:
let event = verifier.construct_event(&sig_header, &raw_body)?;
println!("{} ({})", event.event, event.id);
```

Use `.with_tolerance(Duration::from_secs(60))` to tighten the timestamp window. Low-level primitives are also exported for custom flows:

- `compute_webhook_signature(secret, timestamp, body) -> String`
- `parse_webhook_signature_header(header) -> Result<(timestamp, signature), Error>`

## Session Management

Authenticate and refresh user sessions stored in sealed cookies.

The `cookie_password` MUST be a 64-character hex string that decodes to exactly 32 bytes (256 bits) of high-entropy key material. Generate one with `openssl rand -hex 32` and load it from a secret store.

```rust
let session = client.session(sealed_cookie, cookie_password);

let state = session.authenticate();
if state.authenticated {
    println!("user: {:?}", state.user);
    println!("org:  {:?}", state.organization_id);
} else if state.needs_refresh {
    let refreshed = session.refresh(Default::default()).await?;
    if refreshed.authenticated {
        // Set refreshed.sealed_session as the new cookie value.
    }
}

// Build the AuthKit logout URL for the current session:
let logout_url = session.logout_url(Some("https://example.com/"))?;
```

Standalone helpers are also available without instantiating a manager: `seal_session`, `unseal_session`, plus generic `seal::<T>` / `unseal::<T>` for arbitrary serializable payloads.

## Vault

Store and retrieve encrypted key-value data, with optional client-side encryption:

```rust
use workos::{KeyContext, VaultCreateObjectParams};

let kc = KeyContext { kind: "user".into(), environment_id: env_id.clone() };

// KV operations
let metadata = client.vault().create_object(VaultCreateObjectParams {
    name: "api-token".into(),
    value: "secret-value".into(),
    key_context: Some(kc.clone()),
    description: None,
}).await?;

let obj = client.vault().read_object(&metadata.id).await?;

// Client-side AES-256-GCM encryption (opaque to the WorkOS API):
let result = client.vault().encrypt("sensitive data", kc, "").await?;
let plaintext = client.vault().decrypt(&result.encrypted_data, "").await?;
```

## AuthKit / SSO

URL builders (synchronous, no HTTP) and PKCE flows are exposed via dedicated helpers:

```rust
use workos::AuthKitAuthorizationUrlParams;

// Plain authorization URL
let url = client.authkit().authorization_url(AuthKitAuthorizationUrlParams {
    redirect_uri: "https://example.com/callback".into(),
    ..Default::default()
})?;

// Auto-PKCE: generates code_verifier / code_challenge / state for you
let pkce = client.authkit().pkce_authorization_url(AuthKitAuthorizationUrlParams {
    redirect_uri: "https://example.com/callback".into(),
    ..Default::default()
})?;
// Redirect the user to pkce.url, persist pkce.code_verifier and pkce.state.

// Token exchange after the user returns
let auth = client.authkit().pkce_code_exchange(workos::AuthKitPkceCodeExchangeParams {
    code: returned_code,
    code_verifier: pkce.code_verifier,
}).await?;
```

`client.sso_helpers()` mirrors the same pattern for SSO flows: `authorization_url`, `pkce_authorization_url`, `pkce_code_exchange`, `logout_url`.

`client.authkit()` also exposes `start_device_authorization()` and `poll_device_code()` for CLI device-code flows.

### Public (PKCE-only) clients

For browser, mobile, or other public clients that should not hold an API key, use `PublicClient` — it exposes only the helper surface that's safe without a secret:

```rust
use workos::PublicClient;

let public = PublicClient::new(std::env::var("WORKOS_CLIENT_ID").unwrap());

let pkce = public.authkit_authorization_url(workos::AuthKitAuthorizationUrlParams {
    redirect_uri: "https://example.com/callback".into(),
    ..Default::default()
})?;
```

## JWKS

```rust
let jwks = client.jwks();             // bound to client.client_id()
let url = jwks.jwks_url();            // for cache configuration
let key_set = jwks.fetch().await?;    // pre-warm or share across verifiers
```

Or build the URL directly: `workos::jwks_url(base_url, client_id)`.

## PKCE Utilities

Standalone PKCE helpers, when you need to drive the flow yourself:

```rust
let pair = workos::generate_pkce_pair()?;
// pair.code_verifier, pair.code_challenge, pair.code_challenge_method
```

Also: `generate_code_verifier(length)`, `generate_code_challenge(verifier)`.

## Crate Features

| Feature      | Default | Description                                                                 |
| ------------ | ------- | --------------------------------------------------------------------------- |
| `reqwest`    | yes     | Bundles the built-in `reqwest` HTTP transport. Disable to plug in your own. |
| `rustls-tls` | yes     | Use `rustls` for TLS via reqwest.                                           |
| `native-tls` | no      | Use the platform native TLS stack via reqwest.                              |

To use a custom transport (e.g. for WASM, or to share a pipeline with the rest of your app), implement `workos::transport::HttpTransport` and pass it to `ClientBuilder::transport`:

```toml
# Cargo.toml
workos = { version = "...", default-features = false }
```

```rust
let transport: workos::transport::SharedTransport = std::sync::Arc::new(MyTransport);
let client = workos::Client::builder()
    .api_key("sk_…")
    .transport(transport)
    .build();
```

## Crate Layout

- `workos::Client` / `workos::ClientBuilder` — entry points.
- `workos::resources` — generated API resource types and accessors (one module per service).
- `workos::models` / `workos::enums` — generated request/response types.
- `workos::helpers` — hand-maintained helpers (webhooks, sessions, AuthKit, SSO, PKCE, JWKS, Vault local crypto, public client).
- `workos::pagination` — `Page<T>`, `ListMetadata`, and `auto_paginate`.
- `workos::transport` — `HttpTransport` trait and the bundled `ReqwestTransport`.
- `workos::Error` — the unified error type.

Most types are re-exported at the crate root, so `use workos::Client;` and `use workos::Organization;` both work without reaching into submodules.

## More Information

- [WorkOS Docs](https://workos.com/docs)
- [API Reference](https://workos.com/docs/reference)
- [Issues](https://github.com/workos/workos-rust/issues)
- [Changelog](./CHANGELOG.md)

## License

MIT — see [LICENSE.txt](./LICENSE.txt).
