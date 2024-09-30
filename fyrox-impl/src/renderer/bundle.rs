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

//! The module responsible for bundle generation for rendering optimizations.

use crate::resource::texture::TextureResource;
use crate::{
    asset::untyped::ResourceKind,
    core::{
        algebra::{Matrix4, Vector3},
        color::Color,
        math::{frustum::Frustum, Rect},
        pool::Handle,
        sstorage::ImmutableString,
    },
    graph::BaseSceneGraph,
    material::{shader::SamplerFallback, Material, MaterialResource, PropertyValue},
    renderer::{
        cache::{
            geometry::GeometryCache, shader::ShaderCache, texture::TextureCache,
            uniform::UniformBufferCache, TimeToLive,
        },
        framework::{
            error::FrameworkError,
            framebuffer::FrameBuffer,
            gpu_program::{BuiltInUniform, BuiltInUniformBlock, GpuProgramBinding},
            gpu_texture::GpuTexture,
            state::GlGraphicsServer,
            uniform::StaticUniformBuffer,
            ElementRange,
        },
        LightData, RenderPassStatistics,
    },
    scene::{
        graph::Graph,
        mesh::{
            buffer::{
                BytesStorage, TriangleBuffer, TriangleBufferRefMut, VertexAttributeDescriptor,
                VertexBuffer, VertexBufferRefMut,
            },
            surface::{SurfaceData, SurfaceResource},
            RenderPath,
        },
        node::{Node, RdcControlFlow},
    },
};
use fxhash::{FxBuildHasher, FxHashMap, FxHasher};
use fyrox_core::arrayvec::ArrayVec;
use fyrox_graphics::framebuffer::{ResourceBindGroup, ResourceBinding};
use std::{
    cell::RefCell,
    collections::hash_map::DefaultHasher,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    rc::Rc,
};

/// Observer info contains all the data, that describes an observer. It could be a real camera, light source's
/// "virtual camera" that is used for shadow mapping, etc.
pub struct ObserverInfo {
    /// World-space position of the observer.
    pub observer_position: Vector3<f32>,
    /// Location of the near clipping plane.
    pub z_near: f32,
    /// Location of the far clipping plane.
    pub z_far: f32,
    /// View matrix of the observer.
    pub view_matrix: Matrix4<f32>,
    /// Projection matrix of the observer.
    pub projection_matrix: Matrix4<f32>,
}

/// Render context is used to collect render data from the scene nodes. It provides all required information about
/// the observer (camera, light source virtual camera, etc.), that could be used for culling.
pub struct RenderContext<'a> {
    /// World-space position of the observer.
    pub observer_position: &'a Vector3<f32>,
    /// Location of the near clipping plane.
    pub z_near: f32,
    /// Location of the far clipping plane.
    pub z_far: f32,
    /// View matrix of the observer.
    pub view_matrix: &'a Matrix4<f32>,
    /// Projection matrix of the observer.
    pub projection_matrix: &'a Matrix4<f32>,
    /// Frustum of the observer, it is built using observer's view and projection matrix. Use the frustum to do
    /// frustum culling.
    pub frustum: Option<&'a Frustum>,
    /// Render data bundle storage. Your scene node must write at least one surface instance here for the node to
    /// be rendered.
    pub storage: &'a mut dyn RenderDataBundleStorageTrait,
    /// A reference to the graph that is being rendered. Allows you to get access to other scene nodes to do
    /// some useful job.
    pub graph: &'a Graph,
    /// A name of the render pass for which the context was created for.
    pub render_pass_name: &'a ImmutableString,
}

impl<'a> RenderContext<'a> {
    /// Calculates sorting index using of the given point by transforming it in the view space and
    /// using Z coordinate. This index could be used for back-to-front sorting to prevent blending
    /// issues.
    pub fn calculate_sorting_index(&self, global_position: Vector3<f32>) -> u64 {
        let granularity = 1000.0;
        u64::MAX
            - (self
                .view_matrix
                .transform_point(&(global_position.into()))
                .z
                * granularity) as u64
    }
}

