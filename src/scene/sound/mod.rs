//! Everything related to sound in the engine.

use bitflags::bitflags;
use fyrox_sound::source::SoundSource;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    time::Duration,
};

pub mod context;
pub mod effect;
pub mod listener;

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    define_with,
    scene::base::Base,
    scene::base::BaseBuilder,
    scene::graph::Graph,
    scene::node::Node,
};

// Re-export some the fyrox_sound entities.
pub use fyrox_sound::{
    buffer::{DataSource, SoundBufferResource, SoundBufferResourceLoadError, SoundBufferState},
    context::DistanceModel,
    engine::SoundEngine,
    error::SoundError,
    renderer::{hrtf::*, Renderer},
    source::Status,
};

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

/// Sound source.
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
    pub(crate) native: Cell<Handle<SoundSource>>,
    #[inspect(skip)]
    #[visit(skip)]
    pub(crate) changes: Cell<SoundChanges>,
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
            changes: Cell::new(SoundChanges::NONE),
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
            changes: Cell::new(SoundChanges::NONE),
        }
    }

    /// Changes buffer of source. Source will continue playing from beginning, old
    /// position will be discarded.
    pub fn set_buffer(&mut self, buffer: Option<SoundBufferResource>) {
        self.buffer = buffer;
        self.changes.get_mut().insert(SoundChanges::BUFFER);
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
        self.changes.get_mut().insert(SoundChanges::PLAY_ONCE);
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
        self.changes.get_mut().insert(SoundChanges::GAIN);
    }

    /// Returns current gain (volume) of sound. Value is in 0..1 range.
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Sets panning coefficient. Value must be in -1..+1 range. Where -1 - only left channel will be audible,
    /// 0 - both, +1 - only right.
    pub fn set_panning(&mut self, panning: f32) {
        self.panning = panning.max(-1.0).min(1.0);
        self.changes.get_mut().insert(SoundChanges::PANNING);
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
        self.changes.get_mut().insert(SoundChanges::STATUS);
    }

    /// Changes status to `Paused`
    pub fn pause(&mut self) {
        self.status = Status::Paused;
        self.changes.get_mut().insert(SoundChanges::STATUS);
    }

    /// Enabled or disables sound looping. Looping sound will never stop by itself, but can be stopped or paused
    /// by calling `stop` or `pause` methods. Useful for music, ambient sounds, etc.
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
        self.changes.get_mut().insert(SoundChanges::LOOPING);
    }

    /// Returns looping status.
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Sets sound pitch. Defines "tone" of sounds. Default value is 1.0
    pub fn set_pitch(&mut self, pitch: f64) {
        self.pitch = pitch.abs();
        self.changes.get_mut().insert(SoundChanges::PITCH);
    }

    /// Returns pitch of sound source.
    pub fn pitch(&self) -> f64 {
        self.pitch
    }

    /// Stops sound source. Automatically rewinds streaming buffers.
    pub fn stop(&mut self) {
        self.status = Status::Stopped;
        self.changes.get_mut().insert(SoundChanges::STATUS);
    }

    /// Returns playback duration.
    pub fn playback_time(&self) -> Duration {
        self.playback_time
    }

    /// Sets playback duration.
    pub fn set_playback_time(&mut self, time: Duration) {
        self.playback_time = time;
        self.changes.get_mut().insert(SoundChanges::PLAYBACK_TIME);
    }

    /// Sets radius of imaginable sphere around source in which no distance attenuation is applied.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
        self.changes.get_mut().insert(SoundChanges::RADIUS);
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
        self.changes.get_mut().insert(SoundChanges::ROLLOFF_FACTOR);
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
        self.changes.get_mut().insert(SoundChanges::MAX_DISTANCE);
    }

    /// Returns max distance.
    pub fn max_distance(&self) -> f32 {
        self.max_distance
    }
}

/// Sound builder, allows you to create a new [`Sound`] instance.
pub struct SoundBuilder {
    base_builder: BaseBuilder,
    buffer: Option<SoundBufferResource>,
    play_once: bool,
    gain: f32,
    panning: f32,
    status: Status,
    looping: bool,
    pitch: f64,
    radius: f32,
    max_distance: f32,
    rolloff_factor: f32,
    playback_time: Duration,
}

impl SoundBuilder {
    /// Creates new sound builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
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
        }
    }

    define_with!(
        /// Sets desired buffer. See [`Sound::set_buffer`] for more info.
        fn with_buffer(buffer: Option<SoundBufferResource>)
    );

    define_with!(
        /// Sets play-once mode. See [`Sound::set_play_once`] for more info.
        fn with_play_once(play_once: bool)
    );

    define_with!(
        /// Sets desired gain. See [`Sound::set_gain`] for more info.
        fn with_gain(gain: f32)
    );

    define_with!(
        /// Sets desired panning. See [`Sound::set_panning`] for more info.
        fn with_panning(panning: f32)
    );

    define_with!(
        /// Sets desired status. See [`Sound::play`], [`Sound::stop`], [`Sound::stop`] for more info.
        fn with_status(status: Status)
    );

    define_with!(
        /// Sets desired looping. See [`Sound::set_looping`] for more info.
        fn with_looping(looping: bool)
    );

    define_with!(
        /// Sets desired pitch. See [`Sound::set_pitch`] for more info.
        fn with_pitch(pitch: f64)
    );

    define_with!(
        /// Sets desired radius. See [`Sound::set_radius`] for more info.
        fn with_radius(radius: f32)
    );

    define_with!(
        /// Sets desired max distance. See [`Sound::set_max_distance`] for more info.
        fn with_max_distance(max_distance: f32)
    );

    define_with!(
        /// Sets desired rolloff factor. See [`Sound::set_rolloff_factor`] for more info.
        fn with_rolloff_factor(rolloff_factor: f32)
    );

    define_with!(
        /// Sets desired playback time. See [`Sound::set_playback_time`] for more info.
        fn with_playback_time(playback_time: Duration)
    );

    /// Creates a new [`Sound`] node.
    #[must_use]
    pub fn build_node(self) -> Node {
        Node::Sound(Sound {
            base: self.base_builder.build_base(),
            buffer: self.buffer,
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
            native: Default::default(),
            changes: Cell::new(SoundChanges::NONE),
        })
    }

    /// Create a new [`Sound`] node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
