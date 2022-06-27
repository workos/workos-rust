use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

/// The timestamps for an object.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Timestamps {
    /// The timestamp indicating when the object was created.
    pub created_at: DateTime<FixedOffset>,

    /// The timestamp indicating when the object was last updated.
    pub updated_at: DateTime<FixedOffset>,
}