#[allow(missing_docs)] // TODO
pub struct BundleRenderContext<'a> {
    pub texture_cache: &'a mut TextureCache,
    pub render_pass_name: &'a ImmutableString,
    pub frame_buffer: &'a mut dyn FrameBuffer,
    pub viewport: Rect<i32>,
    pub uniform_buffer_cache: &'a mut UniformBufferCache,

    // Built-in uniforms.
    pub view_projection_matrix: &'a Matrix4<f32>,
    pub use_pom: bool,
    pub light_position: &'a Vector3<f32>,
    pub light_data: Option<&'a LightData>,
    pub ambient_light: Color,
    // TODO: Add depth pre-pass to remove Option here. Current architecture allows only forward
    // renderer to have access to depth buffer that is available from G-Buffer.
    pub scene_depth: Option<&'a Rc<RefCell<dyn GpuTexture>>>,

    pub camera_position: &'a Vector3<f32>,
    pub camera_up_vector: &'a Vector3<f32>,
    pub camera_side_vector: &'a Vector3<f32>,
    pub z_near: f32,
    pub z_far: f32,

    // Fallback textures.
    pub normal_dummy: &'a Rc<RefCell<dyn GpuTexture>>,
    pub white_dummy: &'a Rc<RefCell<dyn GpuTexture>>,
    pub black_dummy: &'a Rc<RefCell<dyn GpuTexture>>,
    pub volume_dummy: &'a Rc<RefCell<dyn GpuTexture>>,
}

