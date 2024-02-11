//! Everything related to sound in the engine.

use crate::{
    core::{
        algebra::Matrix4,
        math::{aabb::AxisAlignedBoundingBox, m4x4_approx_eq},
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    define_with,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, SyncContext, UpdateContext},
    },
};

// Re-export some the fyrox_sound entities.
pub use fyrox_sound::{
    buffer::{
        loader::{SoundBufferImportOptions, SoundBufferLoader},
        DataSource, SoundBuffer, SoundBufferResource, SoundBufferResourceLoadError,
    },
    bus::*,
    context::{DistanceModel, SAMPLE_RATE},
    dsp::{filters::*, DelayLine},
    effects::*,
    engine::SoundEngine,
    error::SoundError,
    hrtf::HrirSphere,
    renderer::{hrtf::*, Renderer},
    source::Status,
};

use crate::scene::Scene;
use fyrox_graph::BaseSceneGraph;
use fyrox_resource::state::ResourceState;
use fyrox_sound::source::SoundSource;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    time::Duration,
};

pub mod context;
pub mod listener;

/// Sound source.
#[derive(Visit, Reflect, Debug)]
pub struct Sound {
    base: Base,

    #[reflect(setter = "set_buffer")]
    buffer: InheritableVariable<Option<SoundBufferResource>>,

    #[reflect(setter = "set_play_once")]
    play_once: InheritableVariable<bool>,

    #[reflect(min_value = 0.0, step = 0.05)]
    #[reflect(setter = "set_gain")]
    gain: InheritableVariable<f32>,

    #[reflect(min_value = -1.0, max_value = 1.0, step = 0.05)]
    #[reflect(setter = "set_panning")]
    panning: InheritableVariable<f32>,

    #[reflect(setter = "set_status")]
    pub(crate) status: InheritableVariable<Status>,

    #[reflect(setter = "set_looping")]
    looping: InheritableVariable<bool>,

    #[reflect(min_value = 0.0, step = 0.05)]
    #[reflect(setter = "set_pitch")]
    pitch: InheritableVariable<f64>,

    #[reflect(min_value = 0.0, step = 0.05)]
    #[reflect(setter = "set_radius")]
    radius: InheritableVariable<f32>,

    #[reflect(min_value = 0.0, step = 0.05)]
    #[reflect(setter = "set_max_distance")]
    max_distance: InheritableVariable<f32>,

    #[reflect(min_value = 0.0, step = 0.05)]
    #[reflect(setter = "set_rolloff_factor")]
    rolloff_factor: InheritableVariable<f32>,

    #[visit(optional)]
    #[reflect(setter = "set_playback_time", min_value = 0.0)]
    playback_time: InheritableVariable<f32>,

    #[reflect(setter = "set_spatial_blend")]
    spatial_blend: InheritableVariable<f32>,

    #[visit(optional)]
    #[reflect(
        description = "A name of a sound effect to which the sound will attach to when instantiated."
    )]
    audio_bus: InheritableVariable<String>,

    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) native: Cell<Handle<SoundSource>>,
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
            buffer: InheritableVariable::new_modified(None),
            play_once: InheritableVariable::new_modified(false),
            gain: InheritableVariable::new_modified(1.0),
            panning: InheritableVariable::new_modified(0.0),
            status: InheritableVariable::new_modified(Status::Stopped),
            looping: InheritableVariable::new_modified(false),
            pitch: InheritableVariable::new_modified(1.0),
            radius: InheritableVariable::new_modified(10.0),
            max_distance: InheritableVariable::new_modified(f32::MAX),
            rolloff_factor: InheritableVariable::new_modified(1.0),
            playback_time: Default::default(),
            spatial_blend: InheritableVariable::new_modified(1.0),
            audio_bus: InheritableVariable::new_modified(AudioBusGraph::PRIMARY_BUS.to_string()),
            native: Default::default(),
        }
    }
}

impl Clone for Sound {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            buffer: self.buffer.clone(),
            play_once: self.play_once.clone(),
            gain: self.gain.clone(),
            panning: self.panning.clone(),
            status: self.status.clone(),
            looping: self.looping.clone(),
            pitch: self.pitch.clone(),
            radius: self.radius.clone(),
            max_distance: self.max_distance.clone(),
            rolloff_factor: self.rolloff_factor.clone(),
            playback_time: self.playback_time.clone(),
            spatial_blend: self.spatial_blend.clone(),
            audio_bus: self.audio_bus.clone(),
            // Do not copy. The copy will have its own native representation.
            native: Default::default(),
        }
    }
}

impl TypeUuidProvider for Sound {
    fn type_uuid() -> Uuid {
        uuid!("28621735-8cd1-4fad-8faf-ecd24bf8aa99")
    }
}

