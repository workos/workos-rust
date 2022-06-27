//! The official SDK for interacting with the [WorkOS](https://workos.com) API.

#![warn(missing_docs)]

mod core;
mod known_or_unknown;
mod workos;

pub mod directory_sync;
pub mod organizations;
pub mod sso;

pub use crate::core::*;
pub use crate::workos::*;
pub use known_or_unknown::*;