impl<'a> BundleRenderContext<'a> {
    #[allow(missing_docs)] // TODO
    pub fn apply_material(
        &mut self,
        material: &Material,
        program_binding: &mut GpuProgramBinding,
        blend_shapes_storage: Option<TextureResource>,
        material_bindings: &mut ArrayVec<ResourceBinding, 32>,
    ) {
        let built_in_uniforms = &program_binding.program.built_in_uniform_locations;

        if let Some(location) = &built_in_uniforms[BuiltInUniform::SceneDepth as usize] {
            if let Some(scene_depth) = self.scene_depth.as_ref() {
                material_bindings.push(ResourceBinding::texture(scene_depth, location));
            }
        }

        if let Some(location) = &built_in_uniforms[BuiltInUniform::BlendShapesStorage as usize] {
            if let Some(texture) = blend_shapes_storage
                .as_ref()
                .and_then(|blend_shapes_storage| {
                    self.texture_cache
                        .get(program_binding.state, blend_shapes_storage)
                })
            {
                material_bindings.push(ResourceBinding::texture(texture, location));
            } else {
                material_bindings.push(ResourceBinding::texture(self.volume_dummy, location));
            }
        }

        // Apply values for built-in uniforms.
        if let Some(location) = &built_in_uniforms[BuiltInUniform::ViewProjectionMatrix as usize] {
            program_binding.set_matrix4(location, self.view_projection_matrix);
        }
        if let Some(location) = &built_in_uniforms[BuiltInUniform::CameraPosition as usize] {
            program_binding.set_vector3(location, self.camera_position);
        }

        if let Some(location) = &built_in_uniforms[BuiltInUniform::CameraUpVector as usize] {
            program_binding.set_vector3(location, self.camera_up_vector);
        }
        if let Some(location) = &built_in_uniforms[BuiltInUniform::CameraSideVector as usize] {
            program_binding.set_vector3(location, self.camera_side_vector);
        }
        if let Some(location) = &built_in_uniforms[BuiltInUniform::ZNear as usize] {
            program_binding.set_f32(location, self.z_near);
        }
        if let Some(location) = &built_in_uniforms[BuiltInUniform::ZFar as usize] {
            program_binding.set_f32(location, self.z_far);
        }

        if let Some(location) = &built_in_uniforms[BuiltInUniform::UsePOM as usize] {
            program_binding.set_bool(location, self.use_pom);
        }
        if let Some(location) = &built_in_uniforms[BuiltInUniform::LightPosition as usize] {
            program_binding.set_vector3(location, self.light_position);
        }

        if let Some(light_data) = self.light_data {
            if let Some(location) = &built_in_uniforms[BuiltInUniform::LightCount as usize] {
                program_binding.set_i32(location, light_data.count as i32);
            }

            if let Some(location) = &built_in_uniforms[BuiltInUniform::LightsColorRadius as usize] {
                program_binding.set_vector4_slice(location, &light_data.color_radius);
            }

            if let Some(location) = &built_in_uniforms[BuiltInUniform::LightsPosition as usize] {
                program_binding.set_vector3_slice(location, &light_data.position);
            }

            if let Some(location) = &built_in_uniforms[BuiltInUniform::LightsDirection as usize] {
                program_binding.set_vector3_slice(location, &light_data.direction);
            }

            if let Some(location) = &built_in_uniforms[BuiltInUniform::LightsParameters as usize] {
                program_binding.set_vector2_slice(location, &light_data.parameters);
            }
        }

        if let Some(location) = &built_in_uniforms[BuiltInUniform::AmbientLight as usize] {
            program_binding.set_srgb_color(location, &self.ambient_light);
        }

        // Apply material properties.
        for (name, value) in material.properties() {
            if let Some(uniform) = program_binding.uniform_location(name) {
                match value {
                    PropertyValue::Float(v) => {
                        program_binding.set_f32(&uniform, *v);
                    }
                    PropertyValue::Int(v) => {
                        program_binding.set_i32(&uniform, *v);
                    }
                    PropertyValue::UInt(v) => {
                        program_binding.set_u32(&uniform, *v);
                    }
                    PropertyValue::Vector2(v) => {
                        program_binding.set_vector2(&uniform, v);
                    }
                    PropertyValue::Vector3(v) => {
                        program_binding.set_vector3(&uniform, v);
                    }
                    PropertyValue::Vector4(v) => {
                        program_binding.set_vector4(&uniform, v);
                    }
                    PropertyValue::Matrix2(v) => {
                        program_binding.set_matrix2(&uniform, v);
                    }
                    PropertyValue::Matrix3(v) => {
                        program_binding.set_matrix3(&uniform, v);
                    }
                    PropertyValue::Matrix4(v) => {
                        program_binding.set_matrix4(&uniform, v);
                    }
                    PropertyValue::Color(v) => {
                        program_binding.set_srgb_color(&uniform, v);
                    }
                    PropertyValue::Bool(v) => {
                        program_binding.set_bool(&uniform, *v);
                    }
                    PropertyValue::Sampler { value, fallback } => {
                        let texture = value
                            .as_ref()
                            .and_then(|t| self.texture_cache.get(program_binding.state, t))
                            .unwrap_or(match fallback {
                                SamplerFallback::White => self.white_dummy,
                                SamplerFallback::Normal => self.normal_dummy,
                                SamplerFallback::Black => self.black_dummy,
                            });

                        material_bindings.push(ResourceBinding::texture(texture, &uniform));
                    }
                    PropertyValue::FloatArray(v) => {
                        program_binding.set_f32_slice(&uniform, v);
                    }
                    PropertyValue::IntArray(v) => {
                        program_binding.set_i32_slice(&uniform, v);
                    }
                    PropertyValue::UIntArray(v) => {
                        program_binding.set_u32_slice(&uniform, v);
                    }
                    PropertyValue::Vector2Array(v) => {
                        program_binding.set_vector2_slice(&uniform, v);
                    }
                    PropertyValue::Vector3Array(v) => {
                        program_binding.set_vector3_slice(&uniform, v);
                    }
                    PropertyValue::Vector4Array(v) => {
                        program_binding.set_vector4_slice(&uniform, v);
                    }
                    PropertyValue::Matrix2Array(v) => {
                        program_binding.set_matrix2_array(&uniform, v);
                    }
                    PropertyValue::Matrix3Array(v) => {
                        program_binding.set_matrix3_array(&uniform, v);
                    }
                    PropertyValue::Matrix4Array(v) => {
                        program_binding.set_matrix4_array(&uniform, v);
                    }
                }
            }
        }
    }
}

