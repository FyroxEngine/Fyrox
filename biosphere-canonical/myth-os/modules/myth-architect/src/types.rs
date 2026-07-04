pub const CRATE_NAME: &str = "myth-architect";
pub const CREST: &str = "Architect";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StructureCategory {
    Settlement,
    Fortification,
    Infrastructure,
    Sacred,
    Commercial,
    Industrial,
    Agricultural,
    Ruin,
    Natural,
    Underground,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlacementRule {
    OnFlatGround { max_slope: f32 },
    NearWater { max_distance: f32 },
    OnHighGround { min_elevation: f32 },
    MinDistanceFromOther { distance: f32 },
    WithinBiome { biome: String },
    Custom { rule_id: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StructureTemplate {
    pub template_id: String,
    pub category: StructureCategory,
    /// Path or MythId to GLB asset
    pub asset_ref: String,
    pub placement_rules: Vec<PlacementRule>,
    pub scale_range: [f32; 2],
    /// Degrees; 0 = free rotation
    pub rotation_snap: f32,
    pub population_capacity: u32,
    pub requires_biome: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArchitectConfig {
    pub structure_seed: u64,
    /// Settlements per 100 km²
    pub settlement_density: f32,
    /// 0.0–1.0 fraction of settlements that are ruined
    pub ruin_fraction: f32,
    pub road_network: bool,
    pub road_width: f32,
    pub wall_probability: f32,
    pub underground_probability: f32,
    pub templates: Vec<StructureTemplate>,
}

impl Default for ArchitectConfig {
    fn default() -> Self {
        Self {
            structure_seed: 0,
            settlement_density: 0.5,
            ruin_fraction: 0.2,
            road_network: true,
            road_width: 4.0,
            wall_probability: 0.3,
            underground_probability: 0.1,
            templates: vec![],
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlacedStructure {
    pub instance_id: String,
    pub template_id: String,
    pub position: [f32; 3],
    pub rotation: f32,
    pub scale: f32,
    pub category: StructureCategory,
}
