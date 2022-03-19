use crate::{
    animation::{
        machine::{
            node::{blend::BlendAnimations, play::PlayAnimation},
            BlendAnimationsByIndex, BlendPose, IndexedBlendInput, ParameterContainer,
        },
        Animation, AnimationContainer, AnimationPose,
    },
    core::{
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
};
use std::cell::Ref;

pub mod blend;
pub mod play;

/// Specialized node that provides animation pose. See documentation for each variant.
#[derive(Debug, Visit)]
pub enum PoseNode {
    /// See docs for `PlayAnimation`.
    PlayAnimation(PlayAnimation),

    /// See docs for `BlendAnimations`.
    BlendAnimations(BlendAnimations),

    /// See docs for `BlendAnimationsByIndex`.
    BlendAnimationsByIndex(BlendAnimationsByIndex),
}

impl Default for PoseNode {
    fn default() -> Self {
        Self::PlayAnimation(Default::default())
    }
}

impl PoseNode {
    /// Creates new node that plays animation.
    pub fn make_play_animation(animation: Handle<Animation>) -> Self {
        Self::PlayAnimation(PlayAnimation::new(animation))
    }

    /// Creates new node that blends multiple poses.
    pub fn make_blend_animations(poses: Vec<BlendPose>) -> Self {
        Self::BlendAnimations(BlendAnimations::new(poses))
    }

    /// Creates new node that blends multiple poses.
    pub fn make_blend_animations_by_index(
        index_parameter: String,
        inputs: Vec<IndexedBlendInput>,
    ) -> Self {
        Self::BlendAnimationsByIndex(BlendAnimationsByIndex::new(index_parameter, inputs))
    }
}

pub trait EvaluatePose {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        animations: &AnimationContainer,
        dt: f32,
    ) -> Ref<AnimationPose>;

    fn pose(&self) -> Ref<AnimationPose>;
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            PoseNode::PlayAnimation(v) => v.$func($($args),*),
            PoseNode::BlendAnimations(v) => v.$func($($args),*),
            PoseNode::BlendAnimationsByIndex(v) => v.$func($($args),*),
        }
    };
}

impl EvaluatePose for PoseNode {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        animations: &AnimationContainer,
        dt: f32,
    ) -> Ref<AnimationPose> {
        static_dispatch!(self, eval_pose, nodes, params, animations, dt)
    }

    fn pose(&self) -> Ref<AnimationPose> {
        static_dispatch!(self, pose,)
    }
}
