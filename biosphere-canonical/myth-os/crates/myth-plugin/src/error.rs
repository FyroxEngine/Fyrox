use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin '{0}' not found in registry")]
    NotFound(String),

    #[error("Addon target plugin '{0}' is not registered")]
    TargetNotFound(String),

    #[error("Plugin '{0}' is already registered")]
    AlreadyRegistered(String),

    #[error("Plugin attach failed: {0}")]
    AttachFailed(String),

    #[error("Plugin detach failed: {0}")]
    DetachFailed(String),

    #[error("Packet processing error in '{plugin}': {reason}")]
    ProcessError { plugin: String, reason: String },

    #[error("Vault error: {0}")]
    Vault(#[from] myth_vault::VaultError),
}

pub type PluginResult<T> = Result<T, PluginError>;
