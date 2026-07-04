use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single horizontal lane in the DAW — holds clips on the timeline
/// and a slot column in session view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id:      Uuid,
    pub name:    String,
    pub kind:    TrackKind,
    pub color:   [u8; 3],
    pub muted:   bool,
    pub soloed:  bool,
    pub armed:   bool,    // record-enabled
    pub height:  f32,     // px, used by arrangement view
    pub visible: bool,
}

impl Track {
    pub fn new(name: impl Into<String>, kind: TrackKind) -> Self {
        Self {
            id:      Uuid::new_v4(),
            name:    name.into(),
            color:   kind.default_color(),
            kind,
            muted:   false,
            soloed:  false,
            armed:   false,
            height:  80.0,
            visible: true,
        }
    }
}

/// Maps directly to the Quantum Quill doc's track categories.
/// Each kind carries a default wire type it emits to the Theater.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    /// Character arcs and persona behavior — emits BHV packets.
    Character,
    /// Emotional layers — emits BHV packets.
    Emotion,
    /// Environmental state (terrain, atmosphere) — emits SPA packets.
    Environment,
    /// Discrete events and triggers — emits EVT packets.
    Event,
    /// Narrative text / lore — emits NAR packets.
    Narrative,
    /// Audio — emits AUD packets.
    Audio,
    /// Effects and modulation — emits CTL packets.
    Effect,
    /// Master bus (one per session, always last).
    Master,
}

impl TrackKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Character   => "Character Arc",
            Self::Emotion     => "Emotion",
            Self::Environment => "Environment",
            Self::Event       => "Event",
            Self::Narrative   => "Narrative",
            Self::Audio       => "Audio",
            Self::Effect      => "Effect",
            Self::Master      => "Master",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Character   => "🎭",
            Self::Emotion     => "💫",
            Self::Environment => "🌍",
            Self::Event       => "⚡",
            Self::Narrative   => "📖",
            Self::Audio       => "🎵",
            Self::Effect      => "🎨",
            Self::Master      => "MSTR",
        }
    }

    /// Wire type this track emits by default.
    pub fn wire_type(self) -> myth_wire::WireType {
        match self {
            Self::Character   => myth_wire::WireType::Behavioral,
            Self::Emotion     => myth_wire::WireType::Behavioral,
            Self::Environment => myth_wire::WireType::Spatial,
            Self::Event       => myth_wire::WireType::Event,
            Self::Narrative   => myth_wire::WireType::Narrative,
            Self::Audio       => myth_wire::WireType::Audio,
            Self::Effect      => myth_wire::WireType::Control,
            Self::Master      => myth_wire::WireType::Control,
        }
    }

    pub fn default_color(self) -> [u8; 3] {
        match self {
            Self::Character   => [255, 107, 107], // coral red
            Self::Emotion     => [78,  205, 196], // teal
            Self::Environment => [69,  183, 209], // sky blue
            Self::Event       => [255, 200,  50], // gold
            Self::Narrative   => [155,  89, 182], // purple
            Self::Audio       => [225, 112,  85], // burnt orange
            Self::Effect      => [162, 155, 254], // lavender
            Self::Master      => [100, 255, 218], // cyan
        }
    }
}
