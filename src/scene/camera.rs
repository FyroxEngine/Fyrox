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

use crate::core::algebra::{Matrix4, Vector2, Vector3, Vector4};
use crate::core::pool::Handle;
use crate::resource::texture::{TextureKind, TexturePixelKind};
use crate::scene::graph::Graph;
use crate::{
    core::{
        math::{ray::Ray, Rect},
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        node::Node,
        VisibilityCache,
    },
};
use rapier3d::na::Point3;
use std::ops::{Deref, DerefMut};

/// See module docs.
#[derive(Debug)]
pub struct Camera {
    base: Base,
    fov: f32,
    z_near: f32,
    z_far: f32,
    viewport: Rect<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    enabled: bool,
    skybox: Option<Box<SkyBox>>,
    environment: Option<Texture>,
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
    fn default() -> Self {
        CameraBuilder::new(BaseBuilder::new()).build_camera()
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
        self.skybox.visit("SkyBox", visitor)?;
        self.environment.visit("Environment", visitor)?;
        // self.visibility_cache intentionally not serialized. It is valid only for one frame.
        visitor.leave_region()
    }
}

impl Camera {
    /// Explicitly calculates view and projection matrices. Normally, you should not call
    /// this method, it will be called automatically when new frame starts.
    #[inline]
    pub fn calculate_matrices(&mut self, frame_size: Vector2<f32>) {
        let pos = self.base.global_position();
        let look = self.base.look_vector();
        let up = self.base.up_vector();

        self.view_matrix = Matrix4::look_at_rh(&Point3::from(pos), &Point3::from(pos + look), &up);

        let viewport = self.viewport_pixels(frame_size);
        let aspect = viewport.w() as f32 / viewport.h() as f32;
        self.projection_matrix =
            Matrix4::new_perspective(aspect, self.fov, self.z_near, self.z_far);
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
    pub fn viewport_pixels(&self, frame_size: Vector2<f32>) -> Rect<i32> {
        Rect::new(
            (self.viewport.x() * frame_size.x) as i32,
            (self.viewport.y() * frame_size.y) as i32,
            (self.viewport.w() * frame_size.x) as i32,
            (self.viewport.h() * frame_size.y) as i32,
        )
    }

    /// Returns current view-projection matrix.
    #[inline]
    pub fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix * self.view_matrix
    }

