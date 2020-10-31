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

use crate::{
    core::{
        math::{mat4::Mat4, ray::Ray, vec2::Vec2, vec3::Vec3, vec4::Vec4, Rect},
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        node::Node,
        VisibilityCache,
    },
};
use std::ops::{Deref, DerefMut};

/// See module docs.
#[derive(Debug)]
pub struct Camera {
    base: Base,
    fov: f32,
    z_near: f32,
    z_far: f32,
    viewport: Rect<f32>,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    enabled: bool,
    skybox: Option<SkyBox>,
    /// Visibility cache allows you to quickly check if object is visible from the camera or not.
    pub visibility_cache: VisibilityCache,
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
        let _ = self.skybox.visit("SkyBox", visitor);
        // self.visibility_cache intentionally not serialized. It is valid only for one frame.
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

    /// Sets new skybox. Could be None if no skybox needed.
    pub fn set_skybox(&mut self, skybox: Option<SkyBox>) -> &mut Self {
        self.skybox = skybox;
        self
    }

    /// Return optional mutable reference to current skybox.
    pub fn skybox_mut(&mut self) -> Option<&mut SkyBox> {
        self.skybox.as_mut()
    }

    /// Return optional shared reference to current skybox.  
    pub fn skybox_ref(&self) -> Option<&SkyBox> {
        self.skybox.as_ref()
    }

    /// Creates picking ray from given screen coordinates.
    pub fn make_ray(&self, screen_coord: Vec2, screen_size: Vec2) -> Ray {
        let viewport = self.viewport_pixels(screen_size);
        let nx = screen_coord.x / (viewport.w as f32) * 2.0 - 1.0;
        // Invert y here because OpenGL has origin at left bottom corner,
        // but window coordinates starts from left *upper* corner.
        let ny = (viewport.h as f32 - screen_coord.y) / (viewport.h as f32) * 2.0 - 1.0;
        let inv_view_proj = self.view_projection_matrix().inverse().unwrap_or_default();
        let near = inv_view_proj.transform_vector4(Vec4::new(nx, ny, -1.0, 1.0));
        let far = inv_view_proj.transform_vector4(Vec4::new(nx, ny, 1.0, 1.0));
        let begin = near.xyz().scale(1.0 / near.w);
        let end = far.xyz().scale(1.0 / far.w);
        Ray::from_two_points(&begin, &end).unwrap_or_default()
    }

    /// Projects given world space point on screen plane.
    pub fn project(&self, world_pos: Vec3, screen_size: Vec2) -> Option<Vec2> {
        let viewport = self.viewport_pixels(screen_size);
        let proj = self
            .view_projection_matrix()
            .transform_vector4(Vec4::from_vec3(world_pos, 1.0));
        if proj.w != 0.0 {
            let k = (1.0 / proj.w) * 0.5;
            Some(Vec2::new(
                viewport.x as f32 + viewport.w as f32 * (proj.x * k + 0.5),
                viewport.h as f32 - (viewport.y as f32 + viewport.h as f32 * (proj.y * k + 0.5)),
            ))
        } else {
            None
        }
    }

    /// Creates a raw copy of a camera node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            fov: self.fov,
            z_near: self.z_near,
            z_far: self.z_far,
            viewport: self.viewport,
            view_matrix: self.view_matrix,
            projection_matrix: self.projection_matrix,
            enabled: self.enabled,
            skybox: self.skybox.clone(),
            // No need to copy cache. It is valid only for one frame.
            visibility_cache: Default::default(),
        }
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
    skybox: Option<SkyBox>,
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
            viewport: Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            skybox: None,
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

    /// Sets desired skybox.
    pub fn with_skybox(mut self, skybox: SkyBox) -> Self {
        self.skybox = Some(skybox);
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
            visibility_cache: Default::default(),
            skybox: self.skybox,
        }
    }

    /// Creates new node instance.
    pub fn build_node(self) -> Node {
        Node::Camera(self.build())
    }
}

/// Skybox is a huge box around camera. Each face has its own texture, when textures are
/// properly made, there is no seams and you get good decoration which contains static
/// skies and/or some other objects (mountains, buildings, etc.). Usually skyboxes used
/// in outdoor scenes, however real use of it limited only by your imagination. Skybox
/// will be drawn first, none of objects could be drawn before skybox.
#[derive(Debug, Clone, Default)]
pub struct SkyBox {
    /// Texture for front face.
    pub front: Option<Texture>,
    /// Texture for back face.
    pub back: Option<Texture>,
    /// Texture for left face.
    pub left: Option<Texture>,
    /// Texture for right face.
    pub right: Option<Texture>,
    /// Texture for top face.
    pub top: Option<Texture>,
    /// Texture for bottom face.
    pub bottom: Option<Texture>,
}

impl SkyBox {
    /// Returns slice with all textures, where: 0 - Front, 1 - Back, 2 - Left, 3 - Right
    /// 4 - Top, 5 - Bottom
    pub fn textures(&self) -> [Option<Texture>; 6] {
        [
            self.front.clone(),
            self.back.clone(),
            self.left.clone(),
            self.right.clone(),
            self.top.clone(),
            self.bottom.clone(),
        ]
    }
}

impl Visit for SkyBox {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.front.visit("Front", visitor)?;
        self.back.visit("Back", visitor)?;
        self.left.visit("Left", visitor)?;
        self.right.visit("Right", visitor)?;
        self.top.visit("Top", visitor)?;
        self.bottom.visit("Bottom", visitor)?;

        visitor.leave_region()
    }
}
