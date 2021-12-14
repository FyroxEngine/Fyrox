#![allow(missing_docs, dead_code)]

use crate::physics3d::rapier::prelude::JointHandle;
use crate::scene::base::BaseBuilder;
use crate::scene::graph::Graph;
use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::desc::JointParamsDesc,
    scene::{base::Base, node::Node},
};
use bitflags::bitflags;
use std::ops::{Deref, DerefMut};

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
