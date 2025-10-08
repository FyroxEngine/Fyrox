// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Contains all methods and structures to create and manage cameras. See [`Camera`] docs for more info.

use crate::{
    asset::{state::LoadError, untyped::ResourceKind},
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3, Vector4},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, ray::Ray, Rect},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        uuid_provider,
        variable::InheritableVariable,
        visitor::{Visit, VisitResult, Visitor},
    },
    graph::BaseSceneGraph,
    resource::texture::{
        TextureKind, TexturePixelKind, TextureResource, TextureResourceExtension, TextureWrapMode,
    },
    scene::{
        base::{Base, BaseBuilder},
        debug::SceneDrawingContext,
        graph::Graph,
        node::constructor::NodeConstructor,
        node::{Node, NodeTrait, UpdateContext},
    },
};
use fyrox_graph::constructor::ConstructorProvider;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Perspective projection make parallel lines to converge at some point. Objects will be smaller
/// with increasing distance. This the projection type "used" by human eyes, photographic lens and
/// it looks most realistic.
#[derive(Reflect, Clone, Debug, PartialEq, Visit, Serialize, Deserialize)]
pub struct PerspectiveProjection {
    /// Vertical angle at the top of viewing frustum, in radians. Larger values will increase field
    /// of view and create fish-eye effect, smaller values could be used to create "binocular" effect
    /// or scope effect.
    #[reflect(min_value = 0.0, max_value = 6.28, step = 0.1)]
    pub fov: f32,
    /// Location of the near clipping plane. If it is larger than [`Self::z_far`] then it will be
    /// treated like far clipping plane.
    #[reflect(min_value = 0.0, step = 0.1)]
    pub z_near: f32,
    /// Location of the far clipping plane. If it is less than [`Self::z_near`] then it will be
    /// treated like near clipping plane.
    #[reflect(min_value = 0.0, step = 0.1)]
    pub z_far: f32,
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        Self {
            fov: 75.0f32.to_radians(),
            z_near: 0.025,
            z_far: 2048.0,
        }
    }
}

impl PerspectiveProjection {
    /// Returns perspective projection matrix.
    #[inline]
    pub fn matrix(&self, frame_size: Vector2<f32>) -> Matrix4<f32> {
        let limit = 10.0 * f32::EPSILON;

        let z_near = self.z_far.min(self.z_near);
        let mut z_far = self.z_far.max(self.z_near);

        // Prevent planes from superimposing which could cause panic.
        if z_far - z_near < limit {
            z_far += limit;
        }

        Matrix4::new_perspective(
            (frame_size.x / frame_size.y).max(limit),
            self.fov,
            z_near,
            z_far,
        )
    }
}

/// Parallel projection. Object's size won't be affected by distance from the viewer, it can be
/// used for 2D games.
#[derive(Reflect, Clone, Debug, PartialEq, Visit, Serialize, Deserialize)]
pub struct OrthographicProjection {
    /// Location of the near clipping plane. If it is larger than [`Self::z_far`] then it will be
    /// treated like far clipping plane.
    #[reflect(min_value = 0.0, step = 0.1)]
    pub z_near: f32,
    /// Location of the far clipping plane. If it is less than [`Self::z_near`] then it will be
    /// treated like near clipping plane.
    #[reflect(min_value = 0.0, step = 0.1)]
    pub z_far: f32,
    /// Vertical size of the "view box". Horizontal size is derived value and depends on the aspect
    /// ratio of the viewport. Any values very close to zero (from both sides) will be clamped to
    /// some minimal value to prevent singularities from occuring.
    #[reflect(step = 0.1)]
    pub vertical_size: f32,
}

impl Default for OrthographicProjection {
    fn default() -> Self {
        Self {
            z_near: 0.0,
            z_far: 2048.0,
            vertical_size: 5.0,
        }
    }
}

