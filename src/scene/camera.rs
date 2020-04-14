//! Contains all methods and structures to create and manage cameras.
//!
//! Camera allows you to see world from specific point in world. Currently only
//! perspective projection is supported.
//!
//! # Multiple cameras
//!
//! rg3d supports multiple cameras per scene, it means that you can create split
//! screen games, make picture-in-picture insertions in your main camera view and
//! any other combinations you need.
//!
//! ## Performance
//!
//! Each camera forces engine to re-render same scene one more time, which may cause
//! almost double load of your GPU.

#![warn(missing_docs)]

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

/// See module docs.
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
    /// Explicitly calculates view and projection matrices. Normally, you should not call
    /// this method, it will be called automatically when new frame starts.
    #[inline]
    pub fn calculate_matrices(&mut self, frame_size: Vec2) {
        let pos = self.base.global_position();
        let look = self.base.look_vector();
        let up = self.base.up_vector();

        if let Some(view_matrix) = Mat4::look_at(pos, pos + look, up) {
            self.view_matrix = view_matrix;
        } else {
            self.view_matrix = Mat4::IDENTITY;
        }
        let viewport = self.viewport_pixels(frame_size);
        let aspect = viewport.w as f32 / viewport.h as f32;
        self.projection_matrix = Mat4::perspective(self.fov, aspect, self.z_near, self.z_far);
    }

    /// Sets new viewport in resolution-independent format. In other words
    /// each parameter of viewport defines portion of your current resolution
    /// in percents. In example viewport (0.0, 0.0, 0.5, 1.0) will force camera
    /// to use left half of your screen and (0.5, 0.0, 0.5, 1.0) - right half.
    /// Why not just use pixels directly? Because you can change resolution while
    /// your application is running and you'd be force to manually recalculate
    /// pixel values everytime when resolution changes.
    pub fn set_viewport(&mut self, viewport: Rect<f32>) -> &mut Self {
        self.viewport = viewport;
        self
    }

    /// Calculates viewport rectangle in pixels based on internal resolution-independent
    /// viewport. It is useful when you need to get real viewport rectangle in pixels.
    #[inline]
    pub fn viewport_pixels(&self, frame_size: Vec2) -> Rect<i32> {
        Rect {
            x: (self.viewport.x * frame_size.x) as i32,
            y: (self.viewport.y * frame_size.y) as i32,
            w: (self.viewport.w * frame_size.x) as i32,
            h: (self.viewport.h * frame_size.y) as i32,
        }
    }

    /// Returns current view-projection matrix.
    #[inline]
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix * self.view_matrix
    }

    /// Returns current projection matrix.
    #[inline]
    pub fn projection_matrix(&self) -> Mat4 {
        self.projection_matrix
    }

    /// Returns current view matrix.
    #[inline]
    pub fn view_matrix(&self) -> Mat4 {
        self.view_matrix
    }

    /// Returns inverse view matrix.
    #[inline]
    pub fn inv_view_matrix(&self) -> Result<Mat4, ()> {
        self.view_matrix.inverse()
    }

    /// Sets far projection plane.
    #[inline]
    pub fn set_z_far(&mut self, z_far: f32) -> &mut Self {
        self.z_far = z_far;
        self
    }

    /// Returns far projection plane.
    #[inline]
    pub fn z_far(&self) -> f32 {
        self.z_far
    }

    /// Sets near projection plane. Typical values: 0.01 - 0.04.
    #[inline]
    pub fn set_z_near(&mut self, z_near: f32) -> &mut Self {
        self.z_near = z_near;
        self
    }

    /// Returns near projection plane.
    #[inline]
    pub fn z_near(&self) -> f32 {
        self.z_near
    }

    /// Sets camera field of view in radians.
    #[inline]
    pub fn set_fov(&mut self, fov: f32) -> &mut Self {
        self.fov = fov;
        self
    }

    /// Returns camera field of view in radians.
    #[inline]
    pub fn fov(&self) -> f32 {
        self.fov
    }

    /// Returns state of camera: enabled or not.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enables or disables camera. Disabled cameras will be ignored during
    /// rendering. This allows you to exclude views from specific cameras from
    /// final picture.
    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }
}

/// Camera builder is used to create new camera in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct CameraBuilder {
    base_builder: BaseBuilder,
    fov: f32,
    z_near: f32,
    z_far: f32,
    viewport: Rect<f32>,
    enabled: bool,
}

impl CameraBuilder {
    /// Creates new camera builder using given base node builder.
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

    /// Sets desired field of view in radians.
    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    /// Sets desired near projection plane.
    pub fn with_z_near(mut self, z_near: f32) -> Self {
        self.z_near = z_near;
        self
    }

    /// Sets desired far projection plane.
    pub fn with_z_far(mut self, z_far: f32) -> Self {
        self.z_far = z_far;
        self
    }

    /// Sets desired viewport.
    pub fn with_viewport(mut self, viewport: Rect<f32>) -> Self {
        self.viewport = viewport;
        self
    }

    /// Sets desired initial state of camera: enabled or disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Creates new instance of camera node. Do not forget to add node to scene,
    /// otherwise it is useless.
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