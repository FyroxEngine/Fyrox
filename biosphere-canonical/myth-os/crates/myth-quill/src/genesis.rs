// GenesisContainer (Level 0) — the sealed universe of a BioSpark project.
//
// The Genesis Container is the root of everything. It seals the entire
// project's physics when `seal()` is called. After sealing, the hierarchy
// is structurally immutable — capsule payloads may update but no new
// Mythos Containers, Containers, or Capsules can be added.
//
// Greater Seal: one per metaverse. No parent.
// Lesser Seal: nested containers. Must declare a harmonic_ratio to parent.
//
// See genesis-container skill for full physics.

use crate::container::MythosContainer;
use crate::lifecycle::{CapacityMetadata, LifecycleState};
use myth_wire::{BDna, MythId, WireType};
use serde::{Deserialize, Serialize};

/// The sealing type of a Genesis Container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SealType {
    /// Exactly one per metaverse. No declared size. Cannot be nested.
    Greater,
    /// Every other sealed container. Can nest inside other Lesser Seals.
    Lesser,
}

/// A seed question answered during container initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedQuestion {
    pub question: String,
    pub answer: String,
}

/// Level 0: the sealed universe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisContainer {
    pub id: MythId,
    pub name: String,
    pub seal_type: SealType,

    /// The primary domain of this genesis (e.g., "narrative", "music", "software").
    pub domain: String,

    /// The seed questions and answers that defined this container at creation.
    pub seed_answers: Vec<SeedQuestion>,

    /// The Mythos Containers (level 1) that live inside this genesis.
    pub mythos: Vec<MythosContainer>,

    pub capacity: CapacityMetadata,
    pub lifecycle: LifecycleState,

    /// Wall-clock timestamp of the seal event. None while Active.
    pub sealed_at: Option<i64>,

    /// The deterministic identity of this genesis universe.
    pub bdna_signature: BDna,

    /// The 17 wire types this genesis explicitly uses.
    pub wire_types: Vec<WireType>,

    /// Base resonance frequency of this genesis universe (Hz).
    pub resonance_hz: f64,

    /// The parent seal's ID, if this is a Lesser Seal nested in another genesis.
    /// None for Greater Seals.
    pub parent_seal_id: Option<MythId>,

    /// The harmonic ratio to the parent seal's frequency.
    /// Required for Lesser Seals; None for Greater Seals.
    pub harmonic_ratio: Option<f64>,
}

impl GenesisContainer {
    /// Create a new active Genesis Container (not yet sealed).
    pub fn new(
        name: impl Into<String>,
        seal_type: SealType,
        domain: impl Into<String>,
    ) -> Self {
        let name_str: String = name.into();
        let bdna = BDna::from_seed(name_str.as_bytes());
        Self {
            id: MythId::new(),
            name: name_str,
            seal_type,
            domain: domain.into(),
            seed_answers: Vec::new(),
            mythos: Vec::new(),
            capacity: CapacityMetadata::default_dynamic(),
            lifecycle: LifecycleState::Seeding,
            sealed_at: None,
            bdna_signature: bdna,
            wire_types: Vec::new(),
            resonance_hz: 432.0, // the canonical genesis base frequency
            parent_seal_id: None,
            harmonic_ratio: None,
        }
    }

    /// Add a Mythos Container to this genesis (only while Active).
    pub fn add_mythos(&mut self, mythos: MythosContainer) -> Result<(), &'static str> {
        if self.lifecycle == LifecycleState::Sealed {
            return Err("genesis is sealed");
        }
        self.mythos.push(mythos);
        self.capacity.child_count += 1;
        Ok(())
    }

    /// Record a seed question/answer pair.
    pub fn answer(&mut self, question: impl Into<String>, answer: impl Into<String>) {
        self.seed_answers.push(SeedQuestion {
            question: question.into(),
            answer: answer.into(),
        });
    }

    /// Activate the genesis (transition from Seeding → Active).
    pub fn activate(&mut self) {
        self.lifecycle = LifecycleState::Active;
    }

    /// Seal the genesis. After this call the hierarchy is structurally immutable.
    pub fn seal(&mut self) {
        self.lifecycle = LifecycleState::Sealed;
        self.capacity.sealed_octave = Some(self.capacity.current_octave);
        self.sealed_at = Some(chrono::Utc::now().timestamp_millis());
    }

    pub fn is_sealed(&self) -> bool {
        self.lifecycle == LifecycleState::Sealed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_genesis_is_seeding() {
        let g = GenesisContainer::new("Test World", SealType::Greater, "narrative");
        assert_eq!(g.lifecycle, LifecycleState::Seeding);
        assert!(!g.is_sealed());
    }

    #[test]
    fn seal_freezes_lifecycle() {
        let mut g = GenesisContainer::new("Test World", SealType::Lesser, "narrative");
        g.activate();
        g.seal();
        assert!(g.is_sealed());
        assert!(g.sealed_at.is_some());
        assert!(g.capacity.sealed_octave.is_some());
    }

    #[test]
    fn cannot_add_mythos_after_seal() {
        let mut g = GenesisContainer::new("Test World", SealType::Greater, "narrative");
        g.activate();
        g.seal();
        let m = MythosContainer::new(g.id.clone(), "An Arc");
        assert!(g.add_mythos(m).is_err());
    }

    #[test]
    fn bdna_is_deterministic_from_name() {
        let g1 = GenesisContainer::new("Aetheria", SealType::Greater, "narrative");
        let g2 = GenesisContainer::new("Aetheria", SealType::Greater, "narrative");
        assert_eq!(g1.bdna_signature, g2.bdna_signature);
    }
}
