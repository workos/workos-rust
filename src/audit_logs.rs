//! A module for interacting with the WorkOS Audit Logs API.

mod operations;
mod types;

pub use operations::*;
pub use types::*;

use crate::WorkOs;

/// Audit Logs.
pub struct AuditLogs<'a> {
    workos: &'a WorkOs,
}

impl<'a> AuditLogs<'a> {
    /// Returns a new [`AuditLogs`] instance for the provided WorkOS client.
    pub fn new(workos: &'a WorkOs) -> Self {
        Self { workos }
    }
}
