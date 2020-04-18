//! Context module.
//!
//! # Overview
//!
//! Context holds all sound sources, feeds renderer with samples, applies effects, applies master gain, etc.
//!

use std::{
    sync::{
        Arc,
        Mutex,
    },
    time::{
        self,
        Duration,
    },
};
use crate::{
    error::SoundError,
    device::run_device,
    listener::Listener,
    source::{
        Status,
        SoundSource,
    },
    renderer::{
        Renderer,
        render_source_default,
    },
    effects::{
        Effect,
        EffectRenderTrait,
    },
    device,
};
use rg3d_core::{
    pool::{Pool, Handle},
    visitor::{Visit, VisitResult, Visitor},
};

/// Distance model defines how volume of sound will decay when distance to listener changes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DistanceModel {
    /// No distance attenuation at all.
    None,

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
    InverseDistance,

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
    LinearDistance,

    /// Distance will decay using following formula:
    ///
    /// `clamped_distance = min(max(distance, radius), max_distance)`
    /// `(clamped_distance / radius) ^ (-rolloff_factor)`
    ///
    /// where - `radius` - of source at which it has maximum volume,
    ///         `max_distance` - distance at which decay will stop,
    ///         `rolloff_factor` - coefficient that defines how fast volume will decay
    ExponentDistance,
}

/// See module docs.
pub struct Context {
    sources: Pool<SoundSource>,
    listener: Listener,
    master_gain: f32,
    render_duration: Duration,
    renderer: Renderer,
    effects: Pool<Effect>,
    distance_model: DistanceModel,
}

impl Context {
    // TODO: This is magic constant that gives 1024 + 1 number when summed with
    //       HRTF length for faster FFT calculations. Find a better way of selecting this.
    pub(in crate) const HRTF_BLOCK_LEN: usize = 513;

    pub(in crate) const HRTF_INTERPOLATION_STEPS: usize = 8;

    pub(in crate) const SAMPLES_PER_CHANNEL: usize = Self::HRTF_BLOCK_LEN * Self::HRTF_INTERPOLATION_STEPS;

    /// Creates new instance of context. Internally context starts new thread which will call render all
    /// sound source and send samples to default output device. This method returns Arc<Mutex<Context>>
    /// because separate thread also uses context.
    pub fn new() -> Result<Arc<Mutex<Self>>, SoundError> {
        let context = Self {
            sources: Pool::new(),
            listener: Listener::new(),
            master_gain: 1.0,
            render_duration: Default::default(),
            renderer: Renderer::Default,
            effects: Pool::new(),
            distance_model: DistanceModel::InverseDistance,
        };

        let context = Arc::new(Mutex::new(context));

        // Run device with a mixer callback. Mixer callback will mix samples
        // from source with a fixed rate.
        run_device(4 * Self::SAMPLES_PER_CHANNEL as u32, {
            let context = context.clone();
            Box::new(move |buf| {
                if let Ok(mut context) = context.lock() {
                    context.render(buf);
                }
            })
        })?;

        Ok(context)
    }

    /// Sets new distance model.
    pub fn set_distance_model(&mut self, distance_model: DistanceModel) {
        self.distance_model = distance_model;
    }

    /// Returns current distance model.
    pub fn distance_model(&self) -> DistanceModel {
        self.distance_model
    }

    /// Adds new effect to effects chain. Each sample from
    pub fn add_effect(&mut self, effect: Effect) -> Handle<Effect> {
        self.effects.spawn(effect)
    }

    /// Removes effect by given handle.
    pub fn remove_effect(&mut self, effect: Handle<Effect>) {
        self.effects.free(effect);
    }

    /// Normalizes given frequency using context's sampling rate. Normalized frequency then can be used
    /// to create filters.
    pub fn normalize_frequency(&self, f: f32) -> f32 {
        f / device::SAMPLE_RATE as f32
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

    /// Sets new master gain. Master gain is used to control total sound volume that will be passed to output
    /// device.
    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain;
    }

    /// Returns master gain.
    pub fn master_gain(&self) -> f32 {
        self.master_gain
    }

    /// Adds new sound source and returns handle of it by which it can be accessed later on.
    pub fn add_source(&mut self, source: SoundSource) -> Handle<SoundSource> {
        self.sources.spawn(source)
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

    /// Returns mutable reference to sound source at given handle. If handle is invalid, this method will panic.
    pub fn source_mut(&mut self, handle: Handle<SoundSource>) -> &mut SoundSource {
        self.sources.borrow_mut(handle)
    }

    /// Returns shared reference to listener. Engine has only one listener.
    pub fn listener(&self) -> &Listener {
        &self.listener
    }

    /// Returns mutable reference to listener. Engine has only one listener.
    pub fn listener_mut(&mut self) -> &mut Listener {
        &mut self.listener
    }

    /// Returns shared reference to effect at given handle. If handle is invalid, this method will panic.
    pub fn effect(&self, handle: Handle<Effect>) -> &Effect {
        self.effects.borrow(handle)
    }

    /// Returns mutable reference to effect at given handle. If handle is invalid, this method will panic.
    pub fn effect_mut(&mut self, handle: Handle<Effect>) -> &mut Effect {
        self.effects.borrow_mut(handle)
    }

    fn render(&mut self, buf: &mut [(f32, f32)]) {
        let last_time = time::Instant::now();

        for i in 0..self.sources.get_capacity() {
            if let Some(source) = self.sources.at(i) {
                if source.is_play_once() && source.status() == Status::Stopped {
                    self.sources.free(self.sources.handle_from_index(i));
                }
            }
        }

        for source in self.sources
            .iter_mut()
            .filter(|s| s.status() == Status::Playing) {
            source.render(buf.len());

            match self.renderer {
                Renderer::Default => {
                    // Simple rendering path. Much faster (4-5 times) than HRTF path.
                    render_source_default(source, &self.listener, self.distance_model, buf);
                }
                Renderer::HrtfRenderer(ref mut hrtf_renderer) => {
                    hrtf_renderer.render_source(source, &self.listener, self.distance_model, buf);
                }
            }
        }

        for effect in self.effects.iter_mut() {
            effect.render(&self.sources, &self.listener, self.distance_model, buf);
        }

        // Apply master gain to be able to control total sound volume.
        for (left, right) in buf {
            *left *= self.master_gain;
            *right *= self.master_gain;
        }

        self.render_duration = time::Instant::now() - last_time;
    }
}

impl Visit for Context {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        if visitor.is_reading() {
            self.sources.clear();
        }

        self.master_gain.visit("MasterGain", visitor)?;
        self.listener.visit("Listener", visitor)?;
        self.sources.visit("Sources", visitor)?;
        self.effects.visit("Effects", visitor)?;

        visitor.leave_region()
    }
}