use serde::{Deserialize, Serialize};
use myth_wire::{BDna, WireType};

/// The irreducible atomic unit of a Genesis Container.
///
/// A Capsule carries a typed payload and a lineage hash that proves
/// its content has not drifted since it was written. Wire type tells
/// any reader what kind of data is in the payload without inspecting it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    /// Hex ID — format: `0x<blake3-prefix>` of the payload bytes.
    pub id: String,

    /// Human-readable name for this capsule.
    pub name: String,

    /// What kind of signal this capsule carries.
    pub wire_type: WireType,

    /// The actual data — JSON for human readability, bincode for transport.
    pub payload: serde_json::Value,

    /// BLAKE3 hash of `payload` bytes. Used by the Gauntlet to detect drift.
    pub lineage_hash: String,

    /// BDna signature of the author/agent that created this capsule.
    pub author_dna: Option<BDna>,

    /// Freeform tags for search and filtering.
    pub tags: Vec<String>,

    /// Unix timestamp (seconds) when this capsule was created.
    pub created_at: i64,
}

impl Capsule {
    /// Create a new capsule, computing the lineage hash automatically.
    pub fn new(
        name: impl Into<String>,
        wire_type: WireType,
        payload: serde_json::Value,
        tags: Vec<String>,
        author_dna: Option<BDna>,
    ) -> Self {
        let payload_bytes = payload.to_string();
        let hash = blake3::hash(payload_bytes.as_bytes());
        let lineage_hash = hex::encode(hash.as_bytes());
        let id = format!("0x{}", &lineage_hash[..16]);
        let created_at = chrono::Utc::now().timestamp();

        Self {
            id,
            name: name.into(),
            wire_type,
            payload,
            lineage_hash,
            author_dna,
            tags,
            created_at,
        }
    }

    /// Verify the payload has not changed since the capsule was created.
    pub fn verify(&self) -> bool {
        let payload_bytes = self.payload.to_string();
        let hash = blake3::hash(payload_bytes.as_bytes());
        let expected = hex::encode(hash.as_bytes());
        expected == self.lineage_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_passes_on_fresh_capsule() {
        let c = Capsule::new(
            "test",
            WireType::Data,
            serde_json::json!({ "key": "value" }),
            vec![],
            None,
        );
        assert!(c.verify());
    }

    #[test]
    fn verify_fails_after_tampering() {
        let mut c = Capsule::new(
            "test",
            WireType::Narrative,
            serde_json::json!({ "lore": "original" }),
            vec![],
            None,
        );
        c.payload = serde_json::json!({ "lore": "tampered" });
        assert!(!c.verify());
    }
}
