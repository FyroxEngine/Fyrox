// VAULT-ATOM-03: Metadata Ingestor — JSON header extraction and validation.
//
// Vault is Capsule-agnostic: it stores raw bytes and MythIds.
// This atom extracts a minimal VaultMeta record from a JSON header
// without importing any Capsule type from higher layers.

use crate::error::{VaultError, VaultResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Minimal metadata tracked by the Vault for any stored payload.
/// Higher-level fields (BDna, heraldry, wire_type) live in myth-quill's Capsule.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VaultMeta {
    pub name: String,
    pub tags: Vec<String>,
}

pub struct MetadataIngestor;

impl MetadataIngestor {
    /// Parse raw JSON bytes into a VaultMeta. Requires at least a "name" field.
    pub fn ingest(raw: &[u8]) -> VaultResult<VaultMeta> {
        let v: Value = serde_json::from_slice(raw)?;
        let name = v["name"]
            .as_str()
            .ok_or_else(|| VaultError::SchemaInvalid {
                reason: "missing 'name' field".into(),
            })?
            .to_string();

        let tags = v["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(VaultMeta { name, tags })
    }
}
