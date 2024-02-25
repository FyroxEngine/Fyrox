//! State is a final "container" for animation pose. See [`State`] docs for more info.

use crate::{
    core::{
        algebra::Vector2,
        pool::{Handle, Pool},
        rand::{self, seq::IteratorRandom},
        reflect::prelude::*,
        visitor::prelude::*,
    },
    machine::{AnimationPoseSource, ParameterContainer, PoseNode},
    Animation, AnimationContainer, AnimationPose, EntityId,
};
use fyrox_core::uuid::{uuid, Uuid};
use fyrox_core::{NameProvider, TypeUuidProvider};
use std::{
    cell::Ref,
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[doc(hidden)]
#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq)]
pub struct StateActionWrapper<T: EntityId>(pub StateAction<T>);

impl<T: EntityId> TypeUuidProvider for StateActionWrapper<T> {
    fn type_uuid() -> Uuid {
        uuid!("d686fac8-5cc1-46b1-82a4-7f4438cc078d")
    }
}

impl<T: EntityId> Deref for StateActionWrapper<T> {
    type Target = StateAction<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: EntityId> DerefMut for StateActionWrapper<T> {
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
#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq, VariantNames, EnumString, AsRefStr)]
pub enum StateAction<T: EntityId> {
    /// No action.
    #[default]
    None,
    /// Rewinds the animation.
    RewindAnimation(Handle<Animation<T>>),
    /// Enables the animation.
    EnableAnimation(Handle<Animation<T>>),
    /// Disables the animation.
    DisableAnimation(Handle<Animation<T>>),
    /// Enables random animation from the list. It could be useful if you want to add randomization
    /// to your state machine. For example, you may have few melee attack animations and all of them
    /// are suitable for every situation, in this case you can add randomization to make attacks less
    /// predictable.
    EnableRandomAnimation(Vec<Handle<Animation<T>>>),
}

impl<T: EntityId> TypeUuidProvider for StateAction<T> {
    fn type_uuid() -> Uuid {
        uuid!("c50a15cc-0f63-4409-bbe0-74b9d3e94755")
    }
}

impl<T: EntityId> StateAction<T> {
    /// Applies the action to the given animation container.
    pub fn apply(&self, animations: &mut AnimationContainer<T>) {
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
            StateAction::EnableRandomAnimation(animation_handles) => {
                if let Some(animation) = animation_handles.iter().choose(&mut rand::thread_rng()) {
                    if let Some(animation) = animations.try_get_mut(*animation) {
                        animation.set_enabled(true);
                    }
                }
            }
        }
    }
}

/// State is a final "container" for animation pose. It has backing pose node which provides a set of values.
/// States can be connected with each other using _transitions_, states with transitions form a state graph.
#[derive(Default, Debug, Visit, Clone, Reflect, PartialEq)]
pub struct State<T: EntityId> {
    /// Position of state on the canvas. It is editor-specific data.
    pub position: Vector2<f32>,

    /// Name of the state.
    pub name: String,

    /// A set of actions that will be executed when entering the state.
    #[visit(optional)]
    pub on_enter_actions: Vec<StateActionWrapper<T>>,

    /// A set of actions that will be executed when leaving the state.
    #[visit(optional)]
    pub on_leave_actions: Vec<StateActionWrapper<T>>,

    /// Root node of the state that provides the state with animation data.
    #[reflect(read_only)]
    pub root: Handle<PoseNode<T>>,
}

impl<T: EntityId> NameProvider for State<T> {
    fn name(&self) -> &str {
        &self.name
    }
}

impl<T: EntityId> State<T> {
    /// Creates new instance of state with a given pose.
    pub fn new(name: &str, root: Handle<PoseNode<T>>) -> Self {
        Self {
            position: Default::default(),
            name: name.to_owned(),
            on_enter_actions: Default::default(),
            on_leave_actions: Default::default(),
            root,
        }
    }

    /// Returns a final pose of the state.
    pub fn pose<'a>(&self, nodes: &'a Pool<PoseNode<T>>) -> Option<Ref<'a, AnimationPose<T>>> {
        nodes.try_borrow(self.root).map(|root| root.pose())
    }

    pub(super) fn update(
        &mut self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        dt: f32,
    ) {
        if let Some(root) = nodes.try_borrow(self.root) {
            root.eval_pose(nodes, params, animations, dt);
        }
    }
}