impl OrthographicProjection {
    /// Returns orthographic projection matrix.
    #[inline]
    pub fn matrix(&self, frame_size: Vector2<f32>) -> Matrix4<f32> {
        fn clamp_to_limit_signed(value: f32, limit: f32) -> f32 {
            if value < 0.0 && -value < limit {
                -limit
            } else if value >= 0.0 && value < limit {
                limit
            } else {
                value
            }
        }

        let limit = 10.0 * f32::EPSILON;

        let aspect = (frame_size.x / frame_size.y).max(limit);

        // Prevent collapsing projection "box" into a point, which could cause panic.
        let vertical_size = clamp_to_limit_signed(self.vertical_size, limit);
        let horizontal_size = clamp_to_limit_signed(aspect * vertical_size, limit);

        let z_near = self.z_far.min(self.z_near);
        let mut z_far = self.z_far.max(self.z_near);

        // Prevent planes from superimposing which could cause panic.
        if z_far - z_near < limit {
            z_far += limit;
        }

        let left = -horizontal_size;
        let top = vertical_size;
        let right = horizontal_size;
        let bottom = -vertical_size;
        Matrix4::new_orthographic(left, right, bottom, top, z_near, z_far)
    }
}

/// A method of projection. Different projection types suitable for different purposes:
///
/// 1) Perspective projection most useful for 3D games, it makes a scene to look most natural,
/// objects will look smaller with increasing distance.
/// 2) Orthographic projection most useful for 2D games, objects won't look smaller with increasing
/// distance.
#[derive(
    Reflect,
    Clone,
    Debug,
    PartialEq,
    Visit,
    AsRefStr,
    EnumString,
    VariantNames,
    Serialize,
    Deserialize,
)]
pub enum Projection {
    /// See [`PerspectiveProjection`] docs.
    Perspective(PerspectiveProjection),
    /// See [`OrthographicProjection`] docs.
    Orthographic(OrthographicProjection),
}

uuid_provider!(Projection = "0eb5bec0-fc4e-4945-99b6-e6c5392ad971");

impl Projection {
    /// Sets the new value for the near clipping plane.
    #[inline]
    #[must_use]
    pub fn with_z_near(mut self, z_near: f32) -> Self {
        match self {
            Projection::Perspective(ref mut v) => v.z_near = z_near,
            Projection::Orthographic(ref mut v) => v.z_near = z_near,
        }
        self
    }

    /// Sets the new value for the far clipping plane.
    #[inline]
    #[must_use]
    pub fn with_z_far(mut self, z_far: f32) -> Self {
        match self {
            Projection::Perspective(ref mut v) => v.z_far = z_far,
            Projection::Orthographic(ref mut v) => v.z_far = z_far,
        }
        self
    }

    /// Sets the new value for the near clipping plane.
    #[inline]
    pub fn set_z_near(&mut self, z_near: f32) {
        match self {
            Projection::Perspective(v) => v.z_near = z_near,
            Projection::Orthographic(v) => v.z_near = z_near,
        }
    }

    /// Sets the new value for the far clipping plane.
    #[inline]
    pub fn set_z_far(&mut self, z_far: f32) {
        match self {
            Projection::Perspective(v) => v.z_far = z_far,
            Projection::Orthographic(v) => v.z_far = z_far,
        }
    }

    /// Returns near clipping plane distance.
    #[inline]
    pub fn z_near(&self) -> f32 {
        match self {
            Projection::Perspective(v) => v.z_near,
            Projection::Orthographic(v) => v.z_near,
        }
    }

    /// Returns far clipping plane distance.
    #[inline]
    pub fn z_far(&self) -> f32 {
        match self {
            Projection::Perspective(v) => v.z_far,
            Projection::Orthographic(v) => v.z_far,
        }
    }

    /// Returns projection matrix.
    #[inline]
    pub fn matrix(&self, frame_size: Vector2<f32>) -> Matrix4<f32> {
        match self {
            Projection::Perspective(v) => v.matrix(frame_size),
            Projection::Orthographic(v) => v.matrix(frame_size),
        }
    }

