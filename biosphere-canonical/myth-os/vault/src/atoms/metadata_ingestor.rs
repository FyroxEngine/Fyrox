// VAULT-ATOM-03: Metadata Ingestor
use crate::error::{VaultError, VaultResult};
use mythos::capsule::CapsuleMeta;
use serde_json::Value;

pub struct MetadataIngestor;

impl MetadataIngestor {
    /// Parse raw JSON header bytes into a validated CapsuleMeta.
    pub fn ingest(raw: &[u8]) -> VaultResult<CapsuleMeta> {
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

        Ok(CapsuleMeta {
            name,
            tags,
            origin_vault: None,
            parent: None,
        })
    }
}
