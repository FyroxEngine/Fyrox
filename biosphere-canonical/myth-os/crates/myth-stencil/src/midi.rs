use serde::{Deserialize, Serialize};

/// Binds a panel node's flex ratio to a MIDI CC knob or fader.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MidiBinding {
    /// MIDI channel (0-based, so 0 = channel 1).
    pub channel: u8,
    /// Control Change number (0–127).
    pub cc: u8,
    /// Minimum flex value when CC = 0.
    pub flex_min: f32,
    /// Maximum flex value when CC = 127.
    pub flex_max: f32,
}

impl MidiBinding {
    pub fn new(channel: u8, cc: u8) -> Self {
        Self { channel, cc, flex_min: 0.1, flex_max: 4.0 }
    }

    /// Map a raw CC value (0–127) to a flex ratio.
    pub fn flex_from_cc(&self, value: u8) -> f32 {
        let t = value as f32 / 127.0;
        self.flex_min + t * (self.flex_max - self.flex_min)
    }
}
