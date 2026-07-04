pub const CRATE_NAME: &str = "myth-chronicle";
pub const CREST: &str = "Chronicle";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventTrigger {
    OnTick { interval_ms: u64 },
    OnCondition { condition_id: String },
    OnWorldEvent { event_type: String },
    AtWorldTime { world_time: f64 },
    OnActorAction { action_type: String },
    Manual,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventOutcome {
    EmitNarrative { template_id: String },
    SpawnActor { archetype_id: String, position: Option<[f32; 3]> },
    ModifyFaction { faction_id: String, disposition_delta: f32 },
    TriggerWeather { weather_event: String },
    AdvanceQuest { quest_id: String, step: u32 },
    Custom { payload: serde_json::Value },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScheduledEvent {
    pub event_id: String,
    pub label: String,
    pub trigger: EventTrigger,
    pub outcomes: Vec<EventOutcome>,
    pub repeatable: bool,
    pub cooldown_ms: u64,
    pub priority: u8,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChronicleConfig {
    pub world_start_time: f64,
    pub time_scale: f32,
    pub max_events_per_tick: u8,
    pub event_history_size: u32,
    pub scheduled_events: Vec<ScheduledEvent>,
    pub arc_mode: bool,
    pub random_event_probability: f32,
}

impl Default for ChronicleConfig {
    fn default() -> Self {
        Self {
            world_start_time: 0.0,
            time_scale: 1.0,
            max_events_per_tick: 4,
            event_history_size: 1000,
            scheduled_events: vec![],
            arc_mode: false,
            random_event_probability: 0.01,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FiredEvent {
    pub event_id: String,
    pub fired_at: f64,
    pub outcomes_triggered: Vec<String>,
}
