//! Context module.
//!
//! # Overview
//!
//! Context is a sort of "sound scene" - an isolated storage for a set of sound sources, effects, filters, etc.
//! fyrox-sound can manage multiple contexts at the same time. Main usage for multiple contexts is a typical
//! situation in games where you have multiple scenes: a scene for main menu, a scene for game level, a scene
//! for inventory and so on. With this approach of multiple contexts it is very easy to manage such scenes:
//! for example your main menu have a complex scene with some sounds and you decide to load a game level -
//! once the level is loaded you just set master gain of main menu context and it will no longer produce any
//! sounds, only your level will do.

use crate::bus::AudioBusGraph;
use crate::{
    listener::Listener,
    pool::Ticket,
    renderer::{render_source_default, Renderer},
    source::{SoundSource, Status},
};
use fyrox_core::{
    pool::{Handle, Pool},
    reflect::prelude::*,
    uuid_provider,
    visitor::prelude::*,
};
use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Sample rate for output device.
/// TODO: Make this configurable, for now its set to most commonly used sample rate of 44100 Hz.
pub const SAMPLE_RATE: u32 = 44100;

/// Distance model defines how volume of sound will decay when distance to listener changes.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Reflect, Visit, AsRefStr, EnumString, VariantNames)]
#[repr(u32)]
pub enum DistanceModel {
    /// No distance attenuation at all.
    None = 0,

    /// Distance will decay using following formula:
    ///
    /// `clamped_distance = min(max(distance, radius), max_distance)`
    /// `attenuation = radius / (radius + rolloff_factor * (clamped_distance - radius))`
    ///
    /// where - `radius` - of source at which it has maximum volume,
    ///         `max_distance` - distance at which decay will stop,
    ///         `rolloff_factor` - coefficient that defines how fast volume will decay
    ///
    /// # Notes
    ///
    /// This is default distance model of context.
    InverseDistance = 1,

    /// Distance will decay using following formula:
    ///
    /// `clamped_distance = min(max(distance, radius), max_distance)`
    /// `attenuation = 1.0 - radius * (clamped_distance - radius) / (max_distance - radius)`
    ///
    /// where - `radius` - of source at which it has maximum volume,
    ///         `max_distance` - distance at which decay will stop
    ///
    /// # Notes
    ///
    /// As you can see `rolloff_factor` is ignored here because of linear law.
    LinearDistance = 2,

    /// Distance will decay using following formula:
    ///
    /// `clamped_distance = min(max(distance, radius), max_distance)`
    /// `(clamped_distance / radius) ^ (-rolloff_factor)`
    ///
    /// where - `radius` - of source at which it has maximum volume,
    ///         `max_distance` - distance at which decay will stop,
    ///         `rolloff_factor` - coefficient that defines how fast volume will decay
    ExponentDistance = 3,
}

uuid_provider!(DistanceModel = "957f3b00-3f89-438c-b1b7-e841e8d75ba9");

impl Default for DistanceModel {
    fn default() -> Self {
        Self::InverseDistance
    }
}

/// See module docs.
#[derive(Clone, Default, Debug, Visit)]
pub struct SoundContext {
    pub(crate) state: Option<Arc<Mutex<State>>>,
}

impl PartialEq for SoundContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(self.state.as_ref().unwrap(), other.state.as_ref().unwrap())
    }
}

/// A set of flags, that can be used to define what should be skipped during the
/// serialization of a sound context.
#[derive(Default, Debug, Clone)]
pub struct SerializationOptions {
    /// All sources won't be serialized, if set.
    pub skip_sources: bool,
    /// Bus graph won't be serialized, if set.
    pub skip_bus_graph: bool,
}

/// Internal state of context.
#[derive(Default, Debug, Clone, Reflect)]
pub struct State {
    sources: Pool<SoundSource>,
    listener: Listener,
    render_duration: Duration,
    renderer: Renderer,
    bus_graph: AudioBusGraph,
    distance_model: DistanceModel,
    paused: bool,
    /// A set of flags, that can be used to define what should be skipped during the
    /// serialization of a sound context.
    #[reflect(hidden)]
    pub serialization_options: SerializationOptions,
}

