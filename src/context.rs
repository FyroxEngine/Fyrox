use std::sync::{Arc, Mutex};
use crate::{
    error::SoundError,
    source::Source,
    device::run_device,
    listener::Listener,
};

use rg3d_core::pool::{Pool, Handle};

pub struct Context {
    sources: Pool<Source>,
    listener: Listener,
    master_gain: f32,
}

impl Context {
    fn init() -> Self {
        Self {
            sources: Pool::new(),
            listener: Listener::new(),
            master_gain: 1.0,
        }
    }

    pub fn new() -> Result<Arc<Mutex<Self>>, SoundError> {
        let context = Arc::new(Mutex::new(Self::init()));

        // Run device with a mixer callback. Mixer callback will mix samples
        // from source with a fixed rate.
        run_device(8820, {
            let context = context.clone();
            Box::new(move |buf| {
                if let Ok(mut context) = context.lock() {
                    for source in context.sources.iter_mut() {
                        source.sample_into(buf);
                    }

                    // Apply master gain to be able to control total sound volume.
                    for (left, right) in buf {
                        *left *= context.master_gain;
                        *right *= context.master_gain;
                    }
                }
            })
        })?;

        Ok(context)
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

    pub fn get_sources_mut(&mut self) -> &mut Pool<Source> {
        &mut self.sources
    }

    pub fn get_listener(&self) -> &Listener {
        &self.listener
    }

    pub fn get_listener_mut(&mut self) -> &mut Listener {
        &mut self.listener
    }

    pub fn update(&mut self) -> Result<(), SoundError> {
        for source in self.sources.iter_mut() {
            source.update(&self.listener)?;
        }
        Ok(())
    }
}