pub const CRATE_NAME: &str = "myth-continuum";
pub const CREST: &str = "Continuum";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PhysicsMode { Deterministic, Stochastic, Simplified, Disabled }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CollisionResponse { Rigid, Soft, Trigger, Ghost }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GravityField {
    pub field_id: String,
    pub direction: [f32; 3],
    pub strength: f32,              // m/s²
    pub falloff_radius: Option<f32>, // None = global
    pub origin: Option<[f32; 3]>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ForceField {
    pub field_id: String,
    pub force_vector: [f32; 3],
    pub strength: f32,
    pub radius: f32,
    pub origin: [f32; 3],
    pub duration_ms: Option<u64>,   // None = permanent
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContinuumConfig {
    pub physics_mode: PhysicsMode,
    pub gravity_fields: Vec<GravityField>,
    pub force_fields: Vec<ForceField>,
    pub tick_rate_hz: u32,          // physics updates per second
    pub sub_steps: u8,              // integration sub-steps per tick (1–8)
    pub collision_response: CollisionResponse,
    pub fluid_simulation: bool,
    pub fluid_viscosity: f32,
    pub erosion_enabled: bool,      // terrain erosion from water/wind
    pub erosion_rate: f32,
    pub time_dilation: f32,         // 1.0 = normal, <1 = slow-mo, >1 = fast-forward
    pub deterministic_seed: u64,
}

impl Default for ContinuumConfig {
    fn default() -> Self {
        Self {
            physics_mode: PhysicsMode::Simplified,
            gravity_fields: vec![GravityField {
                field_id: "default".into(),
                direction: [0.0, -1.0, 0.0],
                strength: 9.81,
                falloff_radius: None,
                origin: None,
            }],
            force_fields: vec![],
            tick_rate_hz: 60,
            sub_steps: 2,
            collision_response: CollisionResponse::Rigid,
            fluid_simulation: false,
            fluid_viscosity: 1.0,
            erosion_enabled: false,
            erosion_rate: 0.001,
            time_dilation: 1.0,
            deterministic_seed: 0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicsEvent {
    pub event_type: String,     // "collision", "threshold_crossed", "field_entered"
    pub actor_ids: Vec<String>,
    pub position: [f32; 3],
    pub impulse: Option<[f32; 3]>,
}
