pub const CRATE_NAME: &str = "myth-animus";
pub const CREST: &str = "Animus";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LodStrategy {
    Distance,
    ScreenSize,
    Manual,
}

impl Default for LodStrategy {
    fn default() -> Self { LodStrategy::Distance }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BodyType {
    Humanoid,
    Quadruped,
    Avian,
    Aquatic,
    Serpentine,
    Insectoid,
    Celestial,
    Abstract,
}

impl Default for BodyType {
    fn default() -> Self { BodyType::Humanoid }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LodLevel {
    pub distance: f32,
    pub triangle_budget: u32,
    pub asset_suffix: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MorphTarget {
    pub target_id: String,
    pub label: String,
    pub default_weight: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimusConfig {
    pub default_body_type: BodyType,
    pub lod_strategy: LodStrategy,
    pub lod_levels: Vec<LodLevel>,
    pub max_morph_targets: u8,
    pub procedural_variation_seed: u64,
    pub procedural_variation_strength: f32,
    pub texture_atlas_resolution: u32,
    pub allow_runtime_retexture: bool,
    pub skeleton_rig_standard: String,
    pub morph_targets: Vec<MorphTarget>,
}

impl Default for AnimusConfig {
    fn default() -> Self {
        Self {
            default_body_type: BodyType::Humanoid,
            lod_strategy: LodStrategy::Distance,
            lod_levels: vec![
                LodLevel { distance: 0.0,   triangle_budget: 10000, asset_suffix: "".into() },
                LodLevel { distance: 50.0,  triangle_budget: 3000,  asset_suffix: "_lod1".into() },
                LodLevel { distance: 150.0, triangle_budget: 500,   asset_suffix: "_lod2".into() },
            ],
            max_morph_targets: 8,
            procedural_variation_seed: 0,
            procedural_variation_strength: 0.1,
            texture_atlas_resolution: 2048,
            allow_runtime_retexture: true,
            skeleton_rig_standard: "humanoid_v1".into(),
            morph_targets: vec![],
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityMeshState {
    pub entity_id: String,
    pub asset_ref: String,
    pub active_lod: u8,
    pub morph_weights: Vec<(String, f32)>,
    pub tint_color: Option<[f32; 4]>,
}
