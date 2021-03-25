//! Sound engine module
//!
//! ## Overview
//!
//! Sound engine manages contexts, feeds output device with data.

use crate::{context::Context, device};
use rg3d_core::visitor::{Visit, VisitResult, Visitor};
use std::sync::{Arc, Mutex};

/// Internal state of sound engine.
#[derive(Default)]
pub struct SoundEngine {
    contexts: Vec<Context>,
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
        device::run_device(4 * Context::SAMPLES_PER_CHANNEL as u32, {
            let state = engine.clone();
            Box::new(move |buf| {
                if let Ok(mut state) = state.lock() {
                    let master_gain = state.master_gain;
                    for context in state.contexts.iter_mut() {
                        context.state().render(master_gain, buf);
                    }
                }
            })
        });

        engine
    }

    /// Adds new context to the engine. Each context must be added to the engine to emit
    /// sounds.
    pub fn add_context(&mut self, context: Context) {
        self.contexts.push(context);
    }

    /// Removes a context from the engine. Removed context will no longer produce any sound.
    pub fn remove_context(&mut self, context: Context) {
        if let Some(position) = self.contexts.iter().position(|c| c == &context) {
            self.contexts.remove(position);
        }
    }

    /// Checks if a context is registered in the engine.
    pub fn has_context(&self, context: &Context) -> bool {
        self.contexts
            .iter()
            .any(|c| Arc::ptr_eq(c.state.as_ref().unwrap(), context.state.as_ref().unwrap()))
    }

    /// Returns a reference to context container.
    pub fn contexts(&self) -> &[Context] {
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