    /// Returns `true` if the current projection is perspective.
    #[inline]
    pub fn is_perspective(&self) -> bool {
        matches!(self, Projection::Perspective(_))
    }

    /// Returns `true` if the current projection is orthographic.
    #[inline]
    pub fn is_orthographic(&self) -> bool {
        matches!(self, Projection::Orthographic(_))
    }
}

impl Default for Projection {
    fn default() -> Self {
        Self::Perspective(PerspectiveProjection::default())
    }
}

/// Exposure is a parameter that describes how many light should be collected for one
/// frame. The higher the value, the more brighter the final frame will be and vice versa.
#[derive(Visit, Copy, Clone, PartialEq, Debug, Reflect, AsRefStr, EnumString, VariantNames)]
pub enum Exposure {
    /// Automatic exposure based on the frame luminance. High luminance values will result
    /// in lower exposure levels and vice versa. This is default option.
    ///
    /// # Equation
    ///
    /// `exposure = key_value / clamp(avg_luminance, min_luminance, max_luminance)`
    Auto {
        /// A key value in the formula above. Default is 0.01556.
        #[reflect(min_value = 0.0, step = 0.1)]
        key_value: f32,
        /// A min luminance value in the formula above. Default is 0.00778.
        #[reflect(min_value = 0.0, step = 0.1)]
        min_luminance: f32,
        /// A max luminance value in the formula above. Default is 64.0.
        #[reflect(min_value = 0.0, step = 0.1)]
        max_luminance: f32,
    },

    /// Specific exposure level. To "disable" any HDR effects use [`std::f32::consts::E`] as a value.
    Manual(f32),
}

uuid_provider!(Exposure = "0e35ee3d-8baa-4b0c-b3dd-6c31a08c121e");

impl Default for Exposure {
    fn default() -> Self {
        Self::Auto {
            key_value: 0.01556,
            min_luminance: 0.00778,
            max_luminance: 64.0,
        }
    }
}

/// Camera allows you to see world from specific point in world. You must have at least one camera in
/// your scene to see anything.
///
/// ## Projection
///
/// There are two main projection modes supported by Camera node: perspective and orthogonal projections.
/// Perspective projection is used primarily to display 3D scenes, while orthogonal projection could be
/// used for both 3D and 2D. Orthogonal projection could also be used in CAD software.
///
/// ## Skybox
///
/// Skybox is a cube around the camera with six textures forming seamless "sky". It could be anything,
/// starting from simple blue sky and ending with outer space.
///
/// ## Multiple cameras
///
/// Fyrox supports multiple cameras per scene, it means that you can create split screen games, make
/// picture-in-picture insertions in your main camera view and any other combinations you need.
///
/// ## Performance
///
/// Each camera forces engine to re-render same scene one more time, which may cause almost double load
/// of your GPU.
#[derive(Debug, Visit, Reflect, Clone, ComponentProvider)]
#[reflect(derived_type = "Node")]
pub struct Camera {
    base: Base,

    #[reflect(setter = "set_projection")]
    projection: InheritableVariable<Projection>,

    #[reflect(setter = "set_viewport")]
    viewport: InheritableVariable<Rect<f32>>,

    #[reflect(setter = "set_enabled")]
    enabled: InheritableVariable<bool>,

    #[reflect(setter = "set_environment")]
    environment: InheritableVariable<Option<TextureResource>>,

    #[reflect(setter = "set_exposure")]
    exposure: InheritableVariable<Exposure>,

    #[reflect(setter = "set_color_grading_lut")]
    color_grading_lut: InheritableVariable<Option<ColorGradingLut>>,

    #[reflect(setter = "set_color_grading_enabled")]
    color_grading_enabled: InheritableVariable<bool>,

    #[reflect(setter = "set_render_target")]
    #[visit(skip)]
    render_target: Option<TextureResource>,

