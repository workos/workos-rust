// @oagen-ignore-file
//! Official Rust SDK for the [WorkOS API](https://workos.com).
//!
//! The SDK is built around a single [`Client`] that exposes one accessor per
//! API surface (SSO, user management, organizations, audit logs, FGA-style
//! authorization, feature flags, vault, …). Each accessor returns a typed
//! resource API whose methods map 1:1 to WorkOS endpoints.
//!
//! # Quick start
//!
//! ```no_run
//! use workos::{Client, organizations::ListOrganizationsParams};
//!
//! # async fn run() -> Result<(), workos::Error> {
//! let client = Client::new(std::env::var("WORKOS_API_KEY").unwrap());
//!
//! let orgs = client
//!     .organizations()
//!     .list_organizations(ListOrganizationsParams::default())
//!     .await?;
//!
//! for org in orgs.data {
//!     println!("{}: {}", org.id, org.name);
//! }
//! # Ok(()) }
//! ```
//!
//! # Custom HTTP transport
//!
//! The default [`Client`] uses `reqwest`, pulled in by the `rustls-tls`
//! (default) or `native-tls` features.
//! For WASM, a shared connection pool, or custom retry/observability layers,
//! disable default features and supply your own [`transport::HttpTransport`]
//! via [`ClientBuilder::transport`].
//!
//! ```toml
//! [dependencies]
//! workos = { version = "2", default-features = false }
//! ```
//!
//! # Modules
//!
//! - [`client`] — [`Client`] and [`ClientBuilder`].
//! - [`resources`] — generated resource APIs, one per WorkOS service.
//! - [`models`] — request/response types.
//! - [`enums`] — generated enum types.
//! - [`error`] — [`Error`] and HTTP error variants.
//! - [`pagination`] — [`Page`], [`ListMetadata`], and the [`auto_paginate`] stream helper.
//! - [`transport`] — pluggable [`transport::HttpTransport`] trait.
//! - [`helpers`] — hand-rolled extras: AuthKit, JWKS, PKCE, sessions, vault crypto, webhook verification.

pub mod client;
pub mod enums;
pub mod error;
pub mod helpers;
pub mod models;
pub mod pagination;
pub mod query;
pub mod resources;
pub mod resources_api;
pub mod secret;
pub mod transport;

pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL, RequestOptions, RequestStrategy};
pub use enums::*;
pub use error::{ApiError, Error};
pub use helpers::*;
pub use models::*;
pub use pagination::{ListMetadata, Page, auto_paginate};
pub use resources::*;
pub use secret::SecretString;
