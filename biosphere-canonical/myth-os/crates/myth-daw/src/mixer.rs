use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::track::TrackKind;

/// The Mixer — one channel strip per track plus a master.
///
/// The mixer doesn't process audio; it holds the parameter state that the
/// Theater reads each tick to weight its output. Fader = intensity weight,
/// not dB gain (there's no audio signal here, just narrative parameter flow).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mixer {
    pub channels: Vec<MixerChannel>,
    pub master:   MixerChannel,
}

impl Mixer {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            master:   MixerChannel::master(),
        }
    }

    pub fn add_channel(&mut self, name: impl Into<String>, kind: TrackKind) -> Uuid {
        let ch = MixerChannel::new(name, kind);
        let id = ch.id;
        self.channels.push(ch);
        id
    }

    pub fn channel_mut(&mut self, id: Uuid) -> Option<&mut MixerChannel> {
        self.channels.iter_mut().find(|c| c.id == id)
    }

    /// Effective fader value accounting for mute and solo.
    /// If any channel is soloed, non-soloed channels return 0.0.
    pub fn effective_level(&self, id: Uuid) -> f64 {
        let any_solo = self.channels.iter().any(|c| c.soloed);
        if let Some(ch) = self.channels.iter().find(|c| c.id == id) {
            if ch.muted { return 0.0; }
            if any_solo && !ch.soloed { return 0.0; }
            ch.fader * self.master.fader
        } else {
            0.0
        }
    }
}

impl Default for Mixer {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerChannel {
    pub id:     Uuid,
    pub name:   String,
    pub kind:   Option<TrackKind>,
    /// 0.0 = silent, 1.0 = unity, >1.0 = boost.
    pub fader:  f64,
    pub muted:  bool,
    pub soloed: bool,
    pub armed:  bool,
    /// Sends to effect returns — (return_id, send_level 0.0–1.0).
    pub sends:  Vec<(Uuid, f64)>,
    /// Peak level seen this tick — used by meter display.
    pub peak:   f64,
    pub color:  Option<[u8; 3]>,
}

impl MixerChannel {
    pub fn new(name: impl Into<String>, kind: TrackKind) -> Self {
        Self {
            id:     Uuid::new_v4(),
            name:   name.into(),
            kind:   Some(kind),
            fader:  1.0,
            muted:  false,
            soloed: false,
            armed:  false,
            sends:  Vec::new(),
            peak:   0.0,
            color:  Some(kind.default_color()),
        }
    }

    fn master() -> Self {
        Self {
            id:     Uuid::new_v4(),
            name:   "Master".into(),
            kind:   None,
            fader:  1.0,
            muted:  false,
            soloed: false,
            armed:  false,
            sends:  Vec::new(),
            peak:   0.0,
            color:  Some([100, 255, 218]),
        }
    }
}
