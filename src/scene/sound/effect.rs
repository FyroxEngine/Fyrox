//! Everything related to effects.

use crate::{
    core::{
        define_is_as, pool::Handle, reflect::prelude::*, variable::InheritableVariable,
        visitor::prelude::*,
    },
    define_with,
    scene::{node::Node, sound::context::SoundContext},
};
use fyrox_sound::dsp::filters::Biquad;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

const DEFAULT_FC: f32 = 0.25615; // 11296 Hz at 44100 Hz sample rate

/// Effect input allows you to setup a source of samples for an effect with an optional filtering.
#[derive(Visit, Reflect, Debug, Default, Clone, PartialEq)]
pub struct EffectInput {
    /// A sound node that will be the source of samples for the effect.
    pub sound: Handle<Node>,
    /// An optional filter that will be applied to all samples coming from sound.
    pub filter: Option<Biquad>,
}

/// Base effect contains common properties for every effect (gain, inputs, etc.)
#[derive(Visit, Reflect, Debug, Clone)]
pub struct BaseEffect {
    #[reflect(setter = "set_name_internal")]
    pub(crate) name: InheritableVariable<String>,

    #[reflect(setter = "set_gain")]
    pub(crate) gain: InheritableVariable<f32>,

    #[reflect(setter = "set_inputs")]
    pub(crate) inputs: InheritableVariable<Vec<EffectInput>>,

    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) native: Cell<Handle<fyrox_sound::effects::Effect>>,
}

impl BaseEffect {
    /// Returns master gain of the effect.
    pub fn gain(&self) -> f32 {
        *self.gain
    }

    /// Sets master gain of the effect.
    pub fn set_gain(&mut self, gain: f32) -> f32 {
        self.gain.set(gain)
    }

    /// Sets new inputs for the effect.
    pub fn set_inputs(&mut self, inputs: Vec<EffectInput>) -> Vec<EffectInput> {
        self.inputs.set(inputs)
    }

    /// Returns shared reference to the inputs array.
    pub fn inputs(&self) -> &Vec<EffectInput> {
        &self.inputs
    }

    /// Returns mutable reference to the inputs array.
    pub fn inputs_mut(&mut self) -> &mut Vec<EffectInput> {
        self.inputs.get_mut()
    }

    /// Returns shared reference to the current name of the effect.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns current name of the effect.
    pub fn name_owned(&self) -> String {
        self.name.get().clone()
    }

    /// Sets new name of the effect.
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.set_name_internal(name.as_ref().to_owned());
    }

    fn set_name_internal(&mut self, name: String) -> String {
        self.name.set(name)
    }
}

impl Default for BaseEffect {
    fn default() -> Self {
        Self {
            name: InheritableVariable::new("".to_string()),
            gain: InheritableVariable::new(1.0),
            inputs: Default::default(),
            native: Default::default(),
        }
    }
}

/// All possible effects in the engine.
#[derive(Visit, Reflect, Debug, AsRefStr, EnumString, EnumVariantNames, Clone)]
pub enum Effect {
    /// See [`ReverbEffect`] docs.
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

impl Effect {
    define_is_as!(Effect : Reverb -> ref ReverbEffect => fn is_reverb, fn as_reverb, fn as_reverb_mut);
}

/// Base effect builder allows you to build an effect.
pub struct BaseEffectBuilder {
    gain: f32,
    inputs: Vec<EffectInput>,
    name: String,
}

impl Default for BaseEffectBuilder {
    fn default() -> Self {
        BaseEffectBuilder::new()
    }
}

impl BaseEffectBuilder {
    /// Creates new base effect builder.
    pub fn new() -> Self {
        Self {
            gain: 1.0,
            inputs: vec![],
            name: "".to_owned(),
        }
    }

    define_with!(
        /// Sets desired name of the effect.
        fn with_name(name: String)
    );

    define_with!(
        /// Sets desired gain of the effect.
        fn with_gain(gain: f32)
    );

    define_with!(
        /// Sets desired inputs of the effect.
        fn with_inputs(inputs: Vec<EffectInput>)
    );

    /// Creates new base effect.
    pub fn build(self) -> BaseEffect {
        BaseEffect {
            name: self.name.into(),
            gain: self.gain.into(),
            inputs: self.inputs.into(),
            native: Default::default(),
        }
    }
}

/// Reverb effect gives you multiple echoes.
#[derive(Visit, Reflect, Debug, Clone)]
pub struct ReverbEffect {
    pub(crate) base: BaseEffect,

    #[reflect(setter = "set_dry")]
    pub(crate) dry: InheritableVariable<f32>,

    #[reflect(setter = "set_wet")]
    pub(crate) wet: InheritableVariable<f32>,

    #[reflect(setter = "set_fc")]
    pub(crate) fc: InheritableVariable<f32>,

    #[reflect(setter = "set_decay_time")]
    pub(crate) decay_time: InheritableVariable<f32>,
}

impl Default for ReverbEffect {
    fn default() -> Self {
        Self {
            base: Default::default(),
            dry: InheritableVariable::new(1.0),
            wet: InheritableVariable::new(1.0),
            fc: InheritableVariable::new(DEFAULT_FC),
            decay_time: InheritableVariable::new(3.0),
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
    pub fn set_dry(&mut self, dry: f32) -> f32 {
        self.dry.set(dry.min(1.0).max(0.0))
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
    pub fn set_wet(&mut self, wet: f32) -> f32 {
        self.wet.set(wet.min(1.0).max(0.0))
    }

    /// Returns stereo mixing coefficient.
    pub fn wet(&self) -> f32 {
        *self.wet
    }

    /// Sets desired duration of reverberation, the more size your environment has,
    /// the larger duration of reverberation should be.
    pub fn set_decay_time(&mut self, decay_time: f32) -> f32 {
        self.decay_time.set(decay_time)
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
    pub fn set_fc(&mut self, fc: f32) -> f32 {
        self.fc.set(fc)
    }

    /// Returns cutoff frequency of lowpass filter in comb filters.
    pub fn fc(&self) -> f32 {
        *self.fc
    }
}

/// Allows you to create a new reverb effect.
pub struct ReverbEffectBuilder {
    base_builder: BaseEffectBuilder,
    dry: f32,
    wet: f32,
    fc: f32,
    decay_time: f32,
}

impl ReverbEffectBuilder {
    /// Creates new reverb effect builder.
    pub fn new(base_builder: BaseEffectBuilder) -> Self {
        Self {
            base_builder,
            dry: 1.0,
            wet: 1.0,
            fc: DEFAULT_FC,
            decay_time: 3.0,
        }
    }

    define_with!(
        /// Sets desired dry coefficient.
        fn with_dry(dry: f32)
    );

    define_with!(
        /// Sets desired wet coefficient.
        fn with_wet(wet: f32)
    );

    define_with!(
        /// Sets desired cutoff frequency.
        fn with_fc(fc: f32)
    );

    define_with!(
        /// Sets desired decay time (in seconds).
        fn with_decay_time(decay_time: f32)
    );

    /// Creates new reverb effect.
    pub fn build_effect(self) -> Effect {
        Effect::Reverb(ReverbEffect {
            base: self.base_builder.build(),
            dry: self.dry.into(),
            wet: self.wet.into(),
            fc: self.fc.into(),
            decay_time: self.decay_time.into(),
        })
    }

    /// Creates new reverb effect and adds it to the context.
    pub fn build(self, context: &mut SoundContext) -> Handle<Effect> {
        context.add_effect(self.build_effect())
    }
}
