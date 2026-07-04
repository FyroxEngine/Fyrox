pub const CRATE_NAME: &str = "myth-instinct";
pub const CREST: &str = "Instinct";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DriveType {
    Hunger,
    Thirst,
    Rest,
    Safety,
    Belonging,
    Esteem,
    Purpose,
    Curiosity,
    Reproduction,
    Territory,
    Dominance,
    Play,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Drive {
    pub drive_type: DriveType,
    pub base_value: f32,
    pub decay_rate: f32,
    pub urgency_threshold: f32,
    pub max_value: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DecisionModel {
    UtilityBased,
    GoalOriented,
    ReactiveFSM,
    BehaviorTree,
    Custom(String),
}

impl Default for DecisionModel {
    fn default() -> Self { DecisionModel::UtilityBased }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmotionState {
    pub valence: f32,
    pub arousal: f32,
    pub dominance: f32,
    pub primary_label: String,
}

impl Default for EmotionState {
    fn default() -> Self {
        Self {
            valence: 0.0,
            arousal: 0.0,
            dominance: 0.5,
            primary_label: "neutral".into(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstinctConfig {
    pub decision_model: DecisionModel,
    pub drives: Vec<Drive>,
    pub tick_rate_ms: u64,
    pub memory_influence: f32,
    pub social_influence: f32,
    pub randomness: f32,
    pub intelligence_modifier: f32,
    pub fear_threshold: f32,
    pub aggression_base: f32,
}

impl Default for InstinctConfig {
    fn default() -> Self {
        Self {
            decision_model: DecisionModel::UtilityBased,
            drives: vec![
                Drive { drive_type: DriveType::Hunger,    base_value: 0.5, decay_rate: 0.001,  urgency_threshold: 0.8, max_value: 1.0 },
                Drive { drive_type: DriveType::Safety,    base_value: 0.7, decay_rate: 0.0,    urgency_threshold: 0.3, max_value: 1.0 },
                Drive { drive_type: DriveType::Belonging, base_value: 0.5, decay_rate: 0.0005, urgency_threshold: 0.7, max_value: 1.0 },
            ],
            tick_rate_ms: 500,
            memory_influence: 0.4,
            social_influence: 0.3,
            randomness: 0.1,
            intelligence_modifier: 1.0,
            fear_threshold: -0.6,
            aggression_base: 0.1,
        }
    }
}
