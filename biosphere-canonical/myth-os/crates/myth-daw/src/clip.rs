use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A placed capsule — either on the arrangement timeline or in a session slot.
///
/// Clips never own their capsule data; they hold a reference ID. The Quantum
/// Vault owns the data. This keeps the DAW layer thin and the storage layer
/// authoritative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id:          Uuid,
    /// The capsule this clip plays back. Resolved via myth-vault at runtime.
    pub capsule_id:  Uuid,
    pub name:        String,
    /// Start position in beats (arrangement view). Zero in session slots.
    pub start_beat:  f64,
    /// Duration in beats. If None, plays to end of capsule.
    pub duration:    Option<f64>,
    pub looping:     bool,
    pub launch_mode: LaunchMode,
    pub state:       ClipState,
    /// Fade in/out in beats.
    pub fade_in:     f64,
    pub fade_out:    f64,
    pub color:       Option<[u8; 3]>,
}

impl Clip {
    pub fn new(capsule_id: Uuid, name: impl Into<String>) -> Self {
        Self {
            id:          Uuid::new_v4(),
            capsule_id,
            name:        name.into(),
            start_beat:  0.0,
            duration:    None,
            looping:     false,
            launch_mode: LaunchMode::Trigger,
            state:       ClipState::Idle,
            fade_in:     0.0,
            fade_out:    0.0,
            color:       None,
        }
    }

    pub fn end_beat(&self) -> Option<f64> {
        self.duration.map(|d| self.start_beat + d)
    }

    /// True if the transport position falls within this clip.
    pub fn is_active_at(&self, beat: f64) -> bool {
        if beat < self.start_beat { return false; }
        match self.end_beat() {
            Some(end) => beat < end,
            None      => true,
        }
    }
}

/// How a session-view clip responds to being triggered.
/// Matches Ableton Live's launch modes — familiar to any DAW user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchMode {
    /// Press once to start, press again to re-trigger from start.
    Trigger,
    /// Hold to play, release to stop.
    Gate,
    /// First press starts, second press stops.
    Toggle,
    /// Repeats on every quantize boundary while held.
    Repeat,
}

impl LaunchMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Trigger => "Trigger",
            Self::Gate    => "Gate",
            Self::Toggle  => "Toggle",
            Self::Repeat  => "Repeat",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipState {
    Idle,
    /// Queued to start on next quantize boundary.
    Queued,
    Playing,
    /// Queued to stop on next quantize boundary.
    Stopping,
}
