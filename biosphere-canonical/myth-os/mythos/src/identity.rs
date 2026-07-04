use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A cryptographically unique identity for every asset, actor, or capsule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MythId(Uuid);

impl MythId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a MythId from an existing UUID (e.g. a vault's persisted id).
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Extract the underlying UUID (for bridging to uuid-based APIs).
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for MythId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MythId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A BLAKE3 fingerprint of a capsule's raw bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Blake3Hash(pub [u8; 32]);

impl std::fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
