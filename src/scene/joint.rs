#![allow(missing_docs, dead_code)]

use crate::{
    core::{
        algebra::{Isometry3, Point3, Translation3, Unit, UnitQuaternion, Vector3},
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::rapier::{
        dynamics::{BallJoint, FixedJoint, JointParams, PrismaticJoint, RevoluteJoint},
        prelude::JointHandle,
    },
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use bitflags::bitflags;
use std::ops::{Deref, DerefMut};

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct BallJointDesc {
    pub local_anchor1: Vector3<f32>,
    pub local_anchor2: Vector3<f32>,
}

#[derive(Clone, Debug, Default, Visit, Inspect)]
pub struct FixedJointDesc {
    pub local_anchor1_translation: Vector3<f32>,
    pub local_anchor1_rotation: UnitQuaternion<f32>,
    pub local_anchor2_translation: Vector3<f32>,
    pub local_anchor2_rotation: UnitQuaternion<f32>,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct PrismaticJointDesc {
    pub local_anchor1: Vector3<f32>,
    pub local_axis1: Vector3<f32>,
    pub local_anchor2: Vector3<f32>,
    pub local_axis2: Vector3<f32>,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct RevoluteJointDesc {
    pub local_anchor1: Vector3<f32>,
    pub local_axis1: Vector3<f32>,
    pub local_anchor2: Vector3<f32>,
    pub local_axis2: Vector3<f32>,
}

#[derive(Clone, Debug, Visit)]
pub enum JointParamsDesc {
    BallJoint(BallJointDesc),
    FixedJoint(FixedJointDesc),
    PrismaticJoint(PrismaticJointDesc),
    RevoluteJoint(RevoluteJointDesc),
}

impl Inspect for JointParamsDesc {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            JointParamsDesc::BallJoint(v) => v.properties(),
            JointParamsDesc::FixedJoint(v) => v.properties(),
            JointParamsDesc::PrismaticJoint(v) => v.properties(),
            JointParamsDesc::RevoluteJoint(v) => v.properties(),
        }
    }
}

impl Default for JointParamsDesc {
    fn default() -> Self {
        Self::BallJoint(Default::default())
    }
}

impl From<JointParamsDesc> for JointParams {
    fn from(params: JointParamsDesc) -> Self {
        match params {
            JointParamsDesc::BallJoint(v) => JointParams::from(BallJoint::new(
                Point3::from(v.local_anchor1),
                Point3::from(v.local_anchor2),
            )),
            JointParamsDesc::FixedJoint(v) => JointParams::from(FixedJoint::new(
                Isometry3 {
                    translation: Translation3 {
                        vector: v.local_anchor1_translation,
                    },
                    rotation: v.local_anchor1_rotation,
                },
                Isometry3 {
                    translation: Translation3 {
                        vector: v.local_anchor2_translation,
                    },
                    rotation: v.local_anchor2_rotation,
                },
            )),
            JointParamsDesc::PrismaticJoint(v) => JointParams::from(PrismaticJoint::new(
                Point3::from(v.local_anchor1),
                Unit::<Vector3<f32>>::new_normalize(v.local_axis1),
                Default::default(), // TODO
                Point3::from(v.local_anchor2),
                Unit::<Vector3<f32>>::new_normalize(v.local_axis2),
                Default::default(), // TODO
            )),
            JointParamsDesc::RevoluteJoint(v) => JointParams::from(RevoluteJoint::new(
                Point3::from(v.local_anchor1),
                Unit::<Vector3<f32>>::new_normalize(v.local_axis1),
                Point3::from(v.local_anchor2),
                Unit::<Vector3<f32>>::new_normalize(v.local_axis2),
            )),
        }
    }
}

impl JointParamsDesc {
    pub(crate) fn from_params(params: &JointParams) -> Self {
        match params {
            JointParams::BallJoint(v) => Self::BallJoint(BallJointDesc {
                local_anchor1: v.local_anchor1.coords,
                local_anchor2: v.local_anchor2.coords,
            }),
            JointParams::FixedJoint(v) => Self::FixedJoint(FixedJointDesc {
                local_anchor1_translation: v.local_frame1.translation.vector,
                local_anchor1_rotation: v.local_frame1.rotation,
                local_anchor2_translation: v.local_frame2.translation.vector,
                local_anchor2_rotation: v.local_frame2.rotation,
            }),
            JointParams::PrismaticJoint(v) => Self::PrismaticJoint(PrismaticJointDesc {
                local_anchor1: v.local_anchor1.coords,
                local_axis1: v.local_axis1().into_inner(),
                local_anchor2: v.local_anchor2.coords,
                local_axis2: v.local_axis2().into_inner(),
            }),
            JointParams::RevoluteJoint(v) => Self::RevoluteJoint(RevoluteJointDesc {
                local_anchor1: v.local_anchor1.coords,
                local_axis1: v.local_axis1.into_inner(),
                local_anchor2: v.local_anchor2.coords,
                local_axis2: v.local_axis2.into_inner(),
            }),
        }
    }
}

bitflags! {
    pub(crate) struct JointChanges: u32 {
        const NONE = 0;
        const BODY1 = 0b0001;
        const BODY2 = 0b0010;
        const PARAMS = 0b0100;
    }
}

pub struct JointParamsRefMut<'a> {
    parent: &'a mut Joint,
}

impl<'a> Drop for JointParamsRefMut<'a> {
    fn drop(&mut self) {
        self.parent.changes.insert(JointChanges::PARAMS);
    }
}

impl<'a> Deref for JointParamsRefMut<'a> {
    type Target = JointParamsDesc;

    fn deref(&self) -> &Self::Target {
        &self.parent.params
    }
}

impl<'a> DerefMut for JointParamsRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.parent.params
    }
}

