//! Plugin Registry — the gatekeeper between Plugin Foundry and the Vault.
//!
//! # Separation of concerns
//!
//! Plugin Foundry (separate app) → submits .wasm → Registry certifies →
//! stamps Heraldry Glyph → indexes in manifest → Vault loads from manifest.
//!
//! The Vault never touches unsigned WASM. The Foundry never touches live world state.
//! The Registry is the clean boundary between the two environments.

pub mod manifest;
pub mod certificate;
pub mod error;

pub use manifest::{PluginManifest, PluginEntry, PluginStatus};
pub use certificate::{PluginCertificate, certify};
pub use error::RegistryError;

use std::path::{Path, PathBuf};

/// The Plugin Registry. Attach to myth-os, not to any Vault.
/// One registry per myth-os installation; shared across all Vaults.
pub struct PluginRegistry {
    /// Root directory: `~/.myth-os/plugins/` or `<project>/plugins/`
    root: PathBuf,
    manifest: PluginManifest,
}

impl PluginRegistry {
    /// Open (or create) a registry at the given root directory.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, RegistryError> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        std::fs::create_dir_all(root.join("wasm"))?;
        std::fs::create_dir_all(root.join("sandbox"))?;

        let manifest = PluginManifest::load_or_create(root.join("manifest.json"))?;
        Ok(Self { root, manifest })
    }

    /// Submit a .wasm file from Plugin Foundry for certification.
    /// Runs the sandbox health check, stamps heraldry, indexes the entry.
    /// Returns the certificate on success — Foundry displays this to the user.
    pub fn submit(
        &mut self,
        wasm_bytes: &[u8],
        meta: SubmitMeta,
    ) -> Result<PluginCertificate, RegistryError> {
        // 1. Hash the bytes — this is the plugin's immutable identity
        let hash = blake3::hash(wasm_bytes);
        let hash_hex = hex::encode(hash.as_bytes());

        // 2. Reject if already certified (idempotent submit)
        if self.manifest.find_by_hash(&hash_hex).is_some() {
            return Err(RegistryError::AlreadyCertified(hash_hex));
        }

        // 3. Sandbox validation (stub — myth-wasm-host does the real check)
        validate_wasm_bytes(wasm_bytes)?;

        // 4. Write to the wasm store
        let wasm_path = self.root.join("wasm").join(format!("{hash_hex}.wasm"));
        std::fs::write(&wasm_path, wasm_bytes)?;

        // 5. Stamp certificate
        let cert = certify(&meta, &hash_hex);

        // 6. Index in manifest
        let entry = PluginEntry {
            id:           cert.plugin_id.clone(),
            name:         meta.name.clone(),
            version:      meta.version.clone(),
            heraldry:     cert.heraldry.clone(),
            hash:         hash_hex,
            wasm_path:    wasm_path.to_string_lossy().into_owned(),
            status:       PluginStatus::Certified,
            certified_at: cert.issued_at.clone(),
            author:       meta.author.clone(),
        };
        self.manifest.add(entry);
        self.manifest.save(self.root.join("manifest.json"))?;

        Ok(cert)
    }

    /// Revoke a plugin by id. Vaults will refuse to load it on next boot.
    pub fn revoke(&mut self, plugin_id: &str) -> Result<(), RegistryError> {
        self.manifest.set_status(plugin_id, PluginStatus::Revoked)?;
        self.manifest.save(self.root.join("manifest.json"))?;
        Ok(())
    }

    /// List all certified (non-revoked) plugins.
    pub fn certified(&self) -> Vec<&PluginEntry> {
        self.manifest.entries_with_status(PluginStatus::Certified)
    }

    /// Look up the .wasm path for a plugin id. Vaults call this to load.
    pub fn wasm_path(&self, plugin_id: &str) -> Option<&str> {
        self.manifest
            .find_by_id(plugin_id)
            .filter(|e| e.status == PluginStatus::Certified)
            .map(|e| e.wasm_path.as_str())
    }
}

/// Metadata the Plugin Foundry sends along with the .wasm bytes.
#[derive(Debug, Clone)]
pub struct SubmitMeta {
    pub name:         String,
    pub version:      String,
    pub author:       String,
    /// Parent crest this plugin attaches under, e.g. "Atlas"
    pub parent_crest: String,
    /// Short symbol for the Glyph, e.g. "Erosion"
    pub glyph_symbol: String,
    pub description:  String,
}

/// Minimal WASM validation before sandbox execution.
/// Full execution test happens in myth-wasm-host with a timeout.
fn validate_wasm_bytes(bytes: &[u8]) -> Result<(), RegistryError> {
    // WASM magic number: \0asm
    if bytes.len() < 4 || &bytes[0..4] != b"\0asm" {
        return Err(RegistryError::InvalidWasm("missing WASM magic number".into()));
    }
    Ok(())
}
