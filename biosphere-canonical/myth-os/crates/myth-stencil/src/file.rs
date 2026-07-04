use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::node::PanelNode;

/// A complete stencil file — the layout tree plus metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StencilFile {
    pub name:    String,
    pub version: u32,
    pub root:    PanelNode,
}

impl StencilFile {
    pub fn new(name: impl Into<String>, root: PanelNode) -> Self {
        Self { name: name.into(), version: 1, root }
    }

    /// Save as a binary `.stencil` file (bincode).
    pub fn save(&self, path: &Path) -> Result<(), StencilError> {
        let bytes = bincode::serialize(self).map_err(StencilError::Encode)?;
        std::fs::write(path, bytes).map_err(StencilError::Io)
    }

    /// Load from a `.stencil` file.
    pub fn load(path: &Path) -> Result<Self, StencilError> {
        let bytes = std::fs::read(path).map_err(StencilError::Io)?;
        bincode::deserialize(&bytes).map_err(StencilError::Decode)
    }
}

#[derive(Debug)]
pub enum StencilError {
    Io(std::io::Error),
    Encode(bincode::Error),
    Decode(bincode::Error),
}

impl std::fmt::Display for StencilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e)     => write!(f, "stencil io: {e}"),
            Self::Encode(e) => write!(f, "stencil encode: {e}"),
            Self::Decode(e) => write!(f, "stencil decode: {e}"),
        }
    }
}

impl std::error::Error for StencilError {}
