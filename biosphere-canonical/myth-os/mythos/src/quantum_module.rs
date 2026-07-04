use serde::{Deserialize, Serialize};

// ── Department ────────────────────────────────────────────────────────────

/// The 4 control buses that map directly to Traktor S4 channels.
/// Each department owns 4 Mythos modules (instruments).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Department {
    Structure,   // Ch1 — MYTH-01 Terrain, 02 Environment, 03 Architect, 04 Lighting
    Entities,    // Ch2 — MYTH-05 Modeling, 06 Choreography, 07 Behavior, 08 Society
    Atmosphere,  // Ch3 — MYTH-09 Sequencer, 10 Story, 11 Memory, 12 Sound
    Dynamics,    // Ch4 — MYTH-13 Logic, 14 Simulation, 15 Forge, 16 Network
}

impl Department {
    /// Representative hex color for this department (used for rack panel accents).
    /// Taken from the first module of each department per canonical registry.
    pub fn color_hex(&self) -> &'static str {
        match self {
            Department::Structure  => "#1e8cff", // Atlas / Terrain blue
            Department::Entities   => "#f4c025", // Animus / Modeling gold
            Department::Atmosphere => "#8c50ff", // Quill / Story violet
            Department::Dynamics   => "#20c8d0", // Axiom / Logic teal
        }
    }
}

// ── Status ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImplementationStatus {
    Built,
    InProgress,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lifecycle {
    Active,
    Sealed,
}

// ── Asset pointers ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleAssets {
    pub icon: String,
    pub preview_image: String,
    pub banner: String,
    pub crest_svg: String,
    pub splash_screen: Option<String>,
    pub audio_preview: Option<String>,
    pub video_preview: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubModuleAssets {
    pub icon: String,
    pub preview_image: String,
}

// ── Capsule (module-level, distinct from mythos::capsule::Capsule) ────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCapsule {
    pub id: String,
    pub data: serde_json::Value,
}

// ── SubModule ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubModule {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub wire_out: String,
    pub description: String,
    pub assets: SubModuleAssets,
    pub capsules: Vec<ModuleCapsule>,
}

// ── Traktor MIDI binding ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraktorBinding {
    pub channel: u8,            // 1–4 (maps to BusChannel)
    pub control_type: String,   // "fader" | "knob" | "jog_wheel" | "pad"
    pub midi_cc: u8,            // raw MIDI CC number (0–127)
    pub parameter: String,      // parameter name this CC controls
    pub scale_min: f32,         // output range minimum
    pub scale_max: f32,         // output range maximum
}

// ── QuantumModule — the central data type ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumModule {
    pub id: String,
    pub name: String,
    pub crest: String,
    pub color: String,         // hex, e.g. "#39ffce"
    pub department: Department,
    pub description: String,
    pub implementation_status: ImplementationStatus,
    pub primary_wire_out: String,
    pub lifecycle: Lifecycle,
    pub capacity: u32,
    pub assets: ModuleAssets,
    pub containers: Vec<SubModule>,
    pub traktor_map: Vec<TraktorBinding>,
}

impl QuantumModule {
    /// Load a QuantumModule from a JSON file.
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Is this module active and usable?
    pub fn is_live(&self) -> bool {
        self.lifecycle == Lifecycle::Active
            && self.implementation_status == ImplementationStatus::Built
    }

    /// MIDI CC numbers this module listens to (for the mixer to route).
    pub fn midi_ccs(&self) -> Vec<u8> {
        self.traktor_map.iter().map(|b| b.midi_cc).collect()
    }
}
