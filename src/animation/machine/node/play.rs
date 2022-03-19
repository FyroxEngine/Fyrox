use crate::{
    animation::{
        machine::{node::EvaluatePose, ParameterContainer, PoseNode},
        Animation, AnimationContainer, AnimationPose,
    },
    core::{
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
};
use std::cell::{Ref, RefCell};

/// Machine node that plays specified animation.
#[derive(Default, Debug, Visit, Clone)]
pub struct PlayAnimation {
    pub animation: Handle<Animation>,
    #[visit(skip)]
    output_pose: RefCell<AnimationPose>,
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
        animations
            .get(self.animation)
            .get_pose()
            .clone_into(&mut self.output_pose.borrow_mut());
        self.output_pose.borrow()
    }

    fn pose(&self) -> Ref<AnimationPose> {
        self.output_pose.borrow()
    }
}