impl State {
    /// Extracts a source from the context and reserves its handle. It is used to temporarily take
    /// ownership over source, and then put node back using given ticket.
    pub fn take_reserve(
        &mut self,
        handle: Handle<SoundSource>,
    ) -> (Ticket<SoundSource>, SoundSource) {
        self.sources.take_reserve(handle)
    }

    /// Puts source back by given ticket.
    pub fn put_back(
        &mut self,
        ticket: Ticket<SoundSource>,
        node: SoundSource,
    ) -> Handle<SoundSource> {
        self.sources.put_back(ticket, node)
    }

    /// Makes source handle vacant again.
    pub fn forget_ticket(&mut self, ticket: Ticket<SoundSource>) {
        self.sources.forget_ticket(ticket)
    }

    /// Pause/unpause the sound context. Paused context won't play any sounds.
    pub fn pause(&mut self, pause: bool) {
        self.paused = pause;
    }

    /// Returns true if the sound context is paused, false - otherwise.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Sets new distance model.
    pub fn set_distance_model(&mut self, distance_model: DistanceModel) {
        self.distance_model = distance_model;
    }

    /// Returns current distance model.
    pub fn distance_model(&self) -> DistanceModel {
        self.distance_model
    }

    /// Normalizes given frequency using context's sampling rate. Normalized frequency then can be used
    /// to create filters.
    pub fn normalize_frequency(&self, f: f32) -> f32 {
        f / SAMPLE_RATE as f32
    }

    /// Returns amount of time context spent on rendering all sound sources.
    pub fn full_render_duration(&self) -> Duration {
        self.render_duration
    }

    /// Sets new renderer.
    pub fn set_renderer(&mut self, renderer: Renderer) -> Renderer {
        std::mem::replace(&mut self.renderer, renderer)
    }

    /// Returns shared reference to current renderer.
    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    /// Returns mutable reference to current renderer.
    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    /// Adds new sound source and returns handle of it by which it can be accessed later on.
    pub fn add_source(&mut self, source: SoundSource) -> Handle<SoundSource> {
        self.sources.spawn(source)
    }

    /// Removes sound source from the context.
    pub fn remove_source(&mut self, source: Handle<SoundSource>) {
        self.sources.free(source);
    }

    /// Returns shared reference to a pool with all sound sources.
    pub fn sources(&self) -> &Pool<SoundSource> {
        &self.sources
    }

    /// Returns mutable reference to a pool with all sound sources.
    pub fn sources_mut(&mut self) -> &mut Pool<SoundSource> {
        &mut self.sources
    }

    /// Returns shared reference to sound source at given handle. If handle is invalid, this method will panic.
    pub fn source(&self, handle: Handle<SoundSource>) -> &SoundSource {
        self.sources.borrow(handle)
    }

    /// Checks whether a handle to a sound source is valid or not.
    pub fn is_valid_handle(&self, handle: Handle<SoundSource>) -> bool {
        self.sources.is_valid_handle(handle)
    }

    /// Returns mutable reference to sound source at given handle. If handle is invalid, this method will panic.
    pub fn source_mut(&mut self, handle: Handle<SoundSource>) -> &mut SoundSource {
        self.sources.borrow_mut(handle)
    }

    /// Returns mutable reference to sound source at given handle. If handle is invalid, this method will panic.
    pub fn try_get_source_mut(&mut self, handle: Handle<SoundSource>) -> Option<&mut SoundSource> {
        self.sources.try_borrow_mut(handle)
    }

    /// Returns shared reference to listener. Engine has only one listener.
    pub fn listener(&self) -> &Listener {
        &self.listener
    }

    /// Returns mutable reference to listener. Engine has only one listener.
    pub fn listener_mut(&mut self) -> &mut Listener {
        &mut self.listener
    }

    /// Returns a reference to the audio bus graph.
    pub fn bus_graph_ref(&self) -> &AudioBusGraph {
        &self.bus_graph
    }

    /// Returns a reference to the audio bus graph.
    pub fn bus_graph_mut(&mut self) -> &mut AudioBusGraph {
        &mut self.bus_graph
    }

