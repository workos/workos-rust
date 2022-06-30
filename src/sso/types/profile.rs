use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::organizations::OrganizationId;
use crate::{KnownOrUnknown, RawAttributes};

use super::{ConnectionId, ConnectionType};

/// The ID of a [`Profile`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProfileId(String);

impl Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ProfileId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ProfileId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// [WorkOS Docs: Profile](https://workos.com/docs/reference/sso/profile)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// The ID of the profile.
    pub id: ProfileId,

    /// The ID of the connection to which the profile belongs.
    pub connection_id: ConnectionId,

    /// The ID of the organization in which the connection resides.
    pub organization_id: Option<OrganizationId>,

    /// The type of connection used to authenticate the user.
    pub connection_type: KnownOrUnknown<ConnectionType, String>,

    /// The unique identifier of the user assigned by the Identity Provider.
    pub idp_id: String,

    /// The user's email address.
    pub email: String,

    /// The user's first name.
    pub first_name: Option<String>,

    /// The user's last name.
    pub last_name: Option<String>,

    /// The raw attributes received from the Identity Provider.
    pub raw_attributes: RawAttributes,
}
