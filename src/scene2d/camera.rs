use crate::{
    core::{
        algebra::Matrix4,
        math::Rect,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene2d::base::Base,
};
use std::ops::{Deref, DerefMut};

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

impl Visit for Camera {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base.visit("Base", visitor)?;
        self.viewport.visit("Viewport", visitor)?;

        visitor.leave_region()
    }
}
