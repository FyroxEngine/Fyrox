use std::sync::{Arc, Mutex};
use crate::{
    error::SoundError,
    source::Source,
    device::run_device,
    pool::{
        Pool,
        Handle
    }
};

pub struct Context {
    sources: Pool<Source>
}

impl Context {
    pub fn new() -> Result<Arc<Mutex<Self>>, SoundError> {
        let context = Arc::new(Mutex::new(Self {
            sources: Pool::new(),
        }));

        // Run device with a mixer callback. Mixer callback will mix samples
        // from source with a fixed rate.
        run_device(8820, {
            let context = context.clone();
            Box::new(move |buf| {
                if let Ok(mut context) = context.lock() {
                    for source in context.sources.iter_mut() {
                        source.sample_into(buf);
                    }
                }
            })
        })?;

        Ok(context)
    }

    pub fn add_source(&mut self, source: Source) -> Handle<Source>{
        self.sources.spawn(source)
    }

    pub fn get_sources_mut(&mut self) -> &mut Pool<Source> {
        &mut self.sources
    }

    pub fn update(&mut self) -> Result<(), SoundError> {
        for source in self.sources.iter_mut() {
            source.update()?;
        }
        Ok(())
    }
}