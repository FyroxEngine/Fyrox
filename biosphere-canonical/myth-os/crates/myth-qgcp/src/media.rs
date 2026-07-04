use serde::{Deserialize, Serialize};
use crate::{QgcpError, SealBlock};

/// The type of binary asset carried by an AssetRef.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Audio,
    Image,
    Video,
    Model,    // GLB / GLTF
    Texture,  // PBR maps, normals, roughness, etc.
    Skybox,
    Font,
    Other(String),
}

/// A reference to a binary asset in the media bundle.
///
/// AssetRef is a pointer, not the data itself. The actual bytes are
/// stored on disk relative to the `.mediagenesis` file or in a packed archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRef {
    /// Stable ID — same asset across versions of the bundle.
    pub asset_id: String,

    pub name: String,

    pub media_type: MediaType,

    /// Path relative to the bundle root directory.
    pub path: String,

    /// Byte size of the asset on disk. 0 if not yet measured.
    pub size_bytes: u64,

    /// BLAKE3 checksum of the raw asset bytes.
    pub blake3_checksum: String,

    /// Optional wire type this asset serves (e.g. AUD for samples, SPA for models).
    pub wire_hint: Option<String>,

    /// Tags for filtering — biome, faction, module, etc.
    pub tags: Vec<String>,
}

impl AssetRef {
    pub fn new(
        name: impl Into<String>,
        media_type: MediaType,
        path: impl Into<String>,
    ) -> Self {
        let name = name.into();
        let id = format!("asset_{}", blake3::hash(name.as_bytes()).to_hex().as_str()[..12].to_string());
        Self {
            asset_id: id,
            name,
            media_type,
            path: path.into(),
            size_bytes: 0,
            blake3_checksum: String::new(),
            wire_hint: None,
            tags: Vec::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_wire_hint(mut self, hint: impl Into<String>) -> Self {
        self.wire_hint = Some(hint.into());
        self
    }
}

/// The media-layer Genesis Container — the "DVD" to WorldGenesis's "CD".
///
/// MediaGenesis carries all the binary assets the client Theater needs
/// to render and play back a world: audio samples, images, GLB models,
/// PBR textures, skyboxes, and any other file-based media.
///
/// The headless server and client agents never load this. Only the
/// client-side Theater adapters (Bevy, CPAL, egui) consume it.
///
/// MediaGenesis always references a companion WorldGenesis by `world_genesis_id`.
/// They are shipped together in a world directory but loaded independently.
///
/// File extension: `.mediagenesis`
/// Short alias:    `MediaGen`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaGenesis {
    pub genesis_id: String,

    /// The WorldGenesis this bundle accompanies.
    pub world_genesis_id: String,

    pub name: String,

    pub description: Option<String>,

    /// All asset references in this bundle, unordered.
    /// Consumers filter by `media_type` or `wire_hint`.
    pub assets: Vec<AssetRef>,

    /// `draft` | `active` | `sealed`
    pub lifecycle: String,

    pub sealed: bool,

    pub seal: Option<SealBlock>,

    pub created_at: i64,

    pub schema_version: String,
}

impl MediaGenesis {
    pub fn new(
        world_genesis_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            genesis_id: format!("media_{}", uuid::Uuid::new_v4().simple()),
            world_genesis_id: world_genesis_id.into(),
            name: name.into(),
            description: None,
            assets: Vec::new(),
            lifecycle: "draft".into(),
            sealed: false,
            seal: None,
            created_at: chrono::Utc::now().timestamp(),
            schema_version: "qgcp-v1.0".into(),
        }
    }

    pub fn add_asset(&mut self, asset: AssetRef) -> Result<(), QgcpError> {
        if self.sealed {
            return Err(QgcpError::Sealed);
        }
        self.assets.push(asset);
        Ok(())
    }

    /// All assets of a given media type.
    pub fn assets_of_type(&self, media_type: &MediaType) -> Vec<&AssetRef> {
        self.assets.iter().filter(|a| &a.media_type == media_type).collect()
    }

    /// All assets tagged with a given wire hint (e.g. "AUD", "SPA").
    pub fn assets_for_wire(&self, wire_code: &str) -> Vec<&AssetRef> {
        self.assets.iter()
            .filter(|a| a.wire_hint.as_deref() == Some(wire_code))
            .collect()
    }

    pub fn seal(&mut self, sealed_by: impl Into<String>) -> Result<&SealBlock, QgcpError> {
        if self.sealed {
            return Err(QgcpError::Sealed);
        }
        let content = serde_json::to_string(&self.assets)?;
        let hash = blake3::hash(content.as_bytes());
        let lineage_hash = hex::encode(hash.as_bytes());

        self.lifecycle = "sealed".into();
        self.sealed = true;
        self.seal = Some(SealBlock::new(lineage_hash, sealed_by));
        Ok(self.seal.as_ref().unwrap())
    }

    pub fn to_json(&self) -> Result<String, QgcpError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(json: &str) -> Result<Self, QgcpError> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_genesis_id_prefixed() {
        let m = MediaGenesis::new("world_abc", "Kasmir-Delta Media");
        assert!(m.genesis_id.starts_with("media_"));
    }

    #[test]
    fn add_and_filter_assets() {
        let mut m = MediaGenesis::new("world_abc", "Test Bundle");
        m.add_asset(
            AssetRef::new("Cymatic Theme", MediaType::Audio, "audio/cymatic_theme.ogg")
                .with_wire_hint("AUD")
        ).unwrap();
        m.add_asset(
            AssetRef::new("ChromaSeraph", MediaType::Model, "models/chroma_seraph.glb")
                .with_wire_hint("SPA")
        ).unwrap();

        assert_eq!(m.assets_of_type(&MediaType::Audio).len(), 1);
        assert_eq!(m.assets_for_wire("SPA").len(), 1);
        assert_eq!(m.assets.len(), 2);
    }

    #[test]
    fn seal_prevents_mutation() {
        let mut m = MediaGenesis::new("world_abc", "Test Bundle");
        m.seal("local-architect").unwrap();
        let asset = AssetRef::new("Late Addition", MediaType::Image, "img/late.png");
        assert!(matches!(m.add_asset(asset), Err(QgcpError::Sealed)));
    }
}
