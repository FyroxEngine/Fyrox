// THEATRE-COMPOSITOR: Bevy resources wrapping the Channel Mixer and Theatre state.

use bevy::prelude::*;
use biospark_theatre::{ChannelMixer, ChannelRouter};

/// The Channel Mixer as a Bevy Resource.
#[derive(Resource)]
pub struct TheatreMixer(pub ChannelMixer);

/// The routing graph as a Bevy Resource.
#[derive(Resource)]
pub struct TheatreRouter(pub ChannelRouter);

/// Theatre timing and playback state.
///
/// Phase 7: `is_playing` gates the beat clock. `bar_count` tracks full bars
/// elapsed since last Stop. Phase 8 will replace this with QuillClock / Mixxx
/// Link sync — the fields remain the same, only `advance_beat` changes.
#[derive(Resource)]
pub struct TheatreState {
    pub tick:       u64,
    /// Beat position within the current bar [0.0, 1.0).
    pub beat:       f32,
    pub tempo_bpm:  f32,
    /// Whether the beat clock is advancing. Toggled by the transport ▶/⏸.
    pub is_playing: bool,
    /// Number of complete bars that have elapsed since the last Stop.
    pub bar_count:  u64,
}

impl Default for TheatreState {
    fn default() -> Self {
        Self {
            tick:      0,
            beat:      0.0,
            tempo_bpm: 120.0,
            is_playing: true,
            bar_count:  0,
        }
    }
}

/// Advance the internal beat clock each frame.
///
/// Respects `is_playing` — does nothing when paused.
/// Increments `bar_count` every time `beat` wraps past 1.0.
///
/// Phase 8: replace body with Mixxx OSC / Ableton Link sync.
pub fn advance_beat(time: Res<Time>, mut state: ResMut<TheatreState>) {
    if !state.is_playing {
        return;
    }
    let beat_duration_secs = 60.0 / state.tempo_bpm;
    let new_beat = state.beat + time.delta_seconds() / beat_duration_secs;
    if new_beat >= 1.0 {
        state.bar_count = state.bar_count.wrapping_add(1);
    }
    state.beat = new_beat % 1.0;
    state.tick = state.tick.wrapping_add(1);
}
