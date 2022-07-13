use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::audit_logs::{AuditLogs, Event};
use crate::organizations::OrganizationId;
use crate::{ResponseExt, WorkOsError, WorkOsResult};

/// The parameters for [`CreateEvent`].
#[derive(Debug, Serialize)]
pub struct CreateEventParams<'a> {
    pub organization_id: &'a OrganizationId,

    pub event: &'a Event,
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
        let url = self.workos.base_url().join("audit_logs/events")?;
        let response = self
            .workos
            .client()
            .post(url)
            .bearer_auth(self.workos.key())
            .json(&params)
            .send()
            .await?
            .handle_unauthorized_error()?;

        match response.error_for_status_ref() {
            Ok(response) => Ok(()),
            Err(err) => {
                #[derive(Debug, serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct JsonSchemaError {
                    pub instance_path: String,
                    pub schema_path: String,
                    pub keyword: String,
                    pub params: HashMap<String, Value>,
                    pub message: String,
                }

                #[derive(Debug, serde::Deserialize)]
                struct ErrorMessage {
                    pub message: String,
                    pub errors: Vec<JsonSchemaError>,
                }

                let raw_body = response.text().await?;
                let raw_body = dbg!(raw_body);

                let message =
                    serde_json::from_str::<ErrorMessage>(&raw_body).expect("failed to parse JSON");
                let message = dbg!(message);

                Err(WorkOsError::RequestError(err))
            }
        }
    }
}
