//! Transition is a connection between two states with a rule that defines possibility of actual transition with blending.

use crate::{
    animation::machine::State,
    core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
    utils::NameProvider,
};

/// Transition is a connection between two states with a rule that defines possibility of actual transition with blending.
#[derive(Default, Debug, Visit, Clone, Reflect, PartialEq)]
pub struct Transition {
    /// The name of the transition, it is used for debug output.
    #[reflect(description = "The name of the transition, it is used for debug output.")]
    pub(crate) name: String,

    /// Total amount of time to transition from `src` to `dst` state.
    #[reflect(description = "Total amount of time (in seconds) to transition \
        from source to destination state")]
    pub(crate) transition_time: f32,

    pub(crate) elapsed_time: f32,

    #[reflect(read_only)]
    pub(crate) source: Handle<State>,

    #[reflect(read_only)]
    pub(crate) dest: Handle<State>,

    /// Identifier of Rule parameter which defines is transition should be activated or not.
    #[reflect(
        description = "Name of the Rule parameter which defines whether transition \
        should be activated or not"
    )]
    pub(crate) rule: String,

    /// If set, then fetched value from `rule` will be inverted. It is useful for cases when you
    /// have a pair of transitions that depend on a single Rule parameter, but have different
    /// directions (A -> B, B -> A).
    #[reflect(
        description = "If set, then fetched value from `rule` will be inverted. It is useful
     for cases when you have a pair of transitions that depend on a single Rule parameter,
      but have different directions (A -> B, B -> A)."
    )]
    pub(crate) invert_rule: bool,

    /// 0 - evaluates `src` pose, 1 - `dest`, 0..1 - blends `src` and `dest`
    pub(crate) blend_factor: f32,
}

impl NameProvider for Transition {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Transition {
    /// Creates a new named transition between two states with a given time and a name of a parameter that
    /// will be used to check if it is possible to activate the transition.
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
            invert_rule: false,
            blend_factor: 0.0,
        }
    }

    /// Returns a reference to the name of the transition.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the amount of time required to perform a transition from source to destination state, in seconds.
    #[inline]
    pub fn transition_time(&self) -> f32 {
        self.transition_time
    }

    /// Returns a handle to source state.
    #[inline]
    pub fn source(&self) -> Handle<State> {
        self.source
    }

    /// Returns a handle to destination state.
    #[inline]
    pub fn dest(&self) -> Handle<State> {
        self.dest
    }

    /// Returns a name of a rule that is used to check if transition can be activated.
    #[inline]
    pub fn rule(&self) -> &str {
        self.rule.as_str()
    }

    /// Returns true if the transition from the source to the destination state was finished.
    #[inline]
    pub fn is_done(&self) -> bool {
        (self.transition_time - self.elapsed_time).abs() <= f32::EPSILON
    }

    /// Returns current blend factor. 0 - evaluates `source` pose, 1 - `destination`, 0..1 - blends `source` and `destination`.
    #[inline]
    pub fn blend_factor(&self) -> f32 {
        self.blend_factor
    }

    /// Tells the transition to invert the value of the transition rule. It is very useful if you have a single parameter, but
    /// two transitions in opposite directions. On the first (direct) transition you leave this flag `false`, but on the second
    /// (opposite) you set it to `true`. In this case the engine will automatically invert transition rule and you don't need
    /// to create a separate transition rule.
    #[inline]
    pub fn set_invert_rule(&mut self, invert: bool) {
        self.invert_rule = invert;
    }

    /// Returns true if the transition will inverse the value of the transition rule, false - otherwise.
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
