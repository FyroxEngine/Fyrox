use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::desc::JointParamsDesc,
    scene::{base::Base, node::Node},
};
use std::ops::{Deref, DerefMut};

#[derive(Visit, Inspect, Debug, Default)]
pub struct Joint {
    base: Base,
    params: JointParamsDesc,
    body1: Handle<Node>,
    body2: Handle<Node>,
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
        }
    }
}