/// Persistent identifier marks drawing data, telling the renderer that the data is the same, no matter from which
/// render bundle it came from. It is used by the renderer to create associated GPU resources.
#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq)]
pub struct PersistentIdentifier(pub u64);

impl PersistentIdentifier {
    /// Creates a new persistent identifier using shared surface data, node handle and an arbitrary index.
    pub fn new_combined(
        surface_data: &SurfaceResource,
        handle: Handle<Node>,
        index: usize,
    ) -> Self {
        let mut hasher = DefaultHasher::new();
        handle.hash(&mut hasher);
        hasher.write_u64(surface_data.key());
        hasher.write_usize(index);
        Self(hasher.finish())
    }
}

/// A set of data of a surface for rendering.  
pub struct SurfaceInstanceData {
    /// A world matrix.
    pub world_transform: Matrix4<f32>,
    /// A set of bone matrices.
    pub bone_matrices: Vec<Matrix4<f32>>,
    /// A set of weights for each blend shape in the surface.
    pub blend_shapes_weights: Vec<f32>,
    /// A range of elements of the instance. Allows you to draw either the full range ([`ElementRange::Full`])
    /// of the graphics primitives from the surface data or just a part of it ([`ElementRange::Specific`]).
    pub element_range: ElementRange,
    /// Persistent identifier of the instance. In most cases it can be generated by [`PersistentIdentifier::new_combined`]
    /// method.
    pub persistent_identifier: PersistentIdentifier,
    /// A handle of a node that emitted this surface data. Could be none, if there's no info about scene node.
    pub node_handle: Handle<Node>,
}

/// A set of surface instances that share the same vertex/index data and a material.
pub struct RenderDataBundle {
    /// A pointer to shared surface data.
    pub data: SurfaceResource,
    /// Amount of time (in seconds) for GPU geometry buffer (vertex + index buffers) generated for
    /// the `data`.
    pub time_to_live: TimeToLive,
    /// A set of instances.
    pub instances: Vec<SurfaceInstanceData>,
    /// A material that is shared across all instances.
    pub material: MaterialResource,
    /// A render path of the bundle.
    pub render_path: RenderPath,
    sort_index: u64,
}

impl Debug for RenderDataBundle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bundle {}: {} instances",
            self.data.key(),
            self.instances.len()
        )
    }
}

