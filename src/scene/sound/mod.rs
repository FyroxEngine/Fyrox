//! Everything related to sound in the engine.

use crate::{
    core::{
        algebra::Matrix4,
        inspect::{Inspect, PropertyInfo},
        math::{aabb::AxisAlignedBoundingBox, m4x4_approx_eq},
        pool::Handle,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    define_with,
    engine::resource_manager::ResourceManager,
    impl_directly_inheritable_entity_trait,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, SyncContext, TypeUuidProvider, UpdateContext},
        variable::{InheritError, TemplateVariable},
        DirectlyInheritableEntity,
    },
    utils::log::Log,
};

// Re-export some the fyrox_sound entities.
pub use fyrox_sound::{
    buffer::{DataSource, SoundBufferResource, SoundBufferResourceLoadError, SoundBufferState},
    context::{DistanceModel, SAMPLE_RATE},
    dsp::{filters::*, DelayLine},
    engine::SoundEngine,
    error::SoundError,
    hrtf::HrirSphere,
    renderer::{hrtf::HrtfRenderer, Renderer},
    source::Status,
};

use fxhash::FxHashMap;
use fyrox_sound::source::SoundSource;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    time::Duration,
};

pub mod context;
pub mod effect;
pub mod listener;

/// Sound source.
#[derive(Visit, Inspect, Debug)]
pub struct Sound {
    base: Base,
    #[inspect(getter = "Deref::deref")]
    buffer: TemplateVariable<Option<SoundBufferResource>>,
    #[inspect(getter = "Deref::deref")]
    play_once: TemplateVariable<bool>,
    #[inspect(min_value = 0.0, step = 0.05, getter = "Deref::deref")]
    gain: TemplateVariable<f32>,
    #[inspect(min_value = -1.0, max_value = 1.0, step = 0.05, getter = "Deref::deref")]
    panning: TemplateVariable<f32>,
    #[inspect(getter = "Deref::deref")]
    pub(crate) status: TemplateVariable<Status>,
    #[inspect(getter = "Deref::deref")]
    looping: TemplateVariable<bool>,
    #[inspect(min_value = 0.0, step = 0.05, getter = "Deref::deref")]
    pitch: TemplateVariable<f64>,
    #[inspect(min_value = 0.0, step = 0.05, getter = "Deref::deref")]
    radius: TemplateVariable<f32>,
    #[inspect(min_value = 0.0, step = 0.05, getter = "Deref::deref")]
    max_distance: TemplateVariable<f32>,
    #[inspect(min_value = 0.0, step = 0.05, getter = "Deref::deref")]
    rolloff_factor: TemplateVariable<f32>,
    #[inspect(getter = "Deref::deref")]
    playback_time: TemplateVariable<Duration>,
    #[inspect(getter = "Deref::deref")]
    spatial_blend: TemplateVariable<f32>,
    #[inspect(skip)]
    #[visit(skip)]
    pub(crate) native: Cell<Handle<SoundSource>>,
}

