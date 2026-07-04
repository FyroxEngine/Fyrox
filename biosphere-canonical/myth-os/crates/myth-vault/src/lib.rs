// myth-vault — Quantum Vault, the persistence layer of the BioSpark ecosystem.
//
// Everything at rest lives here. Vault is content-addressable, BLAKE3-fingerprinted,
// deduplicated, memory-mapped, and append-auditable. It knows nothing about
// renderers, narrative, or Bevy. It speaks raw bytes + MythId.
//
// Dependency rule: myth-wire + memmap2 + serde + chrono + thiserror + tracing only.
// No tokio (Vault is sync). No Bevy. No egui. No audio.

pub mod atoms;
pub mod error;
pub mod registry;

pub use error::{VaultError, VaultResult};
pub use registry::VaultRegistry;
