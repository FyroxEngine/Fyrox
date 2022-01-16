//! Sound scene.

use crate::{
    core::pool::Handle,
    scene::sound::{Sound, SoundChanges},
    utils::log::{Log, MessageKind},
};
use fyrox_sound::{
    context::SoundContext,
    source::SoundSource,
    source::{generic::GenericSourceBuilder, spatial::SpatialSourceBuilder, Status},
};
use std::time::Duration;

/// Sound scene.
#[derive(Default, Debug)]
pub struct SoundScene {
    pub(crate) native: SoundContext,
}

impl SoundScene {
    pub(crate) fn new() -> Self {
        Self {
            native: SoundContext::new(),
        }
    }

    /// Returns amount of time context spent on rendering all sound sources.
    pub fn full_render_duration(&self) -> Duration {
        self.native.state().full_render_duration()
    }

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
                    .with_opt_buffer(sound.buffer().clone())
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
        }
    }
}
