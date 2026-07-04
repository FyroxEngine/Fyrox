use serde::{Deserialize, Serialize};

/// Structural layer within a subsystem (I=ingestion/base, IV=output/archival).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Layer {
    I,
    II,
    III,
    IV,
}

/// Lifecycle state shared across Vault, Core, and Genesis atoms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtomStatus {
    Uninitialized,
    Initialized,
    Running,
    Manifesting,
    Degraded,
    Halted,
}

/// Vault atom's permission to access the Core memory bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultAccess {
    None,
    Read,
    Write,
    Both,
}

/// Minimal descriptor embedded in every atom, used for registry and telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomDescriptor {
    pub id: String,
    pub layer: Layer,
    pub name: &'static str,
    pub function: &'static str,
    pub produces: &'static str,
    pub failure_mode: &'static str,
    pub status: AtomStatus,
}
