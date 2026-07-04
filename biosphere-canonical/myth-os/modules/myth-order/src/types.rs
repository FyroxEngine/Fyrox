pub const CRATE_NAME: &str = "myth-order";
pub const CREST: &str = "Order";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FactionAlignment {
    Lawful,
    Neutral,
    Chaotic,
}

impl Default for FactionAlignment {
    fn default() -> Self { FactionAlignment::Neutral }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GovernmentType {
    Monarchy,
    Democracy,
    Theocracy,
    Oligarchy,
    Tribal,
    Anarchy,
    Corporate,
}

impl Default for GovernmentType {
    fn default() -> Self { GovernmentType::Tribal }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Faction {
    pub faction_id: String,
    pub name: String,
    pub heraldry: String,
    pub alignment: FactionAlignment,
    pub government: GovernmentType,
    pub base_disposition: f32,
    pub economic_strength: f32,
    pub military_strength: f32,
    pub territory_ids: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationshipRule {
    pub faction_a: String,
    pub faction_b: String,
    pub base_disposition: f32,
    pub can_trade: bool,
    pub can_marry: bool,
    pub at_war: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderConfig {
    pub factions: Vec<Faction>,
    pub relationships: Vec<RelationshipRule>,
    pub reputation_decay_rate: f32,
    pub rumor_spread_rate: f32,
    pub max_factions: u8,
    pub crime_system: bool,
    pub law_enforcement_strength: f32,
    pub economy_enabled: bool,
    pub currency_name: String,
    pub social_class_count: u8,
}

impl Default for OrderConfig {
    fn default() -> Self {
        Self {
            factions: vec![],
            relationships: vec![],
            reputation_decay_rate: 0.0001,
            rumor_spread_rate: 0.3,
            max_factions: 16,
            crime_system: true,
            law_enforcement_strength: 0.5,
            economy_enabled: true,
            currency_name: "coin".into(),
            social_class_count: 4,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SocialEvent {
    pub event_id: String,
    pub event_type: String,
    pub actor_ids: Vec<String>,
    pub faction_ids: Vec<String>,
    pub reputation_delta: f32,
    pub timestamp: i64,
}
