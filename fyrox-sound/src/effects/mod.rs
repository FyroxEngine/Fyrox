//! Effects module
//!
//! # Overview
//!
//! Provides unified way of creating and using effects.

use crate::effects::reverb::Reverb;
use fyrox_core::{reflect::prelude::*, visitor::prelude::*};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

pub mod reverb;

/// Attenuation effect.
#[derive(Debug, Clone, Visit, Reflect)]
pub struct Attenuate {
    gain: f32,
}

impl Default for Attenuate {
    fn default() -> Self {
        Self { gain: 1.0 }
    }
}

impl Attenuate {
    /// Creates new attenuation effect.
    pub fn new(gain: f32) -> Self {
        Self {
            gain: gain.max(0.0),
        }
    }
}

impl EffectRenderTrait for Attenuate {
    fn render(&mut self, input: &[(f32, f32)], output: &mut [(f32, f32)]) {
        for ((input_left, input_right), (output_left, output_right)) in
            input.iter().zip(output.iter_mut())
        {
            *output_left = *input_left * self.gain;
            *output_right = *input_right * self.gain;
        }
    }
}

/// See module docs.
#[derive(Debug, Clone, Visit, Reflect, AsRefStr, EnumString, EnumVariantNames)]
pub enum Effect {
    /// Attenuation effect.
    Attenuate(Attenuate),
    /// Reverberation effect. See corresponding module for more info.
    Reverb(Reverb),
}

impl Default for Effect {
    fn default() -> Self {
        Effect::Attenuate(Default::default())
    }
}

pub(crate) trait EffectRenderTrait {
    fn render(&mut self, input: &[(f32, f32)], output: &mut [(f32, f32)]);
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            Effect::Attenuate(v) => v.$func($($args),*),
            Effect::Reverb(v) => v.$func($($args),*),
        }
    };
}

impl EffectRenderTrait for Effect {
    fn render(&mut self, input: &[(f32, f32)], output: &mut [(f32, f32)]) {
        static_dispatch!(self, render, input, output)
    }
}
