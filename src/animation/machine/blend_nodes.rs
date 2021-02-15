use crate::{
    animation::{
        machine::{EvaluatePose, Parameter, ParameterContainer, PoseNode, PoseWeight},
        AnimationContainer, AnimationPose,
    },
    core::{
        pool::{Handle, Pool},
        visitor::{Visit, VisitResult, Visitor},
    },
};
use std::cell::{Cell, Ref, RefCell};

/// Weighted proxy for animation pose.
#[derive(Default)]
pub struct BlendPose {
    weight: PoseWeight,
    pose_source: Handle<PoseNode>,
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

impl Visit for BlendPose {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.weight.visit("Weight", visitor)?;
        self.pose_source.visit("PoseSource", visitor)?;

        visitor.leave_region()
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
#[derive(Default)]
pub struct BlendAnimations {
    pose_sources: Vec<BlendPose>,
    output_pose: RefCell<AnimationPose>,
}

impl BlendAnimations {
    /// Creates new animation blend node with given poses.
    pub fn new(poses: Vec<BlendPose>) -> Self {
        Self {
            pose_sources: poses,
            output_pose: Default::default(),
        }
    }
}

impl Visit for BlendAnimations {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pose_sources.visit("PoseSources", visitor)?;

        visitor.leave_region()
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

            let pose_source =
                nodes[blend_pose.pose_source].eval_pose(nodes, params, animations, dt);
            self.output_pose
                .borrow_mut()
                .blend_with(&pose_source, weight);
        }
        self.output_pose.borrow()
    }
}

#[derive(Default)]
pub struct IndexedBlendInput {
    pub blend_time: f32,
    pub pose_source: Handle<PoseNode>,
}

impl Visit for IndexedBlendInput {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.blend_time.visit("BlendTime", visitor)?;
        self.pose_source.visit("PoseSource", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default)]
pub struct BlendAnimationsByIndex {
    index_parameter: String,
    inputs: Vec<IndexedBlendInput>,
    output_pose: RefCell<AnimationPose>,
    prev_index: Cell<Option<u32>>,
    blend_time: Cell<f32>,
}

impl BlendAnimationsByIndex {
    pub fn new(index_parameter: String, inputs: Vec<IndexedBlendInput>) -> Self {
        Self {
            index_parameter,
            inputs,
            output_pose: RefCell::new(Default::default()),
            prev_index: Cell::new(None),
            blend_time: Cell::new(0.0),
        }
    }
}

impl Visit for BlendAnimationsByIndex {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.index_parameter.visit("IndexParameter", visitor)?;
        self.inputs.visit("Inputs", visitor)?;
        self.prev_index.visit("PrevIndex", visitor)?;
        self.blend_time.visit("BlendTime", visitor)?;

        visitor.leave_region()
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
                    let prev_input = &self.inputs[prev_index as usize];
                    let current_input = &self.inputs[current_index as usize];

                    self.blend_time
                        .set((self.blend_time.get() + dt).min(current_input.blend_time));

                    let interpolator = self.blend_time.get() / current_input.blend_time;

                    self.output_pose.borrow_mut().blend_with(
                        &nodes[prev_input.pose_source].eval_pose(nodes, params, animations, dt),
                        1.0 - interpolator,
                    );
                    self.output_pose.borrow_mut().blend_with(
                        &nodes[current_input.pose_source].eval_pose(nodes, params, animations, dt),
                        interpolator,
                    );

                    if interpolator >= 1.0 {
                        self.prev_index.set(Some(current_index));
                        self.blend_time.set(0.0);
                    }

                    applied = true;
                }
            } else {
                self.prev_index.set(Some(current_index));
            }

            if !applied {
                // Immediately jump to target pose.
                self.blend_time.set(0.0);

                nodes[self.inputs[current_index as usize].pose_source]
                    .eval_pose(nodes, params, animations, dt)
                    .clone_into(&mut *self.output_pose.borrow_mut());
            }
        }

        self.output_pose.borrow()
    }
}
