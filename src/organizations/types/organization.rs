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
    /// The ID of the organization.
    pub id: OrganizationId,

    /// The name of the organization.
    pub name: String,

    /// Whether the connections within this organization should allow profiles
    /// that do not have a domain that is present in the set of the organization's
    /// user email domains.
    ///
    /// See [here](https://workos.com/docs/sso/guide/frequently-asked-questions#allow-profiles-outside-organization)
    /// for more details.
    pub allow_profiles_outside_organization: bool,

    /// The list of user email domains for the organization.
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

/// [WorkOS Docs: Organization Domain](https://workos.com/docs/reference/organization-domain)
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationDomain {
    /// The ID of the organization domain.
    pub id: OrganizationDomainId,

    /// The domain.
    pub domain: String,
}