    #[visit(skip)]
    #[reflect(hidden)]
    view_matrix: Matrix4<f32>,

    #[visit(skip)]
    #[reflect(hidden)]
    projection_matrix: Matrix4<f32>,
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

impl TypeUuidProvider for Camera {
    fn type_uuid() -> Uuid {
        uuid!("198d3aca-433c-4ce1-bb25-3190699b757f")
    }
}

/// A set of camera fitting parameters for different projection modes. You should take these parameters
/// and modify camera position and projection accordingly. In case of perspective projection all you need
/// to do is to set new world-space position of the camera. In cae of orthographic projection, do previous
/// step and also modify vertical size of orthographic projection (see [`OrthographicProjection`] for more
/// info).
pub enum FitParameters {
    /// Fitting parameters for perspective projection.
    Perspective {
        /// New world-space position of the camera.
        position: Vector3<f32>,
        /// Distance from the center of an AABB of the object to the `position`.
        distance: f32,
    },
    /// Fitting parameters for orthographic projection.
    Orthographic {
        /// New world-space position of the camera.
        position: Vector3<f32>,
        /// New vertical size for orthographic projection.
        vertical_size: f32,
    },
}

impl FitParameters {
    fn fallback_perspective() -> Self {
        Self::Perspective {
            position: Default::default(),
            distance: 1.0,
        }
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
        self.projection_matrix = self.projection.matrix(frame_size);
    }

    /// Sets new viewport in resolution-independent format. In other words
    /// each parameter of viewport defines portion of your current resolution
    /// in percents. In example viewport (0.0, 0.0, 0.5, 1.0) will force camera
    /// to use left half of your screen and (0.5, 0.0, 0.5, 1.0) - right half.
    /// Why not just use pixels directly? Because you can change resolution while
    /// your application is running and you'd be force to manually recalculate
    /// pixel values everytime when resolution changes.
    pub fn set_viewport(&mut self, mut viewport: Rect<f32>) -> Rect<f32> {
        viewport.position.x = viewport.position.x.clamp(0.0, 1.0);
        viewport.position.y = viewport.position.y.clamp(0.0, 1.0);
        viewport.size.x = viewport.size.x.clamp(0.0, 1.0);
        viewport.size.y = viewport.size.y.clamp(0.0, 1.0);
        self.viewport.set_value_and_mark_modified(viewport)
    }

    /// Returns current viewport.
    pub fn viewport(&self) -> Rect<f32> {
        *self.viewport
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

    /// Returns current projection mode.
    #[inline]
    pub fn projection(&self) -> &Projection {
        &self.projection
    }

    /// Returns current projection mode.
    #[inline]
    pub fn projection_value(&self) -> Projection {
        (*self.projection).clone()
    }

    /// Returns current projection mode as mutable reference.
    #[inline]
    pub fn projection_mut(&mut self) -> &mut Projection {
        self.projection.get_value_mut_and_mark_modified()
    }

    /// Sets current projection mode.
    #[inline]
    pub fn set_projection(&mut self, projection: Projection) -> Projection {
        self.projection.set_value_and_mark_modified(projection)
    }

    /// Returns state of camera: enabled or not.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        *self.enabled
    }

    /// Enables or disables camera. Disabled cameras will be ignored during
    /// rendering. This allows you to exclude views from specific cameras from
    /// final picture.
    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) -> bool {
        self.enabled.set_value_and_mark_modified(enabled)
    }

    /// Sets new environment.
    pub fn set_environment(
        &mut self,
        environment: Option<TextureResource>,
    ) -> Option<TextureResource> {
        self.environment.set_value_and_mark_modified(environment)
    }

    /// Return optional mutable reference to current environment.
    pub fn environment_mut(&mut self) -> Option<&mut TextureResource> {
        self.environment.get_value_mut_and_mark_modified().as_mut()
    }

