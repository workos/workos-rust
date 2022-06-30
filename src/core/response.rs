use reqwest::{Response, StatusCode};

use crate::{WorkOsError, WorkOsResult};

pub trait ResponseExtensions {
    fn handle_unauthorized<E>(self) -> WorkOsResult<Self, E>
    where
        Self: Sized;
}

impl ResponseExtensions for Response {
    fn handle_unauthorized<E>(self) -> WorkOsResult<Self, E>
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
