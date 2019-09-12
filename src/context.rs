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
                    for sample in buf {
                        let mut left = 0.0;
                        let mut right = 0.0;
                        for source in context.sources.iter_mut() {
                            let (l, r) = source.sample();
                            left += l;
                            right += r;
                        }

                        if left > 1.0 {
                            left = 0.0;
                        } else if left < -1.0 {
                            left = -1.0;
                        }

                        if right > 1.0 {
                            right = 0.0;
                        } else if right < -1.0 {
                            right = -1.0;
                        }

                        sample.left = (left * (std::i16::MAX as f32)) as i16;
                        sample.right = (right * (std::i16::MAX as f32)) as i16;
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
}