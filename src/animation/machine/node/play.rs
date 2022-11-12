use crate::{
    animation::{
        machine::{
            node::{BasePoseNode, EvaluatePose},
            ParameterContainer, PoseNode,
        },
        Animation, AnimationContainer, AnimationPose,
    },
    core::{
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
    },
};
use std::ops::Range;
use std::{
    cell::{Ref, RefCell},
    ops::{Deref, DerefMut},
};

/// Machine node that plays specified animation.
#[derive(Default, Debug, Visit, Clone)]
pub struct PlayAnimation {
    pub base: BasePoseNode,
    pub animation: Handle<Animation>,
    #[visit(skip)]
    pub(crate) output_pose: RefCell<AnimationPose>,
}

impl Deref for PlayAnimation {
    type Target = BasePoseNode;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for PlayAnimation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[derive(Default, Debug, Visit, Clone, Reflect)]
pub struct TimeSlice(pub Range<f32>);

impl PlayAnimation {
    /// Creates new PlayAnimation node with given animation handle.
    pub fn new(animation: Handle<Animation>) -> Self {
        Self {
            base: Default::default(),
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
                .pose()
                .clone_into(&mut self.output_pose.borrow_mut());
        }
        self.output_pose.borrow()
    }

    fn pose(&self) -> Ref<AnimationPose> {
        self.output_pose.borrow()
    }
}
