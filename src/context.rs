use std::{
    sync::{
        Arc,
        Mutex,
    },
    time,
};
use crate::{
    error::SoundError,
    device::run_device,
    listener::Listener,
    source::{
        Status,
        SoundSource
    },
    renderer::{
        Renderer,
        render_source_default,
    },
    effects::Effect,
    device,
};
use rg3d_core::{
    pool::{Pool, Handle},
    visitor::{Visit, VisitResult, Visitor},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DistanceModel {
    None,
    InverseDistance,
    LinearDistance,
    ExponentDistance,
}

pub struct Context {
    sources: Pool<SoundSource>,
    listener: Listener,
    master_gain: f32,
    render_time: f32,
    renderer: Renderer,    effects: Pool<Effect>,
    distance_model: DistanceModel,
}

impl Context {
    // TODO: This is magic constant that gives 1024 + 1 number when summed with
    //       HRTF length for faster FFT calculations. Find a better way of selecting this.
    pub const HRTF_BLOCK_LEN: usize = 513;

    pub const HRTF_INTERPOLATION_STEPS: usize = 8;

    pub const SAMPLES_PER_CHANNEL: usize = Self::HRTF_BLOCK_LEN * Self::HRTF_INTERPOLATION_STEPS;

    pub fn new() -> Result<Arc<Mutex<Self>>, SoundError> {
        let context = Self {
            sources: Pool::new(),
            listener: Listener::new(),
            master_gain: 1.0,
            render_time: 0.0,
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

    fn render(&mut self, buf: &mut [(f32, f32)]) {
        let last_time = time::Instant::now();

        for i in 0..self.sources.get_capacity() {
            if let Some(source) = self.sources.at(i) {
                if source.generic().is_play_once() && source.generic().status() == Status::Stopped {
                    self.sources.free(self.sources.handle_from_index(i));
                }
            }
        }

        for source in self.sources
            .iter_mut()
            .filter(|s| s.generic().status() == Status::Playing) {
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

        // TODO: This requires dry (without any effects such as HRTF) signal as input
        //       so this should be changed when used with HRTF.
        for effect in self.effects.iter_mut() {
            for (left, right) in buf.iter_mut() {
                let (out_left, out_right) = effect.feed(*left, *right);
                *left = out_left;
                *right = out_right;
            }
        }

        // Apply master gain to be able to control total sound volume.
        for (left, right) in buf {
            *left *= self.master_gain;
            *right *= self.master_gain;
        }

        self.render_time = (time::Instant::now() - last_time).as_secs_f32();
    }

    pub fn set_distance_model(&mut self, distance_model: DistanceModel) {
        self.distance_model = distance_model;
    }

    pub fn distance_model(&self) -> DistanceModel {
        self.distance_model
    }

    pub fn add_effect(&mut self, effect: Effect) -> Handle<Effect> {
        self.effects.spawn(effect)
    }

    pub fn remove_effect(&mut self, effect: Handle<Effect>) {
        self.effects.free(effect)
    }

    pub fn normalize_frequency(&self, f: f32) -> f32 {
        f / device::SAMPLE_RATE as f32
    }

    pub fn full_render_time(&self) -> f32 {
        self.render_time
    }

    pub fn set_renderer(&mut self, renderer: Renderer) {
        self.renderer = renderer;
    }

    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain;
    }

    pub fn master_gain(&self) -> f32 {
        self.master_gain
    }

    pub fn add_source(&mut self, source: SoundSource) -> Handle<SoundSource> {
        self.sources.spawn(source)
    }

    pub fn sources(&self) -> &Pool<SoundSource> {
        &self.sources
    }

    pub fn sources_mut(&mut self) -> &mut Pool<SoundSource> {
        &mut self.sources
    }

    pub fn source(&self, handle: Handle<SoundSource>) -> &SoundSource {
        self.sources.borrow(handle)
    }

    pub fn get_source_mut(&mut self, handle: Handle<SoundSource>) -> &mut SoundSource {
        self.sources.borrow_mut(handle)
    }

    pub fn listener(&self) -> &Listener {
        &self.listener
    }

    pub fn listener_mut(&mut self) -> &mut Listener {
        &mut self.listener
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

        visitor.leave_region()
    }
}