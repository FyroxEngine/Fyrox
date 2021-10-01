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
        algebra::{Matrix4, Point3, Vector2, Vector3, Vector4},
        inspect::{Inspect, PropertyInfo},
        math::{ray::Ray, Rect},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::texture::{Texture, TextureError, TextureKind, TexturePixelKind, TextureWrapMode},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
        visibility::VisibilityCache,
    },
};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// Exposure is a parameter that describes how many light should be collected for one
/// frame. The higher the value, the more brighter the final frame will be and vice versa.
#[derive(Visit, Copy, Clone, PartialEq, Debug, Inspect)]
pub enum Exposure {
    /// Automatic exposure based on the frame luminance. High luminance values will result
    /// in lower exposure levels and vice versa. This is default option.
    ///
    /// # Equation
    ///
    /// `exposure = key_value / clamp(avg_luminance, min_luminance, max_luminance)`
    Auto {
        /// A key value in the formula above. Default is 0.01556.
        key_value: f32,
        /// A min luminance value in the formula above. Default is 0.00778.
        min_luminance: f32,
        /// A max luminance value in the formula above. Default is 64.0.
        max_luminance: f32,
    },

    /// Specific exposure level.
    Manual(f32),
}

impl Default for Exposure {
    fn default() -> Self {
        Self::Auto {
            key_value: 0.01556,
            min_luminance: 0.00778,
            max_luminance: 64.0,
        }
    }
}

/// See module docs.
#[derive(Debug, Visit, Inspect)]
pub struct Camera {
    #[inspect(expand)]
    base: Base,
    fov: f32,
    z_near: f32,
    z_far: f32,
    viewport: Rect<f32>,
    #[visit(skip)]
    #[inspect(skip)]
    view_matrix: Matrix4<f32>,
    #[visit(skip)]
    #[inspect(skip)]
    projection_matrix: Matrix4<f32>,
    enabled: bool,
    sky_box: Option<Box<SkyBox>>,
    environment: Option<Texture>,
    #[visit(optional)] // Backward compatibility.
    #[inspect(expand_subtree)]
    exposure: Exposure,
    #[visit(optional)] // Backward compatibility.
    color_grading_lut: Option<ColorGradingLut>,
    #[visit(optional)] // Backward compatibility.
    color_grading_enabled: bool,
    /// Visibility cache allows you to quickly check if object is visible from the camera or not.
    #[visit(skip)]
    #[inspect(skip)]
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
        self.viewport.position.x = self.viewport.position.x.clamp(0.0, 1.0);
        self.viewport.position.y = self.viewport.position.y.clamp(0.0, 1.0);
        self.viewport.size.x = self.viewport.size.x.clamp(0.0, 1.0);
        self.viewport.size.y = self.viewport.size.y.clamp(0.0, 1.0);
        self
    }

    /// Calculates viewport rectangle in pixels based on internal resolution-independent
    /// viewport. It is useful when you need to get real viewport rectangle in pixels.
    ///
    /// # Notes
    ///
    /// Viewport cannot be less than 1x1 pixel in size, so the method clamps values to
    /// range `[1; infinity]`. This is strictly needed because having viewport of 0 in size
    /// will cause panics in various places. It happens because viewport size is used as
    /// divisor in math formulas, but you cannot divide by zero.
    #[inline]
    pub fn viewport_pixels(&self, frame_size: Vector2<f32>) -> Rect<i32> {
        Rect::new(
            (self.viewport.x() * frame_size.x) as i32,
            (self.viewport.y() * frame_size.y) as i32,
            ((self.viewport.w() * frame_size.x) as i32).max(1),
            ((self.viewport.h() * frame_size.y) as i32).max(1),
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
        self.sky_box = skybox.map(Box::new);
        self
    }

    /// Return optional mutable reference to current skybox.
    pub fn skybox_mut(&mut self) -> Option<&mut SkyBox> {
        self.sky_box.as_deref_mut()
    }

    /// Return optional shared reference to current skybox.
    pub fn skybox_ref(&self) -> Option<&SkyBox> {
        self.sky_box.as_deref()
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
            sky_box: self.sky_box.clone(),
            environment: self.environment.clone(),
            exposure: self.exposure,
            color_grading_lut: self.color_grading_lut.clone(),
            color_grading_enabled: self.color_grading_enabled,
            // No need to copy cache. It is valid only for one frame.
            visibility_cache: Default::default(),
        }
    }

    /// Sets new color grading LUT.
    pub fn set_color_grading_map(&mut self, lut: Option<ColorGradingLut>) {
        self.color_grading_lut = lut;
    }

    /// Returns current color grading map.
    pub fn color_grading_lut(&self) -> Option<ColorGradingLut> {
        self.color_grading_lut.clone()
    }

    /// Returns current color grading map by ref.
    pub fn color_grading_lut_ref(&self) -> Option<&ColorGradingLut> {
        self.color_grading_lut.as_ref()
    }

    /// Enables or disables color grading.
    pub fn set_color_grading_enabled(&mut self, enable: bool) {
        self.color_grading_enabled = enable;
    }

    /// Whether color grading enabled or not.
    pub fn color_grading_enabled(&self) -> bool {
        self.color_grading_enabled
    }

    /// Sets new exposure. See `Exposure` struct docs for more info.
    pub fn set_exposure(&mut self, exposure: Exposure) {
        self.exposure = exposure;
    }

    /// Returns current exposure value.
    pub fn exposure(&self) -> Exposure {
        self.exposure
    }
}

