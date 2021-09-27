use crate::{
    core::{
        algebra::{Matrix4, Vector2},
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    scene::base::PhysicsBinding,
    scene2d::{graph::Graph, node::Node, transform::Transform},
};
use std::cell::Cell;

#[derive(Visit, Inspect, Debug)]
pub struct Base {
    #[inspect(expand)]
    transform: Transform,
    #[inspect(skip)]
    pub(in crate) global_transform: Cell<Matrix4<f32>>,
    pub(in crate) visibility: bool,
    #[inspect(skip)]
    pub(in crate) global_visibility: Cell<bool>,
    pub(in crate) parent: Handle<Node>,
    pub(in crate) children: Vec<Handle<Node>>,
    pub(in crate) physics_binding: PhysicsBinding,
    name: String,
}

impl Default for Base {
    fn default() -> Self {
        Self {
            transform: Default::default(),
            global_transform: Cell::new(Matrix4::identity()),
            visibility: true,
            global_visibility: Cell::new(true),
            parent: Default::default(),
            children: Default::default(),
            physics_binding: Default::default(),
            name: Default::default(),
        }
    }
}

impl Base {
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = name.as_ref().to_owned();
    }

    pub fn name(&self) -> &str {
        &self.name
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

    pub fn up(&self) -> Vector2<f32> {
        let m = self.global_transform.get();
        Vector2::new(m[4], m[5])
    }

    pub fn right(&self) -> Vector2<f32> {
        let m = self.global_transform.get();
        Vector2::new(m[0], m[1])
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

    pub fn set_physics_binding(&mut self, binding: PhysicsBinding) {
        self.physics_binding = binding;
    }

    pub fn physics_binding(&self) -> PhysicsBinding {
        self.physics_binding
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            transform: self.transform.clone(),
            global_transform: self.global_transform.clone(),
            visibility: self.visibility,
            global_visibility: self.global_visibility.clone(),
            physics_binding: self.physics_binding,
            name: self.name.clone(),
            // Handles to parent/children are intentionally not copied!
            parent: Default::default(),
            children: Default::default(),
        }
    }
}

pub struct BaseBuilder {
    transform: Transform,
    children: Vec<Handle<Node>>,
    name: String,
    visibility: bool,
    physics_binding: PhysicsBinding,
}

impl Default for BaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseBuilder {
    pub fn new() -> Self {
        Self {
            transform: Default::default(),
            children: Default::default(),
            name: "Base".to_string(),
            visibility: true,
            physics_binding: Default::default(),
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

    pub fn with_name<N: AsRef<str>>(mut self, name: N) -> Self {
        self.name = name.as_ref().to_owned();
        self
    }

    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_physics_binding(mut self, binding: PhysicsBinding) -> Self {
        self.physics_binding = binding;
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
            physics_binding: self.physics_binding,
        }
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(Node::Base(self.build_base()))
    }
}
