use crate::animation::machine::state::StateDefinition;
use crate::{
    animation::machine::State,
    core::{pool::Handle, visitor::prelude::*},
};

/// Transition is a connection between two states with a rule that defines possibility
/// of actual transition with blending.
#[derive(Default, Debug, Visit, Clone)]
pub struct Transition {
    name: String,
    /// Total amount of time to transition from `src` to `dst` state.
    transition_time: f32,
    elapsed_time: f32,
    source: Handle<State>,
    dest: Handle<State>,
    /// Identifier of Rule parameter which defines is transition should be activated or not.
    rule: String,
    /// 0 - evaluates `src` pose, 1 - `dest`, 0..1 - blends `src` and `dest`
    blend_factor: f32,
}

#[derive(Default, Debug, Visit, Clone)]
pub struct TransitionDefinition {
    pub name: String,
    /// Total amount of time to transition from `src` to `dst` state.
    pub transition_time: f32,
    pub source: Handle<StateDefinition>,
    pub dest: Handle<StateDefinition>,
    /// Identifier of Rule parameter which defines is transition should be activated or not.
    pub rule: String,
}

impl Transition {
    pub fn new(
        name: &str,
        src: Handle<State>,
        dest: Handle<State>,
        time: f32,
        rule: &str,
    ) -> Transition {
        Self {
            name: name.to_owned(),
            transition_time: time,
            elapsed_time: 0.0,
            source: src,
            dest,
            rule: rule.to_owned(),
            blend_factor: 0.0,
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    pub fn transition_time(&self) -> f32 {
        self.transition_time
    }

    #[inline]
    pub fn source(&self) -> Handle<State> {
        self.source
    }

    #[inline]
    pub fn dest(&self) -> Handle<State> {
        self.dest
    }

    #[inline]
    pub fn rule(&self) -> &str {
        self.rule.as_str()
    }

    #[inline]
    pub fn is_done(&self) -> bool {
        (self.transition_time - self.elapsed_time).abs() <= f32::EPSILON
    }

    #[inline]
    pub fn blend_factor(&self) -> f32 {
        self.blend_factor
    }

    pub(super) fn reset(&mut self) {
        self.elapsed_time = 0.0;
        self.blend_factor = 0.0;
    }

    pub(super) fn update(&mut self, dt: f32) {
        self.elapsed_time += dt;
        if self.elapsed_time > self.transition_time {
            self.elapsed_time = self.transition_time;
        }
        self.blend_factor = self.elapsed_time / self.transition_time;
    }
}
