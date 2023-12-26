//! Various animation blending nodes.

use crate::{
    core::{
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::{Visit, VisitResult, Visitor},
    },
    machine::{
        node::AnimationEventCollectionStrategy, node::BasePoseNode, AnimationPoseSource, Parameter,
        ParameterContainer, PoseNode, PoseWeight,
    },
    Animation, AnimationContainer, AnimationEvent, AnimationPose, EntityId,
};
use fyrox_core::uuid::{uuid, Uuid};
use fyrox_core::TypeUuidProvider;
use std::cmp::Ordering;
use std::{
    cell::{Cell, Ref, RefCell},
    ops::{Deref, DerefMut},
};

/// Weighted proxy for animation pose. It has an input pose source and a weight, that tells in which proportion
/// the pose should be blended into final pose.
#[derive(Default, Debug, Visit, Clone, Reflect, PartialEq)]
pub struct BlendPose<T: EntityId> {
    /// Weight of the pose.
    pub weight: PoseWeight,

    /// A source of animation pose.
    #[reflect(hidden)]
    pub pose_source: Handle<PoseNode<T>>,
}

impl<T: EntityId> TypeUuidProvider for BlendPose<T> {
    fn type_uuid() -> Uuid {
        uuid!("b01d7639-7b39-4eaf-87e6-29fd5221951b")
    }
}

impl<T: EntityId> BlendPose<T> {
    /// Creates new instance of blend pose with given weight and animation pose.
    pub fn new(weight: PoseWeight, pose_source: Handle<PoseNode<T>>) -> Self {
        Self {
            weight,
            pose_source,
        }
    }

    /// Specialized constructor that creates blend pose with constant weight.
    /// `weight` should be positive.
    pub fn with_constant_weight(weight: f32, pose_source: Handle<PoseNode<T>>) -> Self {
        Self {
            weight: PoseWeight::Constant(weight),
            pose_source,
        }
    }

    /// Specialized constructor that creates blend pose with parametrized weight.
    /// `param_id` must be name of Weight parameter in machine.
    pub fn with_param_weight(param_id: &str, pose_source: Handle<PoseNode<T>>) -> Self {
        Self {
            weight: PoseWeight::Parameter(param_id.to_owned()),
            pose_source,
        }
    }
}

/// Animation blend node. It takes multiple input poses and mixes them together into single pose with specified
/// weights. Could be used to mix hit and run animations for example - once your character got hit, you set some
/// significant weight for hit animation (0.8 for example) and lower weight for run animation (0.2) and it will
/// look like your character got wounded while it still running (probably you should decrease speed here too).
/// Weights can be parametrized, which means that you can dynamically change them in runtime. In our example we
/// can decrease weight of hit animation over time and increase weight of run animation, so character will recover
/// from his wounds.
#[derive(Default, Debug, Visit, Clone, Reflect, PartialEq)]
pub struct BlendAnimations<T: EntityId> {
    /// Base node.
    pub base: BasePoseNode<T>,

    /// A list of pose sources. See [`BlendPose`] docs for more info.
    pub pose_sources: Vec<BlendPose<T>>,

    /// Output pose of the node, contains final result of blending all input poses.
    #[visit(skip)]
    #[reflect(hidden)]
    pub output_pose: RefCell<AnimationPose<T>>,
}

impl<T: EntityId> Deref for BlendAnimations<T> {
    type Target = BasePoseNode<T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T: EntityId> DerefMut for BlendAnimations<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<T: EntityId> BlendAnimations<T> {
    /// Creates new animation blend node with given poses.
    pub fn new(poses: Vec<BlendPose<T>>) -> Self {
        Self {
            base: Default::default(),
            pose_sources: poses,
            output_pose: Default::default(),
        }
    }

    /// Returns a set of handles to children pose nodes.
    pub fn children(&self) -> Vec<Handle<PoseNode<T>>> {
        self.pose_sources.iter().map(|s| s.pose_source).collect()
    }
}

impl<T: EntityId> AnimationPoseSource<T> for BlendAnimations<T> {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        dt: f32,
    ) -> Ref<AnimationPose<T>> {
        self.output_pose.borrow_mut().reset();
        for blend_pose in self.pose_sources.iter() {
            let weight = match blend_pose.weight {
                PoseWeight::Constant(value) => value,
                PoseWeight::Parameter(ref param_id) => {
                    if let Some(Parameter::Weight(weight)) = params.get(param_id) {
                        *weight
                    } else {
                        0.0
                    }
                }
            };

            if let Some(pose_source) = nodes
                .try_borrow(blend_pose.pose_source)
                .map(|pose_source| pose_source.eval_pose(nodes, params, animations, dt))
            {
                self.output_pose
                    .borrow_mut()
                    .blend_with(&pose_source, weight);
            }
        }
        self.output_pose.borrow()
    }

