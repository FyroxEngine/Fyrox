use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Capsule not found: {id}")]
    NotFound { id: String },

    #[error("Integrity check failed for capsule: {id} (expected {expected}, got {actual})")]
    IntegrityViolation { id: String, expected: String, actual: String },

    #[error("Deduplication reference count underflow for: {id}")]
    RefCountUnderflow { id: String },

    #[error("Schema validation failed: {reason}")]
    SchemaInvalid { reason: String },

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Vault is locked (read-only mode active)")]
    Locked,
}

pub type VaultResult<T> = Result<T, VaultError>;