    /// Returns current projection matrix.
    #[inline]
    pub fn projection_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix
    }

    /// Returns current view matrix.
    #[inline]
    pub fn view_matrix(&self) -> Matrix4<f32> {
        self.view_matrix
    }

    /// Returns inverse view matrix.
    #[inline]
    pub fn inv_view_matrix(&self) -> Option<Matrix4<f32>> {
        self.view_matrix.try_inverse()
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
        self.skybox = skybox.map(Box::new);
        self
    }

    /// Return optional mutable reference to current skybox.
    pub fn skybox_mut(&mut self) -> Option<&mut SkyBox> {
        self.skybox.as_deref_mut()
    }

    /// Return optional shared reference to current skybox.
    pub fn skybox_ref(&self) -> Option<&SkyBox> {
        self.skybox.as_deref()
    }

    /// Sets new environment.
    pub fn set_environment(&mut self, environment: Option<Texture>) -> &mut Self {
        self.environment = environment;
        self
    }

    /// Return optional mutable reference to current environment.
    pub fn environment_mut(&mut self) -> Option<&mut Texture> {
        self.environment.as_mut()
    }

    /// Return optional shared reference to current environment.
    pub fn environment_ref(&self) -> Option<&Texture> {
        self.environment.as_ref()
    }

    /// Return current environment map.
    pub fn environment_map(&self) -> Option<Texture> {
        self.environment.clone()
    }

    /// Creates picking ray from given screen coordinates.
    pub fn make_ray(&self, screen_coord: Vector2<f32>, screen_size: Vector2<f32>) -> Ray {
        let viewport = self.viewport_pixels(screen_size);
        let nx = screen_coord.x / (viewport.w() as f32) * 2.0 - 1.0;
        // Invert y here because OpenGL has origin at left bottom corner,
        // but window coordinates starts from left *upper* corner.
        let ny = (viewport.h() as f32 - screen_coord.y) / (viewport.h() as f32) * 2.0 - 1.0;
        let inv_view_proj = self
            .view_projection_matrix()
            .try_inverse()
            .unwrap_or_default();
        let near = inv_view_proj * Vector4::new(nx, ny, -1.0, 1.0);
        let far = inv_view_proj * Vector4::new(nx, ny, 1.0, 1.0);
        let begin = near.xyz().scale(1.0 / near.w);
        let end = far.xyz().scale(1.0 / far.w);
        Ray::from_two_points(begin, end)
    }

    /// Projects given world space point on screen plane.
    pub fn project(
        &self,
        world_pos: Vector3<f32>,
        screen_size: Vector2<f32>,
    ) -> Option<Vector2<f32>> {
        let viewport = self.viewport_pixels(screen_size);
        let proj = self.view_projection_matrix()
            * Vector4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
        if proj.w != 0.0 && proj.z >= 0.0 {
            let k = (1.0 / proj.w) * 0.5;
            Some(Vector2::new(
                viewport.x() as f32 + viewport.w() as f32 * (proj.x * k + 0.5),
                viewport.h() as f32
                    - (viewport.y() as f32 + viewport.h() as f32 * (proj.y * k + 0.5)),
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
            environment: self.environment.clone(),
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
    environment: Option<Texture>,
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
            viewport: Rect::new(0.0, 0.0, 1.0, 1.0),
            skybox: None,
            environment: None,
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

    /// Sets desired environment map.
    pub fn with_environment(mut self, environment: Texture) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Creates new instance of camera.
    pub fn build_camera(self) -> Camera {
        Camera {
            enabled: self.enabled,
            base: self.base_builder.build_base(),
            fov: self.fov,
            z_near: self.z_near,
            z_far: self.z_far,
            viewport: self.viewport,
            // No need to calculate these matrices - they'll be automatically
            // recalculated before rendering.
            view_matrix: Matrix4::identity(),
            projection_matrix: Matrix4::identity(),
            visibility_cache: Default::default(),
            skybox: self.skybox.map(Box::new),
            environment: self.environment,
        }
    }

    /// Creates new instance of camera node.
    pub fn build_node(self) -> Node {
        Node::Camera(self.build_camera())
    }

    /// Creates new instance of camera node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

/// SkyBox builder is used to create new skybox in declarative manner.
pub struct SkyBoxBuilder {
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

impl SkyBoxBuilder {
    /// Sets desired front face of cubemap.
    pub fn with_front(mut self, texture: Texture) -> Self {
        self.front = Some(texture);
        self
    }

    /// Sets desired back face of cubemap.
    pub fn with_back(mut self, texture: Texture) -> Self {
        self.back = Some(texture);
        self
    }

    /// Sets desired left face of cubemap.
    pub fn with_left(mut self, texture: Texture) -> Self {
        self.left = Some(texture);
        self
    }

    /// Sets desired right face of cubemap.
    pub fn with_right(mut self, texture: Texture) -> Self {
        self.right = Some(texture);
        self
    }

    /// Sets desired top face of cubemap.
    pub fn with_top(mut self, texture: Texture) -> Self {
        self.top = Some(texture);
        self
    }

    /// Sets desired front face of cubemap.
    pub fn with_bottom(mut self, texture: Texture) -> Self {
        self.bottom = Some(texture);
        self
    }

    /// Creates a new instance of skybox.
    pub fn build(self) -> Result<SkyBox, SkyBoxError> {
        let mut skybox = SkyBox {
            left: self.left,
            right: self.right,
            top: self.top,
            bottom: self.bottom,
            front: self.front,
            back: self.back,
            cubemap: None,
        };

        skybox.create_cubemap()?;

        Ok(skybox)
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
    /// Cubemap texture
    pub cubemap: Option<Texture>,
}

/// An error that may occur during skybox creation.
#[derive(Debug)]
pub enum SkyBoxError {
    /// Texture kind is not TextureKind::Rectangle
    UnsupportedTextureKind(TextureKind),
}

impl SkyBox {
    /// Returns cubemap texture
    pub fn cubemap(&self) -> Option<Texture> {
        self.cubemap.clone()
    }

    /// Creates a cubemap using provided faces. If some face has not been
    /// provided correcponding side will be black.
    /// It will fail if provided face's kind is not TextureKind::Rectangle
    pub fn create_cubemap(&mut self) -> Result<(), SkyBoxError> {
        let (kind, pixel_kind, bytes_per_face) = self
            .textures()
            .iter()
            .skip_while(|face| face.is_none())
            .next()
            .map_or(
                (
                    TextureKind::Rectangle {
                        width: 1,
                        height: 1,
                    },
                    TexturePixelKind::R8,
                    1,
                ),
                |face| {
                    let face = face.clone().unwrap();
                    let data = face.data_ref();

                    (data.kind(), data.pixel_kind(), data.data().len())
                },
            );

        let (width, height) = match kind {
            TextureKind::Rectangle { width, height } => (width, height),
            _ => return Err(SkyBoxError::UnsupportedTextureKind(kind)),
        };

        let mut data = Vec::<u8>::with_capacity(bytes_per_face * 6);
        for face in self.textures().iter() {
            if let Some(f) = face.clone() {
                data.extend(f.data_ref().data());
            } else {
                let black_face_data = vec![0; bytes_per_face];
                data.extend(black_face_data);
            }
        }

        let cubemap =
            Texture::from_bytes(TextureKind::Cube { width, height }, pixel_kind, data, false);

        self.cubemap = cubemap;

        Ok(())
    }

    /// Returns slice with all textures, where: 0 - Left, 1 - Right, 2 - Top, 3 - Bottom
    /// 4 - Front, 5 - Back
    fn textures(&self) -> [Option<Texture>; 6] {
        [
            self.left.clone(),
            self.right.clone(),
            self.top.clone(),
            self.bottom.clone(),
            self.front.clone(),
            self.back.clone(),
        ]
    }
}

impl Visit for SkyBox {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.left.visit("Left", visitor)?;
        self.right.visit("Right", visitor)?;
        self.top.visit("Top", visitor)?;
        self.bottom.visit("Bottom", visitor)?;
        self.front.visit("Front", visitor)?;
        self.back.visit("Back", visitor)?;

        visitor.leave_region()
    }
}
