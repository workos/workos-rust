use serde::{Deserialize, Serialize};

/// `KnownOrUnknown` is a type that respresents either a known value ([`Known`])
/// or an unknown value ([`Unknown`]).
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KnownOrUnknown<K, U> {
    /// A known value.
    Known(K),

    /// An unknown value.
    Unknown(U),
}
