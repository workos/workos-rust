mod operations;
mod types;

pub use operations::*;
pub use types::*;

use crate::WorkOs;

pub struct Passwordless<'a> {
    workos: &'a WorkOs,
}

impl<'a> Passwordless<'a> {
    /// Returns a new [`Passwordless`] instance for the provided WorkOS client.
    pub fn new(workos: &'a WorkOs) -> Self {
        Self { workos }
    }
}
