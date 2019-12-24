/// Sound renderer.

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
        Source,
        SourceKind,
    },
    renderer::Renderer,
};
use rg3d_core::{
    pool::{Pool, Handle},
    visitor::{Visit, VisitResult, Visitor},
};
use crate::renderer::{render_source_default};

pub struct Context {
    sources: Pool<Source>,
    listener: Listener,
    master_gain: f32,
    render_time: f32,
    renderer: Renderer,
}

impl Context {
    // TODO: This is magic constant that gives 4096 (power of two) number when summed with
    //       HRTF length for faster FFT calculations. Find a better way of selecting this.
    pub const SAMPLE_PER_CHANNEL: usize = 3584;

    pub fn new() -> Result<Arc<Mutex<Self>>, SoundError> {
        let context = Self {
            sources: Pool::new(),
            listener: Listener::new(),
            master_gain: 1.0,
            render_time: 0.0,
            renderer: Renderer::Default,
        };

        let context = Arc::new(Mutex::new(context));

        // Run device with a mixer callback. Mixer callback will mix samples
        // from source with a fixed rate.
        run_device(4 * Self::SAMPLE_PER_CHANNEL as u32, {
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
        let current_time = time::Instant::now();

        for source in self.sources.iter_mut() {
            if source.get_status() != Status::Playing {
                continue;
            }

            match self.renderer {
                Renderer::Default => {
                    // Simple rendering path. Much faster (8-10 times) than HRTF path.
                    render_source_default(source, buf);
                }
                Renderer::HrtfRenderer(ref mut hrtf_renderer) => {
                    match source.get_kind() {
                        SourceKind::Flat => {
                            render_source_default(source, buf);
                        }
                        SourceKind::Spatial(_) => {
                            hrtf_renderer.render_source(source, buf);
                        }
                    }
                }
            }
        }

        // Apply master gain to be able to control total sound volume.
        for (left, right) in buf {
            *left *= self.master_gain;
            *right *= self.master_gain;
        }

        self.render_time = (time::Instant::now() - current_time).as_secs_f32();
    }

    pub fn get_render_time(&self) -> f32 {
        self.render_time
    }

    pub fn set_renderer(&mut self, renderer: Renderer) {
        self.renderer = renderer;
    }

    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain;
    }

    pub fn get_master_gain(&self) -> f32 {
        self.master_gain
    }

    pub fn add_source(&mut self, source: Source) -> Handle<Source> {
        self.sources.spawn(source)
    }

    pub fn get_sources(&self) -> &Pool<Source> {
        &self.sources
    }

    pub fn get_sources_mut(&mut self) -> &mut Pool<Source> {
        &mut self.sources
    }

    pub fn get_source(&self, handle: Handle<Source>) -> &Source {
        self.sources.borrow(handle)
    }

    pub fn get_source_mut(&mut self, handle: Handle<Source>) -> &mut Source {
        self.sources.borrow_mut(handle)
    }

    pub fn get_listener(&self) -> &Listener {
        &self.listener
    }

    pub fn get_listener_mut(&mut self) -> &mut Listener {
        &mut self.listener
    }

    pub fn update(&mut self) -> Result<(), SoundError> {
        self.listener.update();
        for source in self.sources.iter_mut() {
            source.update(&self.listener)?;
        }
        for i in 0..self.sources.get_capacity() {
            if let Some(source) = self.sources.at(i) {
                if source.is_play_once() && source.get_status() == Status::Stopped {
                    self.sources.free(self.sources.handle_from_index(i));
                }
            }
        }
        Ok(())
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