impl RenderDataBundle {
    /// Draws the entire bundle to the specified frame buffer with the specified rendering environment.
    pub fn render_to_frame_buffer<F>(
        &self,
        server: &GlGraphicsServer,
        geometry_cache: &mut GeometryCache,
        shader_cache: &mut ShaderCache,
        mut instance_filter: F,
        mut render_context: BundleRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError>
    where
        F: FnMut(&SurfaceInstanceData) -> bool,
    {
        let mut stats = RenderPassStatistics::default();

        let mut material_state = self.material.state();

        let Some(material) = material_state.data() else {
            return Ok(stats);
        };

        let Some(geometry) = geometry_cache.get(server, &self.data, self.time_to_live) else {
            return Ok(stats);
        };

        let blend_shapes_storage = self
            .data
            .data_ref()
            .blend_shapes_container
            .as_ref()
            .and_then(|c| c.blend_shape_storage.clone());

        let Some(render_pass) =
            shader_cache
                .get(server, material.shader())
                .and_then(|shader_set| {
                    shader_set
                        .render_passes
                        .get(render_context.render_pass_name)
                })
        else {
            return Ok(stats);
        };

        let mut program_binding = render_pass.program.bind(server);
        let mut material_bindings = ArrayVec::<ResourceBinding, 32>::new();
        render_context.apply_material(
            material,
            &mut program_binding,
            blend_shapes_storage,
            &mut material_bindings,
        );

        for instance in self.instances.iter() {
            if !instance_filter(instance) {
                continue;
            }
            let mut instance_bindings = ArrayVec::<ResourceBinding, 32>::new();

            let built_in_uniform_blocks = &program_binding.program.built_in_uniform_blocks;

            if let Some(location) =
                &built_in_uniform_blocks[BuiltInUniformBlock::BoneMatrices as usize]
            {
                let mut uniform_buffer = StaticUniformBuffer::<16384>::new();
                uniform_buffer.push_slice(&instance.bone_matrices);
                let bytes = uniform_buffer.finish();
                let buffer = render_context
                    .uniform_buffer_cache
                    .get_or_create(program_binding.state, 16384)?;
                buffer.write_data(&bytes)?;
                instance_bindings.push(ResourceBinding::Buffer {
                    buffer,
                    shader_location: *location,
                })
            }

            if let Some(location) =
                &built_in_uniform_blocks[BuiltInUniformBlock::InstanceData as usize]
            {
                let mut uniform_buffer = StaticUniformBuffer::<4096>::new();
                uniform_buffer
                    .push(&instance.world_transform)
                    .push(&(render_context.view_projection_matrix * instance.world_transform))
                    .push(&(instance.blend_shapes_weights.len() as i32))
                    .push(&(!instance.bone_matrices.is_empty()))
                    .push_slice(&instance.blend_shapes_weights);
                let bytes = uniform_buffer.finish();
                let buffer = render_context
                    .uniform_buffer_cache
                    .get_or_create(program_binding.state, 4096)?;
                buffer.write_data(&bytes)?;
                instance_bindings.push(ResourceBinding::Buffer {
                    buffer,
                    shader_location: *location,
                })
            }

            stats += render_context.frame_buffer.draw(
                geometry,
                render_context.viewport,
                &render_pass.program,
                &render_pass.draw_params,
                &[
                    ResourceBindGroup {
                        bindings: &material_bindings,
                    },
                    ResourceBindGroup {
                        bindings: &instance_bindings,
                    },
                ],
                instance.element_range,
                &mut |_| {},
            )?;
        }

        Ok(stats)
    }
}

/// A trait for an entity that can collect render data.
pub trait RenderDataBundleStorageTrait {
    /// Adds a new mesh to the bundle storage using the given set of vertices and triangles. This
    /// method automatically creates a render bundle according to a hash of the following parameters:
    ///
    /// - Material
    /// - Vertex Type
    /// - Render Path
    ///
    /// If one of these parameters is different, then a new bundle will be created and used to store
    /// the given vertices and indices. If an appropriate bundle exists, the method will store
    /// the given vertices and the triangles in it.
    ///
    /// ## When to use
    ///
    /// This method is used to reduce amount of draw calls of underlying GAPI, by merging small
    /// portions of data into one big block that shares drawing parameters and can be rendered in
    /// a single draw call. The vertices in this case should be pre-processed by applying world
    /// transform to them. This is so-called dynamic batching.
    ///
    /// Do not use this method if you have a mesh with lots of vertices and triangles, because
    /// pre-processing them on CPU could take more time than rendering them directly on GPU one-by-one.
    fn push_triangles(
        &mut self,
        layout: &[VertexAttributeDescriptor],
        material: &MaterialResource,
        render_path: RenderPath,
        sort_index: u64,
        node_handle: Handle<Node>,
        func: &mut dyn FnMut(VertexBufferRefMut, TriangleBufferRefMut),
    );

