//! The official SDK for interacting with the [WorkOS](https://workos.com) API.

#![warn(missing_docs)]

mod core;
mod known_or_unknown;
mod serialization;
mod workos;

pub mod admin_portal;
pub mod directory_sync;
pub mod mfa;
pub mod organizations;
pub mod passwordless;
pub mod sso;

pub use crate::core::*;
pub use crate::workos::*;
pub use known_or_unknown::*;
