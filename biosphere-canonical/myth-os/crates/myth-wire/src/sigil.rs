// Sigil — routing identity for CELL capabilities.
//
// Parallel to Glyph (used by CAPSULEs), but Sigils belong to CELLs inside
// an ActorGenesis. A Sigil names an action ("FIRE-CAST", "LOCK-PICK", "ERUPT"),
// its tier, and its runtime cost parameters.

use serde::{Deserialize, Serialize};

/// Routing identity for a CELL action capability.
///
/// `symbol`     — human-readable action name, e.g. "FIRE-CAST".
/// `tier`       — 0 = passive sensor, 1–16 = action tiers (matches intelligence_tier scale).
/// `cooldown_ms`— minimum milliseconds between consecutive activations.
/// `energy_cost`— abstract energy units consumed per activation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sigil {
    pub symbol: String,
    pub tier: u8,
    pub cooldown_ms: u64,
    pub energy_cost: f32,
}

impl Sigil {
    pub fn new(symbol: impl Into<String>, tier: u8) -> Self {
        Self {
            symbol: symbol.into(),
            tier: tier.min(16),
            cooldown_ms: 0,
            energy_cost: 0.0,
        }
    }

    pub fn with_cooldown(mut self, ms: u64) -> Self {
        self.cooldown_ms = ms;
        self
    }

    pub fn with_energy_cost(mut self, cost: f32) -> Self {
        self.energy_cost = cost;
        self
    }
}
