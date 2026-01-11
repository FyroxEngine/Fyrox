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

//! Reflection probe is an object that allows "capturing" a scene content in a cube texture, that
//! can later be used to render reflections and be used as a source of ambient lighting for a scene.
//! See [`ReflectionProbe`] docs for more info.

use crate::{
    core::{
        algebra::Vector3,
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    graph::{constructor::ConstructorProvider, SceneGraph},
    scene::EnvironmentLightingSource,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{constructor::NodeConstructor, Node, NodeTrait, UpdateContext},
    },
};
use fyrox_core::color::Color;
use fyrox_texture::{TextureResource, TextureResourceExtension};
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

const DEFAULT_RESOLUTION: usize = 512;

/// Update mode of reflection probes.
#[derive(
    Clone,
    Reflect,
    PartialEq,
    Default,
    Debug,
    Visit,
    TypeUuidProvider,
    AsRefStr,
    EnumString,
    VariantNames,
)]
#[type_uuid(id = "66450303-4f6c-4456-bde5-a7309f25b7ce")]
pub enum UpdateMode {
    /// The probe will be updated once it is created and its content won't change until the
    /// [`ReflectionProbe::force_update`] call.
    #[default]
    Once,

    /// The probe will be updated each frame. This option may lead to performance issues and
    /// should be used with caution.
    EachFrame,
}

/// Reflection probe is an object that allows "capturing" a scene content in a cube texture, that
/// can later be used to render reflections and be used as a source of ambient lighting for a scene.
///
/// ## Update Mode
///
/// Reflection probe can be updated either once or every frame. The default mode is [`UpdateMode::Once`].
/// If you need dynamic reflections, then use [`UpdateMode::EachFrame`] mode. However, it may lead
/// to performance issues.
///
/// ## Performance
///
/// Reflection probe renders the scene six times which is quite slow. In most cases, it does not matter
/// because most of the probes can be updated just once (static probes). Such probes can have
/// increased resolution.
///
/// Dynamic probes are the heaviest and require careful performance tweaking. There should be a balance
/// between the resolution and the speed. Reflection probes does frustum culling, so some part of
/// the scene geometry can be excluded. This functionality can be tweaked by setting the far clipping
/// plane distance to lower values to prevent the probe to render distant objects.
///
/// ## Interaction With Cameras
///
/// When rendering, the engine will automatically pick a reflection probe for a camera. It is done
/// by a simple point-box intersection test. This reflection probe will then be used for rendering
/// using the camera.
///
/// ## Example
///
/// The following example creates a new reflection probe 20 units wide in all directions, centered
/// at (0.0, 10.0, 0.0) point with a rendering position offset by 10 units along X axis.
///
/// ```rust
/// use fyrox_impl::{
///     core::{algebra::Vector3, pool::Handle},
///     scene::{
///         base::BaseBuilder,
///         graph::Graph,
///         node::Node,
///         probe::{ReflectionProbeBuilder, UpdateMode},
///         transform::TransformBuilder,
///     },
/// };
///
/// fn create_probe(graph: &mut Graph) -> Handle<Node> {
///     ReflectionProbeBuilder::new(
///         BaseBuilder::new().with_local_transform(
///             TransformBuilder::new()
///                 // The center of the probe's bounding box is located 10 units above the ground.
///                 .with_local_position(Vector3::new(0.0, 10.0, 0.0))
///                 // The size of the probe's bounding box is 20 units.
///                 .with_local_scale(Vector3::repeat(20.0))
///                 .build(),
///         ),
///     )
///     // Set resolution of the probe.
///     .with_resolution(256)
///     // The probe will capture the scene once it is created.
///     .with_update_mode(UpdateMode::Once)
///     // Set the capture point slightly off-center. The probe will capture the scene at
///     // (10.0, 10.0, 0.0) point.
///     .with_rendering_local_position(Vector3::new(10.0, 0.0, 0.0))
///     .build(graph)
/// }
/// ```
#[derive(Clone, Reflect, Debug, Visit, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "7e0c138f-e371-4045-bd2c-ff5b165c7ee6")]
#[reflect(derived_type = "Node")]
#[visit(optional, post_visit_method = "on_visited")]
pub struct ReflectionProbe {
    base: Base,

    /// Defines rendering position in local coordinate space of the probe. The scene will be captured
    /// from the point ignoring the orientation of the probe.
    pub rendering_position: InheritableVariable<Vector3<f32>>,

    /// Resolution of the probe. It defines the size of the cube map face and thus the overall
    /// quality of the image. The larger the value, the more detailed reflections will be. Large
    /// values may slow down rendering of the probe.
    #[reflect(max_value = 2048.0, min_value = 16.0, setter = "set_resolution")]
    pub resolution: InheritableVariable<usize>,

    /// Position of the near clipping plane.
    #[reflect(min_value = 0.0)]
    pub z_near: InheritableVariable<f32>,

    /// Position of the far clipping plane. This parameter can be decreased to improve performance.
    #[reflect(min_value = 0.0)]
    pub z_far: InheritableVariable<f32>,

    /// Update mode of the probe. See [`UpdateMode`] docs for more info.
    pub update_mode: InheritableVariable<UpdateMode>,

    /// Ambient lighting of the reflection probe. This value is used only if the `environment` is
    /// set to [`EnvironmentLightingSource::AmbientColor`].
    pub ambient_lighting_color: InheritableVariable<Color>,

    /// Environment lighting source of the reflection probe.
    pub environment_lighting_source: InheritableVariable<EnvironmentLightingSource>,

