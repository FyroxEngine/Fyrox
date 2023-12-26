//! Transition is a connection between two states with a rule that defines possibility of actual transition with blending.

use crate::{
    core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
    machine::{Parameter, ParameterContainer, State},
    Animation, AnimationContainer,
};
use fyrox_core::{uuid_provider, NameProvider};
use std::any::{type_name, Any, TypeId};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

macro_rules! define_two_args_node {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            /// Left argument.
            pub lhs: Box<LogicNode>,
            /// Right argument.
            pub rhs: Box<LogicNode>,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    lhs: Box::new(LogicNode::Parameter(Default::default())),
                    rhs: Box::new(LogicNode::Parameter(Default::default())),
                }
            }
        }

        impl Visit for $name {
            fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
                let mut guard = visitor.enter_region(name)?;

                self.lhs.visit("Lhs", &mut guard)?;
                self.rhs.visit("Rhs", &mut guard)?;

                Ok(())
            }
        }

        impl Reflect for $name {
            fn type_name(&self) -> &'static str {
                type_name::<Self>()
            }

            fn doc(&self) -> &'static str {
                ""
            }

            fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
                func(&[
                    FieldInfo {
                        owner_type_id: TypeId::of::<Self>(),
                        name: "Lhs",
                        display_name: "Lhs",
                        description: "",
                        type_name: type_name::<Self>(),
                        value: &*self.lhs,
                        reflect_value: &*self.lhs,
                        read_only: false,
                        immutable_collection: false,
                        min_value: None,
                        max_value: None,
                        step: None,
                        precision: None,
                        doc: "",
                    },
                    FieldInfo {
                        owner_type_id: TypeId::of::<Self>(),
                        name: "Rhs",
                        display_name: "Rhs",
                        description: "",
                        type_name: type_name::<Self>(),
                        value: &*self.rhs,
                        reflect_value: &*self.rhs,
                        read_only: false,
                        immutable_collection: false,
                        min_value: None,
                        max_value: None,
                        step: None,
                        precision: None,doc: "",
                    },
                ])
            }

            fn into_any(self: Box<Self>) -> Box<dyn Any> {
                self
            }

            fn as_any(&self, func: &mut dyn FnMut(&dyn ::core::any::Any)) {
                func(self)
            }

            fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn ::core::any::Any)) {
                func(self)
            }

            fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
                func(self)
            }

            fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
                func(self)
            }

            fn set(
                &mut self,
                value: Box<dyn Reflect>,
            ) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
                let this = std::mem::replace(self, value.take()?);
                Ok(Box::new(this))
            }

            fn fields(&self, func: &mut dyn FnMut(&[&dyn Reflect])) {
                func(&[&self.lhs, &self.rhs])
            }

           fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [&mut dyn Reflect])) {
                func(&mut [&mut self.lhs, &mut self.rhs])
            }

            fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
                func(match name {
                    "Lhs" => Some(&self.lhs),
                    "Rhs" => Some(&self.rhs),
                    _ => None,
                })
            }

            fn field_mut(
                &mut self,
                name: &str,
                func: &mut dyn FnMut(Option<&mut dyn Reflect>),
            ) {
                func(match name {
                    "Lhs" => Some(&mut self.lhs),
                    "Rhs" => Some(&mut self.rhs),
                    _ => None,
                })
            }
        }
    };
}

define_two_args_node!(
    /// Calculates logical AND between two arguments. Output value will be `true` iff both of the arguments is `true`.
    AndNode
);
define_two_args_node!(
    /// Calculates logical OR between two arguments. Output value will be `true` iff any of the arguments is `true`.
    OrNode
);
define_two_args_node!(
    /// Calculates logical XOR (excluding OR) between two arguments. Output value will be `true` iff the arguments differ.
    XorNode
);

/// Calculates logical NOT of an argument. Output value will be `true` if the value of the argument is `false`.
#[derive(Debug, Clone, PartialEq)]
pub struct NotNode {
    /// Argument to be negated.
    pub lhs: Box<LogicNode>,
}

impl Default for NotNode {
    fn default() -> Self {
        Self {
            lhs: Box::new(LogicNode::Parameter(Default::default())),
        }
    }
}

impl Visit for NotNode {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut guard = visitor.enter_region(name)?;

        self.lhs.visit("Lhs", &mut guard)?;

        Ok(())
    }
}

impl Reflect for NotNode {
    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn doc(&self) -> &'static str {
        ""
    }

    fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
        func(&[FieldInfo {
            owner_type_id: TypeId::of::<Self>(),
            name: "Lhs",
            display_name: "Lhs",
            description: "",
            type_name: type_name::<Self>(),
            value: &*self.lhs,
            reflect_value: &*self.lhs,
            read_only: false,
            immutable_collection: false,
            min_value: None,
            max_value: None,
            step: None,
            precision: None,
            doc: "",
        }])
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self, func: &mut dyn FnMut(&dyn ::core::any::Any)) {
        func(self)
    }

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn ::core::any::Any)) {
        func(self)
    }

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        func(self)
    }

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        func(self)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        let this = std::mem::replace(self, value.take()?);
        Ok(Box::new(this))
    }

    fn fields(&self, func: &mut dyn FnMut(&[&dyn Reflect])) {
        func(&[&self.lhs])
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [&mut dyn Reflect])) {
        func(&mut [&mut self.lhs])
    }

    fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        func(match name {
            "Lhs" => Some(&self.lhs),
            _ => None,
        })
    }

    fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
        func(match name {
            "Lhs" => Some(&mut self.lhs),
            _ => None,
        })
    }
}

