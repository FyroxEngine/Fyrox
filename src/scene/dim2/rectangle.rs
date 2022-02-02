use crate::{
    core::{
        color::Color,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
        variable::TemplateVariable,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Visit, Inspect, Debug, Default)]
pub struct Rectangle {
    base: Base,

    #[inspect(getter = "Deref::deref")]
    texture: TemplateVariable<Option<Texture>>,

    #[inspect(getter = "Deref::deref")]
    color: TemplateVariable<Color>,
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

    pub fn texture_value(&self) -> Option<Texture> {
        (*self.texture).clone()
    }

    pub fn set_texture(&mut self, texture: Option<Texture>) {
        self.texture.set(texture);
    }

    pub fn color(&self) -> Color {
        *self.color
    }

    pub fn set_color(&mut self, color: Color) {
        self.color.set(color);
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            texture: self.texture.clone(),
            color: self.color.clone(),
        }
    }

    pub(crate) fn restore_resources(&mut self, resource_manager: ResourceManager) {
        resource_manager
            .state()
            .containers_mut()
            .textures
            .try_restore_template_resource(&mut self.texture);
    }

    // Prefab inheritance resolving.
    pub(crate) fn inherit(&mut self, parent: &Node) {
        self.base.inherit_properties(parent);
        if let Node::Rectangle(parent) = parent {
            self.texture.try_inherit(&parent.texture);
            self.color.try_inherit(&parent.color);
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
            texture: self.texture.into(),
            color: self.color.into(),
        })
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
