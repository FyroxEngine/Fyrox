pub const CRATE_NAME: &str = "myth-mythos";
pub const CREST: &str = "Mythos";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SeasonMode {
    Static,
    Cycling,
    Reversed,
    Custom,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WeatherEvent {
    Storm,
    Drought,
    Fog,
    Blizzard,
    Heatwave,
    Calm,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MythosConfig {
    pub weather_seed: u64,
    /// Celsius at sea level
    pub temperature_base: f32,
    /// Degrees per 100m elevation
    pub temperature_lapse_rate: f32,
    /// Normalized direction vector
    pub wind_direction: [f32; 3],
    /// m/s
    pub wind_strength: f32,
    /// 0.0–1.0
    pub wind_variability: f32,
    /// mm/day base
    pub precipitation_rate: f32,
    /// 0.0–1.0
    pub cloud_coverage: f32,
    /// 0.0–1.0
    pub humidity_base: f32,
    pub season_mode: SeasonMode,
    pub season_length_days: u32,
    /// Affects sound propagation and visibility
    pub atmospheric_density: f32,
    /// Humidity level that triggers fog
    pub fog_threshold: f32,
}

impl Default for MythosConfig {
    fn default() -> Self {
        Self {
            weather_seed: 0,
            temperature_base: 15.0,
            temperature_lapse_rate: 0.65,
            wind_direction: [1.0, 0.0, 0.0],
            wind_strength: 5.0,
            wind_variability: 0.3,
            precipitation_rate: 2.0,
            cloud_coverage: 0.4,
            humidity_base: 0.5,
            season_mode: SeasonMode::Cycling,
            season_length_days: 90,
            atmospheric_density: 1.0,
            fog_threshold: 0.85,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeatherState {
    pub temperature: f32,
    pub humidity: f32,
    pub wind_velocity: [f32; 3],
    pub precipitation: f32,
    pub cloud_coverage: f32,
    /// km
    pub visibility: f32,
    pub active_events: Vec<WeatherEvent>,
}
