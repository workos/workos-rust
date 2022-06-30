use reqwest::{Response, StatusCode};

use crate::{WorkOsError, WorkOsResult};

pub trait ResponseExt
where
    Self: Sized,
{
    /// Handles an unauthorized error from the WorkOS API by converting it into a
    /// [`WorkOsError::Unauthorized`] response.
    fn handle_unauthorized_error<E>(self) -> WorkOsResult<Self, E>;

    /// Handles a generic error from the WorkOS API by converting it into a
    /// [`WorkOsError::RequestError`] response.
    fn handle_generic_error<E>(self) -> WorkOsResult<Self, E>;

    /// Handles an unauthorized or generic error from the WorkOS API.
    fn handle_unauthorized_or_generic_error<E>(self) -> WorkOsResult<Self, E>;
}

impl ResponseExt for Response {
    fn handle_unauthorized_error<E>(self) -> WorkOsResult<Self, E> {
        if self.status() == StatusCode::UNAUTHORIZED {
            Err(WorkOsError::Unauthorized)
        } else {
            Ok(self)
        }
    }

    fn handle_generic_error<E>(self) -> WorkOsResult<Self, E> {
        match self.error_for_status() {
            Ok(response) => Ok(response),
            Err(err) => Err(WorkOsError::RequestError(err)),
        }
    }

    fn handle_unauthorized_or_generic_error<E>(self) -> WorkOsResult<Self, E> {
        self.handle_unauthorized_error()?.handle_generic_error()
    }
}
