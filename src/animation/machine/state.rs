//! State is a final "container" for animation pose. See [`State`] docs for more info.

use crate::{
    animation::{
        machine::{EvaluatePose, ParameterContainer, PoseNode},
        Animation, AnimationContainer, AnimationPose,
    },
    core::{
        algebra::Vector2,
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
    },
    utils::NameProvider,
};
use std::cell::Ref;
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

#[doc(hidden)]
#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq)]
pub struct StateActionWrapper(pub StateAction);

impl Deref for StateActionWrapper {
    type Target = StateAction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StateActionWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// An action, that will be executed by a state. It usually used to rewind, enable/disable animations
/// when entering or leaving states. This is useful in situations when you have a one-shot animation
/// and you need to rewind it before when entering some state. For example, you may have looped idle
/// state and one-shot attack state. In this case, you need to use [`StateAction::RewindAnimation`]
/// to tell the engine to automatically rewind the animation before using it. Otherwise, when the
/// transition will happen, the animation could be ended already and you'll get "frozen" animation.
#[derive(
    Default, Debug, Visit, Reflect, Clone, PartialEq, EnumVariantNames, EnumString, AsRefStr,
)]
pub enum StateAction {
    /// No action.
    #[default]
    None,
    /// Rewinds the animation.
    RewindAnimation(Handle<Animation>),
    /// Enables the animation.
    EnableAnimation(Handle<Animation>),
    /// Disables the animation.
    DisableAnimation(Handle<Animation>),
}

impl StateAction {
    /// Applies the action to the given animation container.
    pub fn apply(&self, animations: &mut AnimationContainer) {
        match self {
            StateAction::None => {}
            StateAction::RewindAnimation(animation) => {
                if let Some(animation) = animations.try_get_mut(*animation) {
                    animation.rewind();
                }
            }
            StateAction::EnableAnimation(animation) => {
                if let Some(animation) = animations.try_get_mut(*animation) {
                    animation.set_enabled(true);
                }
            }
            StateAction::DisableAnimation(animation) => {
                if let Some(animation) = animations.try_get_mut(*animation) {
                    animation.set_enabled(false);
                }
            }
        }
    }
}

/// State is a final "container" for animation pose. It has backing pose node which provides a set of values.
/// States can be connected with each other using _transitions_, states with transitions form a state graph.
#[derive(Default, Debug, Visit, Clone, Reflect, PartialEq)]
pub struct State {
    /// Position of state on the canvas. It is editor-specific data.
    pub position: Vector2<f32>,

    /// Name of the state.
    pub name: String,

    /// A set of actions that will be executed when entering the state.
    #[visit(optional)]
    pub on_enter_actions: Vec<StateActionWrapper>,

    /// A set of actions that will be executed when leaving the state.
    #[visit(optional)]
    pub on_leave_actions: Vec<StateActionWrapper>,

    /// Root node of the state that provides the state with animation data.
    #[reflect(read_only)]
    pub root: Handle<PoseNode>,
}

impl NameProvider for State {
    fn name(&self) -> &str {
        &self.name
    }
}

impl State {
    /// Creates new instance of state with a given pose.
    pub fn new(name: &str, root: Handle<PoseNode>) -> Self {
        Self {
            position: Default::default(),
            name: name.to_owned(),
            on_enter_actions: Default::default(),
            on_leave_actions: Default::default(),
            root,
        }
    }

    /// Returns a final pose of the state.
    pub fn pose<'a>(&self, nodes: &'a Pool<PoseNode>) -> Option<Ref<'a, AnimationPose>> {
        nodes.try_borrow(self.root).map(|root| root.pose())
    }

    pub(super) fn update(
        &mut self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        animations: &AnimationContainer,
        dt: f32,
    ) {
        if let Some(root) = nodes.try_borrow(self.root) {
            root.eval_pose(nodes, params, animations, dt);
        }
    }
}
