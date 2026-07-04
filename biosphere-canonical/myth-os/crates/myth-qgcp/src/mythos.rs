use serde::{Deserialize, Serialize};
use myth_wire::WireType;
use crate::{Container, QgcpError, MAX_CONTAINERS};

/// A high-level domain or arc — holds up to 16 Containers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythosModule {
    /// Format: `MYTH-<idx>` e.g. `MYTH-01`
    pub id: String,

    pub name: String,

    /// Optional display color (hex string e.g. `#4A90E2`)
    pub color: Option<String>,

    /// Which department owns this module e.g. "Narrative", "Core", "Atlas"
    pub department: Option<String>,

    pub description: Option<String>,

    /// The primary wire type this module emits.
    pub primary_wire_out: WireType,

    /// Whether this module has been implemented or is a shell placeholder.
    pub built: bool,

    pub containers: Vec<Container>,
}

impl MythosModule {
    pub fn new(id: impl Into<String>, name: impl Into<String>, wire_out: WireType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color: None,
            department: None,
            description: None,
            primary_wire_out: wire_out,
            built: false,
            containers: Vec::new(),
        }
    }

    pub fn add_container(&mut self, container: Container) -> Result<(), QgcpError> {
        if self.containers.len() >= MAX_CONTAINERS {
            return Err(QgcpError::ContainerOverflow(self.containers.len() + 1));
        }
        self.containers.push(container);
        Ok(())
    }

    pub fn verify_integrity(&self) -> bool {
        self.containers.iter().all(|c| c.verify_integrity())
    }
}