    /// Return optional shared reference to current environment.
    pub fn environment_ref(&self) -> Option<&TextureResource> {
        self.environment.as_ref()
    }

    /// Return current environment map.
    pub fn environment_map(&self) -> Option<TextureResource> {
        (*self.environment).clone()
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

    /// Calculates new fitting parameters for the given axis-aligned bounding box using current camera's
    /// global transform and provided aspect ratio. See [`FitParameters`] docs for more info.
    ///
    /// This method returns fitting parameters and **do not** modify camera's state. It is needed, because in
    /// some cases your camera could be attached to some sort of a hinge node and setting its local position
    /// in order to fit it to the given AABB would break the preset spatial relations between nodes. Instead,
    /// the method returns a set of parameters that can be used as you want.
    #[inline]
    #[must_use]
    pub fn fit(
        &self,
        aabb: &AxisAlignedBoundingBox,
        aspect_ratio: f32,
        scale: f32,
    ) -> FitParameters {
        if aabb.is_invalid_or_degenerate() {
            return FitParameters::fallback_perspective();
        }

        let look_vector = self
            .look_vector()
            .try_normalize(f32::EPSILON)
            .unwrap_or_default();

        match self.projection.deref() {
            Projection::Perspective(perspective) => {
                let radius = aabb.half_extents().max();

                let denominator = (perspective.fov * 0.5).sin();
                if denominator == 0.0 {
                    return FitParameters::fallback_perspective();
                }

                let distance = radius / denominator * scale;
                FitParameters::Perspective {
                    position: aabb.center() - look_vector.scale(distance),
                    distance,
                }
            }
            Projection::Orthographic(_) => {
                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                let mut max_x = -f32::MAX;
                let mut max_y = -f32::MAX;
                let inv = self.global_transform().try_inverse().unwrap_or_default();
                for point in aabb.corners() {
                    let local = inv.transform_point(&Point3::from(point));
                    if local.x < min_x {
                        min_x = local.x;
                    }
                    if local.y < min_y {
                        min_y = local.y;
                    }
                    if local.x > max_x {
                        max_x = local.x;
                    }
                    if local.y > max_y {
                        max_y = local.y;
                    }
                }

                FitParameters::Orthographic {
                    position: aabb.center()
                        - look_vector.scale((aabb.max - aabb.min).norm() * scale),
                    vertical_size: (max_y - min_y).max((max_x - min_x) * aspect_ratio) * scale,
                }
            }
        }
    }

    /// Returns current frustum of the camera.
    #[inline]
    pub fn frustum(&self) -> Frustum {
        Frustum::from_view_projection_matrix(self.view_projection_matrix()).unwrap_or_default()
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

    /// Sets new color grading LUT.
    pub fn set_color_grading_lut(
        &mut self,
        lut: Option<ColorGradingLut>,
    ) -> Option<ColorGradingLut> {
        self.color_grading_lut.set_value_and_mark_modified(lut)
    }

    /// Returns current color grading map.
    pub fn color_grading_lut(&self) -> Option<ColorGradingLut> {
        (*self.color_grading_lut).clone()
    }

    /// Returns current color grading map by ref.
    pub fn color_grading_lut_ref(&self) -> Option<&ColorGradingLut> {
        self.color_grading_lut.as_ref()
    }

    /// Enables or disables color grading.
    pub fn set_color_grading_enabled(&mut self, enable: bool) -> bool {
        self.color_grading_enabled
            .set_value_and_mark_modified(enable)
    }

    /// Whether color grading enabled or not.
    pub fn color_grading_enabled(&self) -> bool {
        *self.color_grading_enabled
    }

    /// Sets new exposure. See `Exposure` struct docs for more info.
    pub fn set_exposure(&mut self, exposure: Exposure) -> Exposure {
        self.exposure.set_value_and_mark_modified(exposure)
    }

    /// Returns current exposure value.
    pub fn exposure(&self) -> Exposure {
        *self.exposure
    }

    /// Sets a new render target of the camera. If set, the camera will render to the specified
    /// render target and will not appear in the final frame. Typical usage is something like this:
    ///
    /// ```rust
    /// # use fyrox_impl::scene::camera::Camera;
    /// # use fyrox_texture::{TextureResource, TextureResourceExtension};
    /// fn set_render_target(camera: &mut Camera) {
    ///     // Create a render target of 256x256 pixels. The size of the render target can be changed
    ///     // at runtime, and the engine will automatically adjust GPU resources for you. The render
    ///     // target is a resource, thus it can be shared across multiple "users". For instance, you
    ///     // can apply this render target texture to a quad in your game world, and it will make a
    ///     // sort of virtual camera (surveillance camera).
    ///     let render_target = TextureResource::new_render_target(256, 256);
    ///     camera.set_render_target(Some(render_target));
    /// }
    /// ```
    ///
    /// # Serialization
    ///
    /// The render target is non-serializable, and you have to re-create it after deserialization.
    pub fn set_render_target(
        &mut self,
        render_target: Option<TextureResource>,
    ) -> Option<TextureResource> {
        std::mem::replace(&mut self.render_target, render_target)
    }

    /// Returns a reference to the current render target (if any).
    pub fn render_target(&self) -> Option<&TextureResource> {
        self.render_target.as_ref()
    }
}

impl ConstructorProvider<Node, Graph> for Camera {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>().with_variant("Camera", |_| {
            CameraBuilder::new(BaseBuilder::new().with_name("Camera"))
                .build_node()
                .into()
        })
    }
}

impl NodeTrait for Camera {
    /// Returns current **local-space** bounding box.
    #[inline]
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        // TODO: Maybe calculate AABB using frustum corners?
        self.base.local_bounding_box()
    }

