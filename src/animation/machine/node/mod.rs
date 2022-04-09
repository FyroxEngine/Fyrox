use crate::animation::machine::state::StateDefinition;
use crate::{
    animation::{
        machine::{
            node::{
                blend::{
                    BlendAnimations, BlendAnimationsByIndexDefinition, BlendAnimationsDefinition,
                },
                play::{PlayAnimation, PlayAnimationDefinition},
            },
            BlendAnimationsByIndex, BlendPose, IndexedBlendInput, ParameterContainer,
        },
        Animation, AnimationContainer, AnimationPose,
    },
    core::{
        algebra::Vector2,
        inspect::{Inspect, PropertyInfo},
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
};
use std::{
    cell::Ref,
    ops::{Deref, DerefMut},
};

pub mod blend;
pub mod play;

/// Specialized node that provides animation pose. See documentation for each variant.
#[derive(Debug, Visit, Clone)]
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

#[derive(Default, Debug, Visit, Clone, Inspect)]
pub struct BasePoseNodeDefinition {
    pub position: Vector2<f32>,
    pub parent_state: Handle<StateDefinition>,
}

#[derive(Debug, Visit, Clone)]
pub enum PoseNodeDefinition {
    PlayAnimation(PlayAnimationDefinition),
    BlendAnimations(BlendAnimationsDefinition),
    BlendAnimationsByIndex(BlendAnimationsByIndexDefinition),
}

impl Default for PoseNodeDefinition {
    fn default() -> Self {
        Self::PlayAnimation(Default::default())
    }
}

impl Deref for PoseNodeDefinition {
    type Target = BasePoseNodeDefinition;

    fn deref(&self) -> &Self::Target {
        match self {
            PoseNodeDefinition::PlayAnimation(v) => v,
            PoseNodeDefinition::BlendAnimations(v) => v,
            PoseNodeDefinition::BlendAnimationsByIndex(v) => v,
        }
    }
}

impl DerefMut for PoseNodeDefinition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            PoseNodeDefinition::PlayAnimation(v) => v,
            PoseNodeDefinition::BlendAnimations(v) => v,
            PoseNodeDefinition::BlendAnimationsByIndex(v) => v,
        }
    }
}

impl Inspect for PoseNodeDefinition {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            PoseNodeDefinition::PlayAnimation(v) => v.properties(),
            PoseNodeDefinition::BlendAnimations(v) => v.properties(),
            PoseNodeDefinition::BlendAnimationsByIndex(v) => v.properties(),
        }
    }
}