impl Sound {
    /// Changes buffer of source. Source will continue playing from beginning, old
    /// position will be discarded.
    pub fn set_buffer(
        &mut self,
        buffer: Option<SoundBufferResource>,
    ) -> Option<SoundBufferResource> {
        self.buffer.set_value_and_mark_modified(buffer)
    }

    /// Returns current buffer if any.
    pub fn buffer(&self) -> Option<SoundBufferResource> {
        (*self.buffer).clone()
    }

    /// Marks buffer for single play. It will be automatically destroyed when it will finish playing.
    ///
    /// # Notes
    ///
    /// Make sure you not using handles to "play once" sounds, attempt to get reference of "play once" sound
    /// may result in panic if source already deleted. Looping sources will never be automatically deleted
    /// because their playback never stops.
    pub fn set_play_once(&mut self, play_once: bool) -> bool {
        self.play_once.set_value_and_mark_modified(play_once)
    }

    /// Returns true if this source is marked for single play, false - otherwise.
    pub fn is_play_once(&self) -> bool {
        *self.play_once
    }

    /// Sets spatial blend factor. It defines how much the source will be 2D and 3D sound at the same
    /// time. Set it to 0.0 to make the sound fully 2D and 1.0 to make it fully 3D. Middle values
    /// will make sound proportionally 2D and 3D at the same time.
    pub fn set_spatial_blend(&mut self, k: f32) -> f32 {
        self.spatial_blend
            .set_value_and_mark_modified(k.clamp(0.0, 1.0))
    }

    /// Returns spatial blend factor.
    pub fn spatial_blend(&self) -> f32 {
        *self.spatial_blend
    }

    /// Sets new gain (volume) of sound. Value should be in 0..1 range, but it is not clamped
    /// and larger values can be used to "overdrive" sound.
    ///
    /// # Notes
    ///
    /// Physical volume has non-linear scale (logarithmic) so perception of sound at 0.25 gain
    /// will be different if logarithmic scale was used.
    pub fn set_gain(&mut self, gain: f32) -> f32 {
        self.gain.set_value_and_mark_modified(gain)
    }

    /// Returns current gain (volume) of sound. Value is in 0..1 range.
    pub fn gain(&self) -> f32 {
        *self.gain
    }

    /// Sets panning coefficient. Value must be in -1..+1 range. Where -1 - only left channel will be audible,
    /// 0 - both, +1 - only right.
    pub fn set_panning(&mut self, panning: f32) -> f32 {
        self.panning
            .set_value_and_mark_modified(panning.clamp(-1.0, 1.0))
    }

    /// Returns current panning coefficient in -1..+1 range. For more info see `set_panning`. Default value is 0.
    pub fn panning(&self) -> f32 {
        *self.panning
    }

    /// Sets playback status.    
    pub fn set_status(&mut self, status: Status) -> Status {
        let prev = self.status();
        match status {
            Status::Stopped => self.stop(),
            Status::Playing => self.play(),
            Status::Paused => self.pause(),
        }
        prev
    }

    /// Returns status of sound source.
    pub fn status(&self) -> Status {
        *self.status
    }

    /// Changes status to `Playing`.
    pub fn play(&mut self) {
        self.status.set_value_and_mark_modified(Status::Playing);
    }

    /// Tries to play a sound. Will succeed, only if the sound is not already playing. Could be useful, if you need
    /// to prevent the sound to start over until it is fully played.
    pub fn try_play(&mut self) -> bool {
        if *self.status == Status::Playing {
            false
        } else {
            self.play();
            true
        }
    }

    /// Changes status to `Paused`
    pub fn pause(&mut self) {
        self.status.set_value_and_mark_modified(Status::Paused);
    }

    /// Enabled or disables sound looping. Looping sound will never stop by itself, but can be stopped or paused
    /// by calling `stop` or `pause` methods. Useful for music, ambient sounds, etc.
    pub fn set_looping(&mut self, looping: bool) -> bool {
        self.looping.set_value_and_mark_modified(looping)
    }

    /// Returns looping status.
    pub fn is_looping(&self) -> bool {
        *self.looping
    }

    /// Sets sound pitch. Defines "tone" of sounds. Default value is 1.0
    pub fn set_pitch(&mut self, pitch: f64) -> f64 {
        self.pitch.set_value_and_mark_modified(pitch.abs())
    }

    /// Returns pitch of sound source.
    pub fn pitch(&self) -> f64 {
        *self.pitch
    }

    /// Stops sound source. Automatically rewinds streaming buffers.
    pub fn stop(&mut self) {
        self.status.set_value_and_mark_modified(Status::Stopped);
    }

    /// Returns playback time.
    pub fn playback_time(&self) -> f32 {
        *self.playback_time
    }

    /// Sets playback duration.
    pub fn set_playback_time(&mut self, time: f32) -> f32 {
        self.playback_time.set_value_and_mark_modified(time)
    }

    /// Sets radius of imaginable sphere around source in which no distance attenuation is applied.
    pub fn set_radius(&mut self, radius: f32) -> f32 {
        self.radius.set_value_and_mark_modified(radius)
    }

