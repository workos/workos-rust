use reqwest::{Response, StatusCode};

use crate::{WorkOsError, WorkOsResult};

pub trait ResponseExtensions {
    fn ensure_successful<E>(self) -> WorkOsResult<Self, E>
    where
        Self: Sized;
}

impl ResponseExtensions for Response {
    fn ensure_successful<E>(self) -> WorkOsResult<Self, E>
    where
        Self: Sized,
    {
        match self.error_for_status() {
            Ok(response) => Ok(response),
            Err(err) => match err.status() {
                Some(StatusCode::UNAUTHORIZED) => Err(WorkOsError::Unauthorized),
                _ => Err(WorkOsError::RequestError(err)),
            },
        }
    }
}