/// All possible error that may occur during color grading look-up table creation.
#[derive(Debug, thiserror::Error)]
pub enum ColorGradingLutCreationError {
    /// There is not enough data in provided texture to build LUT.
    #[error(
        "There is not enough data in provided texture to build LUT. Required: {}, current: {}.",
        required,
        current
    )]
    NotEnoughData {
        /// Required amount of bytes.
        required: usize,
        /// Actual data size.
        current: usize,
    },

    /// Pixel format is not supported. It must be either RGB8 or RGBA8.
    #[error("Pixel format is not supported. It must be either RGB8 or RGBA8, but texture has {0:?} pixel format")]
    InvalidPixelFormat(TexturePixelKind),

    /// Texture error.
    #[error("Texture load error: {0:?}")]
    Texture(Option<Arc<TextureError>>),
}

/// Color grading look up table (LUT). Color grading is used to modify color space of the
/// rendered frame; it maps one color space to another. It is widely used effect in games,
/// you've probably noticed either "warmness" or "coldness" in colors in various scenes in
/// games - this is achieved by color grading.
///
/// See [more info in Unreal engine docs](https://docs.unrealengine.com/4.26/en-US/RenderingAndGraphics/PostProcessEffects/UsingLUTs/)
#[derive(Visit, Clone, Default, Debug)]
pub struct ColorGradingLut {
    #[visit(skip)]
    lut: Option<Texture>,
    unwrapped_lut: Option<Texture>,
}

impl ColorGradingLut {
    /// Creates 3D look-up texture from 2D strip.
    ///
    /// # Input Texture Requirements
    ///
    /// Width: 1024px
    /// Height: 16px
    /// Pixel Format: RGB8/RGBA8
    ///
    /// # Usage
    ///
    /// Typical usage would be:
    ///
    /// ```no_run
    /// use rg3d::scene::camera::ColorGradingLut;
    /// use rg3d::engine::resource_manager::{TextureImportOptions, ResourceManager};
    /// use rg3d::resource::texture::CompressionOptions;
    ///
    /// async fn create_lut(resource_manager: ResourceManager) -> ColorGradingLut {
    ///     ColorGradingLut::new(resource_manager.request_texture(
    ///         "examples/data/warm_color_lut.jpg",
    ///         Some(
    ///            // It is important to prevent engine from automatic texture compression.
    ///            // LUT can be constructed **only** from uncompressed RGB8/RGBA8 texture.
    ///             TextureImportOptions::default().with_compression(CompressionOptions::NoCompression),
    ///         ),
    ///     ))
    ///     .await
    ///     .unwrap()
    /// }
    /// ```
    ///
    /// Then pass LUT to either CameraBuilder or to camera instance, and don't forget to enable
    /// color grading.
    pub async fn new(unwrapped_lut: Texture) -> Result<Self, ColorGradingLutCreationError> {
        match unwrapped_lut.await {
            Ok(unwrapped_lut) => {
                let data = unwrapped_lut.data_ref();

                if data.pixel_kind() != TexturePixelKind::RGBA8
                    && data.pixel_kind() != TexturePixelKind::RGB8
                {
                    return Err(ColorGradingLutCreationError::InvalidPixelFormat(
                        data.pixel_kind(),
                    ));
                }

                let bytes = data.data();

                const RGBA8_SIZE: usize = 16 * 16 * 16 * 4;
                const RGB8_SIZE: usize = 16 * 16 * 16 * 3;

                if data.pixel_kind() == TexturePixelKind::RGBA8 {
                    if bytes.len() != RGBA8_SIZE {
                        return Err(ColorGradingLutCreationError::NotEnoughData {
                            required: RGBA8_SIZE,
                            current: bytes.len(),
                        });
                    }
                } else if bytes.len() != RGB8_SIZE {
                    return Err(ColorGradingLutCreationError::NotEnoughData {
                        required: RGB8_SIZE,
                        current: bytes.len(),
                    });
                }

                let pixel_size = if data.pixel_kind() == TexturePixelKind::RGBA8 {
                    4
                } else {
                    3
                };

                let mut lut_bytes = Vec::with_capacity(16 * 16 * 16 * 3);

                for z in 0..16 {
                    for y in 0..16 {
                        for x in 0..16 {
                            let pixel_index = z * 16 + y * 16 * 16 + x;
                            let pixel_byte_pos = pixel_index * pixel_size;

                            lut_bytes.push(bytes[pixel_byte_pos]); // R
                            lut_bytes.push(bytes[pixel_byte_pos + 1]); // G
                            lut_bytes.push(bytes[pixel_byte_pos + 2]); // B
                        }
                    }
                }

                let lut = Texture::from_bytes(
                    TextureKind::Volume {
                        width: 16,
                        height: 16,
                        depth: 16,
                    },
                    TexturePixelKind::RGB8,
                    lut_bytes,
                    false,
                )
                .unwrap();

                let mut lut_ref = lut.data_ref();

                lut_ref.set_s_wrap_mode(TextureWrapMode::ClampToEdge);
                lut_ref.set_t_wrap_mode(TextureWrapMode::ClampToEdge);

                drop(lut_ref);
                drop(data);

                Ok(Self {
                    lut: Some(lut),
                    unwrapped_lut: Some(unwrapped_lut),
                })
            }
            Err(e) => Err(ColorGradingLutCreationError::Texture(e)),
        }
    }

