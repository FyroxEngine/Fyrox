use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        visitor::prelude::*,
    },
    scene::variable::TemplateVariable,
};

#[derive(Visit, Inspect, Debug)]
pub struct BaseEffect {
    gain: TemplateVariable<f32>,
}

impl Default for BaseEffect {
    fn default() -> Self {
        Self {
            gain: TemplateVariable::new(1.0),
        }
    }
}

#[derive(Visit, Inspect, Debug)]
pub enum Effect {
    Reverb(ReverbEffect),
}

impl Default for Effect {
    fn default() -> Self {
        Self::Reverb(Default::default())
    }
}

#[derive(Visit, Inspect, Debug)]
pub struct ReverbEffect {
    base: BaseEffect,
    dry: TemplateVariable<f32>,
    wet: TemplateVariable<f32>,
    fc: TemplateVariable<f32>,
}

impl Default for ReverbEffect {
    fn default() -> Self {
        Self {
            base: Default::default(),
            dry: TemplateVariable::new(1.0),
            wet: TemplateVariable::new(1.0),
            fc: TemplateVariable::new(0.25615), // 11296 Hz at 44100 Hz sample rate
        }
    }
}
