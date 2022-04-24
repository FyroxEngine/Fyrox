use crate::{
    animation::{
        machine::{node::PoseNodeDefinition, EvaluatePose, ParameterContainer, PoseNode},
        AnimationContainer, AnimationPose,
    },
    core::{
        algebra::Vector2,
        inspect::{Inspect, PropertyInfo},
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
};
use std::cell::Ref;

/// State is a final "container" for animation pose. It has backing pose node which provides a
/// set of values.
#[derive(Default, Debug, Visit, Clone)]
pub struct State {
    pub definition: Handle<StateDefinition>,
    pub(crate) name: String,
    pub(crate) root: Handle<PoseNode>,
}

#[derive(Default, Debug, Visit, Clone, Inspect)]
pub struct StateDefinition {
    pub position: Vector2<f32>,
    pub name: String,
    #[inspect(skip)]
    pub root: Handle<PoseNodeDefinition>,
}

impl State {
    /// Creates new instance of state with a given pose.
    pub fn new(name: &str, root: Handle<PoseNode>) -> Self {
        Self {
            definition: Default::default(),
            name: name.to_owned(),
            root,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

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
