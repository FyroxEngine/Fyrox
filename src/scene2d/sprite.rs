use crate::{
    core::{
        algebra::{Point3, Vector2},
        color::Color,
        math::Rect,
        pool::Handle,
        visitor::prelude::*,
    },
    resource::texture::Texture,
    scene2d::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Visit)]
pub struct Sprite {
    base: Base,
    texture: Option<Texture>,
    color: Color,
    size: f32,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            base: Default::default(),
            texture: None,
            color: Default::default(),
            size: 16.0,
        }
    }
}

impl Deref for Sprite {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Sprite {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Sprite {
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

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn set_size(&mut self, size: f32) {
        self.size = size;
    }

    pub fn local_bounds(&self) -> Rect<f32> {
        Rect {
            position: self.local_transform().position(),
            size: Vector2::new(self.size, self.size),
        }
    }

    pub fn global_bounds(&self) -> Rect<f32> {
        let mut bounds = Rect::default();
        let local_top_left = self.local_transform().position();
        let local_bottom_right = local_top_left + Vector2::new(self.size, self.size);
        let global_top_left = self
            .global_transform()
            .transform_point(&Point3::new(local_top_left.x, local_top_left.y, 0.0))
            .coords;
        let global_bottom_right = self
            .global_transform()
            .transform_point(&Point3::new(
                local_bottom_right.x,
                local_bottom_right.y,
                0.0,
            ))
            .coords;
        bounds.push(global_top_left.xy());
        bounds.push(global_bottom_right.xy());
        bounds
    }
}

pub struct SpriteBuilder {
    base_builder: BaseBuilder,
    texture: Option<Texture>,
    color: Color,
    size: f32,
}

impl SpriteBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            texture: None,
            color: Color::WHITE,
            size: 16.0,
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

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(Node::Sprite(Sprite {
            base: self.base_builder.build_base(),
            texture: self.texture,
            color: self.color,
            size: self.size,
        }))
    }
}
