use crate::{
    animation::{
        machine::{
            node::{BasePoseNodeDefinition, EvaluatePose},
            ParameterContainer, PoseNode,
        },
        Animation, AnimationContainer, AnimationPose,
    },
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
};
use std::{
    cell::{Ref, RefCell},
    ops::{Deref, DerefMut},
};

/// Machine node that plays specified animation.
#[derive(Default, Debug, Visit, Clone)]
pub struct PlayAnimation {
    pub animation: Handle<Animation>,
    #[visit(skip)]
    output_pose: RefCell<AnimationPose>,
}

#[derive(Default, Debug, Visit, Clone, Inspect)]
pub struct PlayAnimationDefinition {
    pub base: BasePoseNodeDefinition,
    pub animation: String,
}

impl Deref for PlayAnimationDefinition {
    type Target = BasePoseNodeDefinition;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for PlayAnimationDefinition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl PlayAnimation {
    /// Creates new PlayAnimation node with given animation handle.
    pub fn new(animation: Handle<Animation>) -> Self {
        Self {
            animation,
            output_pose: Default::default(),
        }
    }
}

impl EvaluatePose for PlayAnimation {
    fn eval_pose(
        &self,
        _nodes: &Pool<PoseNode>,
        _params: &ParameterContainer,
        animations: &AnimationContainer,
        _dt: f32,
    ) -> Ref<AnimationPose> {
        if let Some(animation) = animations.try_get(self.animation) {
            animation
                .get_pose()
                .clone_into(&mut self.output_pose.borrow_mut());
        }
        self.output_pose.borrow()
    }

    fn pose(&self) -> Ref<AnimationPose> {
        self.output_pose.borrow()
    }
}
