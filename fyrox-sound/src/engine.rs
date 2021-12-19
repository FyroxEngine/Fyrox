//! Sound engine module
//!
//! ## Overview
//!
//! Sound engine manages contexts, feeds output device with data.

use crate::{context::SoundContext, device};
use fyrox_core::visitor::{Visit, VisitResult, Visitor};
use std::sync::{Arc, Mutex};

/// Internal state of sound engine.
#[derive(Default)]
pub struct SoundEngine {
    contexts: Vec<SoundContext>,
    master_gain: f32,
}

impl SoundEngine {
    /// Creates new instance of a sound engine. It is possible to have multiple engine running at
    /// the same time, but you shouldn't do this because you can create multiple contexts which
    /// should cover 99% of use cases.
    pub fn new() -> Arc<Mutex<Self>> {
        let engine = Arc::new(Mutex::new(Self {
            contexts: Default::default(),
            master_gain: 1.0,
        }));

        // Run the default output device. Internally it creates separate thread, so we have
        // to share sound engine instance with it, this is the only reason why it is wrapped
        // in Arc<Mutex<>>
        device::run_device(4 * SoundContext::SAMPLES_PER_CHANNEL as u32, {
            let state = engine.clone();
            move |buf| {
                if let Ok(mut state) = state.lock() {
                    state.render_inner(buf);
                }
            }
        });

        engine
    }

    /// Creates new instance of a sound engine without running a device thread. The user must
    /// periodically run [`Self::render`].
    pub fn without_device() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            contexts: Default::default(),
            master_gain: 1.0,
        }))
    }

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

    /// Set global sound volume in [0; 1] range.
    pub fn set_master_gain(&mut self, master_gain: f32) {
        self.master_gain = master_gain;
    }

    /// Returns global sound volume in [0; 1] range.
    pub fn master_gain(&self) -> f32 {
        self.master_gain
    }

    /// Returns the length of buf to be passed to [`Self::render()`].
    pub fn render_buffer_len() -> usize {
        SoundContext::SAMPLES_PER_CHANNEL
    }

    /// Renders the sound into buf. The buf must have at least [`Self::render_buffer_len()`]
    /// elements. This method must be used if and only if the engine was created via
    /// [`Self::without_device`].
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
        let master_gain = self.master_gain;
        for context in self.contexts.iter_mut() {
            context.state().render(master_gain, buf);
        }
    }
}

impl Visit for SoundEngine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        if visitor.is_reading() {
            self.contexts.clear();
        }

        self.master_gain.visit("MasterGain", visitor)?;
        self.contexts.visit("Contexts", visitor)?;

        visitor.leave_region()
    }
}
