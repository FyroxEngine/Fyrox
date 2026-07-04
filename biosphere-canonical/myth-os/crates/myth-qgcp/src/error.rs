use thiserror::Error;
use crate::{MAX_CAPSULES, MAX_CONTAINERS, MAX_MYTHOS};

#[derive(Debug, Error)]
pub enum QgcpError {
    #[error("Mythos count {0} exceeds the capacity law limit of {MAX_MYTHOS}")]
    MythosOverflow(usize),

    #[error("Container count {0} exceeds the capacity law limit of {MAX_CONTAINERS}")]
    ContainerOverflow(usize),

    #[error("Capsule count {0} exceeds the capacity law limit of {MAX_CAPSULES}")]
    CapsuleOverflow(usize),

    #[error("Genesis is sealed — no mutations permitted")]
    Sealed,

    #[error("Lineage hash mismatch: expected {expected}, got {actual}")]
    LineageMismatch { expected: String, actual: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),
}