    /// Returns radius of source.
    pub fn radius(&self) -> f32 {
        *self.radius
    }

    /// Sets rolloff factor. Rolloff factor is used in distance attenuation and has different meaning
    /// in various distance models. It is applicable only for InverseDistance and ExponentDistance
    /// distance models. See DistanceModel docs for formulae.
    pub fn set_rolloff_factor(&mut self, rolloff_factor: f32) -> f32 {
        self.rolloff_factor
            .set_value_and_mark_modified(rolloff_factor)
    }

    /// Returns rolloff factor.
    pub fn rolloff_factor(&self) -> f32 {
        *self.rolloff_factor
    }

    /// Sets maximum distance until which distance gain will be applicable. Basically it doing this
    /// min(max(distance, radius), max_distance) which clamps distance in radius..max_distance range.
    /// From listener's perspective this will sound like source has stopped decreasing its volume even
    /// if distance continue to grow.
    pub fn set_max_distance(&mut self, max_distance: f32) -> f32 {
        self.max_distance.set_value_and_mark_modified(max_distance)
    }

    /// Returns max distance.
    pub fn max_distance(&self) -> f32 {
        *self.max_distance
    }

    /// Sets new audio bus name to which the sound will be attached.
    pub fn set_audio_bus(&mut self, name: String) {
        self.audio_bus.set_value_and_mark_modified(name);
    }

    /// Returns the name of an audio bus to which the sound is attached.
    pub fn audio_bus(&self) -> &str {
        &self.audio_bus
    }
}

impl NodeTrait for Sound {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::unit()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn on_removed_from_graph(&mut self, graph: &mut Graph) {
        graph
            .sound_context
            .remove_sound(self.native.get(), &self.name);
        self.native.set(Default::default());
    }

    fn sync_native(&self, _self_handle: Handle<Node>, context: &mut SyncContext) {
        context.sound_context.sync_to_sound(
            _self_handle,
            self,
            context.switches.and_then(|s| s.node_overrides.as_ref()),
        )
    }

    fn sync_transform(&self, new_global_transform: &Matrix4<f32>, context: &mut SyncContext) {
        if !m4x4_approx_eq(new_global_transform, &self.global_transform()) {
            context.sound_context.set_sound_position(self);
        }
    }

    fn is_alive(&self) -> bool {
        if self.is_play_once() {
            self.status() != Status::Stopped
        } else {
            true
        }
    }

    fn update(&mut self, context: &mut UpdateContext) {
        context.sound_context.sync_with_sound(self);
    }

    fn validate(&self, _scene: &Scene) -> Result<(), String> {
        match self.buffer.as_ref() {
            Some(buffer) => {
                let header = buffer.header();
                match header.state {
                    ResourceState::Pending { .. } | ResourceState::Ok(_) => Ok(()),
                    ResourceState::LoadError { ref error, .. } => {
                        match &error.0 {
                            None => Err("Sound buffer is failed to load, the reason is unknown!"
                                .to_string()),
                            Some(err) => {
                                Err(format!("Sound buffer is failed to load. Reason: {:?}", err))
                            }
                        }
                    }
                }
            }
            None => Err("Sound buffer is not set, the sound won't play!".to_string()),
        }
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
    spatial_blend: f32,
    audio_bus: String,
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
            spatial_blend: 1.0,
            playback_time: Default::default(),
            audio_bus: AudioBusGraph::PRIMARY_BUS.to_string(),
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
        /// Sets desired spatial blend factor. See [`Sound::set_spatial_blend`] for more info.
        fn with_spatial_blend_factor(spatial_blend: f32)
    );

    define_with!(
        /// Sets desired playback time. See [`Sound::set_playback_time`] for more info.
        fn with_playback_time(playback_time: Duration)
    );

    define_with!(
        /// Sets desired playback time. See [`Sound::set_audio_bus`] for more info.
        fn with_audio_bus(audio_bus: String)
    );

    /// Creates a new [`Sound`] node.
    #[must_use]
    pub fn build_sound(self) -> Sound {
        Sound {
            base: self.base_builder.build_base(),
            buffer: self.buffer.into(),
            play_once: self.play_once.into(),
            gain: self.gain.into(),
            panning: self.panning.into(),
            status: self.status.into(),
            looping: self.looping.into(),
            pitch: self.pitch.into(),
            radius: self.radius.into(),
            max_distance: self.max_distance.into(),
            rolloff_factor: self.rolloff_factor.into(),
            playback_time: self.playback_time.as_secs_f32().into(),
            spatial_blend: self.spatial_blend.into(),
            audio_bus: self.audio_bus.into(),
            native: Default::default(),
        }
    }

    /// Creates a new [`Sound`] node.
    #[must_use]
    pub fn build_node(self) -> Node {
        Node::new(self.build_sound())
    }

    /// Create a new [`Sound`] node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
