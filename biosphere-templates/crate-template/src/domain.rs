use serde::{Deserialize, Serialize};
use std::fmt;

// ── Core type ──────────────────────────────────────────────────────────────

/// DESCRIBE_THIS_TYPE: what it represents in the Quantum Genesis hierarchy.
///
/// Capacity Law reminder:
///   Genesis (≤16 Mythos) → Mythos (≤16 Containers) → Container (≤16 Capsules)
///
/// Every entity must have:
///   - A UUID id
///   - A human-readable name
///   - A B-DNA record (provenance / lineage)
///   - A Lifecycle state (Seeding → Active → Sealed → Archived → Deprecated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleType {
    pub id: String,
    pub name: String,
    // TODO: add your fields
}

impl ExampleType {
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
        }
    }
}

impl fmt::Display for ExampleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

// ── Heraldry classification hint ───────────────────────────────────────────
//
// Which level does this type live at?
//
//   Level 1 — Genesis   → Seal (Greater / Lesser)
//   Level 2 — Mythos    → Crest (Core, Atlas, Vault, Mythos, Codex, Loom,
//                                 Composer, Forge, Order, Mind, Soul + ≤5 custom)
//   Level 3 — Container → Glyph | Device | Emblem
//   Level 4 — Capsule   → Trait | Mark | Token | Sigil
//
// Set heraldry via fyrox_biosphere::heraldry::SymbolicType when registering
// with the container hierarchy.

// ── Wire Type hint ─────────────────────────────────────────────────────────
//
// Which wire type carries data to/from this type?
//
//   DAT  CTL  AUD  NAR  TMP  AGT  VIS  SPA  BHV  SOC  ENR  IDN  EVT  AST  MET  LGC
//
// DAT is the universal fallback. Choose the most specific type that applies.
// Wire connections are declared via fyrox_biosphere::wire::WirePort.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_unique_id() {
        let a = ExampleType::new("Alpha".into());
        let b = ExampleType::new("Beta".into());
        assert_ne!(a.id, b.id);
    }
}
