//! Sound context.

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
    resource::model::Model,
    scene::{
        node::Node,
        sound::{effect::Effect, Sound},
    },
    utils::log::{Log, MessageKind},
};
use fyrox_sound::{
    context::DistanceModel,
    effects::{reverb::Reverb, BaseEffect, EffectInput},
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
    // A model resource from which this context was instantiated from.
    pub(crate) resource: Option<Model>,
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
            resource: None,
            native: fyrox_sound::context::SoundContext::new(),
        }
    }
}

impl SoundContext {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Adds new effect and returns its handle.
    pub fn add_effect(&mut self, effect: Effect) -> Handle<Effect> {
        self.effects.spawn(effect)
    }

    /// Removes specified effect.
    pub fn remove_effect(&mut self, effect: Handle<Effect>) -> Effect {
        self.effects.free(effect)
    }

    /// Borrows effects.
    pub fn effect(&self, handle: Handle<Effect>) -> &Effect {
        &self.effects[handle]
    }

    /// Borrows effects as mutable.
    pub fn effect_mut(&mut self, handle: Handle<Effect>) -> &mut Effect {
        &mut self.effects[handle]
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

    pub(crate) fn update(&mut self, nodes: &Pool<Node>) {
        let mut state = self.native.state();

        for effect in self.effects.iter() {
            if effect.native.get().is_some() {
                let native_effect = state.effect_mut(effect.native.get());
                if let (
                    fyrox_sound::effects::Effect::Reverb(native_reverb),
                    Effect::Reverb(reverb),
                ) = (native_effect, effect)
                {
                    reverb.decay_time.try_sync_model(|v| {
                        native_reverb.set_decay_time(Duration::from_secs_f32(v))
                    });
                    reverb.gain.try_sync_model(|v| native_reverb.set_gain(v));
                    reverb.wet.try_sync_model(|v| native_reverb.set_wet(v));
                    reverb.dry.try_sync_model(|v| native_reverb.set_dry(v));
                    reverb.fc.try_sync_model(|v| native_reverb.set_fc(v));
                    reverb.inputs.try_sync_model(|v| {
                        native_reverb.clear_inputs();
                        for input in v.iter() {
                            if let Some(Node::Sound(sound)) = nodes.try_borrow(*input) {
                                native_reverb.add_input(EffectInput::direct(sound.native.get()));
                            }
                        }
                    });
                }
            } else {
                match effect {
                    Effect::Reverb(reverb) => {
                        let mut native_reverb = Reverb::new(BaseEffect::default());
                        native_reverb.set_gain(reverb.gain());
                        native_reverb.set_fc(reverb.fc());
                        native_reverb.set_decay_time(Duration::from_secs_f32(reverb.decay_time()));
                        native_reverb.set_dry(reverb.dry());
                        native_reverb.set_wet(reverb.wet());
                        for input in reverb.inputs.iter() {
                            if let Some(Node::Sound(sound)) = nodes.try_borrow(*input) {
                                native_reverb.add_input(EffectInput::direct(sound.native.get()));
                            }
                        }
                        let native =
                            state.add_effect(fyrox_sound::effects::Effect::Reverb(native_reverb));
                        reverb.native.set(native);
                    }
                }
            }
        }
    }

    pub(crate) fn remove_sound(&mut self, sound: Handle<SoundSource>) {
        self.native.state().remove_source(sound);
    }

    pub(crate) fn sync_with_sound(&self, sound: &mut Sound) {
        if let Some(SoundSource::Spatial(spatial)) =
            self.native.state().try_get_source_mut(sound.native.get())
        {
            // Sync back.
            sound.status.set_silent(spatial.status());
            sound.playback_time.set_silent(spatial.playback_time());
        }
    }

    pub(crate) fn sync_to_sound(&mut self, sound: &Sound) {
        if sound.native.get().is_some() {
            let mut state = self.native.state();
            let spatial = state.source_mut(sound.native.get()).spatial_mut();

            sound.max_distance.try_sync_model(|v| {
                spatial.set_max_distance(v);
            });
            sound.rolloff_factor.try_sync_model(|v| {
                spatial.set_rolloff_factor(v);
            });
            sound.radius.try_sync_model(|v| {
                spatial.set_radius(v);
            });
            sound.playback_time.try_sync_model(|v| {
                spatial.set_playback_time(v);
            });
            sound.pitch.try_sync_model(|v| {
                spatial.set_pitch(v);
            });
            sound.looping.try_sync_model(|v| {
                spatial.set_looping(v);
            });
            sound.panning.try_sync_model(|v| {
                spatial.set_panning(v);
            });
            sound.gain.try_sync_model(|v| {
                spatial.set_gain(v);
            });
            sound.buffer.try_sync_model(|v| {
                Log::verify(spatial.set_buffer(v));
            });
            sound.status.try_sync_model(|v| match v {
                Status::Stopped => {
                    Log::verify(spatial.stop());
                }
                Status::Playing => {
                    spatial.play();
                }
                Status::Paused => {
                    spatial.pause();
                }
            });
        } else {
            let source = SpatialSourceBuilder::new(
                GenericSourceBuilder::new()
                    .with_gain(sound.gain())
                    .with_opt_buffer(sound.buffer())
                    .with_looping(sound.is_looping())
                    .with_panning(sound.panning())
                    .with_pitch(sound.pitch())
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
