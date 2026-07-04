#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Plugin already certified: {0}")]
    AlreadyCertified(String),
    #[error("Invalid WASM: {0}")]
    InvalidWasm(String),
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Plugin is revoked and cannot be loaded: {0}")]
    Revoked(String),
}