    /// Adds a new surface instance to the storage. The method will automatically put the instance
    /// in the appropriate bundle. Bundle selection is done using the material, surface data, render
    /// path. If only one of these parameters is different, then the surface instance will be put
    /// in a separate bundle.
    fn push(
        &mut self,
        data: &SurfaceResource,
        material: &MaterialResource,
        render_path: RenderPath,
        sort_index: u64,
        instance_data: SurfaceInstanceData,
    );
}

/// Bundle storage handles bundle generation for a scene before rendering. It is used to optimize
/// rendering by reducing amount of state changes of OpenGL context.
#[derive(Default)]
pub struct RenderDataBundleStorage {
    bundle_map: FxHashMap<u64, usize>,
    /// A sorted list of bundles.
    pub bundles: Vec<RenderDataBundle>,
}

impl RenderDataBundleStorage {
    /// Creates a new render bundle storage from the given graph and observer info. It "asks" every node in the
    /// graph one-by-one to give render data which is then put in the storage, sorted and ready for rendering.
    /// Frustum culling is done on scene node side ([`crate::scene::node::NodeTrait::collect_render_data`]).
    pub fn from_graph(
        graph: &Graph,
        observer_info: ObserverInfo,
        render_pass_name: ImmutableString,
    ) -> Self {
        // Aim for the worst-case scenario when every node has unique render data.
        let capacity = graph.node_count() as usize;
        let mut storage = Self {
            bundle_map: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher::default()),
            bundles: Vec::with_capacity(capacity),
        };

        let mut lod_filter = vec![true; graph.capacity() as usize];
        for node in graph.linear_iter() {
            if let Some(lod_group) = node.lod_group() {
                for level in lod_group.levels.iter() {
                    for &object in level.objects.iter() {
                        if let Some(object_ref) = graph.try_get(object) {
                            let distance = observer_info
                                .observer_position
                                .metric_distance(&object_ref.global_position());
                            let z_range = observer_info.z_far - observer_info.z_near;
                            let normalized_distance = (distance - observer_info.z_near) / z_range;
                            let visible = normalized_distance >= level.begin()
                                && normalized_distance <= level.end();
                            lod_filter[object.index() as usize] = visible;
                        }
                    }
                }
            }
        }

        let frustum = Frustum::from_view_projection_matrix(
            observer_info.projection_matrix * observer_info.view_matrix,
        )
        .unwrap_or_default();

        let mut ctx = RenderContext {
            observer_position: &observer_info.observer_position,
            z_near: observer_info.z_near,
            z_far: observer_info.z_far,
            view_matrix: &observer_info.view_matrix,
            projection_matrix: &observer_info.projection_matrix,
            frustum: Some(&frustum),
            storage: &mut storage,
            graph,
            render_pass_name: &render_pass_name,
        };

        #[inline(always)]
        fn iterate_recursive(
            node_handle: Handle<Node>,
            graph: &Graph,
            lod_filter: &[bool],
            ctx: &mut RenderContext,
        ) {
            if lod_filter[node_handle.index() as usize] {
                let node = graph.node(node_handle);
                if let RdcControlFlow::Continue = node.collect_render_data(ctx) {
                    for child in node.children() {
                        iterate_recursive(*child, graph, lod_filter, ctx);
                    }
                }
            }
        }

        iterate_recursive(graph.root(), graph, &lod_filter, &mut ctx);

        storage.sort();

        storage
    }

    /// Sorts the bundles by their respective sort index.
    pub fn sort(&mut self) {
        self.bundles.sort_unstable_by_key(|b| b.sort_index);
    }
}

