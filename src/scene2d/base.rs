use crate::{
    core::{
        algebra::{Matrix4, Vector2},
        pool::Handle,
        visitor::prelude::*,
    },
    scene2d::{graph::Graph, node::Node, transform::Transform},
};
use std::cell::Cell;

#[derive(Default, Visit)]
pub struct Base {
    transform: Transform,
    pub(in crate) global_transform: Cell<Matrix4<f32>>,
    pub(in crate) visibility: bool,
    pub(in crate) global_visibility: Cell<bool>,
    pub(in crate) parent: Handle<Node>,
    pub(in crate) children: Vec<Handle<Node>>,
    name: String,
}

impl Base {
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = name.as_ref().to_owned();
    }

    pub fn parent(&self) -> Handle<Node> {
        self.parent
    }

    pub fn global_visibility(&self) -> bool {
        self.global_visibility.get()
    }

    pub fn visibility(&self) -> bool {
        self.visibility
    }

    pub fn global_position(&self) -> Vector2<f32> {
        let m = self.global_transform.get();
        Vector2::new(m[12], m[13])
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

    pub fn global_transform(&self) -> Matrix4<f32> {
        self.global_transform.get()
    }
}

pub struct BaseBuilder {
    transform: Transform,
    children: Vec<Handle<Node>>,
    name: String,
    visibility: bool,
}

impl BaseBuilder {
    pub fn new() -> Self {
        Self {
            transform: Default::default(),
            children: Default::default(),
            name: "Base".to_string(),
            visibility: true,
        }
    }

    pub fn with_local_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Sets desired list of children nodes.
    pub fn with_children<'a, I: IntoIterator<Item = &'a Handle<Node>>>(
        mut self,
        children: I,
    ) -> Self {
        for &child in children.into_iter() {
            if child.is_some() {
                self.children.push(child)
            }
        }
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn build_base(self) -> Base {
        Base {
            transform: self.transform,
            global_transform: Cell::new(Matrix4::identity()),
            visibility: self.visibility,
            global_visibility: Cell::new(false),
            parent: Default::default(),
            children: self.children,
            name: self.name,
        }
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(Node::Base(self.build_base()))
    }
}
