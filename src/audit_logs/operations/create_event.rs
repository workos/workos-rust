use async_trait::async_trait;
use serde::Serialize;
use thiserror::Error;

use crate::audit_logs::AuditLogs;
use crate::organizations::OrganizationId;
use crate::{ResponseExt, WorkOsError, WorkOsResult};

/// The parameters for [`CreateEvent`].
#[derive(Debug, Serialize)]
pub struct CreateEventParams<'a> {
    pub organization_id: &'a OrganizationId,
}

/// An error returned from [`CreateEvent`].
#[derive(Debug, Error)]
pub enum CreateEventError {}

impl From<CreateEventError> for WorkOsError<CreateEventError> {
    fn from(err: CreateEventError) -> Self {
        Self::Operation(err)
    }
}

#[async_trait]
pub trait CreateEvent {
    async fn create_event(
        &self,
        params: &CreateEventParams<'_>,
    ) -> WorkOsResult<(), CreateEventError>;
}

#[async_trait]
impl<'a> CreateEvent for AuditLogs<'a> {
    async fn create_event(
        &self,
        params: &CreateEventParams<'_>,
    ) -> WorkOsResult<(), CreateEventError> {
        let url = self.workos.base_url().join("logs")?;
        self.workos
            .client()
            .post(url)
            .bearer_auth(self.workos.key())
            .send()
            .await?
            .handle_unauthorized_or_generic_error()?;

        Ok(())
    }
}
