use crate::{
    core::{algebra::Matrix3, pool::Handle, visitor::prelude::*},
    scene2d::{node::Node, transform::Transform},
};

#[derive(Default, Visit)]
pub struct Base {
    transform: Transform,
    global_transform: Matrix3<f32>,
    pub(in crate) parent: Handle<Node>,
    pub(in crate) children: Vec<Handle<Node>>,
    name: String,
}

impl Base {
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = name.as_ref().to_owned();
    }

    pub fn children(&self) -> &[Handle<Node>] {
        &self.children
    }

    pub fn local_transform(&self) -> &Transform {
        &self.transform
    }

    pub fn local_transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    pub fn global_transform(&self) -> &Matrix3<f32> {
        &self.global_transform
    }
}
