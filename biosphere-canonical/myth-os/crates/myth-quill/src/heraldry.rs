// Heraldric position — the positional identity of a Capsule in the Heraldric Order.
//
// Every Capsule has two heraldric positions: the position it was born into
// (heraldic_birth — immutable) and the position it currently holds
// (heraldic_current — may change through narrative events).
//
// The full heraldry system (sigils, coats of arms, rank meanings) is defined
// in the biospark-heraldry skill (not yet written). This file defines the
// minimal structural type that Capsule and Container need to compile.
//
// DO NOT add fields here without the heraldry skill as a reference.

use serde::{Deserialize, Serialize};

/// A position in the Heraldric Order.
///
/// `rank` is the octave depth (0 = Prime — the highest order).
/// `order` is the position within that rank (0-indexed).
///
/// Full meaning of rank/order combinations is defined in the heraldry skill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeraldricPosition {
    /// Octave depth in the heraldric hierarchy. 0 = Prime (highest order).
    pub rank: u8,
    /// Position within the rank. 0-indexed.
    pub order: u32,
}

impl HeraldricPosition {
    pub fn new(rank: u8, order: u32) -> Self {
        Self { rank, order }
    }

    /// The prime position — rank 0, order 0. Assigned to Genesis-level entities.
    pub fn prime() -> Self {
        Self { rank: 0, order: 0 }
    }
}

impl std::fmt::Display for HeraldricPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "R{}:O{}", self.rank, self.order)
    }
}