    fn pose(&self) -> Ref<AnimationPose<T>> {
        self.output_pose.borrow()
    }

    fn collect_animation_events(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        strategy: AnimationEventCollectionStrategy,
    ) -> Vec<(Handle<Animation<T>>, AnimationEvent)> {
        match strategy {
            AnimationEventCollectionStrategy::All => {
                let mut events = Vec::new();
                for pose in self.pose_sources.iter() {
                    if let Some(source) = nodes.try_borrow(pose.pose_source) {
                        events.extend(
                            source.collect_animation_events(nodes, params, animations, strategy),
                        );
                    }
                }
                events
            }
            AnimationEventCollectionStrategy::MaxWeight => {
                if let Some((pose, _)) = self
                    .pose_sources
                    .iter()
                    .filter_map(|s| s.weight.value(params).map(|w| (s, w)))
                    .max_by(|(_, w1), (_, w2)| w1.partial_cmp(w2).unwrap_or(Ordering::Equal))
                {
                    if let Some(pose_source) = nodes.try_borrow(pose.pose_source) {
                        return pose_source
                            .collect_animation_events(nodes, params, animations, strategy);
                    }
                }

                Default::default()
            }
            AnimationEventCollectionStrategy::MinWeight => {
                if let Some((pose, _)) = self
                    .pose_sources
                    .iter()
                    .filter_map(|s| s.weight.value(params).map(|w| (s, w)))
                    .min_by(|(_, w1), (_, w2)| w1.partial_cmp(w2).unwrap_or(Ordering::Equal))
                {
                    if let Some(pose_source) = nodes.try_borrow(pose.pose_source) {
                        return pose_source
                            .collect_animation_events(nodes, params, animations, strategy);
                    }
                }

                Default::default()
            }
        }
    }
}

/// An animation pose with specific blend time. Blend time tells the engine how many time it should use to perform
/// blending to this pose.
#[derive(Default, Debug, Visit, Clone, Reflect, PartialEq)]
pub struct IndexedBlendInput<T: EntityId> {
    /// Blend time tells the engine how many time it should use to perform blending to this pose.
    pub blend_time: f32,

    /// A handle to pose node source.
    #[reflect(hidden)]
    pub pose_source: Handle<PoseNode<T>>,
}

impl<T: EntityId> TypeUuidProvider for IndexedBlendInput<T> {
    fn type_uuid() -> Uuid {
        uuid!("92fcc992-9a68-4152-8449-657546faa286")
    }
}

/// A node that switches between given animations using index and smoothly blends from one animation to another
/// while switching. It is very useful for situations when you need to switch between different animations. For
/// example you could have an `aim` state, it is suitable for any weapon (you don't need to create a ton of states
/// like `aim_rifle`, `aim_pistol`, etc), but actual weapon holding animation should be different based on actual
/// weapon a character is holding. In this case you create a BlendAnimationsByIndex node, add a few inputs where
/// each input uses different weapon holding animation and in your game all you need to do is to set an index
/// parameter in the machine parameters. The node will automatically perform smooth transition between different
/// animations.
#[derive(Default, Debug, Visit, Clone, Reflect, PartialEq)]
pub struct BlendAnimationsByIndex<T: EntityId> {
    /// Base node.
    pub base: BasePoseNode<T>,

    /// A name of index parameter that will be used to switch between input poses.
    pub index_parameter: String,

    /// A set of input poses.
    pub inputs: Vec<IndexedBlendInput<T>>,

    /// Index of a previously active input pose.
    #[reflect(hidden)]
    pub prev_index: Cell<Option<u32>>,

    /// Current blend time.
    #[reflect(hidden)]
    pub blend_time: Cell<f32>,

    /// Output pose of the node.
    #[visit(skip)]
    #[reflect(hidden)]
    pub output_pose: RefCell<AnimationPose<T>>,
}

