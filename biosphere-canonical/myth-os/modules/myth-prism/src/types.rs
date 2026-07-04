pub const CRATE_NAME: &str = "myth-prism";
pub const CREST: &str = "Prism";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SunType {
    MainSequence,
    RedGiant,
    BinarySystem,
    Artificial,
    None,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MoonPhase {
    New,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
    Full,
    WaningGibbous,
    LastQuarter,
    WaningCrescent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CelestialBody {
    pub name: String,
    /// `SunType::None` means this is a moon or planet
    pub sun_type: SunType,
    /// Kelvin
    pub color_temperature: u32,
    /// Lux multiplier
    pub intensity: f32,
    /// Degrees in sky
    pub angular_size: f32,
    pub orbit_period_days: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrismConfig {
    /// Real-time minutes per in-world day
    pub day_length_minutes: f32,
    pub celestial_bodies: Vec<CelestialBody>,
    /// Minimum ambient light 0.0–1.0 (moonless night)
    pub ambient_min: f32,
    /// Maximum ambient 0.0–1.0 (noon)
    pub ambient_max: f32,
    /// RGB 0.0–1.0
    pub sky_color_dawn: [f32; 3],
    pub sky_color_day: [f32; 3],
    pub sky_color_dusk: [f32; 3],
    pub sky_color_night: [f32; 3],
    /// 0 = hard shadows, 1 = fully soft
    pub shadow_softness: f32,
    /// World units
    pub shadow_distance: f32,
    /// Visible at night, 0.0–1.0
    pub star_field_intensity: f32,
    /// 0.0–1.0
    pub aurora_probability: f32,
    pub volumetric_fog: bool,
}

impl Default for PrismConfig {
    fn default() -> Self {
        Self {
            day_length_minutes: 24.0,
            celestial_bodies: vec![],
            ambient_min: 0.05,
            ambient_max: 1.0,
            sky_color_dawn: [1.0, 0.5, 0.2],
            sky_color_day: [0.4, 0.6, 1.0],
            sky_color_dusk: [1.0, 0.4, 0.1],
            sky_color_night: [0.02, 0.02, 0.08],
            shadow_softness: 0.3,
            shadow_distance: 500.0,
            star_field_intensity: 0.8,
            aurora_probability: 0.0,
            volumetric_fog: false,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LightState {
    /// 0.0–1.0 (0 = midnight, 0.5 = noon)
    pub time_of_day: f32,
    pub sun_direction: [f32; 3],
    pub sun_color: [f32; 3],
    pub sun_intensity: f32,
    pub ambient_intensity: f32,
    pub sky_color: [f32; 3],
    pub moon_phase: MoonPhase,
    pub moon_intensity: f32,
}
