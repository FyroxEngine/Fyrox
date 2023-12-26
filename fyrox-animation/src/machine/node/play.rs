//! A simplest pose node that extracts pose from a specific animation and prepares it for further use.

use crate::{
    core::{
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
    },
    machine::{
        node::AnimationEventCollectionStrategy,
        node::{AnimationPoseSource, BasePoseNode},
        ParameterContainer, PoseNode,
    },
    Animation, AnimationContainer, AnimationEvent, AnimationPose, EntityId,
};
use std::{
    cell::{Ref, RefCell},
    ops::{Deref, DerefMut},
};

/// A simplest pose node that extracts pose from a specific animation and prepares it for further use.
/// Animation handle should point to an animation in some animation container see [`AnimationContainer`] docs
/// for more info.
#[derive(Default, Debug, Visit, Clone, Reflect, PartialEq)]
pub struct PlayAnimation<T: EntityId> {
    /// Base node.
    pub base: BasePoseNode<T>,

    /// A handle to animation.
    pub animation: Handle<Animation<T>>,

    /// Output pose, it contains a filtered (see [`crate::machine::LayerMask`] for more info) pose from
    /// the animation specified by the `animation` field.
    #[visit(skip)]
    #[reflect(hidden)]
    pub output_pose: RefCell<AnimationPose<T>>,
}

impl<T: EntityId> Deref for PlayAnimation<T> {
    type Target = BasePoseNode<T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T: EntityId> DerefMut for PlayAnimation<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<T: EntityId> PlayAnimation<T> {
    /// Creates new PlayAnimation node with given animation handle.
    pub fn new(animation: Handle<Animation<T>>) -> Self {
        Self {
            base: Default::default(),
            animation,
            output_pose: Default::default(),
        }
    }
}

impl<T: EntityId> AnimationPoseSource<T> for PlayAnimation<T> {
    fn eval_pose(
        &self,
        _nodes: &Pool<PoseNode<T>>,
        _params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        _dt: f32,
    ) -> Ref<AnimationPose<T>> {
        if let Some(animation) = animations.try_get(self.animation) {
            let mut output_pose = self.output_pose.borrow_mut();
            animation.pose().clone_into(&mut output_pose);
            // Pass the root motion (if any) so it will be blended correctly.
            output_pose.set_root_motion(animation.root_motion().cloned());
        }
        self.output_pose.borrow()
    }

    fn pose(&self) -> Ref<AnimationPose<T>> {
        self.output_pose.borrow()
    }

    fn collect_animation_events(
        &self,
        _nodes: &Pool<PoseNode<T>>,
        _params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        _strategy: AnimationEventCollectionStrategy,
    ) -> Vec<(Handle<Animation<T>>, AnimationEvent)> {
        animations
            .try_get(self.animation)
            .map(|a| {
                a.events_ref()
                    .iter()
                    .map(|e| (self.animation, e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