impl<T: EntityId> Deref for BlendAnimationsByIndex<T> {
    type Target = BasePoseNode<T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T: EntityId> DerefMut for BlendAnimationsByIndex<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<T: EntityId> BlendAnimationsByIndex<T> {
    /// Creates new [`BlendAnimationsByIndex`] node using given index parameter name and a set of inputs.
    pub fn new(index_parameter: String, inputs: Vec<IndexedBlendInput<T>>) -> Self {
        Self {
            base: Default::default(),
            index_parameter,
            inputs,
            output_pose: RefCell::new(Default::default()),
            prev_index: Cell::new(None),
            blend_time: Cell::new(0.0),
        }
    }

    /// Return a set of handle of children nodes.
    pub fn children(&self) -> Vec<Handle<PoseNode<T>>> {
        self.inputs.iter().map(|s| s.pose_source).collect()
    }
}

impl<T: EntityId> AnimationPoseSource<T> for BlendAnimationsByIndex<T> {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        dt: f32,
    ) -> Ref<AnimationPose<T>> {
        self.output_pose.borrow_mut().reset();

        if let Some(&Parameter::Index(current_index)) = params.get(&self.index_parameter) {
            let mut applied = false;

            if let Some(prev_index) = self.prev_index.get() {
                if prev_index != current_index {
                    if let (Some(prev_input), Some(current_input)) = (
                        self.inputs.get(prev_index as usize),
                        self.inputs.get(current_index as usize),
                    ) {
                        self.blend_time
                            .set((self.blend_time.get() + dt).min(current_input.blend_time));

                        let interpolator = self.blend_time.get() / current_input.blend_time;

                        self.output_pose.borrow_mut().blend_with(
                            &nodes[prev_input.pose_source].eval_pose(nodes, params, animations, dt),
                            1.0 - interpolator,
                        );
                        self.output_pose.borrow_mut().blend_with(
                            &nodes[current_input.pose_source]
                                .eval_pose(nodes, params, animations, dt),
                            interpolator,
                        );

                        if interpolator >= 1.0 {
                            self.prev_index.set(Some(current_index));
                            self.blend_time.set(0.0);
                        }

                        applied = true;
                    }
                }
            } else {
                self.prev_index.set(Some(current_index));
            }

            if !applied {
                // Immediately jump to target pose (if any).
                self.blend_time.set(0.0);

                if let Some(current_input) = self.inputs.get(current_index as usize) {
                    nodes[current_input.pose_source]
                        .eval_pose(nodes, params, animations, dt)
                        .clone_into(&mut self.output_pose.borrow_mut());
                }
            }
        }

        self.output_pose.borrow()
    }

    fn pose(&self) -> Ref<AnimationPose<T>> {
        self.output_pose.borrow()
    }

    fn collect_animation_events(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        strategy: AnimationEventCollectionStrategy,
    ) -> Vec<(Handle<Animation<T>>, AnimationEvent)> {
        if let Some(&Parameter::Index(current_index)) = params.get(&self.index_parameter) {
            if let Some(prev_index) = self.prev_index.get() {
                if prev_index != current_index {
                    if let (Some(prev_input), Some(current_input)) = (
                        self.inputs.get(prev_index as usize),
                        self.inputs.get(current_index as usize),
                    ) {
                        let interpolator = self.blend_time.get() / current_input.blend_time;

                        match strategy {
                            AnimationEventCollectionStrategy::All => {
                                let mut events = Vec::new();
                                for input in [prev_input, current_input] {
                                    if let Some(source) = nodes.try_borrow(input.pose_source) {
                                        events.extend(source.collect_animation_events(
                                            nodes, params, animations, strategy,
                                        ));
                                    }
                                }
                                return events;
                            }
                            AnimationEventCollectionStrategy::MaxWeight => {
                                let input = if interpolator < 0.5 {
                                    prev_input
                                } else {
                                    current_input
                                };

                                if let Some(pose_source) = nodes.try_borrow(input.pose_source) {
                                    return pose_source.collect_animation_events(
                                        nodes, params, animations, strategy,
                                    );
                                }
                            }
                            AnimationEventCollectionStrategy::MinWeight => {
                                let input = if interpolator < 0.5 {
                                    current_input
                                } else {
                                    prev_input
                                };

                                if let Some(pose_source) = nodes.try_borrow(input.pose_source) {
                                    return pose_source.collect_animation_events(
                                        nodes, params, animations, strategy,
                                    );
                                }
                            }
                        }
                    }
                } else {
                    // In case where the transition is done, all the strategies does the same - just collects events
                    // from active pose node.
                    if let Some(current_input) = self.inputs.get(current_index as usize) {
                        if let Some(pose_source) = nodes.try_borrow(current_input.pose_source) {
                            return pose_source
                                .collect_animation_events(nodes, params, animations, strategy);
                        }
                    }
                }
            }
        }

        Default::default()
    }
}
