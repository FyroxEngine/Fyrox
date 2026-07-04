// Cell — atomic capability unit inside an ActorGenesis.
//
// A Cell is a mix of runtime ATOM + CAPSULE scoped to an Actor.
// Where CAPSULEs use Glyphs for routing, CELLs use Sigils.
// CELLs enable actors to DO things: cast spells, pick locks, erupt.

use serde::{Deserialize, Serialize};
use myth_wire::{Sigil, WireType};

/// One atomic capability unit belonging to an Actor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    /// Unique identifier within the ActorContainer, e.g. "CELL-07".
    pub cell_id: String,

    /// Human-readable capability name, e.g. "Pyroclastic Blast".
    pub name: String,

    /// Sigil that identifies and governs this capability's routing and cost.
    pub sigil: Sigil,

    /// The wire type of data this cell produces when activated.
    pub wire_type: WireType,

    /// Runtime payload — parameters, state, or configuration for this cell.
    pub payload: serde_json::Value,

    /// Content-addressed lineage hash (blake3 hex) for audit / replay.
    pub lineage_hash: String,

    /// Optional reference to a runtime ATOM node that implements this cell's action.
    /// `None` means the cell is purely data-driven (CAPSULE-style).
    pub action_atom: Option<String>,

    pub tags: Vec<String>,

    pub created_at: i64,
}

impl Cell {
    pub fn new(
        cell_id: impl Into<String>,
        name: impl Into<String>,
        sigil: Sigil,
        wire_type: WireType,
    ) -> Self {
        Self {
            cell_id: cell_id.into(),
            name: name.into(),
            sigil,
            wire_type,
            payload: serde_json::Value::Null,
            lineage_hash: String::new(),
            action_atom: None,
            tags: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_action_atom(mut self, atom_id: impl Into<String>) -> Self {
        self.action_atom = Some(atom_id.into());
        self
    }
}
