//! Sound context.

use crate::{
    core::{
        log::{Log, MessageKind},
        pool::Handle,
        visitor::prelude::*,
    },
    scene::{node::Node, sound::Sound},
};
use fxhash::FxHashSet;
use fyrox_sound::{
    bus::AudioBusGraph,
    context::DistanceModel,
    renderer::Renderer,
    source::{SoundSource, SoundSourceBuilder, Status},
};
use std::{sync::MutexGuard, time::Duration};

/// Sound context.
#[derive(Debug, Visit)]
pub struct SoundContext {
    #[visit(optional)]
    pub(crate) native: fyrox_sound::context::SoundContext,
}

/// Proxy for guarded access to the sound context.
pub struct SoundContextGuard<'a> {
    guard: MutexGuard<'a, fyrox_sound::context::State>,
}

impl<'a> SoundContextGuard<'a> {
    /// Returns a reference to the audio bus graph.
    pub fn bus_graph_ref(&self) -> &AudioBusGraph {
        self.guard.bus_graph_ref()
    }

    /// Returns a reference to the audio bus graph.
    pub fn bus_graph_mut(&mut self) -> &mut AudioBusGraph {
        self.guard.bus_graph_mut()
    }

    /// Pause/unpause the sound context. Paused context won't play any sounds.
    pub fn pause(&mut self, pause: bool) {
        self.guard.pause(pause);
    }

    /// Returns true if the sound context is paused, false - otherwise.
    pub fn is_paused(&self) -> bool {
        self.guard.is_paused()
    }

    /// Sets new distance model.
    pub fn set_distance_model(&mut self, distance_model: DistanceModel) {
        self.guard.set_distance_model(distance_model);
    }

    /// Returns current distance model.
    pub fn distance_model(&self) -> DistanceModel {
        self.guard.distance_model()
    }

    /// Normalizes given frequency using context's sampling rate. Normalized frequency then can be used
    /// to create filters.
    pub fn normalize_frequency(&self, f: f32) -> f32 {
        self.guard.normalize_frequency(f)
    }

    /// Returns amount of time context spent on rendering all sound sources.
    pub fn full_render_duration(&self) -> Duration {
        self.guard.full_render_duration()
    }

    /// Returns current renderer.
    pub fn renderer(&self) -> Renderer {
        self.guard.renderer().clone()
    }

    /// Returns current renderer.
    pub fn renderer_ref(&self) -> &Renderer {
        self.guard.renderer()
    }

    /// Returns current renderer.
    pub fn renderer_ref_mut(&mut self) -> &mut Renderer {
        self.guard.renderer_mut()
    }

    /// Sets new renderer.
    pub fn set_renderer(&mut self, renderer: Renderer) -> Renderer {
        self.guard.set_renderer(renderer)
    }

    /// Destroys all backing sound entities.
    pub fn destroy_sound_sources(&mut self) {
        self.guard.sources_mut().clear();
    }
}

impl Default for SoundContext {
    fn default() -> Self {
        let native = fyrox_sound::context::SoundContext::new();
        let mut state = native.state();
        // There's no need to serialize native sources, because they'll be re-created automatically.
        state.serialization_options.skip_sources = true;
        drop(state);
        Self { native }
    }
}

impl SoundContext {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Creates a full copy of the context, instead of shallow that could be done via [`Clone::clone`]
    pub fn deep_clone(&self) -> Self {
        Self {
            native: self.native.deep_clone(),
        }
    }

    /// Returns locked inner state of the sound context.
    pub fn state(&self) -> SoundContextGuard {
        SoundContextGuard {
            guard: self.native.state(),
        }
    }

    pub(crate) fn remove_sound(&mut self, sound: Handle<SoundSource>, name: &str) {
        let mut state = self.native.state();
        if state.is_valid_handle(sound) {
            state.remove_source(sound);

            Log::info(format!(
                "Native sound source was removed for node: {}",
                name
            ));
        }
    }

    pub(crate) fn set_sound_position(&mut self, sound: &Sound) {
        if let Some(source) = self.native.state().try_get_source_mut(sound.native.get()) {
            source.set_position(sound.global_position());
        }
    }

    pub(crate) fn sync_with_sound(&self, sound: &mut Sound) {
        if let Some(source) = self.native.state().try_get_source_mut(sound.native.get()) {
            // Sync back.
            sound.status.set_value_silent(source.status());
            sound
                .playback_time
                .set_value_silent(source.playback_time().as_secs_f32());
        }
    }

    pub(crate) fn sync_to_sound(
        &mut self,
        sound_handle: Handle<Node>,
        sound: &Sound,
        node_overrides: Option<&FxHashSet<Handle<Node>>>,
    ) {
        if !sound.is_globally_enabled()
            || !node_overrides.map_or(true, |f| f.contains(&sound_handle))
        {
            self.remove_sound(sound.native.get(), &sound.name);
            sound.native.set(Default::default());
            return;
        }

        if sound.native.get().is_some() {
            let mut state = self.native.state();
            let source = state.source_mut(sound.native.get());
            sound.buffer.try_sync_model(|v| {
                Log::verify(source.set_buffer(v));
            });
            sound.max_distance.try_sync_model(|v| {
                source.set_max_distance(v);
            });
            sound.rolloff_factor.try_sync_model(|v| {
                source.set_rolloff_factor(v);
            });
            sound.radius.try_sync_model(|v| {
                source.set_radius(v);
            });
            sound.playback_time.try_sync_model(|v| {
                source.set_playback_time(Duration::from_secs_f32(v));
            });
            sound.pitch.try_sync_model(|v| {
                source.set_pitch(v);
            });
            sound.looping.try_sync_model(|v| {
                source.set_looping(v);
            });
            sound.panning.try_sync_model(|v| {
                source.set_panning(v);
            });
            sound.gain.try_sync_model(|v| {
                source.set_gain(v);
            });
            sound
                .spatial_blend
                .try_sync_model(|v| source.set_spatial_blend(v));
            sound.status.try_sync_model(|v| match v {
                Status::Stopped => {
                    Log::verify(source.stop());
                }
                Status::Playing => {
                    source.play();
                }
                Status::Paused => {
                    source.pause();
                }
            });
            sound.audio_bus.try_sync_model(|audio_bus| {
                source.set_bus(audio_bus);
            });
        } else {
            match SoundSourceBuilder::new()
                .with_gain(sound.gain())
                .with_opt_buffer(sound.buffer())
                .with_looping(sound.is_looping())
                .with_panning(sound.panning())
                .with_pitch(sound.pitch())
                .with_status(sound.status())
                .with_playback_time(Duration::from_secs_f32(sound.playback_time()))
                .with_position(sound.global_position())
                .with_radius(sound.radius())
                .with_max_distance(sound.max_distance())
                .with_bus(sound.audio_bus())
                .with_rolloff_factor(sound.rolloff_factor())
                .build()
            {
                Ok(source) => {
                    sound.native.set(self.native.state().add_source(source));

                    Log::writeln(
                        MessageKind::Information,
                        format!("Native sound source was created for node: {}", sound.name()),
                    );
                }
                Err(err) => {
                    Log::writeln(
                        MessageKind::Error,
                        format!(
                            "Unable to create native sound source for node: {}. Reason: {:?}",
                            sound.name(),
                            err
                        ),
                    );
                }
            }
        }
    }
}
