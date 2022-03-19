use crate::core::visitor::prelude::*;
use fxhash::FxHashMap;

/// Machine parameter.  Machine uses various parameters for specific actions. For example
/// Rule parameter is used to check where transition from a state to state is possible.
/// See module docs for example.
#[derive(Copy, Clone, Debug, Visit)]
pub enum Parameter {
    /// Weight parameter is used to control blend weight in BlendAnimation node.
    Weight(f32),

    /// Rule parameter is used to check where transition from a state to state is possible.
    Rule(bool),

    /// An index of pose.
    Index(u32),
}

impl Default for Parameter {
    fn default() -> Self {
        Self::Weight(0.0)
    }
}

/// Specific animation pose weight.
#[derive(Debug, Visit, Clone)]
pub enum PoseWeight {
    /// Fixed scalar value. Should not be negative (can't even realize what will happen
    /// with negative weight here)
    Constant(f32),

    /// Reference to Weight parameter with given name.
    Parameter(String),
}

impl Default for PoseWeight {
    fn default() -> Self {
        Self::Constant(0.0)
    }
}

pub type ParameterContainer = FxHashMap<String, Parameter>;
