// Container hierarchy — the structural nesting of the Quantum Ecosystem.
//
// Genesis Container → Mythos Container → Container → Capsule
//
// Each level has its own capacity (governed by the Octave Capacity Law),
// lifecycle state, and resonance frequency.
// See genesis-container skill for the full spec.

use crate::capsule::Capsule;
use crate::lifecycle::{CapacityMetadata, LifecycleState};
use myth_wire::MythId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Container (Level 2) ──────────────────────────────────────────────────────

/// Level 2: a sub-collection within a Mythos Container.
/// Holds Capsules (level 3 — the leaves).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: MythId,
    pub parent_mythos_id: MythId,
    pub name: String,
    pub capsules: Vec<Capsule>,
    pub capacity: CapacityMetadata,
    pub lifecycle: LifecycleState,
    /// Resonance frequency in Hz.
    pub resonance_hz: f64,
    /// Arbitrary domain metadata (genre, era, element, etc.).
    pub metadata: HashMap<String, String>,
}

impl Container {
    pub fn new(parent_mythos_id: MythId, name: impl Into<String>) -> Self {
        Self {
            id: MythId::new(),
            parent_mythos_id,
            name: name.into(),
            capsules: Vec::new(),
            capacity: CapacityMetadata::default_fixed(),
            lifecycle: LifecycleState::Active,
            resonance_hz: 440.0,
            metadata: HashMap::new(),
        }
    }

    /// Add a Capsule. Returns Err if the container is full (fixed mode)
    /// or sealed.
    pub fn add(&mut self, capsule: Capsule) -> Result<(), &'static str> {
        if self.lifecycle == LifecycleState::Sealed {
            return Err("container is sealed");
        }
        if self.capacity.is_full() {
            return Err("container is at capacity");
        }
        self.capsules.push(capsule);
        self.capacity.child_count += 1;
        Ok(())
    }

    pub fn seal(&mut self) {
        self.lifecycle = LifecycleState::Sealed;
        self.capacity.sealed_octave = Some(self.capacity.current_octave);
    }
}

// ── MythosContainer (Level 1) ────────────────────────────────────────────────

/// Level 1: a major domain — arc, collection, album, module, or region.
/// Holds Containers (level 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythosContainer {
    pub id: MythId,
    pub parent_genesis_id: MythId,
    pub name: String,
    /// Optional heraldic crest identifier.
    pub crest: Option<String>,
    pub containers: Vec<Container>,
    pub capacity: CapacityMetadata,
    pub lifecycle: LifecycleState,
    pub resonance_hz: f64,
}

impl MythosContainer {
    pub fn new(parent_genesis_id: MythId, name: impl Into<String>) -> Self {
        Self {
            id: MythId::new(),
            parent_genesis_id,
            name: name.into(),
            crest: None,
            containers: Vec::new(),
            capacity: CapacityMetadata::default_fixed(),
            lifecycle: LifecycleState::Active,
            resonance_hz: 440.0,
        }
    }

    pub fn add_container(&mut self, container: Container) -> Result<(), &'static str> {
        if self.lifecycle == LifecycleState::Sealed {
            return Err("mythos container is sealed");
        }
        if self.capacity.is_full() {
            return Err("mythos container is at capacity");
        }
        self.containers.push(container);
        self.capacity.child_count += 1;
        Ok(())
    }

    pub fn seal(&mut self) {
        self.lifecycle = LifecycleState::Sealed;
        self.capacity.sealed_octave = Some(self.capacity.current_octave);
    }
}
