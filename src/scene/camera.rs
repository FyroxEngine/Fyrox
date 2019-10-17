use rg3d_core::{
    visitor::{Visitor, VisitResult, Visit},
    math::{
        Rect,
        mat4::Mat4,
        vec2::Vec2
    }
};
use crate::scene::node::{CommonNodeData, NodeTrait};

#[derive(Clone)]
pub struct Camera {
    common: CommonNodeData,
    fov: f32,
    z_near: f32,
    z_far: f32,
    viewport: Rect<f32>,
    view_matrix: Mat4,
    projection_matrix: Mat4,
}

impl Default for Camera {
    fn default() -> Camera {
        let fov: f32 = 75.0;
        let z_near: f32 = 0.025;
        let z_far: f32 = 2048.0;

        Camera {
            common: Default::default(),
            fov,
            z_near,
            z_far,
            view_matrix: Mat4::IDENTITY,
            projection_matrix: Mat4::perspective(
                fov.to_radians(),
                1.0,
                z_near,
                z_far
            ),
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
        self.common.visit("Common", visitor)?;
        visitor.leave_region()
    }
}

impl Camera {
    #[inline]
    pub fn calculate_matrices(&mut self, aspect: f32) {
        let pos = self.get_global_position();
        let look = self.get_look_vector();
        let up = self.get_up_vector();

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

impl_node_trait!(Camera);
impl_node_trait_private!(Camera);