    /// Returns current **world-space** bounding box.
    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn update(&mut self, context: &mut UpdateContext) {
        let frame_size = if let Some(TextureKind::Rectangle { width, height }) = self
            .render_target
            .as_ref()
            .and_then(|rt| rt.data_ref().as_loaded_ref().map(|rt| rt.kind()))
        {
            Vector2::new(width as f32, height as f32)
        } else {
            context.frame_size
        };

        self.calculate_matrices(frame_size);
    }

    fn debug_draw(&self, ctx: &mut SceneDrawingContext) {
        let transform = self.global_transform_without_scaling();
        ctx.draw_pyramid(
            self.frustum().center(),
            self.frustum().right_top_front_corner(),
            self.frustum().left_top_front_corner(),
            self.frustum().left_bottom_front_corner(),
            self.frustum().right_bottom_front_corner(),
            Color::GREEN,
            transform,
        );
    }
}

/// All possible error that may occur during color grading look-up table creation.
#[derive(Debug)]
pub enum ColorGradingLutCreationError {
    /// There is not enough data in provided texture to build LUT.
    NotEnoughData {
        /// Required amount of bytes.
        required: usize,
        /// Actual data size.
        current: usize,
    },

    /// Pixel format is not supported. It must be either RGB8 or RGBA8.
    InvalidPixelFormat(TexturePixelKind),

    /// Texture error.
    Texture(LoadError),
}

impl Display for ColorGradingLutCreationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorGradingLutCreationError::NotEnoughData { required, current } => {
                write!(
                    f,
                    "There is not enough data in provided \
                texture to build LUT. Required: {required}, current: {current}.",
                )
            }
            ColorGradingLutCreationError::InvalidPixelFormat(v) => {
                write!(
                    f,
                    "Pixel format is not supported. It must be either RGB8 \
                or RGBA8, but texture has {v:?} pixel format"
                )
            }
            ColorGradingLutCreationError::Texture(v) => {
                write!(f, "Texture load error: {v}")
            }
        }
    }
}

