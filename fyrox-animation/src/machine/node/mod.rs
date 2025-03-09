// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Node is a part of animation blending tree, that backs a state with animation data. See [`PoseNode`] docs for
//! more info.

use crate::{
    core::{
        algebra::Vector2,
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
    },
    machine::{
        node::{blend::BlendAnimations, blendspace::BlendSpace, play::PlayAnimation},
        BlendAnimationsByIndex, BlendPose, IndexedBlendInput, ParameterContainer, State,
    },
    Animation, AnimationContainer, AnimationEvent, AnimationPose, EntityId,
};
use fxhash::FxHashSet;
use std::{
    cell::Ref,
    ops::{Deref, DerefMut},
};

pub mod blend;
pub mod blendspace;
pub mod play;

/// A set of common data fields that is used in every node.
#[derive(Debug, Visit, Clone, Default, Reflect, PartialEq)]
pub struct BasePoseNode<T: EntityId> {
    /// Position on the canvas, it is editor-specific data.
    pub position: Vector2<f32>,

    /// A handle of parent state that "owns" the node.
    #[reflect(hidden)]
    pub parent_state: Handle<State<T>>,
}

/// Specialized node that provides animation pose. See documentation for each variant.
#[derive(Debug, Visit, Clone, Reflect, PartialEq)]
pub enum PoseNode<T: EntityId> {
    /// See docs for [`PlayAnimation`].
    PlayAnimation(PlayAnimation<T>),

    /// See docs for [`BlendAnimations`].
    BlendAnimations(BlendAnimations<T>),

    /// See docs for [`BlendAnimationsByIndex`].
    BlendAnimationsByIndex(BlendAnimationsByIndex<T>),

    /// See doc for [`BlendSpace`]
    BlendSpace(BlendSpace<T>),
}

impl<T: EntityId> Default for PoseNode<T> {
    fn default() -> Self {
        Self::PlayAnimation(Default::default())
    }
}

impl<T: EntityId> PoseNode<T> {
    /// Creates new node that plays an animation.
    pub fn make_play_animation(animation: Handle<Animation<T>>) -> Self {
        Self::PlayAnimation(PlayAnimation::new(animation))
    }

    /// Creates new node that blends multiple poses into one.
    pub fn make_blend_animations(poses: Vec<BlendPose<T>>) -> Self {
        Self::BlendAnimations(BlendAnimations::new(poses))
    }

    /// Creates new node that switches between given animations using index and smoothly blends from
    /// one animation to another while switching.
    pub fn make_blend_animations_by_index(
        index_parameter: String,
        inputs: Vec<IndexedBlendInput<T>>,
    ) -> Self {
        Self::BlendAnimationsByIndex(BlendAnimationsByIndex::new(index_parameter, inputs))
    }

    /// Returns a set of handles to children pose nodes.
    pub fn children(&self) -> Vec<Handle<PoseNode<T>>> {
        match self {
            Self::PlayAnimation(_) => {
                // No children nodes.
                vec![]
            }
            Self::BlendAnimations(blend_animations) => blend_animations.children(),
            Self::BlendAnimationsByIndex(blend_by_index) => blend_by_index.children(),
            Self::BlendSpace(blend_space) => blend_space.children(),
        }
    }

    /// Collects all animation handles used by this node and its descendants.
    pub fn collect_animations(
        &self,
        nodes: &Pool<PoseNode<T>>,
        animations: &mut FxHashSet<Handle<Animation<T>>>,
    ) {
        match self {
            PoseNode::PlayAnimation(play_animation) => {
                animations.insert(play_animation.animation);
            }
            PoseNode::BlendAnimations(blend_animations) => {
                for input in blend_animations.pose_sources.iter() {
                    if let Some(source) = nodes.try_borrow(input.pose_source) {
                        source.collect_animations(nodes, animations)
                    }
                }
            }
            PoseNode::BlendAnimationsByIndex(blend_animations_by_index) => {
                for input in blend_animations_by_index.inputs.iter() {
                    if let Some(source) = nodes.try_borrow(input.pose_source) {
                        source.collect_animations(nodes, animations)
                    }
                }
            }
            PoseNode::BlendSpace(blend_space) => {
                for point in blend_space.points() {
                    if let Some(source) = nodes.try_borrow(point.pose_source) {
                        source.collect_animations(nodes, animations)
                    }
                }
            }
        }
    }
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            PoseNode::PlayAnimation(v) => v.$func($($args),*),
            PoseNode::BlendAnimations(v) => v.$func($($args),*),
            PoseNode::BlendAnimationsByIndex(v) => v.$func($($args),*),
            PoseNode::BlendSpace(v) => v.$func($($args),*),
        }
    };
}

impl<T: EntityId> Deref for PoseNode<T> {
    type Target = BasePoseNode<T>;

    fn deref(&self) -> &Self::Target {
        static_dispatch!(self, deref,)
    }
}

impl<T: EntityId> DerefMut for PoseNode<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        static_dispatch!(self, deref_mut,)
    }
}

/// A way of animation events collection.
#[derive(Copy, Clone, Debug)]
pub enum AnimationEventCollectionStrategy {
    /// Collect all events.
    All,
    /// Blending nodes will only emit events from nodes or states with max weight.
    MaxWeight,
    /// Blending nodes will only emit events from nodes or states with min weight.
    MinWeight,
}

/// A trait that responsible for animation pose evaluation.
pub trait AnimationPoseSource<T: EntityId> {
    /// Evaluates animation pose and returns a reference to it.
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        dt: f32,
    ) -> Ref<AnimationPose<T>>;

    /// Returns a reference to inner pose of a node.
    fn pose(&self) -> Ref<AnimationPose<T>>;

    /// Collects animation events from internals.
    fn collect_animation_events(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        strategy: AnimationEventCollectionStrategy,
    ) -> Vec<(Handle<Animation<T>>, AnimationEvent)>;
}

impl<T: EntityId> AnimationPoseSource<T> for PoseNode<T> {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        dt: f32,
    ) -> Ref<AnimationPose<T>> {
        static_dispatch!(self, eval_pose, nodes, params, animations, dt)
    }

    fn pose(&self) -> Ref<AnimationPose<T>> {
        static_dispatch!(self, pose,)
    }

    fn collect_animation_events(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        strategy: AnimationEventCollectionStrategy,
    ) -> Vec<(Handle<Animation<T>>, AnimationEvent)> {
        static_dispatch!(
            self,
            collect_animation_events,
            nodes,
            params,
            animations,
            strategy
        )
    }
}