#[derive(Visit, Inspect, Debug)]
pub struct Joint {
    base: Base,
    params: JointParamsDesc,
    body1: Handle<Node>,
    body2: Handle<Node>,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) native: JointHandle,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) changes: JointChanges,
}

impl Default for Joint {
    fn default() -> Self {
        Self {
            base: Default::default(),
            params: Default::default(),
            body1: Default::default(),
            body2: Default::default(),
            native: JointHandle::invalid(),
            changes: JointChanges::NONE,
        }
    }
}

impl Deref for Joint {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Joint {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Joint {
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            params: self.params.clone(),
            body1: self.body1,
            body2: self.body2,
            native: JointHandle::invalid(),
            changes: JointChanges::NONE,
        }
    }

    pub fn params(&self) -> &JointParamsDesc {
        &self.params
    }

    pub fn params_mut(&mut self) -> JointParamsRefMut {
        JointParamsRefMut { parent: self }
    }

    pub fn set_body1(&mut self, handle: Handle<Node>) {
        self.body1 = handle;
        self.changes.insert(JointChanges::BODY1);
    }

    pub fn body1(&self) -> Handle<Node> {
        self.body1
    }

    pub fn set_body2(&mut self, handle: Handle<Node>) {
        self.body2 = handle;
        self.changes.insert(JointChanges::BODY2);
    }

    pub fn body2(&self) -> Handle<Node> {
        self.body2
    }
}

pub struct JointBuilder {
    base_builder: BaseBuilder,
    params: JointParamsDesc,
    body1: Handle<Node>,
    body2: Handle<Node>,
}

impl JointBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            params: Default::default(),
            body1: Default::default(),
            body2: Default::default(),
        }
    }

    pub fn with_params(mut self, params: JointParamsDesc) -> Self {
        self.params = params;
        self
    }

    pub fn with_body1(mut self, body1: Handle<Node>) -> Self {
        self.body1 = body1;
        self
    }

    pub fn with_body2(mut self, body2: Handle<Node>) -> Self {
        self.body2 = body2;
        self
    }
    pub fn build_node(self) -> Node {
        Node::Joint(Joint {
            base: self.base_builder.build_base(),
            params: self.params,
            body1: self.body1,
            body2: self.body2,
            native: JointHandle::invalid(),
            changes: JointChanges::NONE,
        })
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