    pub(crate) fn render(&mut self, output_device_buffer: &mut [(f32, f32)]) {
        let last_time = fyrox_core::instant::Instant::now();

        if !self.paused {
            self.sources.retain(|source| {
                let done = source.is_play_once() && source.status() == Status::Stopped;
                !done
            });

            self.bus_graph.begin_render(output_device_buffer.len());

            // Render sounds to respective audio buses.
            for source in self
                .sources
                .iter_mut()
                .filter(|s| s.status() == Status::Playing)
            {
                if let Some(bus_input_buffer) = self.bus_graph.try_get_bus_input_buffer(&source.bus)
                {
                    source.render(output_device_buffer.len());

                    match self.renderer {
                        Renderer::Default => {
                            // Simple rendering path. Much faster (4-5 times) than HRTF path.
                            render_source_default(
                                source,
                                &self.listener,
                                self.distance_model,
                                bus_input_buffer,
                            );
                        }
                        Renderer::HrtfRenderer(ref mut hrtf_renderer) => {
                            hrtf_renderer.render_source(
                                source,
                                &self.listener,
                                self.distance_model,
                                bus_input_buffer,
                            );
                        }
                    }
                }
            }

            self.bus_graph.end_render(output_device_buffer);
        }

        self.render_duration = fyrox_core::instant::Instant::now() - last_time;
    }
}

impl SoundContext {
    /// TODO: This is magic constant that gives 1024 + 1 number when summed with
    ///       HRTF length for faster FFT calculations. Find a better way of selecting this.
    pub const HRTF_BLOCK_LEN: usize = 513;

    pub(crate) const HRTF_INTERPOLATION_STEPS: usize = 4;

    pub(crate) const SAMPLES_PER_CHANNEL: usize =
        Self::HRTF_BLOCK_LEN * Self::HRTF_INTERPOLATION_STEPS;

    /// Creates new instance of context. Internally context starts new thread which will call render all
    /// sound source and send samples to default output device. This method returns `Arc<Mutex<Context>>`
    /// because separate thread also uses context.
    pub fn new() -> Self {
        Self {
            state: Some(Arc::new(Mutex::new(State {
                sources: Pool::new(),
                listener: Listener::new(),
                render_duration: Default::default(),
                renderer: Renderer::Default,
                bus_graph: AudioBusGraph::new(),
                distance_model: DistanceModel::InverseDistance,
                paused: false,
                serialization_options: Default::default(),
            }))),
        }
    }

    /// Returns internal state of the context.
    ///
    /// ## Deadlocks
    ///
    /// This method internally locks a mutex, so if you'll try to do something like this:
    ///
    /// ```no_run
    /// # use fyrox_sound::context::SoundContext;
    /// # let ctx = SoundContext::new();
    /// let state = ctx.state();
    /// // Do something
    /// // ...
    /// ctx.state(); // This will cause a deadlock.
    /// ```
    ///
    /// You'll get a deadlock, so general rule here is to not store result of this method
    /// anywhere.
    pub fn state(&self) -> MutexGuard<'_, State> {
        self.state.as_ref().unwrap().lock().unwrap()
    }

    /// Creates deep copy instead of shallow which is done by clone().
    pub fn deep_clone(&self) -> SoundContext {
        SoundContext {
            state: Some(Arc::new(Mutex::new(self.state().clone()))),
        }
    }

    /// Returns true if context is corrupted.
    pub fn is_invalid(&self) -> bool {
        self.state.is_none()
    }
}

impl Visit for State {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.is_reading() {
            self.sources.clear();
            self.renderer = Renderer::Default;
        }

        let mut region = visitor.enter_region(name)?;

        self.listener.visit("Listener", &mut region)?;
        if !self.serialization_options.skip_sources {
            let _ = self.sources.visit("Sources", &mut region);
        }
        if !self.serialization_options.skip_bus_graph {
            let _ = self.bus_graph.visit("BusGraph", &mut region);
        }
        self.renderer.visit("Renderer", &mut region)?;
        self.paused.visit("Paused", &mut region)?;
        self.distance_model.visit("DistanceModel", &mut region)?;

        Ok(())
    }
}
