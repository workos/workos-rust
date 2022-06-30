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
        if self.status() == StatusCode::UNAUTHORIZED {
            Err(WorkOsError::Unauthorized)
        } else {
            Ok(self)
        }
    }
}
