use crate::{
    animation::{
        machine::{
            node::{BasePoseNode, BasePoseNodeDefinition, PoseNodeDefinition},
            EvaluatePose, Parameter, ParameterContainer, PoseNode, PoseWeight,
        },
        AnimationContainer, AnimationPose,
    },
    core::{
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::{Visit, VisitResult, Visitor},
    },
};
use std::{
    cell::{Cell, Ref, RefCell},
    ops::{Deref, DerefMut},
};

/// Weighted proxy for animation pose.
#[derive(Default, Debug, Visit, Clone)]
pub struct BlendPose {
    pub weight: PoseWeight,
    pub pose_source: Handle<PoseNode>,
}

#[derive(Default, Debug, Visit, Clone, Reflect)]
pub struct BlendPoseDefinition {
    pub weight: PoseWeight,
    #[reflect(hidden)]
    pub pose_source: Handle<PoseNodeDefinition>,
}

impl BlendPose {
    /// Creates new instance of blend pose with given weight and animation pose.
    pub fn new(weight: PoseWeight, pose_source: Handle<PoseNode>) -> Self {
        Self {
            weight,
            pose_source,
        }
    }

    /// Specialized constructor that creates blend pose with constant weight.
    /// `weight` should be positive.
    pub fn with_constant_weight(weight: f32, pose_source: Handle<PoseNode>) -> Self {
        Self {
            weight: PoseWeight::Constant(weight),
            pose_source,
        }
    }

    /// Specialized constructor that creates blend pose with parametrized weight.
    /// `param_id` must be name of Weight parameter in machine.
    pub fn with_param_weight(param_id: &str, pose_source: Handle<PoseNode>) -> Self {
        Self {
            weight: PoseWeight::Parameter(param_id.to_owned()),
            pose_source,
        }
    }
}

/// Animation blend node. It takes multiple input poses and mixes them together into
/// single pose with specified weights. Could be used to mix hit and run animations
/// for example - once your character got hit, you set some significant weight for
/// hit animation (0.8 for example) and lower weight for run animation (0.2) and it
/// will look like your character got wounded while it still running (probably you
/// should decrease speed here too). Weights can be parametrized, which means that
/// you can dynamically change them in runtime. In our example we can decrease weight
/// of hit animation over time and increase weight of run animation, so character will
/// recover from his wounds.
#[derive(Default, Debug, Visit, Clone)]
pub struct BlendAnimations {
    pub base: BasePoseNode,
    pub pose_sources: Vec<BlendPose>,
    #[visit(skip)]
    pub(crate) output_pose: RefCell<AnimationPose>,
}

impl Deref for BlendAnimations {
    type Target = BasePoseNode;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BlendAnimations {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[derive(Default, Debug, Visit, Clone, Reflect)]
pub struct BlendAnimationsDefinition {
    pub base: BasePoseNodeDefinition,
    pub pose_sources: Vec<BlendPoseDefinition>,
}

impl BlendAnimationsDefinition {
    pub fn children(&self) -> Vec<Handle<PoseNodeDefinition>> {
        self.pose_sources.iter().map(|s| s.pose_source).collect()
    }
}

impl Deref for BlendAnimationsDefinition {
    type Target = BasePoseNodeDefinition;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BlendAnimationsDefinition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl BlendAnimations {
    /// Creates new animation blend node with given poses.
    pub fn new(poses: Vec<BlendPose>) -> Self {
        Self {
            base: Default::default(),
            pose_sources: poses,
            output_pose: Default::default(),
        }
    }
}

impl EvaluatePose for BlendAnimations {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        animations: &AnimationContainer,
        dt: f32,
    ) -> Ref<AnimationPose> {
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

    fn pose(&self) -> Ref<AnimationPose> {
        self.output_pose.borrow()
    }
}

#[derive(Default, Debug, Visit, Clone)]
pub struct IndexedBlendInput {
    pub blend_time: f32,
    pub pose_source: Handle<PoseNode>,
}

#[derive(Default, Debug, Visit, Clone, Reflect)]
pub struct IndexedBlendInputDefinition {
    pub blend_time: f32,
    #[reflect(hidden)]
    pub pose_source: Handle<PoseNodeDefinition>,
}

#[derive(Default, Debug, Visit, Clone)]
pub struct BlendAnimationsByIndex {
    pub base: BasePoseNode,
    pub(crate) index_parameter: String,
    pub inputs: Vec<IndexedBlendInput>,
    pub(crate) prev_index: Cell<Option<u32>>,
    pub(crate) blend_time: Cell<f32>,
    #[visit(skip)]
    pub(crate) output_pose: RefCell<AnimationPose>,
}

impl Deref for BlendAnimationsByIndex {
    type Target = BasePoseNode;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BlendAnimationsByIndex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[derive(Default, Debug, Visit, Clone, Reflect)]
pub struct BlendAnimationsByIndexDefinition {
    pub base: BasePoseNodeDefinition,
    pub index_parameter: String,
    pub inputs: Vec<IndexedBlendInputDefinition>,
}

impl BlendAnimationsByIndexDefinition {
    pub fn children(&self) -> Vec<Handle<PoseNodeDefinition>> {
        self.inputs.iter().map(|s| s.pose_source).collect()
    }
}

impl Deref for BlendAnimationsByIndexDefinition {
    type Target = BasePoseNodeDefinition;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BlendAnimationsByIndexDefinition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl BlendAnimationsByIndex {
    pub fn new(index_parameter: String, inputs: Vec<IndexedBlendInput>) -> Self {
        Self {
            base: Default::default(),
            index_parameter,
            inputs,
            output_pose: RefCell::new(Default::default()),
            prev_index: Cell::new(None),
            blend_time: Cell::new(0.0),
        }
    }
}

impl EvaluatePose for BlendAnimationsByIndex {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        animations: &AnimationContainer,
        dt: f32,
    ) -> Ref<AnimationPose> {
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

    fn pose(&self) -> Ref<AnimationPose> {
        self.output_pose.borrow()
    }
}
