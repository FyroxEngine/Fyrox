// Manifest generator — produces the JSON Capsule record for an asset.
// Each rendered asset gets a sidecar .json that describes it fully.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::AssetConfig;
use crate::prompt;

/// The JSON manifest written alongside each rendered asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    // Identity
    pub id:           String,
    pub name:         String,
    pub stem:         String,
    pub created_at:   String,

    // Classification
    pub domain:       String,
    pub zone:         Option<String>,
    pub function:     Option<String>,
    pub variant:      Option<String>,
    pub scale:        Option<String>,
    pub letter:       Option<String>,

    // Socket map
    pub sockets:      SocketMap,

    // Render parameters
    pub render:       RenderParams,

    // Character (only for char domain)
    pub character:    Option<CharacterParams>,

    // Meta / Quantum
    pub quantum_module: Option<String>,
    pub resonance_hz:   Option<f64>,
    pub tags:           Vec<String>,

    // Grid dimensions derived from scale token
    pub grid:         [u32; 2],

    // The full prompt string for AI generation
    pub prompt:       String,

    // File references (relative paths, filled in after render)
    pub files:        FileRefs,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SocketMap {
    pub north: Option<String>,
    pub south: Option<String>,
    pub east:  Option<String>,
    pub west:  Option<String>,
    pub up:    Option<String>,
    pub down:  Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RenderParams {
    pub background: Option<String>,
    pub direction:  Option<String>,
    pub angle:      Option<String>,
    pub shader:     Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterParams {
    pub pose:    Option<String>,
    pub rig:     Option<String>,
    pub faction: Option<String>,
    pub role:    Option<String>,
    pub lod:     Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileRefs {
    /// GLTF source model (relative to asset root)
    pub gltf:      Option<String>,
    /// Rendered sprite sheet or single PNG
    pub sprite:    Option<String>,
    /// Normal map (if applicable)
    pub normal:    Option<String>,
    /// Thumbnail for the Vault browser
    pub thumbnail: Option<String>,
}

/// Build a manifest from a config.  `letter` is `None` for the primary asset,
/// `Some("A")`, `Some("B")` etc. for multi-variant runs.
pub fn build(cfg: &AssetConfig, letter: Option<&str>) -> AssetManifest {
    let stem = prompt::build_stem(cfg, letter);
    let name = make_display_name(&stem);

    let sockets = if let Some(ref s) = cfg.sockets {
        SocketMap {
            north: s.north.clone(),
            south: s.south.clone(),
            east:  s.east.clone(),
            west:  s.west.clone(),
            up:    s.up.clone(),
            down:  s.down.clone(),
        }
    } else {
        SocketMap::default()
    };

    let character = cfg.character.as_ref().map(|ch| CharacterParams {
        pose:    ch.pose.clone(),
        rig:     ch.rig.clone(),
        faction: ch.faction.clone(),
        role:    ch.role.clone(),
        lod:     ch.lod.clone(),
    });

    // Build file refs using the same stem so names are deterministic
    let dir_prefix = prompt::build_dir(cfg);
    let dir_str    = dir_prefix.to_string_lossy().replace('\\', "/");

    let files = FileRefs {
        gltf:      Some(format!("{dir_str}/{stem}.gltf")),
        sprite:    Some(format!("{dir_str}/{stem}.png")),
        normal:    Some(format!("{dir_str}/{stem}_NRM.png")),
        thumbnail: Some(format!("{dir_str}/{stem}_THUMB.png")),
    };

    AssetManifest {
        id:         Uuid::new_v4().to_string(),
        name,
        stem:       stem.clone(),
        created_at: Utc::now().to_rfc3339(),

        domain:   cfg.asset.resolved_domain(),
        zone:     cfg.asset.zone.clone(),
        function: cfg.asset.function.clone(),
        variant:  cfg.asset.variant.clone(),
        scale:    cfg.asset.scale.clone(),
        letter:   letter.map(str::to_uppercase),

        sockets,

        render: RenderParams {
            background: cfg.render.background.clone(),
            direction:  cfg.render.direction.clone(),
            angle:      cfg.render.angle.clone(),
            shader:     cfg.render.shader.clone(),
        },

        character,

        quantum_module: cfg.meta.quantum_module.clone(),
        resonance_hz:   cfg.meta.resonance_hz,
        tags:           cfg.meta.tags.clone().unwrap_or_default(),

        grid: cfg.asset.grid_size(),

        prompt: prompt::build(cfg),

        files,
    }
}

/// Write a manifest to disk as a sidecar JSON file.
/// Returns the path written.
pub fn write(manifest: &AssetManifest, out_dir: &std::path::Path) -> Result<std::path::PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join(format!("{}.json", manifest.stem));
    let json = serde_json::to_string_pretty(manifest)?;
    std::fs::write(&path, json)?;
    Ok(path)
}

/// Write a combined catalogue JSON containing all manifests in a run.
pub fn write_catalogue(
    manifests: &[AssetManifest],
    out_dir:   &std::path::Path,
    name:      &str,
) -> Result<std::path::PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join(format!("{name}.catalogue.json"));
    let json = serde_json::to_string_pretty(manifests)?;
    std::fs::write(&path, json)?;
    Ok(path)
}

// "CAVE_ENTRANCE_ORGANIC_2X2_A" → "Cave Entrance Organic 2x2 A"
fn make_display_name(stem: &str) -> String {
    stem.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None    => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
