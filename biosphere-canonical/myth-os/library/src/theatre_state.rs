// Theatre Mixer State — Bevy Resource wrapping the actual ChannelMixer.
//
// One TheatreMixerState exists globally. When a Stage vault is entered,
// its channel configuration is loaded here. The egui mixer UI reads/writes
// this resource; changes are flushed to the vault's VaultRegistry on save.
//
// FrameContext is fed from CoreStatus tick/beat/tempo so the Theatre clock
// is synchronised with myth-core.

use bevy::prelude::*;
use biospark_theatre::{ChannelMixer, FrameContext, LayerType};

// ── Resource ──────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct TheatreMixerState {
    pub mixer:   ChannelMixer,
    pub frame:   FrameContext,
    /// True when the mixer has unsaved changes that need flushing to the vault.
    pub dirty:   bool,
}

impl Default for TheatreMixerState {
    fn default() -> Self {
        Self {
            mixer: default_mixer(),
            frame: FrameContext::default(),
            dirty: false,
        }
    }
}

impl TheatreMixerState {
    /// Sync the FrameContext from myth-core clock data.
    /// Called each frame by `sync_frame_context`.
    pub fn sync_clock(&mut self, tick: u64, beat: f32, tempo_bpm: f32, delta_ms: f32) {
        self.frame.tick      = tick;
        self.frame.beat      = beat;
        self.frame.tempo_bpm = tempo_bpm;
        self.frame.delta_ms  = delta_ms;
    }
}

/// Build the default 8-channel mixer pre-populated for a Stage vault.
fn default_mixer() -> ChannelMixer {
    let mut m = ChannelMixer::new(16);

    let channels = [
        ("BG Wash",   LayerType::Gl),
        ("Generative",LayerType::P5),
        ("Scene A",   LayerType::Bv),
        ("Scene B",   LayerType::Bv),
        ("FX Layer",  LayerType::Gl),
        ("HTML Panel",LayerType::Ht),
        ("Audio 1",   LayerType::Au),
        ("Audio 2",   LayerType::Au),
    ];

    for (name, layer_type) in channels {
        let _ = m.add_channel(name, layer_type);
    }

    m
}

// ── Bevy systems ──────────────────────────────────────────────────────────────

/// Each frame: pull tick/beat/tempo from CoreStatus into TheatreMixerState.
pub fn sync_frame_context(
    core:    Res<crate::core_status::CoreStatus>,
    mut tmx: ResMut<TheatreMixerState>,
    time:    Res<Time>,
) {
    tmx.sync_clock(
        core.tick(),
        core.beat(),
        core.tempo_bpm(),
        time.delta_seconds() * 1000.0,
    );
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TheatreStatePlugin;

impl Plugin for TheatreStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TheatreMixerState>()
           .add_systems(Update, sync_frame_context);
    }
}
