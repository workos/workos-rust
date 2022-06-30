use reqwest::{Response, StatusCode};

use crate::{WorkOsError, WorkOsResult};

pub trait ResponseExtensions
where
    Self: Sized,
{
    fn handle_unauthorized<E>(self) -> WorkOsResult<Self, E>;
}

impl ResponseExtensions for Response {
    fn handle_unauthorized<E>(self) -> WorkOsResult<Self, E> {
        if self.status() == StatusCode::UNAUTHORIZED {
            Err(WorkOsError::Unauthorized)
        } else {
            Ok(self)
        }
    }
}
