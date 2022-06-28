mod operations;
mod types;

pub use operations::*;
pub use types::*;

use crate::WorkOs;

/// Admin Portal.
///
/// [WorkOS Docs: Admin Portal Guide](https://workos.com/docs/admin-portal/guide)
pub struct AdminPortal<'a> {
    workos: &'a WorkOs,
}

impl<'a> AdminPortal<'a> {
    /// Returns a new [`AdminPortal`] instance for the provided WorkOS client.
    pub fn new(workos: &'a WorkOs) -> Self {
        Self { workos }
    }
}