/// A node responsible for logical operations evaluation. It can have any number of descendant nodes.
///
/// # Examples
///
/// ```rust
/// use fyrox_animation::AnimationContainer;
/// use fyrox_animation::machine::{
///     transition::{AndNode, LogicNode, NotNode},
///     Parameter, ParameterContainer,
/// };
///
/// let mut parameters = ParameterContainer::default();
/// parameters.add("Run", Parameter::Rule(false));
/// parameters.add("Jump", Parameter::Rule(true));
///
/// // !Run && Jump
/// let transition_logic = LogicNode::And(AndNode {
///     lhs: Box::new(LogicNode::Not(NotNode {
///         lhs: Box::new(LogicNode::Parameter("Run".to_string())),
///     })),
///     rhs: Box::new(LogicNode::Parameter("Jump".to_string())),
/// });
///
/// assert_eq!(transition_logic.calculate_value(&parameters, &AnimationContainer::default()), true);
/// ```
#[derive(Debug, Visit, Clone, Reflect, PartialEq, AsRefStr, EnumString, EnumVariantNames)]
pub enum LogicNode {
    /// Fetches a value of `Rule` parameter and returns its value. `false` if the parameter is not found.
    Parameter(String),
    /// Calculates logical AND between two arguments. Output value will be `true` iff both of the arguments is `true`.
    And(AndNode),
    /// Calculates logical OR between two arguments. Output value will be `true` iff any of the arguments is `true`.
    Or(OrNode),
    /// Calculates logical XOR (excluding OR) between two arguments. Output value will be `true` iff the arguments differ.
    Xor(XorNode),
    /// Calculates logical NOT of an argument. Output value will be `true` if the value of the argument is `false`.
    Not(NotNode),
    /// Returns `true` if the animation has ended, `false` - otherwise.
    IsAnimationEnded(Handle<Animation>),
}

uuid_provider!(LogicNode = "98a5b767-5560-4ed7-ad40-1625a8868e39");

impl Default for LogicNode {
    fn default() -> Self {
        Self::Parameter(Default::default())
    }
}

impl LogicNode {
    /// Calculates final value of the logic node.
    pub fn calculate_value(
        &self,
        parameters: &ParameterContainer,
        animations: &AnimationContainer,
    ) -> bool {
        match self {
            LogicNode::Parameter(rule_name) => parameters.get(rule_name).map_or(false, |p| {
                if let Parameter::Rule(rule_value) = p {
                    *rule_value
                } else {
                    false
                }
            }),
            LogicNode::And(and) => {
                let lhs_value = and.lhs.calculate_value(parameters, animations);
                let rhs_value = and.rhs.calculate_value(parameters, animations);
                lhs_value & rhs_value
            }
            LogicNode::Or(or) => {
                let lhs_value = or.lhs.calculate_value(parameters, animations);
                let rhs_value = or.rhs.calculate_value(parameters, animations);
                lhs_value | rhs_value
            }
            LogicNode::Xor(or) => {
                let lhs_value = or.lhs.calculate_value(parameters, animations);
                let rhs_value = or.rhs.calculate_value(parameters, animations);
                lhs_value ^ rhs_value
            }
            LogicNode::Not(node) => !node.lhs.calculate_value(parameters, animations),
            LogicNode::IsAnimationEnded(animation) => animations
                .try_get(*animation)
                .map_or(true, |a| a.has_ended()),
        }
    }
}

/// Transition is a connection between two states with a rule that defines possibility of actual transition with blending.
#[derive(Default, Debug, Clone, Reflect, PartialEq)]
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

    #[reflect(
        description = "Computational graph that can use any amount of Rule parameters to calculate transition value."
    )]
    pub(crate) condition: LogicNode,

    /// 0 - evaluates `src` pose, 1 - `dest`, 0..1 - blends `src` and `dest`
    pub(crate) blend_factor: f32,
}

impl Visit for Transition {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut guard = visitor.enter_region(name)?;

        self.name.visit("Name", &mut guard)?;
        self.transition_time.visit("TransitionTime", &mut guard)?;
        self.source.visit("Source", &mut guard)?;
        self.dest.visit("Dest", &mut guard)?;
        self.blend_factor.visit("BlendFactor", &mut guard)?;

        if guard.is_reading() {
            if self.condition.visit("Condition", &mut guard).is_err() {
                // Try to convert the old version.
                let mut invert_rule = false;
                let mut rule: String = Default::default();

                invert_rule.visit("InvertRule", &mut guard)?;
                rule.visit("Rule", &mut guard)?;

                if invert_rule {
                    self.condition = LogicNode::Not(NotNode {
                        lhs: Box::new(LogicNode::Parameter(rule)),
                    });
                } else {
                    self.condition = LogicNode::Parameter(rule);
                }
            }
        } else {
            self.condition.visit("Condition", &mut guard)?;
        }

        Ok(())
    }
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
            blend_factor: 0.0,
            condition: LogicNode::Parameter(rule.to_owned()),
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

    /// Sets new condition for the transition.
    pub fn set_condition(&mut self, condition: LogicNode) {
        self.condition = condition;
    }

    /// Returns a reference to the current condition of the transition.
    pub fn condition(&self) -> &LogicNode {
        &self.condition
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
