use serde::{Deserialize, Serialize};

/// The type of a [`Directory`](crate::directory_sync::Directory).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectoryType {
    /// Azure AD SCIM v2.0.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/azure-ad-scim)
    #[serde(rename = "azure scim v2.0")]
    AzureScimV2_0,

    /// BambooHR.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/bamboo-hr)
    #[serde(rename = "bamboohr")]
    BambooHr,

    /// Breathe HR.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/breathe-hr)
    #[serde(rename = "breathe hr")]
    BreatheHr,

    /// CyberArk SCIM v2.0.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/cyberark-scim)
    #[serde(rename = "cyberark scim v2.0")]
    CyberArkScimV2_0,

    /// Generic SCIM v1.1.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/scim-v1-1)
    #[serde(rename = "generic scim v1.1")]
    GenericScimV1_1,

    /// Generic SCIM v2.0.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/scim-v2-0)
    #[serde(rename = "generic scim v2.0")]
    GenericScimV2_0,

    /// Google Workspace.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/google-workspace)
    #[serde(rename = "gsuite directory")]
    GoogleWorkspace,

    /// Hibob.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/hibob)
    #[serde(rename = "hibob")]
    Hibob,

    /// JumpCloud SCIM v2.0.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/jumpcloud-scim)
    #[serde(rename = "jump cloud scim v2.0")]
    JumpCloudScimV2_0,

    /// Okta SCIM v1.1.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/okta-scim-v1-1)
    #[serde(rename = "okta scim v1.1")]
    OktaScimV1_1,

    /// Okta SCIM v2.0.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/okta-scim-v2-0)
    #[serde(rename = "okta scim v2.0")]
    OktaScimV2_0,

    /// OneLogin SCIM v2.0.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/onelogin-scim)
    #[serde(rename = "onelogin scim v2.0")]
    OneLoginScimV2_0,

    /// People HR.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/people-hr)
    #[serde(rename = "people hr")]
    PeopleHr,

    /// PingFederate SCIM v2.0.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/pingfederate-scim)
    #[serde(rename = "pingfederate scim v2.0")]
    PingFederateScimV2_0,

    /// Rippling.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/rippling)
    #[serde(rename = "rippling")]
    Rippling,

    /// Workday.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/workday)
    #[serde(rename = "workday")]
    Workday,
}
