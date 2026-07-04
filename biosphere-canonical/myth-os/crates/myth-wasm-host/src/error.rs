#[derive(Debug, thiserror::Error)]
pub enum WasmHostError {
    #[error("Wasmtime error: {0}")]
    Wasmtime(#[from] wasmtime::Error),
    #[error("Plugin not certified: {0}")]
    NotCertified(String),
    #[error("Plugin revoked: {0}")]
    Revoked(String),
    #[error("Plugin timed out (infinite loop suspected)")]
    Timeout,
    #[error("ABI error — plugin export missing: {0}")]
    MissingExport(String),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Registry error: {0}")]
    Registry(#[from] myth_plugin_registry::RegistryError),
}
