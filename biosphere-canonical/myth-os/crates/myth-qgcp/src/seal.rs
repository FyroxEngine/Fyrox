use serde::{Deserialize, Serialize};

/// The immutability lock for a Genesis Container.
///
/// Once sealed, the container's content is frozen. The lineage_hash covers
/// the entire serialized container — any drift is detectable.
/// This is the Gauntlet checkpoint built into the data structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealBlock {
    pub seal_id: String,

    /// BLAKE3 hash of the entire serialized GenesisContainer at seal time.
    pub lineage_hash: String,

    /// Unix timestamp when the seal was applied.
    pub sealed_at: i64,

    /// Who/what applied the seal — agent BDna string or "local-architect"
    pub sealed_by: String,

    /// Human-readable note about why this was sealed.
    pub note: Option<String>,
}

impl SealBlock {
    pub fn new(lineage_hash: String, sealed_by: impl Into<String>) -> Self {
        Self {
            seal_id: format!("seal_{}", uuid::Uuid::new_v4()),
            lineage_hash,
            sealed_at: chrono::Utc::now().timestamp(),
            sealed_by: sealed_by.into(),
            note: None,
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}
