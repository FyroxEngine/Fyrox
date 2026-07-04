// Asset config — the TOML schema that drives everything else.
// One .toml file = one asset family (potentially multiple letter variants).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetConfig {
    pub asset:     AssetInfo,
    pub sockets:   Option<Sockets>,
    pub render:    RenderInfo,
    pub meta:      MetaInfo,
    pub character: Option<CharacterInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetInfo {
    /// Primary asset type token: CAVE_ENTRANCE, SKY_PLATFORM, AIRSHIP_HULL, etc.
    #[serde(rename = "type")]
    pub asset_type: String,

    /// Domain bucket: arch / mech / bio / char / prop
    pub domain:   Option<String>,

    /// Zone: CAVE, SKY, ALIEN, LUMINARITE, VOID, XYRONA, OCEAN, VOLCANIC, etc.
    pub zone:     Option<String>,

    /// Structural function: ENTRANCE, CORRIDOR, CHAMBER, HULL, DECK, SPIRE, etc.
    pub function: Option<String>,

    /// Style variant: ORGANIC, ARMORED, RUINED, PRISTINE, CRYSTALLINE, etc.
    pub variant:  Option<String>,

    /// Grid footprint: 1X1, 2X1, 2X2, 1X1X2, etc.
    pub scale:    Option<String>,

    /// Single letter for this specific variant: A, B, C, D
    pub letter:   Option<String>,

    /// Auto-generate this many lettered variants (A, B, C…)
    pub variants: Option<u8>,
}

impl AssetInfo {
    /// Derive domain from zone if not explicitly set.
    pub fn resolved_domain(&self) -> String {
        if let Some(ref d) = self.domain {
            return d.to_lowercase();
        }
        match self.zone.as_deref().unwrap_or("").to_uppercase().as_str() {
            "CAVE" | "UNDERGROUND" | "DEEP" | "VOID" | "FUNGAL" |
            "SKY" | "LUMINARITE" | "OCEAN" | "VOLCANIC"         => "arch".into(),
            "AIRSHIP" | "MECHANICAL" | "INDUSTRIAL"             => "mech".into(),
            "ALIEN" | "ORGANIC" | "JUNGLE" | "XYRONA"           => "bio".into(),
            _                                                    => "prop".into(),
        }
    }

    /// Parse "2X1" → (2, 1). Returns (1, 1) on failure.
    pub fn grid_size(&self) -> [u32; 2] {
        let s = self.scale.as_deref().unwrap_or("1X1").to_uppercase();
        let parts: Vec<&str> = s.split('X').collect();
        let w = parts.first().and_then(|p| p.parse().ok()).unwrap_or(1);
        let h = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(1);
        [w, h]
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Sockets {
    pub north: Option<String>,
    pub south: Option<String>,
    pub east:  Option<String>,
    pub west:  Option<String>,
    pub up:    Option<String>,
    pub down:  Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderInfo {
    /// TRANSPARENT, BLACK, WHITE
    pub background: Option<String>,
    /// ISW, INE, INW, ISE, FRONT, SIDE, TOP
    pub direction:  Option<String>,
    /// ISOMETRIC, PERSPECTIVE, ORTHOGRAPHIC
    pub angle:      Option<String>,
    /// PBR, UNLIT, MATCAP
    pub shader:     Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetaInfo {
    /// One of the 18 Quantum modules
    pub quantum_module: Option<String>,
    /// Resonance frequency in Hz
    pub resonance_hz:   Option<f64>,
    /// Searchable tags
    pub tags:           Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharacterInfo {
    /// T_POSE, A_POSE, IDLE, ACTION
    pub pose:    Option<String>,
    /// HUMANOID, QUADRUPED, SERPENTINE, WINGED, TENTACLED, MECHANICAL
    pub rig:     Option<String>,
    /// VENTURAN, HYDRALIS, LUMINAR, XYRONA, NEXARI, etc.
    pub faction: Option<String>,
    /// ARCHIVER, GUARD, TRADER, SCHOLAR, CREATURE, etc.
    pub role:    Option<String>,
    /// HIGH, MID, LOW
    pub lod:     Option<String>,
}

/// Load and parse a config file.
pub fn load(path: &std::path::Path) -> anyhow::Result<AssetConfig> {
    let src = std::fs::read_to_string(path)?;
    let cfg: AssetConfig = toml::from_str(&src)?;
    Ok(cfg)
}
