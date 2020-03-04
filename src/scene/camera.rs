use crate::{
    core::{
        visitor::{
            Visitor,
            VisitResult,
            Visit
        },
        math::{
            Rect,
            mat4::Mat4,
            vec2::Vec2,
        },
    },
    scene::base::{
        Base,
        AsBase,
        BaseBuilder,
    },
};

#[derive(Clone)]
pub struct Camera {
    base: Base,
    fov: f32,
    z_near: f32,
    z_far: f32,
    viewport: Rect<f32>,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    enabled: bool,
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
        CameraBuilder::new(BaseBuilder::new()).build()
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
        self.enabled.visit("Enabled", visitor)?;
        visitor.leave_region()
    }
}

impl Camera {
    #[inline]
    pub fn calculate_matrices(&mut self, frame_size: Vec2) {
        let pos = self.base.get_global_position();
        let look = self.base.get_look_vector();
        let up = self.base.get_up_vector();

        if let Some(view_matrix) = Mat4::look_at(pos, pos + look, up) {
            self.view_matrix = view_matrix;
        } else {
            self.view_matrix = Mat4::IDENTITY;
        }
        let viewport = self.viewport_pixels(frame_size);
        self.projection_matrix = Mat4::perspective(self.fov, viewport.w as f32 / viewport.h as f32, self.z_near, self.z_far);
    }

    #[inline]
    pub fn viewport_pixels(&self, frame_size: Vec2) -> Rect<i32> {
        Rect {
            x: (self.viewport.x * frame_size.x) as i32,
            y: (self.viewport.y * frame_size.y) as i32,
            w: (self.viewport.w * frame_size.x) as i32,
            h: (self.viewport.h * frame_size.y) as i32,
        }
    }

    #[inline]
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix * self.view_matrix
    }

    #[inline]
    pub fn projection_matrix(&self) -> Mat4 {
        self.projection_matrix
    }

    #[inline]
    pub fn view_matrix(&self) -> Mat4 {
        self.view_matrix
    }

    #[inline]
    pub fn inv_view_matrix(&self) -> Result<Mat4, ()> {
        self.view_matrix.inverse()
    }

    #[inline]
    pub fn set_z_far(&mut self, z_far: f32) -> &mut Self {
        self.z_far = z_far;
        self
    }

    #[inline]
    pub fn z_far(&self) -> f32 {
        self.z_far
    }

    #[inline]
    pub fn set_z_near(&mut self, z_near: f32) -> &mut Self {
        self.z_near = z_near;
        self
    }

    #[inline]
    pub fn z_near(&self) -> f32 {
        self.z_near
    }

    /// In radians
    #[inline]
    pub fn set_fov(&mut self, fov: f32) -> &mut Self {
        self.fov = fov;
        self
    }

    #[inline]
    pub fn fov(&self) -> f32 {
        self.fov
    }

    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }
}

pub struct CameraBuilder {
    base_builder: BaseBuilder,
    fov: f32,
    z_near: f32,
    z_far: f32,
    viewport: Rect<f32>,
    enabled: bool,
}

impl CameraBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            enabled: true,
            base_builder,
            fov: 75.0f32.to_radians(),
            z_near: 0.025,
            z_far: 2048.0,
            viewport: Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 },
        }
    }

    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    pub fn with_z_near(mut self, z_near: f32) -> Self {
        self.z_near = z_near;
        self
    }

    pub fn with_z_far(mut self, z_far: f32) -> Self {
        self.z_far = z_far;
        self
    }

    pub fn with_viewport(mut self, viewport: Rect<f32>) -> Self {
        self.viewport = viewport;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn build(self) -> Camera {
        Camera {
            enabled: self.enabled,
            base: self.base_builder.build(),
            fov: self.fov,
            z_near: self.z_near,
            z_far: self.z_far,
            viewport: self.viewport,
            // No need to calculate these matrices - they'll be automatically
            // recalculated before rendering.
            view_matrix: Mat4::IDENTITY,
            projection_matrix: Mat4::IDENTITY,
        }
    }
}