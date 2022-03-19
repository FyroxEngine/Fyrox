use crate::{
    animation::{
        machine::{EvaluatePose, ParameterContainer, PoseNode},
        AnimationContainer, AnimationPose,
    },
    core::{
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
};
use std::cell::Ref;

/// State is a
#[derive(Default, Debug, Visit)]
pub struct State {
    name: String,
    root: Handle<PoseNode>,
}

impl State {
    /// Creates new instance of state with a given pose.
    pub fn new(name: &str, root: Handle<PoseNode>) -> Self {
        Self {
            name: name.to_owned(),
            root,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn pose<'a>(&self, nodes: &'a Pool<PoseNode>) -> Ref<'a, AnimationPose> {
        nodes[self.root].pose()
    }

    pub(super) fn update(
        &mut self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        animations: &AnimationContainer,
        dt: f32,
    ) {
        nodes
            .borrow(self.root)
            .eval_pose(nodes, params, animations, dt);
    }
}
