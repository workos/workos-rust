use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::Timestamps;

/// The ID of an [`Organization`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OrganizationId(String);

impl Display for OrganizationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for OrganizationId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for OrganizationId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// [WorkOS Docs: Organization](https://workos.com/docs/reference/organization)
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub allow_profiles_outside_organization: bool,
    pub domains: Vec<OrganizationDomain>,

    /// The timestamps for the organization.
    #[serde(flatten)]
    pub timestamps: Timestamps,
}

/// The ID of an [`OrganizationDomain`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OrganizationDomainId(String);

impl Display for OrganizationDomainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for OrganizationDomainId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for OrganizationDomainId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationDomain {
    pub id: OrganizationDomainId,
    pub domain: String,
}
