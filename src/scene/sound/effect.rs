use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    scene::variable::TemplateVariable,
};
use std::cell::Cell;
use std::ops::{Deref, DerefMut};

#[derive(Visit, Inspect, Debug)]
pub struct BaseEffect {
    pub(crate) gain: TemplateVariable<f32>,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) native: Cell<Handle<fyrox_sound::effects::Effect>>,
}

impl BaseEffect {
    /// Returns master gain of the effect.
    pub fn gain(&self) -> f32 {
        *self.gain
    }

    /// Sets master gain of the effect.
    pub fn set_gain(&mut self, gain: f32) {
        self.gain.set(gain);
    }
}

impl Default for BaseEffect {
    fn default() -> Self {
        Self {
            gain: TemplateVariable::new(1.0),
            native: Default::default(),
        }
    }
}

#[derive(Visit, Inspect, Debug)]
pub enum Effect {
    Reverb(ReverbEffect),
}

impl Deref for Effect {
    type Target = BaseEffect;

    fn deref(&self) -> &Self::Target {
        match self {
            Effect::Reverb(v) => v,
        }
    }
}

impl DerefMut for Effect {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Effect::Reverb(v) => v,
        }
    }
}

impl Default for Effect {
    fn default() -> Self {
        Self::Reverb(Default::default())
    }
}

#[derive(Visit, Inspect, Debug)]
pub struct ReverbEffect {
    pub(crate) base: BaseEffect,
    pub(crate) dry: TemplateVariable<f32>,
    pub(crate) wet: TemplateVariable<f32>,
    pub(crate) fc: TemplateVariable<f32>,
    pub(crate) decay_time: TemplateVariable<f32>,
}

impl Default for ReverbEffect {
    fn default() -> Self {
        Self {
            base: Default::default(),
            dry: TemplateVariable::new(1.0),
            wet: TemplateVariable::new(1.0),
            fc: TemplateVariable::new(0.25615), // 11296 Hz at 44100 Hz sample rate
            decay_time: TemplateVariable::new(3.0),
        }
    }
}

impl Deref for ReverbEffect {
    type Target = BaseEffect;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for ReverbEffect {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl ReverbEffect {
    /// Sets how much of input signal should be passed to output without any processing.
    /// Default value is 1.0.
    pub fn set_dry(&mut self, dry: f32) {
        self.dry.set(dry.min(1.0).max(0.0));
    }

    /// Returns dry part.
    pub fn dry(&self) -> f32 {
        *self.dry
    }

    /// Sets stereo mixing of processed signal.
    /// 0.0 - left is left, right is right
    /// 1.0 - right is left, left is right.
    /// 0.5 - left is (left + right) * 0.5, right is (left + right) * 0.5
    /// and so on.
    pub fn set_wet(&mut self, wet: f32) {
        self.wet.set(wet.min(1.0).max(0.0));
    }

    /// Returns stereo mixing coefficient.
    pub fn wet(&self) -> f32 {
        *self.wet
    }

    /// Sets desired duration of reverberation, the more size your environment has,
    /// the larger duration of reverberation should be.
    pub fn set_decay_time(&mut self, decay_time: f32) {
        self.decay_time.set(decay_time);
    }

    /// Returns current decay time.
    pub fn decay_time(&self) -> f32 {
        *self.decay_time
    }

    /// Sets cutoff frequency for lowpass filter in comb filters. Basically this parameter defines
    /// "tone" of reflections, when frequency is higher - then more high frequencies will be in
    /// output signal, and vice versa. For example if you have environment with high absorption of
    /// high frequencies, then sound in reality will be muffled - to simulate this you could set
    /// frequency to 3-4 kHz.
    ///
    /// # Notes
    ///
    /// This method uses normalized frequency as input, this means that you should divide your desired
    /// frequency in hertz by sample rate of sound context. Context has `normalize_frequency` method
    /// exactly for this purpose.
    pub fn set_fc(&mut self, fc: f32) {
        self.fc.set(fc);
    }

    /// Returns cutoff frequency of lowpass filter in comb filters.
    pub fn fc(&self) -> f32 {
        *self.fc
    }
}
