use crate::{
    animation::machine::{state::StateDefinition, State},
    core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
};

/// Transition is a connection between two states with a rule that defines possibility
/// of actual transition with blending.
#[derive(Default, Debug, Visit, Clone)]
pub struct Transition {
    pub definition: Handle<TransitionDefinition>,
    pub(crate) name: String,
    /// Total amount of time to transition from `src` to `dst` state.
    pub(crate) transition_time: f32,
    pub(crate) elapsed_time: f32,
    pub(crate) source: Handle<State>,
    pub(crate) dest: Handle<State>,
    /// Identifier of Rule parameter which defines is transition should be activated or not.
    pub(crate) rule: String,
    /// If set, then fetched value from `rule` will be inverted. It is useful for cases when you
    /// have a pair of transitions that depend on a single Rule parameter, but have different
    /// directions (A -> B, B -> A).
    pub(crate) invert_rule: bool,
    /// 0 - evaluates `src` pose, 1 - `dest`, 0..1 - blends `src` and `dest`
    pub(crate) blend_factor: f32,
}

#[derive(Default, Debug, Visit, Clone, Reflect)]
pub struct TransitionDefinition {
    #[reflect(description = "The name of the transition, it is used for debug output.")]
    pub name: String,
    /// Total amount of time to transition from `src` to `dst` state.
    #[reflect(description = "Total amount of time (in seconds) to transition \
        from source to destination state")]
    pub transition_time: f32,
    /// Identifier of Rule parameter which defines is transition should be activated or not.
    #[reflect(
        description = "Name of the Rule parameter which defines whether transition \
        should be activated or not"
    )]
    pub rule: String,
    #[reflect(hidden)]
    pub source: Handle<StateDefinition>,
    #[reflect(hidden)]
    pub dest: Handle<StateDefinition>,
    /// If set, then fetched value from `rule` will be inverted. It is useful for cases when you
    /// have a pair of transitions that depend on a single Rule parameter, but have different
    /// directions (A -> B, B -> A).
    #[reflect(
        description = "If set, then fetched value from `rule` will be inverted. It is useful
     for cases when you have a pair of transitions that depend on a single Rule parameter,
      but have different directions (A -> B, B -> A)."
    )]
    pub invert_rule: bool,
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
            definition: Default::default(),
            name: name.to_owned(),
            transition_time: time,
            elapsed_time: 0.0,
            source: src,
            dest,
            rule: rule.to_owned(),
            invert_rule: false,
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

    #[inline]
    pub fn set_invert_rule(&mut self, invert: bool) {
        self.invert_rule = invert;
    }

    #[inline]
    pub fn is_invert_rule(&self) -> bool {
        self.invert_rule
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