impl RenderDataBundleStorageTrait for RenderDataBundleStorage {
    /// Adds a new mesh to the bundle storage using the given set of vertices and triangles. This
    /// method automatically creates a render bundle according to a hash of the following parameters:
    ///
    /// - Material
    /// - Vertex Type
    /// - Render Path
    ///
    /// If one of these parameters is different, then a new bundle will be created and used to store
    /// the given vertices and indices. If an appropriate bundle exists, the method will store the
    /// given vertices and the triangles in it.
    ///
    /// ## When to use
    ///
    /// This method is used to reduce amount of draw calls of underlying GAPI, by merging small
    /// portions of data into one big block that shares drawing parameters and can be rendered in
    /// a single draw call. The vertices in this case should be pre-processed by applying world
    /// transform to them.
    ///
    /// Do not use this method if you have a mesh with lots of vertices and triangles, because
    /// pre-processing them on CPU could take more time than rendering them directly on GPU one-by-one.
    fn push_triangles(
        &mut self,
        layout: &[VertexAttributeDescriptor],
        material: &MaterialResource,
        render_path: RenderPath,
        sort_index: u64,
        node_handle: Handle<Node>,
        func: &mut dyn FnMut(VertexBufferRefMut, TriangleBufferRefMut),
    ) {
        let mut hasher = FxHasher::default();
        hasher.write_u64(material.key());
        layout.hash(&mut hasher);
        hasher.write_u32(render_path as u32);
        let key = hasher.finish();

        let bundle = if let Some(&bundle_index) = self.bundle_map.get(&key) {
            self.bundles.get_mut(bundle_index).unwrap()
        } else {
            let default_capacity = 4096;

            // Initialize empty vertex buffer.
            let vertex_buffer = VertexBuffer::new_with_layout(
                layout,
                0,
                BytesStorage::with_capacity(default_capacity),
            )
            .unwrap();

            // Initialize empty triangle buffer.
            let triangle_buffer = TriangleBuffer::new(Vec::with_capacity(default_capacity * 3));

            // Create temporary surface data (valid for one frame).
            let data = SurfaceResource::new_ok(
                ResourceKind::Embedded,
                SurfaceData::new(vertex_buffer, triangle_buffer),
            );

            self.bundle_map.insert(key, self.bundles.len());
            let persistent_identifier = PersistentIdentifier::new_combined(&data, node_handle, 0);
            self.bundles.push(RenderDataBundle {
                data,
                sort_index,
                instances: vec![
                    // Each bundle must have at least one instance to be rendered.
                    SurfaceInstanceData {
                        world_transform: Matrix4::identity(),
                        bone_matrices: Default::default(),
                        blend_shapes_weights: Default::default(),
                        element_range: Default::default(),
                        persistent_identifier,
                        node_handle,
                    },
                ],
                material: material.clone(),
                render_path,
                // Temporary buffer lives one frame.
                time_to_live: TimeToLive(0.0),
            });
            self.bundles.last_mut().unwrap()
        };

        let mut data = bundle.data.data_ref();
        let data = &mut *data;

        let vertex_buffer = data.vertex_buffer.modify();
        let triangle_buffer = data.geometry_buffer.modify();

        func(vertex_buffer, triangle_buffer);
    }

    /// Adds a new surface instance to the storage. The method will automatically put the instance in the appropriate
    /// bundle. Bundle selection is done using the material, surface data, render path. If only one
    /// of these parameters is different, then the surface instance will be put in a separate bundle.
    fn push(
        &mut self,
        data: &SurfaceResource,
        material: &MaterialResource,
        render_path: RenderPath,
        sort_index: u64,
        instance_data: SurfaceInstanceData,
    ) {
        let mut hasher = FxHasher::default();
        hasher.write_u64(material.key());
        hasher.write_u64(data.key());
        hasher.write_u32(render_path as u32);
        let key = hasher.finish();

        let bundle = if let Some(&bundle_index) = self.bundle_map.get(&key) {
            self.bundles.get_mut(bundle_index).unwrap()
        } else {
            self.bundle_map.insert(key, self.bundles.len());
            self.bundles.push(RenderDataBundle {
                data: data.clone(),
                sort_index,
                instances: Default::default(),
                material: material.clone(),
                render_path,
                time_to_live: Default::default(),
            });
            self.bundles.last_mut().unwrap()
        };

        bundle.instances.push(instance_data)
    }
}
