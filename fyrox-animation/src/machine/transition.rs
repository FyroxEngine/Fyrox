//! Transition is a connection between two states with a rule that defines possibility of actual transition with blending.

use crate::{
    core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
    machine::{Parameter, ParameterContainer, State},
    Animation, AnimationContainer, EntityId,
};
use fyrox_core::uuid::{uuid, Uuid};
use fyrox_core::{NameProvider, TypeUuidProvider};
use std::any::{type_name, Any, TypeId};
use strum_macros::{AsRefStr, EnumString, VariantNames};

macro_rules! define_two_args_node {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name <T:EntityId> {
            /// Left argument.
            pub lhs: Box<LogicNode<T>>,
            /// Right argument.
            pub rhs: Box<LogicNode<T>>,
        }

        impl<T:EntityId> Default for $name<T> {
            fn default() -> Self {
                Self {
                    lhs: Box::new(LogicNode::Parameter(Default::default())),
                    rhs: Box::new(LogicNode::Parameter(Default::default())),
                }
            }
        }

        impl<T:EntityId> Visit for $name<T> {
            fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
                let mut guard = visitor.enter_region(name)?;

                self.lhs.visit("Lhs", &mut guard)?;
                self.rhs.visit("Rhs", &mut guard)?;

                Ok(())
            }
        }

        impl<T:EntityId> Reflect for $name<T> {
            fn source_path() -> &'static str {
                file!()
            }

            fn type_name(&self) -> &'static str {
                type_name::<Self>()
            }

            fn doc(&self) -> &'static str {
                ""
            }

            fn assembly_name(&self) -> &'static str {
                env!("CARGO_PKG_NAME")
            }

            fn type_assembly_name() -> &'static str {
                env!("CARGO_PKG_NAME")
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
pub struct NotNode<T: EntityId> {
    /// Argument to be negated.
    pub lhs: Box<LogicNode<T>>,
}

impl<T: EntityId> Default for NotNode<T> {
    fn default() -> Self {
        Self {
            lhs: Box::new(LogicNode::Parameter(Default::default())),
        }
    }
}

impl<T: EntityId> Visit for NotNode<T> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut guard = visitor.enter_region(name)?;

        self.lhs.visit("Lhs", &mut guard)?;

        Ok(())
    }
}

impl<T: EntityId> Reflect for NotNode<T> {
    fn source_path() -> &'static str {
        file!()
    }

    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn doc(&self) -> &'static str {
        ""
    }

    fn assembly_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn type_assembly_name() -> &'static str {
        env!("CARGO_PKG_NAME")
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
/// use fyrox_core::pool::ErasedHandle;
///
/// let mut parameters = ParameterContainer::default();
/// parameters.add("Run", Parameter::Rule(false));
/// parameters.add("Jump", Parameter::Rule(true));
///
/// // !Run && Jump
/// let transition_logic = LogicNode::<ErasedHandle>::And(AndNode {
///     lhs: Box::new(LogicNode::Not(NotNode {
///         lhs: Box::new(LogicNode::Parameter("Run".to_string())),
///     })),
///     rhs: Box::new(LogicNode::Parameter("Jump".to_string())),
/// });
///
/// assert_eq!(transition_logic.calculate_value(&parameters, &AnimationContainer::default()), true);
/// ```
#[derive(Debug, Visit, Clone, Reflect, PartialEq, AsRefStr, EnumString, VariantNames)]
pub enum LogicNode<T: EntityId> {
    /// Fetches a value of `Rule` parameter and returns its value. `false` if the parameter is not found.
    Parameter(String),
    /// Calculates logical AND between two arguments. Output value will be `true` iff both of the arguments is `true`.
    And(AndNode<T>),
    /// Calculates logical OR between two arguments. Output value will be `true` iff any of the arguments is `true`.
    Or(OrNode<T>),
    /// Calculates logical XOR (excluding OR) between two arguments. Output value will be `true` iff the arguments differ.
    Xor(XorNode<T>),
    /// Calculates logical NOT of an argument. Output value will be `true` if the value of the argument is `false`.
    Not(NotNode<T>),
    /// Returns `true` if the animation has ended, `false` - otherwise.
    IsAnimationEnded(Handle<Animation<T>>),
}

impl<T: EntityId> TypeUuidProvider for LogicNode<T> {
    fn type_uuid() -> Uuid {
        uuid!("98a5b767-5560-4ed7-ad40-1625a8868e39")
    }
}

impl<T: EntityId> Default for LogicNode<T> {
    fn default() -> Self {
        Self::Parameter(Default::default())
    }
}

impl<T: EntityId> LogicNode<T> {
    /// Calculates final value of the logic node.
    pub fn calculate_value(
        &self,
        parameters: &ParameterContainer,
        animations: &AnimationContainer<T>,
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
pub struct Transition<T: EntityId> {
    /// The name of the transition, it is used for debug output.
    #[reflect(description = "The name of the transition, it is used for debug output.")]
    pub(crate) name: String,

    /// Total amount of time to transition from `src` to `dst` state.
    #[reflect(description = "Total amount of time (in seconds) to transition \
        from source to destination state")]
    pub(crate) transition_time: f32,

    pub(crate) elapsed_time: f32,

    #[reflect(read_only)]
    pub(crate) source: Handle<State<T>>,

    #[reflect(read_only)]
    pub(crate) dest: Handle<State<T>>,

    #[reflect(
        description = "Computational graph that can use any amount of Rule parameters to calculate transition value."
    )]
    pub(crate) condition: LogicNode<T>,

    /// 0 - evaluates `src` pose, 1 - `dest`, 0..1 - blends `src` and `dest`
    pub(crate) blend_factor: f32,
}

impl<T: EntityId> Visit for Transition<T> {
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

impl<T: EntityId> NameProvider for Transition<T> {
    fn name(&self) -> &str {
        &self.name
    }
}

impl<T: EntityId> Transition<T> {
    /// Creates a new named transition between two states with a given time and a name of a parameter that
    /// will be used to check if it is possible to activate the transition.
    pub fn new(
        name: &str,
        src: Handle<State<T>>,
        dest: Handle<State<T>>,
        time: f32,
        rule: &str,
    ) -> Transition<T> {
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
    pub fn source(&self) -> Handle<State<T>> {
        self.source
    }

    /// Returns a handle to destination state.
    #[inline]
    pub fn dest(&self) -> Handle<State<T>> {
        self.dest
    }

    /// Sets new condition for the transition.
    pub fn set_condition(&mut self, condition: LogicNode<T>) {
        self.condition = condition;
    }

    /// Returns a reference to the current condition of the transition.
    pub fn condition(&self) -> &LogicNode<T> {
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
