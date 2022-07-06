use std::collections::HashMap;
use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub action: String,

    pub version: u32,

    pub occurred_at: Timestamp,

    pub actor: Actor,

    pub targets: Vec<Target>,

    pub context: HashMap<String, Value>,

    pub metadata: HashMap<String, Value>,
}

/// The ID of an [`Actor`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ActorId(String);

impl Display for ActorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ActorId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ActorId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub id: ActorId,

    pub name: String,

    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub id: String,

    pub r#type: String,
}