    /// A flag, that defines whether the probe should be updated or not.
    #[reflect(hidden)]
    #[visit(skip)]
    pub need_update: bool,

    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) updated: Cell<bool>,
    #[reflect(hidden)]
    render_target: TextureResource,
}

impl Default for ReflectionProbe {
    fn default() -> Self {
        Self {
            base: Default::default(),
            rendering_position: Default::default(),
            resolution: DEFAULT_RESOLUTION.into(),
            z_near: 0.001.into(),
            z_far: 128.0.into(),
            update_mode: Default::default(),
            ambient_lighting_color: Color::repeat_opaque(120).into(),
            environment_lighting_source: Default::default(),
            need_update: true,
            updated: Cell::new(false),
            render_target: TextureResource::new_cube_render_target(DEFAULT_RESOLUTION as u32),
        }
    }
}

impl ReflectionProbe {
    /// Sets the desired resolution of the reflection probe.
    pub fn set_resolution(&mut self, resolution: usize) -> usize {
        let old = self.resolution.set_value_and_mark_modified(resolution);
        self.recreate_render_target();
        old
    }

    /// Returns current resolution of the reflection probe.
    pub fn resolution(&self) -> usize {
        *self.resolution
    }

    /// Returns current render target of the reflection probe.
    pub fn render_target(&self) -> &TextureResource {
        &self.render_target
    }

    /// Schedules update of the reflection probe for the next frame.
    pub fn force_update(&mut self) {
        self.need_update = true;
        self.updated.set(false);
    }

    /// Calculates position of the rendering point in global coordinates.
    pub fn global_rendering_position(&self) -> Vector3<f32> {
        self.global_position() + *self.rendering_position
    }

    fn recreate_render_target(&mut self) {
        self.render_target = TextureResource::new_cube_render_target(*self.resolution as u32);
        self.force_update();
    }

    fn on_visited(&mut self, visitor: &mut Visitor) {
        if visitor.is_reading() {
            self.recreate_render_target();
        }
    }
}

impl Deref for ReflectionProbe {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for ReflectionProbe {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl ConstructorProvider<Node, Graph> for ReflectionProbe {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_group("Light")
            .with_variant("Reflection Probe", |_| {
                ReflectionProbeBuilder::new(BaseBuilder::new().with_name("Reflection Probe"))
                    .build_node()
                    .into()
            })
    }
}

impl NodeTrait for ReflectionProbe {
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn update(&mut self, _context: &mut UpdateContext) {
        match *self.update_mode {
            UpdateMode::Once => {
                if self.need_update {
                    self.updated.set(false);
                    self.need_update = false;
                }
            }
            UpdateMode::EachFrame => {
                self.updated.set(false);
            }
        }
    }
}

/// Allows you to create a reflection probe node declaratively.
pub struct ReflectionProbeBuilder {
    base_builder: BaseBuilder,
    offset: Vector3<f32>,
    z_near: f32,
    z_far: f32,
    resolution: usize,
    update_mode: UpdateMode,
    ambient_lighting_color: Color,
    environment_lighting_source: EnvironmentLightingSource,
}

impl ReflectionProbeBuilder {
    /// Creates a new reflection probe builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            offset: Default::default(),
            z_near: 0.1,
            z_far: 32.0,
            resolution: DEFAULT_RESOLUTION,
            update_mode: Default::default(),
            ambient_lighting_color: Color::repeat_opaque(120),
            environment_lighting_source: Default::default(),
        }
    }

    /// Sets the desired offset of the reflection probe.
    pub fn with_rendering_local_position(mut self, offset: Vector3<f32>) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the desired position of the near clipping plane.
    pub fn with_z_near(mut self, z_near: f32) -> Self {
        self.z_near = z_near;
        self
    }

    /// Sets the desired position of the far clipping plane.
    pub fn with_z_far(mut self, z_far: f32) -> Self {
        self.z_far = z_far;
        self
    }

    /// Sets the desired resolution of the probe. See [`ReflectionProbe`] docs for more info.
    pub fn with_resolution(mut self, resolution: usize) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets the desired update mode of the probe. See [`UpdateMode`] docs for more info.
    pub fn with_update_mode(mut self, mode: UpdateMode) -> Self {
        self.update_mode = mode;
        self
    }

    /// Environment lighting source of the reflection probe.
    pub fn with_environment(
        mut self,
        environment_lighting_source: EnvironmentLightingSource,
    ) -> Self {
        self.environment_lighting_source = environment_lighting_source;
        self
    }

    /// Sets the ambient lighting color of the reflection probe.
    pub fn with_ambient_lighting_color(mut self, ambient_lighting_color: Color) -> Self {
        self.ambient_lighting_color = ambient_lighting_color;
        self
    }

    /// Creates a new reflection probe node.
    pub fn build_node(self) -> Node {
        Node::new(ReflectionProbe {
            base: self.base_builder.build_base(),
            rendering_position: self.offset.into(),
            resolution: self.resolution.into(),
            z_near: self.z_near.into(),
            z_far: self.z_far.into(),
            update_mode: self.update_mode.into(),
            ambient_lighting_color: self.ambient_lighting_color.into(),
            environment_lighting_source: self.environment_lighting_source.into(),
            need_update: true,
            updated: Cell::new(false),
            render_target: TextureResource::new_cube_render_target(self.resolution as u32),
        })
    }

    /// Creates a new reflection probe node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<ReflectionProbe> {
        graph.add_node(self.build_node()).to_variant()
    }
}
