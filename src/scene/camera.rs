use rg3d_core::{
    visitor::{Visitor, VisitResult, Visit},
    math::{
        Rect,
        mat4::Mat4,
        vec2::Vec2,
    },
};
use crate::scene::base::{Base, AsBase};

#[derive(Clone)]
pub struct Camera {
    base: Base,
    fov: f32,
    z_near: f32,
    z_far: f32,
    viewport: Rect<f32>,
    view_matrix: Mat4,
    projection_matrix: Mat4,
}

impl AsBase for Camera {
    fn base(&self) -> &Base {
        &self.base
    }

    fn base_mut(&mut self) -> &mut Base {
        &mut self.base
    }
}

impl Default for Camera {
    fn default() -> Camera {
        let fov = 75.0f32;
        let z_near = 0.025;
        let z_far = 2048.0;

        Camera {
            fov,
            z_far,
            z_near,
            projection_matrix: Mat4::perspective(
                fov.to_radians(),
                1.0,
                z_near,
                z_far,
            ),
            base: Base::default(),
            view_matrix: Mat4::IDENTITY,
            viewport: Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 },
        }
    }
}

impl Visit for Camera {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        self.fov.visit("Fov", visitor)?;
        self.z_near.visit("ZNear", visitor)?;
        self.z_far.visit("ZFar", visitor)?;
        self.viewport.visit("Viewport", visitor)?;
        self.base.visit("Base", visitor)?;
        visitor.leave_region()
    }
}

impl Camera {
    #[inline]
    pub fn calculate_matrices(&mut self, aspect: f32) {
        let pos = self.base.get_global_position();
        let look = self.base.get_look_vector();
        let up = self.base.get_up_vector();

        if let Some(view_matrix) = Mat4::look_at(pos, pos + look, up) {
            self.view_matrix = view_matrix;
        } else {
            self.view_matrix = Mat4::IDENTITY;
        }
        self.projection_matrix = Mat4::perspective(self.fov.to_radians(), aspect, self.z_near, self.z_far);
    }

    #[inline]
    pub fn get_viewport_pixels(&self, client_size: Vec2) -> Rect<i32> {
        Rect {
            x: (self.viewport.x * client_size.x) as i32,
            y: (self.viewport.y * client_size.y) as i32,
            w: (self.viewport.w * client_size.x) as i32,
            h: (self.viewport.h * client_size.y) as i32,
        }
    }

    #[inline]
    pub fn get_view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix * self.view_matrix
    }

    #[inline]
    pub fn get_projection_matrix(&self) -> Mat4 {
        self.projection_matrix
    }

    #[inline]
    pub fn get_view_matrix(&self) -> Mat4 {
        self.view_matrix
    }

    #[inline]
    pub fn get_inv_view_matrix(&self) -> Result<Mat4, ()> {
        self.view_matrix.inverse()
    }

    #[inline]
    pub fn get_z_far(&self) -> f32 {
        self.z_far
    }

    #[inline]
    pub fn get_z_near(&self) -> f32 {
        self.z_near
    }
}