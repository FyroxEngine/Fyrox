use crate::core::{reflect::prelude::*, visitor::prelude::*};
use fxhash::FxHashMap;
use fyrox_core::parking_lot::Mutex;
use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// Machine parameter.  Machine uses various parameters for specific actions. For example
/// Rule parameter is used to check where transition from a state to state is possible.
/// See module docs for example.
#[derive(Copy, Clone, Debug, Reflect, Visit, EnumVariantNames, EnumString, AsRefStr)]
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
#[derive(Debug, Visit, Clone, Reflect, EnumVariantNames, EnumString, AsRefStr)]
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

#[derive(Reflect, Visit, Default, Debug, Clone)]
pub struct ParameterDefinition {
    pub name: String,
    pub value: Parameter,
}

#[derive(Default, Debug, Clone)]
struct Wrapper {
    parameters: Vec<ParameterDefinition>,
    dirty: Cell<bool>,
}

impl Visit for Wrapper {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.parameters.visit(name, visitor)
    }
}

impl Deref for Wrapper {
    type Target = Vec<ParameterDefinition>;

    fn deref(&self) -> &Self::Target {
        &self.parameters
    }
}

impl DerefMut for Wrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty.set(true);
        &mut self.parameters
    }
}

#[derive(Reflect, Visit, Default, Debug)]
pub struct ParameterContainer {
    #[reflect(deref)]
    parameters: Wrapper,

    #[reflect(hidden)]
    #[visit(skip)]
    lookup: Mutex<FxHashMap<String, usize>>,
}

impl Clone for ParameterContainer {
    fn clone(&self) -> Self {
        Self {
            parameters: self.parameters.clone(),
            lookup: Mutex::new(self.lookup.lock().clone()),
        }
    }
}

impl ParameterContainer {
    fn update_index(&self) {
        if self.parameters.dirty.get() {
            *self.lookup.lock() = self
                .parameters
                .parameters
                .iter()
                .enumerate()
                .map(|(i, p)| (p.name.clone(), i))
                .collect();
            self.parameters.dirty.set(false);
        }
    }

    pub fn add(&mut self, name: &str, value: Parameter) {
        self.parameters.push(ParameterDefinition {
            name: name.to_string(),
            value,
        })
    }

    pub fn get(&self, name: &str) -> Option<&Parameter> {
        self.update_index();
        self.lookup
            .lock()
            .get(name)
            .and_then(|i| self.parameters.parameters.get(*i).map(|d| &d.value))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Parameter> {
        self.update_index();
        self.lookup
            .lock()
            .get(name)
            .and_then(|i| self.parameters.parameters.get_mut(*i).map(|d| &mut d.value))
    }
}
