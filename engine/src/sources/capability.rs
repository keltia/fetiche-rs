//! All about `Capability`.

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Represents the various levels of data access capability.
///
/// The `Capability` enum is used to specify the type of operations
/// a component or entity is allowed to perform. This enables fine-grained
/// control over source or actor behavior based on their permissions.
///
/// # Variants
///
/// - **None**: Indicates no specific capabilities; the entity can exist but cannot perform any operation.
/// - **Fetch**: Allows fetching data from a source.
/// - **Read**: Grants permission to read data but does not necessarily allow fetching.
/// - **Stream**: Enables streaming data from a source.
///
#[derive(Clone, Copy, Debug, Default, Deserialize, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Capability {
    #[default]
    None = 0,
    Fetch = 1,
    Read = 2,
    Stream = 3,
    Invalid = 255,
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Capability::None => "none",
            Capability::Read => "read",
            Capability::Fetch => "fetch",
            Capability::Stream => "stream",
            Capability::Invalid => "invalid",
        };
        write!(f, "{s}")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_capability_default() {
        let capability: Capability = Default::default();
        assert_eq!(capability, Capability::None);
    }

    #[test]
    fn test_capability_ordering() {
        assert!(Capability::None < Capability::Fetch);
        assert!(Capability::Fetch < Capability::Read);
        assert!(Capability::Read < Capability::Stream);
    }

    #[test]
    fn test_capability_display() {
        assert_eq!(Capability::None.to_string(), "none");
        assert_eq!(Capability::Fetch.to_string(), "fetch");
        assert_eq!(Capability::Read.to_string(), "read");
        assert_eq!(Capability::Stream.to_string(), "stream");
    }

    #[test]
    fn test_capability_serialization() {
        let capability = Capability::Read;
        let serialized = serde_json::to_string(&capability).unwrap();
        assert_eq!(serialized, "\"read\"");
    }

    #[test]
    fn test_capability_deserialization() {
        let json = "\"stream\"";
        let capability: Capability = serde_json::from_str(json).unwrap();
        assert_eq!(capability, Capability::Stream);
    }

    #[test]
    fn test_capability_invalid_deserialization() {
        let json = "\"invalid\"";
        let result: Result<Capability, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
