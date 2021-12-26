use crate::{
    core::{
        color::Color,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Visit, Inspect, Debug)]
pub struct Rectangle {
    base: Base,
    texture: Option<Texture>,
    color: Color,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            base: Default::default(),
            texture: None,
            color: Default::default(),
        }
    }
}

impl Deref for Rectangle {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Rectangle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Rectangle {
    pub fn texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    pub fn set_texture(&mut self, texture: Option<Texture>) {
        self.texture = texture;
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            texture: self.texture.clone(),
            color: self.color,
        }
    }
}

pub struct RectangleBuilder {
    base_builder: BaseBuilder,
    texture: Option<Texture>,
    color: Color,
}

impl RectangleBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            texture: None,
            color: Color::WHITE,
        }
    }

    pub fn with_texture(mut self, texture: Texture) -> Self {
        self.texture = Some(texture);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn build_node(self) -> Node {
        Node::Rectangle(Rectangle {
            base: self.base_builder.build_base(),
            texture: self.texture,
            color: self.color,
        })
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
