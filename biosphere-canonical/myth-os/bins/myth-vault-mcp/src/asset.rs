//! Asset metadata — what gets stored alongside raw bytes in the vault.

use serde::{Deserialize, Serialize};

/// Every asset in the Master Vault carries this metadata envelope.
/// Stored as a sidecar JSON at the same MythId with a ".meta" suffix tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMeta {
    pub myth_id:     String,
    pub name:        String,
    pub asset_type:  AssetType,
    pub tags:        Vec<String>,
    pub description: String,
    pub author:      String,
    pub created_at:  String,
    /// Original filename — informational only
    pub filename:    String,
    /// Byte size
    pub size:        usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    /// GLB / GLTF 3D models
    Model3d,
    /// PNG / JPG / HDR / EXR textures
    Texture,
    /// WAV / MP3 / FLAC / OGG audio
    Audio,
    /// GLSL / WGSL shader source
    Shader,
    /// Certified .wasm plugin
    WasmPlugin,
    /// MOLECULE — JSON ATOM graph
    Molecule,
    /// CAPSULE data payload
    Capsule,
    /// Everything else
    Raw,
}

impl AssetType {
    pub fn from_filename(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.ends_with(".glb") || lower.ends_with(".gltf") {
            Self::Model3d
        } else if lower.ends_with(".png") || lower.ends_with(".jpg")
               || lower.ends_with(".jpeg") || lower.ends_with(".hdr")
               || lower.ends_with(".exr") || lower.ends_with(".webp") {
            Self::Texture
        } else if lower.ends_with(".wav") || lower.ends_with(".mp3")
               || lower.ends_with(".flac") || lower.ends_with(".ogg") {
            Self::Audio
        } else if lower.ends_with(".glsl") || lower.ends_with(".wgsl")
               || lower.ends_with(".vert") || lower.ends_with(".frag") {
            Self::Shader
        } else if lower.ends_with(".wasm") {
            Self::WasmPlugin
        } else if lower.ends_with(".molecule.json") {
            Self::Molecule
        } else {
            Self::Raw
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Model3d    => "3D Model",
            Self::Texture    => "Texture",
            Self::Audio      => "Audio",
            Self::Shader     => "Shader",
            Self::WasmPlugin => "WASM Plugin",
            Self::Molecule   => "Molecule",
            Self::Capsule    => "Capsule",
            Self::Raw        => "Raw",
        }
    }
}
