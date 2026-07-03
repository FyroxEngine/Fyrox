use serde::{Deserialize, Serialize};

/// Serializable configuration for this module.
///
/// Loaded at startup, saved when modified. Lives outside the Genesis Container
/// hierarchy — this is meta/operational config, not world data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    pub enabled: bool,
    // TODO: add your fields
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}
