//! Everything related to sound in the engine.

use crate::core::{
    inspect::{Inspect, PropertyInfo},
    pool::Handle,
    visitor::prelude::*,
};
use bitflags::bitflags;
use fyrox_sound::{buffer::SoundBufferResource, source::SoundSource};
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::scene::base::Base;
pub use fyrox_sound::source::Status;

bitflags! {
    pub(crate) struct SoundChanges: u32 {
        const NONE = 0;
        const GAIN = 0b0000_0001;
        const PANNING = 0b0000_0010;
        const LOOPING = 0b0000_0100;
        const PITCH = 0b0000_1000;
        const RADIUS = 0b0001_0000;
        const MAX_DISTANCE = 0b0010_0000;
        const ROLLOFF_FACTOR = 0b0100_0000;
        const PLAYBACK_TIME = 0b1000_0000;
        const BUFFER = 0b0001_0000_0000;
        const PLAY_ONCE = 0b0010_0000_0000;
        const STATUS = 0b0100_0000_0000;
    }
}

#[derive(Visit, Inspect, Debug)]
pub struct Sound {
    base: Base,
    buffer: Option<SoundBufferResource>,
    play_once: bool,
    #[inspect(min_value = 0.0, step = 0.05)]
    gain: f32,
    #[inspect(min_value = -1.0, max_value = 1.0, step = 0.05)]
    panning: f32,
    status: Status,
    looping: bool,
    #[inspect(min_value = 0.0, step = 0.05)]
    pitch: f64,
    #[inspect(min_value = 0.0, step = 0.05)]
    radius: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    max_distance: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    rolloff_factor: f32,
    playback_time: Duration,
    #[inspect(skip)]
    #[visit(skip)]
    pub(crate) native: Handle<SoundSource>,
    #[inspect(skip)]
    #[visit(skip)]
    pub(crate) changes: SoundChanges,
}

impl Deref for Sound {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Sound {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            base: Default::default(),
            buffer: None,
            play_once: false,
            gain: 1.0,
            panning: 0.0,
            status: Status::Stopped,
            looping: false,
            pitch: 1.0,
            radius: 10.0,
            max_distance: f32::MAX,
            rolloff_factor: 1.0,
            playback_time: Default::default(),
            native: Default::default(),
            changes: SoundChanges::NONE,
        }
    }
}

impl Sound {
    /// Creates a raw copy of this node. For internal use only.
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            buffer: self.buffer.clone(),
            play_once: self.play_once,
            gain: self.gain,
            panning: self.panning,
            status: self.status,
            looping: self.looping,
            pitch: self.pitch,
            radius: self.radius,
            max_distance: self.max_distance,
            rolloff_factor: self.rolloff_factor,
            playback_time: self.playback_time,
            // Do not copy.
            native: Default::default(),
            changes: SoundChanges::NONE,
        }
    }

    /// Changes buffer of source. Source will continue playing from beginning, old
    /// position will be discarded.
    pub fn set_buffer(&mut self, buffer: Option<SoundBufferResource>) {
        self.buffer = buffer;
        self.changes.insert(SoundChanges::BUFFER);
    }

    /// Returns current buffer if any.
    pub fn buffer(&self) -> Option<SoundBufferResource> {
        self.buffer.clone()
    }

    /// Marks buffer for single play. It will be automatically destroyed when it will finish playing.
    ///
    /// # Notes
    ///
    /// Make sure you not using handles to "play once" sounds, attempt to get reference of "play once" sound
    /// may result in panic if source already deleted. Looping sources will never be automatically deleted
    /// because their playback never stops.
    pub fn set_play_once(&mut self, play_once: bool) {
        self.play_once = play_once;
        self.changes.insert(SoundChanges::PLAY_ONCE);
    }

    /// Returns true if this source is marked for single play, false - otherwise.
    pub fn is_play_once(&self) -> bool {
        self.play_once
    }

    /// Sets new gain (volume) of sound. Value should be in 0..1 range, but it is not clamped
    /// and larger values can be used to "overdrive" sound.
    ///
    /// # Notes
    ///
    /// Physical volume has non-linear scale (logarithmic) so perception of sound at 0.25 gain
    /// will be different if logarithmic scale was used.
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
        self.changes.insert(SoundChanges::GAIN);
    }

    /// Returns current gain (volume) of sound. Value is in 0..1 range.
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Sets panning coefficient. Value must be in -1..+1 range. Where -1 - only left channel will be audible,
    /// 0 - both, +1 - only right.
    pub fn set_panning(&mut self, panning: f32) {
        self.panning = panning.max(-1.0).min(1.0);
        self.changes.insert(SoundChanges::PANNING);
    }

    /// Returns current panning coefficient in -1..+1 range. For more info see `set_panning`. Default value is 0.
    pub fn panning(&self) -> f32 {
        self.panning
    }

    /// Returns status of sound source.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Changes status to `Playing`.
    pub fn play(&mut self) {
        self.status = Status::Playing;
        self.changes.insert(SoundChanges::STATUS);
    }

    /// Changes status to `Paused`
    pub fn pause(&mut self) {
        self.status = Status::Paused;
        self.changes.insert(SoundChanges::STATUS);
    }

    /// Enabled or disables sound looping. Looping sound will never stop by itself, but can be stopped or paused
    /// by calling `stop` or `pause` methods. Useful for music, ambient sounds, etc.
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
        self.changes.insert(SoundChanges::LOOPING);
    }

    /// Returns looping status.
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Sets sound pitch. Defines "tone" of sounds. Default value is 1.0
    pub fn set_pitch(&mut self, pitch: f64) {
        self.pitch = pitch.abs();
        self.changes.insert(SoundChanges::PITCH);
    }

    /// Returns pitch of sound source.
    pub fn pitch(&self) -> f64 {
        self.pitch
    }

    /// Stops sound source. Automatically rewinds streaming buffers.
    pub fn stop(&mut self) {
        self.status = Status::Stopped;
        self.changes.insert(SoundChanges::STATUS);
    }

    /// Returns playback duration.
    pub fn playback_time(&self) -> Duration {
        self.playback_time
    }

    /// Sets playback duration.
    pub fn set_playback_time(&mut self, time: Duration) {
        self.playback_time = time;
        self.changes.insert(SoundChanges::PLAYBACK_TIME);
    }

    /// Sets radius of imaginable sphere around source in which no distance attenuation is applied.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
        self.changes.insert(SoundChanges::RADIUS);
    }

    /// Returns radius of source.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets rolloff factor. Rolloff factor is used in distance attenuation and has different meaning
    /// in various distance models. It is applicable only for InverseDistance and ExponentDistance
    /// distance models. See DistanceModel docs for formulae.
    pub fn set_rolloff_factor(&mut self, rolloff_factor: f32) {
        self.rolloff_factor = rolloff_factor;
        self.changes.insert(SoundChanges::ROLLOFF_FACTOR);
    }

    /// Returns rolloff factor.
    pub fn rolloff_factor(&self) -> f32 {
        self.rolloff_factor
    }

    /// Sets maximum distance until which distance gain will be applicable. Basically it doing this
    /// min(max(distance, radius), max_distance) which clamps distance in radius..max_distance range.
    /// From listener's perspective this will sound like source has stopped decreasing its volume even
    /// if distance continue to grow.
    pub fn set_max_distance(&mut self, max_distance: f32) {
        self.max_distance = max_distance;
        self.changes.insert(SoundChanges::MAX_DISTANCE);
    }

    /// Returns max distance.
    pub fn max_distance(&self) -> f32 {
        self.max_distance
    }
}
