pub const CRATE_NAME: &str = "myth-composer";
pub const CREST: &str = "Composer";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AudioLayer { Ambient, Music, SFX, Voice, Foley, Stinger }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MusicMode { Adaptive, Linear, Reactive, Generative, Silent }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioCue {
    pub cue_id: String,
    pub asset_ref: String,
    pub layer: AudioLayer,
    pub loop_mode: bool,
    pub volume: f32,
    pub pitch: f32,
    pub spatial: bool,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    pub trigger_event: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MusicState {
    pub tension: f32,
    pub mood: f32,
    pub active_theme: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComposerConfig {
    pub music_mode: MusicMode,
    pub master_volume: f32,
    pub layer_volumes: Vec<(AudioLayer, f32)>,
    pub audio_cues: Vec<AudioCue>,
    pub spatial_audio_distance: f32,
    pub reverb_enabled: bool,
    pub reverb_preset: String,
    pub dynamic_music: bool,
    pub stinger_probability: f32,
    pub silence_threshold_ms: u32,
    pub sample_rate: u32,
}

impl Default for ComposerConfig {
    fn default() -> Self {
        Self {
            music_mode: MusicMode::Adaptive,
            master_volume: 0.8,
            layer_volumes: vec![
                (AudioLayer::Ambient, 0.7),
                (AudioLayer::Music,   0.6),
                (AudioLayer::SFX,     0.8),
                (AudioLayer::Voice,   1.0),
            ],
            audio_cues: vec![],
            spatial_audio_distance: 100.0,
            reverb_enabled: true,
            reverb_preset: "outdoor".into(),
            dynamic_music: true,
            stinger_probability: 0.2,
            silence_threshold_ms: 5000,
            sample_rate: 48000,
        }
    }
}
