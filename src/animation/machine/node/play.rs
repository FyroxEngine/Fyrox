use crate::{
    animation::{
        machine::{
            node::{BasePoseNode, BasePoseNodeDefinition, EvaluatePose},
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

#[derive(Default, Debug, Visit, Clone, Inspect)]
pub struct TimeSlice(pub Range<f32>);

#[derive(Debug, Visit, Clone, Inspect)]
pub struct PlayAnimationDefinition {
    pub base: BasePoseNodeDefinition,
    pub animation: String,
    #[visit(optional)] // Backward compatibility
    pub speed: f32,
    #[visit(optional)] // Backward compatibility
    pub time_slice: Option<TimeSlice>,
}

impl Default for PlayAnimationDefinition {
    fn default() -> Self {
        Self {
            base: Default::default(),
            animation: "".to_string(),
            speed: 1.0,
            time_slice: None,
        }
    }
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
                .get_pose()
                .clone_into(&mut self.output_pose.borrow_mut());
        }
        self.output_pose.borrow()
    }

    fn pose(&self) -> Ref<AnimationPose> {
        self.output_pose.borrow()
    }
}
