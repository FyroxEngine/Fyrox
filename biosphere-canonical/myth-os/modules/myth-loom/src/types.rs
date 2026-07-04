pub const CRATE_NAME: &str = "myth-loom";
pub const CREST: &str = "Loom";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LocomotionMode {
    Biped,
    Quadruped,
    Flight,
    Swim,
    Crawl,
    Teleport,
    Custom(String),
}

impl Default for LocomotionMode {
    fn default() -> Self { LocomotionMode::Biped }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AnimationLayer {
    Base,
    Overlay,
    Additive,
}

impl Default for AnimationLayer {
    fn default() -> Self { AnimationLayer::Base }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimationClip {
    pub clip_id: String,
    pub asset_ref: String,
    pub layer: AnimationLayer,
    pub loop_mode: bool,
    pub blend_duration: f32,
    pub playback_speed: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MovementProfile {
    pub walk_speed: f32,
    pub run_speed: f32,
    pub sprint_speed: f32,
    pub swim_speed: f32,
    pub fly_speed: f32,
    pub turn_rate: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub jump_height: f32,
}

impl Default for MovementProfile {
    fn default() -> Self {
        Self {
            walk_speed: 1.4,
            run_speed: 4.0,
            sprint_speed: 7.0,
            swim_speed: 1.0,
            fly_speed: 0.0,
            turn_rate: 180.0,
            acceleration: 8.0,
            deceleration: 12.0,
            jump_height: 1.2,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoomConfig {
    pub locomotion_mode: LocomotionMode,
    pub movement_profile: MovementProfile,
    pub animation_clips: Vec<AnimationClip>,
    pub root_motion: bool,
    pub footstep_ik: bool,
    pub procedural_idle_sway: f32,
    pub crowd_avoidance_radius: f32,
    pub formation_capable: bool,
    pub gesture_library: Vec<String>,
}

impl Default for LoomConfig {
    fn default() -> Self {
        Self {
            locomotion_mode: LocomotionMode::Biped,
            movement_profile: MovementProfile::default(),
            animation_clips: vec![],
            root_motion: false,
            footstep_ik: true,
            procedural_idle_sway: 0.2,
            crowd_avoidance_radius: 0.6,
            formation_capable: false,
            gesture_library: vec![],
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimationState {
    pub entity_id: String,
    pub active_clip: String,
    pub playback_position: f32,
    pub velocity: [f32; 3],
    pub is_grounded: bool,
}
