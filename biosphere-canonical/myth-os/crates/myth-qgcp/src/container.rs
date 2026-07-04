use serde::{Deserialize, Serialize};
use myth_wire::WireType;
use crate::{Capsule, QgcpError, MAX_CAPSULES};

/// A scene, module, or location — holds up to 16 Capsules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    /// Format: `CONT-<mythos-idx>-<container-idx>` e.g. `CONT-01-03`
    pub id: String,

    pub name: String,

    /// The primary wire type this container outputs.
    pub wire_out: WireType,

    pub description: Option<String>,

    pub capsules: Vec<Capsule>,
}

impl Container {
    pub fn new(id: impl Into<String>, name: impl Into<String>, wire_out: WireType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            wire_out,
            description: None,
            capsules: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn add_capsule(&mut self, capsule: Capsule) -> Result<(), QgcpError> {
        if self.capsules.len() >= MAX_CAPSULES {
            return Err(QgcpError::CapsuleOverflow(self.capsules.len() + 1));
        }
        self.capsules.push(capsule);
        Ok(())
    }

    /// Verify all capsules in this container have intact lineage hashes.
    pub fn verify_integrity(&self) -> bool {
        self.capsules.iter().all(|c| c.verify())
    }
}
