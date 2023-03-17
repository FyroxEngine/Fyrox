//! Sound engine module
//!
//! ## Overview
//!
//! Sound engine manages contexts, feeds output device with data.

use crate::context::{SoundContext, SAMPLE_RATE};
use fyrox_core::visitor::{Visit, VisitResult, Visitor};
use std::error::Error;
use std::sync::{Arc, Mutex, MutexGuard};

/// Sound engine manages contexts, feeds output device with data. Sound engine instance can be cloned,
/// however this is always a "shallow" clone, because actual sound engine data is wrapped in Arc.
#[derive(Clone)]
pub struct SoundEngine(Arc<Mutex<State>>);

impl Default for SoundEngine {
    fn default() -> Self {
        Self::without_device()
    }
}

/// Internal state of the sound engine.
pub struct State {
    contexts: Vec<SoundContext>,
    output_device: Option<Box<dyn tinyaudio::BaseAudioOutputDevice>>,
}

impl SoundEngine {
    /// Creates new instance of the sound engine. It is possible to have multiple engines running at
    /// the same time, but you shouldn't do this because you can create multiple contexts which
    /// should cover 99% of use cases.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let engine = Self::without_device();
        engine.initialize_audio_output_device()?;
        Ok(engine)
    }

    /// Creates new instance of a sound engine without OS audio output device (so called headless mode).
    /// The user should periodically run [`State::render`] if they want to implement their own sample sending
    /// method to an output device (or a file, etc.).
    pub fn without_device() -> Self {
        Self(Arc::new(Mutex::new(State {
            contexts: Default::default(),
            output_device: None,
        })))
    }

    /// Tries to initialize default audio output device.
    pub fn initialize_audio_output_device(&self) -> Result<(), Box<dyn Error>> {
        let state = self.clone();

        let device = tinyaudio::run_output_device(
            tinyaudio::OutputDeviceParameters {
                sample_rate: SAMPLE_RATE as usize,
                channels_count: 2,
                channel_sample_count: SoundContext::SAMPLES_PER_CHANNEL,
            },
            {
                move |buf| {
                    // SAFETY: This is safe as long as channels count above is 2.
                    let data = unsafe {
                        std::slice::from_raw_parts_mut(
                            buf.as_mut_ptr() as *mut (f32, f32),
                            buf.len() / 2,
                        )
                    };

                    state.state().render(data);
                }
            },
        )?;

        self.state().output_device = Some(device);

        Ok(())
    }

    /// Destroys current audio output device (if any).
    pub fn destroy_audio_output_device(&self) {
        self.state().output_device = None;
    }

    /// Provides direct access to actual engine data.
    pub fn state(&self) -> MutexGuard<State> {
        self.0.lock().unwrap()
    }
}

impl State {
    /// Adds new context to the engine. Each context must be added to the engine to emit
    /// sounds.
    pub fn add_context(&mut self, context: SoundContext) {
        self.contexts.push(context);
    }

    /// Removes a context from the engine. Removed context will no longer produce any sound.
    pub fn remove_context(&mut self, context: SoundContext) {
        if let Some(position) = self.contexts.iter().position(|c| c == &context) {
            self.contexts.remove(position);
        }
    }

    /// Removes all contexts from the engine.
    pub fn remove_all_contexts(&mut self) {
        self.contexts.clear()
    }

    /// Checks if a context is registered in the engine.
    pub fn has_context(&self, context: &SoundContext) -> bool {
        self.contexts
            .iter()
            .any(|c| Arc::ptr_eq(c.state.as_ref().unwrap(), context.state.as_ref().unwrap()))
    }

    /// Returns a reference to context container.
    pub fn contexts(&self) -> &[SoundContext] {
        &self.contexts
    }

    /// Returns the length of buf to be passed to [`Self::render()`].
    pub fn render_buffer_len() -> usize {
        SoundContext::SAMPLES_PER_CHANNEL
    }

    /// Renders the sound into buf. The buf must have at least [`Self::render_buffer_len()`]
    /// elements. This method must be used if and only if the engine was created via
    /// [`SoundEngine::without_device`].
    ///
    /// ## Deadlocks
    ///
    /// This method internally locks added sound contexts so it must be called when all the contexts
    /// are unlocked or you'll get a deadlock.
    pub fn render(&mut self, buf: &mut [(f32, f32)]) {
        buf.fill((0.0, 0.0));
        self.render_inner(buf);
    }

    fn render_inner(&mut self, buf: &mut [(f32, f32)]) {
        for context in self.contexts.iter_mut() {
            context.state().render(buf);
        }
    }
}

impl Visit for State {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.is_reading() {
            self.contexts.clear();
        }

        let mut region = visitor.enter_region(name)?;

        self.contexts.visit("Contexts", &mut region)?;

        Ok(())
    }
}