impl_directly_inheritable_entity_trait!(Sound;
    status,
    buffer,
    play_once,
    gain,
    panning,
    looping,
    pitch,
    radius,
    max_distance,
    rolloff_factor,
    playback_time
);

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
            buffer: TemplateVariable::new(None),
            play_once: TemplateVariable::new(false),
            gain: TemplateVariable::new(1.0),
            panning: TemplateVariable::new(0.0),
            status: TemplateVariable::new(Status::Stopped),
            looping: TemplateVariable::new(false),
            pitch: TemplateVariable::new(1.0),
            radius: TemplateVariable::new(10.0),
            max_distance: TemplateVariable::new(f32::MAX),
            rolloff_factor: TemplateVariable::new(1.0),
            playback_time: Default::default(),
            spatial_blend: TemplateVariable::new(1.0),
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
            // Do not copy.
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
    pub fn set_buffer(&mut self, buffer: Option<SoundBufferResource>) {
        self.buffer.set(buffer);
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
    pub fn set_play_once(&mut self, play_once: bool) {
        self.play_once.set(play_once);
    }

    /// Returns true if this source is marked for single play, false - otherwise.
    pub fn is_play_once(&self) -> bool {
        *self.play_once
    }

    /// Sets spatial blend factor. It defines how much the source will be 2D and 3D sound at the same
    /// time. Set it to 0.0 to make the sound fully 2D and 1.0 to make it fully 3D. Middle values
    /// will make sound proportionally 2D and 3D at the same time.
    pub fn set_spatial_blend(&mut self, k: f32) {
        self.spatial_blend.set(k.clamp(0.0, 1.0));
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
    pub fn set_gain(&mut self, gain: f32) {
        self.gain.set(gain);
    }

    /// Returns current gain (volume) of sound. Value is in 0..1 range.
    pub fn gain(&self) -> f32 {
        *self.gain
    }

    /// Sets panning coefficient. Value must be in -1..+1 range. Where -1 - only left channel will be audible,
    /// 0 - both, +1 - only right.
    pub fn set_panning(&mut self, panning: f32) {
        self.panning.set(panning.max(-1.0).min(1.0));
    }

    /// Returns current panning coefficient in -1..+1 range. For more info see `set_panning`. Default value is 0.
    pub fn panning(&self) -> f32 {
        *self.panning
    }

    /// Sets playback status.    
    pub fn set_status(&mut self, status: Status) {
        match status {
            Status::Stopped => self.stop(),
            Status::Playing => self.play(),
            Status::Paused => self.pause(),
        }
    }

    /// Returns status of sound source.
    pub fn status(&self) -> Status {
        *self.status
    }

    /// Changes status to `Playing`.
    pub fn play(&mut self) {
        self.status.set(Status::Playing);
    }

    /// Changes status to `Paused`
    pub fn pause(&mut self) {
        self.status.set(Status::Paused);
    }

    /// Enabled or disables sound looping. Looping sound will never stop by itself, but can be stopped or paused
    /// by calling `stop` or `pause` methods. Useful for music, ambient sounds, etc.
    pub fn set_looping(&mut self, looping: bool) {
        self.looping.set(looping);
    }

    /// Returns looping status.
    pub fn is_looping(&self) -> bool {
        *self.looping
    }

    /// Sets sound pitch. Defines "tone" of sounds. Default value is 1.0
    pub fn set_pitch(&mut self, pitch: f64) {
        self.pitch.set(pitch.abs());
    }

    /// Returns pitch of sound source.
    pub fn pitch(&self) -> f64 {
        *self.pitch
    }

    /// Stops sound source. Automatically rewinds streaming buffers.
    pub fn stop(&mut self) {
        self.status.set(Status::Stopped);
    }

    /// Returns playback duration.
    pub fn playback_time(&self) -> Duration {
        *self.playback_time
    }

    /// Sets playback duration.
    pub fn set_playback_time(&mut self, time: Duration) {
        self.playback_time.set(time);
    }

    /// Sets radius of imaginable sphere around source in which no distance attenuation is applied.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius.set(radius);
    }

    /// Returns radius of source.
    pub fn radius(&self) -> f32 {
        *self.radius
    }

    /// Sets rolloff factor. Rolloff factor is used in distance attenuation and has different meaning
    /// in various distance models. It is applicable only for InverseDistance and ExponentDistance
    /// distance models. See DistanceModel docs for formulae.
    pub fn set_rolloff_factor(&mut self, rolloff_factor: f32) {
        self.rolloff_factor.set(rolloff_factor);
    }

    /// Returns rolloff factor.
    pub fn rolloff_factor(&self) -> f32 {
        *self.rolloff_factor
    }

    /// Sets maximum distance until which distance gain will be applicable. Basically it doing this
    /// min(max(distance, radius), max_distance) which clamps distance in radius..max_distance range.
    /// From listener's perspective this will sound like source has stopped decreasing its volume even
    /// if distance continue to grow.
    pub fn set_max_distance(&mut self, max_distance: f32) {
        self.max_distance.set(max_distance);
    }

    /// Returns max distance.
    pub fn max_distance(&self) -> f32 {
        *self.max_distance
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

    // Prefab inheritance resolving.
    fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        self.base.inherit_properties(parent)?;
        if let Some(parent) = parent.cast::<Sound>() {
            self.try_inherit_self_properties(parent)?;
        }
        Ok(())
    }

    fn reset_inheritable_properties(&mut self) {
        self.base.reset_inheritable_properties();
        self.reset_self_inheritable_properties();
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        if let Some(buffer) = self.buffer() {
            let state = buffer.state();
            self.set_buffer(Some(resource_manager.request_sound_buffer(state.path())));
        }
    }

    fn remap_handles(&mut self, old_new_mapping: &FxHashMap<Handle<Node>, Handle<Node>>) {
        self.base.remap_handles(old_new_mapping);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn clean_up(&mut self, graph: &mut Graph) {
        graph.sound_context.remove_sound(self.native.get());

        Log::info(format!(
            "Native sound source was removed for node: {}",
            self.name()
        ));
    }

    fn sync_native(&self, _self_handle: Handle<Node>, context: &mut SyncContext) {
        context.sound_context.sync_to_sound(self)
    }

    fn sync_transform(&self, new_global_transform: &Matrix4<f32>, context: &mut SyncContext) {
        if !m4x4_approx_eq(new_global_transform, &self.global_transform()) {
            context.sound_context.set_sound_position(self);
        }
    }

    fn update(&mut self, context: &mut UpdateContext) -> bool {
        context.sound_context.sync_with_sound(self);

        self.base.update_lifetime(context.dt)
            && !(self.is_play_once() && self.status() == Status::Stopped)
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
            playback_time: self.playback_time.into(),
            spatial_blend: self.spatial_blend.into(),
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

#[cfg(test)]
mod test {
    use crate::scene::{
        base::{test::check_inheritable_properties_equality, BaseBuilder},
        node::NodeTrait,
        sound::{Sound, SoundBuilder},
    };
    use fyrox_sound::source::Status;
    use std::time::Duration;

    #[test]
    fn test_sound_inheritance() {
        let parent = SoundBuilder::new(BaseBuilder::new())
            .with_radius(1.0)
            .with_gain(2.0)
            .with_status(Status::Paused)
            .with_pitch(2.0)
            .with_playback_time(Duration::from_secs(2))
            .with_looping(true)
            .with_play_once(true)
            .with_panning(0.1)
            .build_node();

        let mut child = SoundBuilder::new(BaseBuilder::new()).build_sound();

        child.inherit(&parent).unwrap();

        let parent = parent.cast::<Sound>().unwrap();

        check_inheritable_properties_equality(&child.base, &parent.base);
        check_inheritable_properties_equality(&child, parent);
    }
}
