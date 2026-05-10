# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0]

This release is a ground-up rebuild of the SDK. Every resource module is now
generated from the WorkOS OpenAPI spec by `oagen`; only a thin async client,
the helper layer, and the pagination/transport plumbing are hand-maintained.

### Added

- Async, builder-based `Client` with configurable timeout, retry budget, base
  URL, and pluggable transport (`reqwest` by default; `rustls-tls` and
  `native-tls` are exposed as crate features).
- Generated resource APIs covering Organizations, User Management, SSO,
  Directory Sync, Audit Logs, Authorization (FGA), Vault, Webhooks, Events,
  API Keys, Admin Portal, Connect, Feature Flags, Groups, Multi-factor Auth,
  Pipes, Radar, and Widgets.
- `RequestOptions` with `idempotency_key(...)` and `header(...)` setters; each
  generated method now has a companion `*_with_options(..., Some(&opts))`.
- Structured `ApiError` carrying `status`, `code`, `message`, `request_id`,
  `Retry-After`, full headers, and the raw response body. `Error` exposes
  `request_id()`, `code()`, `retry_after()`, plus `is_unauthorized()`,
  `is_not_found()`, `is_rate_limited()`, and `is_server_error()` predicates.
- Cursor-based auto-pagination: every list endpoint generates a
  `*_auto_paging(...)` method returning `impl futures_util::Stream`. The
  shared `auto_paginate(fetch)` helper is also re-exported for custom flows.
- Hand-maintained helper layer for AuthKit, SSO URL builders, PKCE flows,
  webhook signature verification, sealed sessions, JWKS, Vault local crypto,
  Passwordless, and a public PKCE-only client.
- Path parameters are percent-encoded as URL segments before interpolation.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` is enforced in CI;
  `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
  run on every change. Rust `1.88` (edition 2024) is pinned via
  `rust-toolchain.toml`.

### Changed

- The crate is now async-first and depends on `tokio`. Synchronous wrappers
  from earlier `0.x` releases are no longer provided.

## [0.2.0] - 2022-07-14

### Added

- Added `organization_id` to `DirectoryUser`s and `DirectoryGroup`s ([#84](https://github.com/workos/workos-rust/pull/84))

## [0.1.1] - 2022-07-11

### Changed

- Updated the endpoints used for `ChallengeFactor` and `VerifyChallenge` operations ([#81](https://github.com/workos/workos-rust/pull/81))
- Changed project status to "experimental" ([#82](https://github.com/workos/workos-rust/pull/82))

## [0.1.0] - 2022-07-01

### Added

- Initial release

[unreleased]: https://github.com/workos/workos-rust/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/workos/workos-rust/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/workos/workos-rust/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/workos/workos-rust/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/workos/workos-rust/releases/tag/66a4c78...v0.1.0
