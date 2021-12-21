use crate::core::{
    algebra::{Matrix4, Vector2},
    inspect::{Inspect, PropertyInfo},
    math::Rect,
    pool::Handle,
    visitor::prelude::*,
};
use crate::scene::base::{Base, BaseBuilder};
use crate::scene::graph::Graph;
use crate::scene::node::Node;
use std::ops::{Deref, DerefMut};

#[derive(Visit, Inspect, Debug)]
pub struct Camera {
    base: Base,
    viewport: Rect<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    enabled: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            base: Default::default(),
            viewport: Rect::new(0.0, 0.0, 1.0, 1.0),
            view_matrix: Matrix4::identity(),
            projection_matrix: Matrix4::identity(),
            enabled: true,
        }
    }
}

impl Deref for Camera {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Camera {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Camera {
    /// Calculates viewport rectangle in pixels based on internal resolution-independent
    /// viewport. It is useful when you need to get real viewport rectangle in pixels.
    #[inline]
    pub fn viewport_pixels(&self, frame_size: Vector2<f32>) -> Rect<i32> {
        Rect::new(
            (self.viewport.x() * frame_size.x) as i32,
            (self.viewport.y() * frame_size.y) as i32,
            (self.viewport.w() * frame_size.x) as i32,
            (self.viewport.h() * frame_size.y) as i32,
        )
    }

    pub fn set_viewport(&mut self, viewport: Rect<f32>) {
        self.viewport = viewport;
    }

    pub fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix * self.view_matrix
    }

    pub fn update(&mut self, render_target_size: Vector2<f32>) {
        self.projection_matrix = Matrix4::new_orthographic(
            0.0,
            render_target_size.x,
            render_target_size.y,
            0.0,
            0.0,
            1.0,
        );

        self.view_matrix = self
            .global_transform()
            .try_inverse()
            .unwrap_or_else(Matrix4::identity);
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            viewport: self.viewport,
            view_matrix: self.view_matrix,
            projection_matrix: self.projection_matrix,
            enabled: self.enabled,
        }
    }
}

pub struct CameraBuilder {
    base_builder: BaseBuilder,
    viewport: Rect<f32>,
    enabled: bool,
}

impl CameraBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            viewport: Rect::new(0.0, 0.0, 1.0, 1.0),
            enabled: true,
        }
    }

    pub fn with_viewport(mut self, viewport: Rect<f32>) -> Self {
        self.viewport = viewport;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(Node::Camera2D(Camera {
            base: self.base_builder.build_base(),
            viewport: self.viewport,
            view_matrix: Matrix4::identity(),
            projection_matrix: Default::default(),
            enabled: self.enabled,
        }))
    }
}
