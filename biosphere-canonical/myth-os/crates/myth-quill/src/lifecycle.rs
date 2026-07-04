// Lifecycle and capacity types — the Octave Capacity Law applied to all containers.
//
// Every Genesis Container, Mythos Container, and Container declares a growth mode
// at seeding. Sealed containers are structurally immutable.
// See genesis-container skill for the full Octave Capacity Law.

use serde::{Deserialize, Serialize};

/// Whether a container was declared fixed-size or grows dynamically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrowthMode {
    /// Fixed (VHD-style): octave declared at seeding, never changes.
    /// Best for world containers where the boundary is the creative constraint.
    Fixed,
    /// Dynamic (VHDX-style): starts at octave 1, expands as content is added.
    /// Best for character/actor containers that grow organically.
    Dynamic,
}

/// The five states of any container's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleState {
    /// Questions being answered — the container is being configured.
    Seeding,
    /// Open and accepting writes.
    Active,
    /// Structurally immutable. Payload updates allowed but hierarchy is frozen.
    Sealed,
    /// Read-only, discoverable in search.
    Archived,
    /// Scheduled for purge — no new reads should be served.
    Deprecated,
}

/// Capacity tracking for any container level.
/// `2.pow(current_octave)` is always the current maximum child count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityMetadata {
    pub growth_mode: GrowthMode,

    /// For Fixed mode: the declared ceiling. Never changes after seeding.
    pub declared_octave: Option<u8>,

    /// For Dynamic mode: the estimated ceiling, adjustable until sealed.
    pub estimated_ceiling_octave: Option<u8>,

    /// The actual current octave (expands automatically in Dynamic mode).
    pub current_octave: u8,

    /// Set at seal time. None while Active.
    pub sealed_octave: Option<u8>,

    /// How many times the estimated ceiling was revised before sealing.
    /// Metadata for world health reasoning by the agent layer.
    pub ceiling_adjustments: u32,

    /// Current number of direct children.
    pub child_count: u32,
}

impl CapacityMetadata {
    /// Maximum children at the current octave.
    pub fn max_children(&self) -> u32 {
        2u32.pow(self.current_octave as u32)
    }

    /// Fraction of current capacity consumed (0.0 – 1.0).
    pub fn utilization(&self) -> f64 {
        self.child_count as f64 / self.max_children() as f64
    }

    /// True if no more children can be added at the current octave.
    /// Dynamic containers will expand; Fixed containers are at their ceiling.
    pub fn is_full(&self) -> bool {
        self.child_count >= self.max_children()
    }

    /// Standard octave-4 capacity metadata for most containers.
    pub fn default_fixed() -> Self {
        Self {
            growth_mode: GrowthMode::Fixed,
            declared_octave: Some(4),
            estimated_ceiling_octave: None,
            current_octave: 4,
            sealed_octave: None,
            ceiling_adjustments: 0,
            child_count: 0,
        }
    }

    /// Dynamic container starting at octave 1.
    pub fn default_dynamic() -> Self {
        Self {
            growth_mode: GrowthMode::Dynamic,
            declared_octave: None,
            estimated_ceiling_octave: Some(4),
            current_octave: 1,
            sealed_octave: None,
            ceiling_adjustments: 0,
            child_count: 0,
        }
    }
}
