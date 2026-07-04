// ActorContainer — a named group of up to 16 Cells in an ActorGenesis.
//
// Mirrors the Container type but is scoped to actors.
// Uses Cells (not Capsules) as its leaf unit.

use serde::{Deserialize, Serialize};
use myth_wire::WireType;
use crate::cell::Cell;

/// Maximum Cells per ActorContainer — mirrors MAX_CAPSULES.
pub const MAX_CELLS: usize = 16;

/// A named capability group within an ActorGenesis.
///
/// Actors have up to 16 ActorContainers (mirroring the 16-container law).
/// Each ActorContainer holds up to 16 Cells.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorContainer {
    /// Identifier, e.g. "ACONT-00" through "ACONT-15".
    pub id: String,

    /// Human-readable group name, e.g. "Combat Actions", "Memory Shards".
    pub name: String,

    /// The primary wire type produced by this container's cells.
    pub wire_out: WireType,

    pub description: Option<String>,

    /// The capability cells belonging to this container (0–16).
    pub cells: Vec<Cell>,
}

impl ActorContainer {
    pub fn new(id: impl Into<String>, name: impl Into<String>, wire_out: WireType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            wire_out,
            description: None,
            cells: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn add_cell(&mut self, cell: Cell) -> Result<(), crate::error::QgcpError> {
        if self.cells.len() >= MAX_CELLS {
            return Err(crate::error::QgcpError::MythosOverflow(self.cells.len() + 1));
        }
        self.cells.push(cell);
        Ok(())
    }
}
