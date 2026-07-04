pub const CRATE_NAME: &str = "myth-forge";
pub const CREST: &str = "Forge";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ForgeOutputType { Actor, Item, Structure, Effect, Concept }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Ingredient {
    pub ingredient_id: String,
    pub asset_ref: Option<String>,
    pub quantity: u32,
    pub tags_required: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blueprint {
    pub blueprint_id: String,
    pub label: String,
    pub output_type: ForgeOutputType,
    pub output_archetype: String,
    pub ingredients: Vec<Ingredient>,
    pub required_skill: Option<String>,
    pub required_skill_level: u8,
    pub output_quantity: u32,
    pub success_rate: f32,          // 0.0–1.0 (1.0 = always succeeds)
    pub failure_output: Option<String>, // fallback blueprint_id on failure
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ForgeConfig {
    pub blueprints: Vec<Blueprint>,
    pub stamp_identity: bool,       // brand output with bDNA lineage
    pub quality_variance: f32,      // 0.0–1.0 output quality spread
    pub crafting_time_multiplier: f32,
    pub auto_discover: bool,        // agents can discover blueprints by experimentation
    pub discovery_probability: f32,
    pub max_blueprints: u16,
    pub allow_cursed_items: bool,   // items with negative effects
    pub rarity_tiers: Vec<String>,  // ["common","uncommon","rare","epic","legendary","mythic"]
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self {
            blueprints: vec![],
            stamp_identity: true,
            quality_variance: 0.1,
            crafting_time_multiplier: 1.0,
            auto_discover: false,
            discovery_probability: 0.05,
            max_blueprints: 256,
            allow_cursed_items: true,
            rarity_tiers: vec!["common","uncommon","rare","epic","legendary","mythic"]
                .into_iter().map(|s| s.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ForgeResult {
    pub blueprint_id: String,
    pub success: bool,
    pub output_asset_ref: Option<String>,
    pub quality: f32,
    pub stamped_identity: Option<String>,   // bDNA hex
}
