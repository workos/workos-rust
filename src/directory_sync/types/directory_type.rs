use serde::{Deserialize, Serialize};

/// The type of a [`Directory`].
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectoryType {
    /// Bamboo HR.
    ///
    /// [WorkOS Docs: Integration Guide](https://workos.com/docs/integrations/bamboo-hr)
    #[serde(rename = "bamboohr")]
    BambooHr,
}