/// Color grading look up table (LUT). Color grading is used to modify color space of the
/// rendered frame; it maps one color space to another. It is widely used effect in games,
/// you've probably noticed either "warmness" or "coldness" in colors in various scenes in
/// games - this is achieved by color grading.
///
/// See [more info in Unreal engine docs](https://docs.unrealengine.com/4.26/en-US/RenderingAndGraphics/PostProcessEffects/UsingLUTs/)
#[derive(Visit, Clone, Default, PartialEq, Debug, Reflect, Eq)]
pub struct ColorGradingLut {
    unwrapped_lut: Option<TextureResource>,

    #[visit(skip)]
    #[reflect(hidden)]
    lut: Option<TextureResource>,
}

uuid_provider!(ColorGradingLut = "bca9c90a-7cde-4960-8814-c132edfc9614");

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
    /// # use fyrox_impl::scene::camera::ColorGradingLut;
    /// # use fyrox_impl::asset::manager::{ResourceManager};
    /// # use fyrox_impl::resource::texture::Texture;
    ///
    /// async fn create_lut(resource_manager: ResourceManager) -> ColorGradingLut {
    ///     ColorGradingLut::new(resource_manager.request::<Texture>(
    ///         "your_lut.jpg",
    ///     ))
    ///     .await
    ///     .unwrap()
    /// }
    /// ```
    ///
    /// Then pass LUT to either CameraBuilder or to camera instance, and don't forget to enable
    /// color grading.
    pub async fn new(unwrapped_lut: TextureResource) -> Result<Self, ColorGradingLutCreationError> {
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

                let lut = TextureResource::from_bytes(
                    Uuid::new_v4(),
                    TextureKind::Volume {
                        width: 16,
                        height: 16,
                        depth: 16,
                    },
                    TexturePixelKind::RGB8,
                    lut_bytes,
                    ResourceKind::Embedded,
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
    pub fn unwrapped_lut(&self) -> TextureResource {
        self.unwrapped_lut.clone().unwrap()
    }

    /// Returns 3D color grading look-up table ready for use on GPU.
    pub fn lut(&self) -> TextureResource {
        self.lut.clone().unwrap()
    }

    /// Returns 3D color grading look-up table by ref ready for use on GPU.
    pub fn lut_ref(&self) -> &TextureResource {
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
    environment: Option<TextureResource>,
    exposure: Exposure,
    color_grading_lut: Option<ColorGradingLut>,
    color_grading_enabled: bool,
    projection: Projection,
    render_target: Option<TextureResource>,
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
            environment: None,
            exposure: Exposure::Manual(std::f32::consts::E),
            color_grading_lut: None,
            color_grading_enabled: false,
            projection: Projection::default(),
            render_target: None,
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

    /// Sets desired environment map.
    pub fn with_environment(mut self, environment: TextureResource) -> Self {
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

    /// Sets desired exposure options.
    pub fn with_exposure(mut self, exposure: Exposure) -> Self {
        self.exposure = exposure;
        self
    }

    /// Sets desired projection mode.
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.projection = projection;
        self
    }

    /// Sets desired render target for the camera.
    pub fn with_render_target(mut self, render_target: Option<TextureResource>) -> Self {
        self.render_target = render_target;
        self
    }

    /// Creates new instance of camera.
    pub fn build_camera(self) -> Camera {
        Camera {
            enabled: self.enabled.into(),
            base: self.base_builder.build_base(),
            projection: self.projection.into(),
            viewport: self.viewport.into(),
            // No need to calculate these matrices - they'll be automatically
            // recalculated before rendering.
            view_matrix: Matrix4::identity(),
            projection_matrix: Matrix4::identity(),
            environment: self.environment.into(),
            exposure: self.exposure.into(),
            color_grading_lut: self.color_grading_lut.into(),
            color_grading_enabled: self.color_grading_enabled.into(),
            render_target: self.render_target,
        }
    }

    /// Creates new instance of camera node.
    pub fn build_node(self) -> Node {
        Node::new(self.build_camera())
    }

    /// Creates new instance of camera node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
