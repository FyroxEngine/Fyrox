// Capsule — the atomic unit of the Quantum Ecosystem.
//
// Every entity, asset, character, event, or object at rest is a Capsule.
// Capsules are the leaf nodes of the Genesis Container hierarchy.
// They carry a BDna (deterministic identity), a WireType (what kind of data),
// a raw payload (bincode-encoded), and heraldric positions (birth and current).
//
// Nothing without provenance enters a sealed Genesis Container.
// Every Capsule must have a lineage_hash before it can be archived.

use crate::heraldry::HeraldricPosition;
use chrono::{DateTime, Utc};
use myth_wire::{BDna, Blake3Hash, MythId, WireType};
use serde::{Deserialize, Serialize};

/// The atomic unit. One thing that can be stored, identified, and routed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    /// Unique identity for this specific capsule instance.
    pub id: MythId,

    /// The container this capsule belongs to.
    pub parent_container_id: MythId,

    /// Human-readable name (not unique — MythId is the canonical key).
    pub name: String,

    /// What kind of signal this capsule carries. Determines payload schema.
    pub wire_type: WireType,

    /// Bincode-encoded payload. Schema defined by `wire_type`.
    pub payload: Vec<u8>,

    /// BLAKE3 hash of the payload — the lineage fingerprint.
    /// Must be set before a capsule can enter a sealed Genesis Container.
    pub lineage_hash: Option<Blake3Hash>,

    /// When this capsule was created (wall clock, not simulation tick).
    pub created_at: DateTime<Utc>,

    /// Free-form searchable tags.
    pub tags: Vec<String>,

    /// Resonance frequency of this node in the Harmonic Tensor Graph (Hz).
    pub resonance_hz: f64,

    /// Heraldric position at birth (immutable after creation).
    pub heraldic_birth: HeraldricPosition,

    /// Current heraldric position (may change through narrative events).
    pub heraldic_current: HeraldricPosition,

    /// The 64-position deterministic identity signature.
    pub bdna: BDna,
}

impl Capsule {
    /// Create a new Capsule with auto-generated id, BDna from the name seed,
    /// and birth heraldric position set to prime.
    pub fn new(
        parent_container_id: MythId,
        name: impl Into<String>,
        wire_type: WireType,
        payload: Vec<u8>,
    ) -> Self {
        let name = name.into();
        let id = MythId::new();
        let bdna = BDna::from_seed(name.as_bytes());
        let birth = HeraldricPosition::prime();
        Self {
            id,
            parent_container_id,
            name,
            wire_type,
            payload,
            lineage_hash: None,
            created_at: Utc::now(),
            tags: Vec::new(),
            resonance_hz: 440.0, // A4 — the canonical default resonance
            heraldic_birth: birth.clone(),
            heraldic_current: birth,
            bdna,
        }
    }

    /// Compute and set the lineage_hash from the current payload.
    /// Call this before attempting to archive the capsule.
    pub fn seal_lineage(&mut self) {
        self.lineage_hash = Some(Blake3Hash::of(&self.payload));
    }

    /// True if this capsule has a lineage_hash set (provenance established).
    pub fn has_provenance(&self) -> bool {
        self.lineage_hash.is_some()
    }

    /// Raw byte length of the payload.
    pub fn byte_len(&self) -> usize {
        self.payload.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_wire::MythId;

    #[test]
    fn new_capsule_has_no_lineage() {
        let c = Capsule::new(
            MythId::new(),
            "Test Actor",
            WireType::Behavioral,
            vec![1, 2, 3],
        );
        assert!(!c.has_provenance());
    }

    #[test]
    fn seal_lineage_sets_hash() {
        let mut c = Capsule::new(
            MythId::new(),
            "Test Actor",
            WireType::Behavioral,
            vec![1, 2, 3],
        );
        c.seal_lineage();
        assert!(c.has_provenance());
    }

    #[test]
    fn bdna_is_64_bits() {
        let c = Capsule::new(MythId::new(), "World Builder", WireType::Spatial, vec![]);
        assert_eq!(c.bdna.bits().len(), 64);
    }
}
