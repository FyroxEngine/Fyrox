//! Sound context.

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
    scene::{
        node::Node,
        sound::{effect::Effect, Sound, SoundChanges},
    },
    utils::log::{Log, MessageKind},
};
use fyrox_sound::{
    context::DistanceModel,
    renderer::Renderer,
    source::{generic::GenericSourceBuilder, spatial::SpatialSourceBuilder, SoundSource, Status},
};
use std::time::Duration;

/// Sound context.
#[derive(Debug, Visit, Inspect)]
pub struct SoundContext {
    master_gain: f32,
    renderer: Renderer,
    distance_model: DistanceModel,
    paused: bool,
    effects: Pool<Effect>,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) native: fyrox_sound::context::SoundContext,
}

impl Default for SoundContext {
    fn default() -> Self {
        Self {
            master_gain: 1.0,
            renderer: Default::default(),
            distance_model: Default::default(),
            paused: false,
            effects: Default::default(),
            native: fyrox_sound::context::SoundContext::new(),
        }
    }
}

impl SoundContext {
    pub(crate) fn new() -> Self {
        Self {
            master_gain: 1.0,
            renderer: Default::default(),
            distance_model: Default::default(),
            paused: false,
            effects: Default::default(),
            native: fyrox_sound::context::SoundContext::new(),
        }
    }

    /// Pause/unpause the sound context. Paused context won't play any sounds.
    pub fn pause(&mut self, pause: bool) {
        self.paused = pause;
        self.native.state().pause(self.paused);
    }

    /// Returns true if the sound context is paused, false - otherwise.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Sets new distance model.
    pub fn set_distance_model(&mut self, distance_model: DistanceModel) {
        self.distance_model = distance_model;
        self.native.state().set_distance_model(self.distance_model);
    }

    /// Returns current distance model.
    pub fn distance_model(&self) -> DistanceModel {
        self.distance_model
    }

    /// Normalizes given frequency using context's sampling rate. Normalized frequency then can be used
    /// to create filters.
    pub fn normalize_frequency(&self, f: f32) -> f32 {
        self.native.state().normalize_frequency(f)
    }

    /// Returns amount of time context spent on rendering all sound sources.
    pub fn full_render_duration(&self) -> Duration {
        self.native.state().full_render_duration()
    }

    /// Sets new renderer.
    pub fn set_renderer(&mut self, renderer: Renderer) -> Renderer {
        self.native.state().set_renderer(renderer)
    }

    /// Sets new master gain. Master gain is used to control total sound volume that will be passed to output
    /// device.
    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain;
        self.native.state().set_master_gain(self.master_gain)
    }

    /// Returns master gain.
    pub fn master_gain(&self) -> f32 {
        self.master_gain
    }

    pub(crate) fn update(&mut self, nodes: &Pool<Node>) {}

    pub(crate) fn remove_sound(&mut self, sound: Handle<SoundSource>) {
        self.native.state().remove_source(sound);
    }

    pub(crate) fn sync_sound(&mut self, sound: &Sound) {
        if sound.native.get().is_some() {
            let mut changes = sound.changes.get();
            if !changes.is_empty() {
                let mut state = self.native.state();
                let spatial = state.source_mut(sound.native.get()).spatial_mut();

                if changes.contains(SoundChanges::MAX_DISTANCE) {
                    spatial.set_max_distance(sound.max_distance);
                    changes.remove(SoundChanges::MAX_DISTANCE);
                }
                if changes.contains(SoundChanges::ROLLOFF_FACTOR) {
                    spatial.set_rolloff_factor(sound.rolloff_factor);
                    changes.remove(SoundChanges::ROLLOFF_FACTOR);
                }
                if changes.contains(SoundChanges::RADIUS) {
                    spatial.set_radius(sound.radius);
                    changes.remove(SoundChanges::RADIUS);
                }
                if changes.contains(SoundChanges::PLAYBACK_TIME) {
                    spatial.set_playback_time(sound.playback_time);
                    changes.remove(SoundChanges::PLAYBACK_TIME);
                }
                if changes.contains(SoundChanges::STATUS) {
                    match sound.status {
                        Status::Stopped => {
                            Log::verify(spatial.stop());
                        }
                        Status::Playing => {
                            spatial.play();
                        }
                        Status::Paused => {
                            spatial.pause();
                        }
                    }
                    changes.remove(SoundChanges::STATUS);
                }
                if changes.contains(SoundChanges::PITCH) {
                    spatial.set_pitch(sound.pitch);
                    changes.remove(SoundChanges::PITCH);
                }
                if changes.contains(SoundChanges::LOOPING) {
                    spatial.set_looping(sound.looping);
                    changes.remove(SoundChanges::LOOPING);
                }
                if changes.contains(SoundChanges::PANNING) {
                    spatial.set_panning(sound.panning);
                    changes.remove(SoundChanges::PANNING);
                }
                if changes.contains(SoundChanges::GAIN) {
                    spatial.set_gain(sound.gain);
                    changes.remove(SoundChanges::GAIN);
                }
                if changes.contains(SoundChanges::BUFFER) {
                    Log::verify(spatial.set_buffer(sound.buffer()));
                    changes.remove(SoundChanges::BUFFER);
                }

                if !changes.is_empty() {
                    Log::writeln(
                        MessageKind::Warning,
                        format!(
                            "Some changes were not applied to sound! Changes: {}",
                            changes.bits
                        ),
                    )
                }

                sound.changes.set(changes);
            }
        } else {
            let source = SpatialSourceBuilder::new(
                GenericSourceBuilder::new()
                    .with_gain(sound.gain())
                    .with_opt_buffer(sound.buffer())
                    .with_looping(sound.is_looping())
                    .with_panning(sound.panning())
                    .with_pitch(sound.pitch())
                    .with_play_once(sound.is_play_once())
                    .with_status(sound.status())
                    .build()
                    .unwrap(),
            )
            .with_position(sound.global_position())
            .with_radius(sound.radius())
            .with_max_distance(sound.max_distance())
            .with_rolloff_factor(sound.rolloff_factor())
            .build_source();

            sound.native.set(self.native.state().add_source(source));

            Log::writeln(
                MessageKind::Information,
                format!("Native sound source was created for node: {}", sound.name()),
            );
        }
    }
}