    /// Returns color grading unwrapped look-up table. This is initial texture that was
    /// used to create the look-up table.
    pub fn unwrapped_lut(&self) -> Texture {
        self.unwrapped_lut.clone().unwrap()
    }

    /// Returns 3D color grading look-up table ready for use on GPU.
    pub fn lut(&self) -> Texture {
        self.lut.clone().unwrap()
    }

    /// Returns 3D color grading look-up table by ref ready for use on GPU.
    pub fn lut_ref(&self) -> &Texture {
        self.lut.as_ref().unwrap()
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
    exposure: Exposure,
    color_grading_lut: Option<ColorGradingLut>,
    color_grading_enabled: bool,
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
            exposure: Default::default(),
            color_grading_lut: None,
            color_grading_enabled: false,
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

    /// Sets desired color grading LUT.
    pub fn with_color_grading_lut(mut self, lut: ColorGradingLut) -> Self {
        self.color_grading_lut = Some(lut);
        self
    }

    /// Sets whether color grading should be enabled or not.
    pub fn with_color_grading_enabled(mut self, enabled: bool) -> Self {
        self.color_grading_enabled = enabled;
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
            sky_box: self.skybox.map(Box::new),
            environment: self.environment,
            exposure: self.exposure,
            color_grading_lut: self.color_grading_lut,
            color_grading_enabled: self.color_grading_enabled,
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
    pub(in crate) front: Option<Texture>,
    /// Texture for back face.
    pub(in crate) back: Option<Texture>,
    /// Texture for left face.
    pub(in crate) left: Option<Texture>,
    /// Texture for right face.
    pub(in crate) right: Option<Texture>,
    /// Texture for top face.
    pub(in crate) top: Option<Texture>,
    /// Texture for bottom face.
    pub(in crate) bottom: Option<Texture>,
    /// Cubemap texture
    pub(in crate) cubemap: Option<Texture>,
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
    /// provided corresponding side will be black.
    /// It will fail if provided face's kind is not TextureKind::Rectangle
    pub fn create_cubemap(&mut self) -> Result<(), SkyBoxError> {
        let (kind, pixel_kind, bytes_per_face) =
            self.textures().iter().find(|face| face.is_some()).map_or(
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
    /// 4 - Front, 5 - Back.
    ///
    /// # Important notes.
    ///
    /// These textures are **not** used for rendering! The renderer uses cube map made of these
    /// textures. Public access for these textures is needed in case you need to read internals
    /// of the textures.
    pub fn textures(&self) -> [Option<Texture>; 6] {
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
