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

use crate::{
    asset::untyped::ResourceKind,
    core::{
        algebra::Vector4,
        algebra::{Matrix4, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        math::{frustum::Frustum, Rect},
        pool::Handle,
        sstorage::ImmutableString,
    },
    graph::BaseSceneGraph,
    material::{shader::SamplerFallback, Material, MaterialResource, PropertyValue},
    renderer::{
        cache::{
            geometry::GeometryCache,
            shader::ShaderCache,
            texture::TextureCache,
            uniform::{UniformBlockLocation, UniformBufferCache, UniformMemoryAllocator},
            TimeToLive,
        },
        framework::{
            buffer::Buffer,
            error::FrameworkError,
            framebuffer::{FrameBuffer, ResourceBindGroup, ResourceBinding},
            gpu_program::{BuiltInUniform, BuiltInUniformBlock, GpuProgram},
            gpu_texture::GpuTexture,
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            ElementRange,
        },
        LightData, RenderPassStatistics, MAX_BONE_MATRICES,
    },
    resource::texture::TextureResource,
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
    pub bone_matrices_stub_uniform_buffer: &'a dyn Buffer,
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,

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
        server: &dyn GraphicsServer,
        material: &Material,
        program: &dyn GpuProgram,
        blend_shapes_storage: Option<TextureResource>,
        material_bindings: &mut ArrayVec<ResourceBinding, 32>,
    ) -> Result<(), FrameworkError> {
        let built_in_uniforms = program.built_in_uniform_locations();

        // Collect texture bindings.
        if let Some(location) = &built_in_uniforms[BuiltInUniform::SceneDepth as usize] {
            if let Some(scene_depth) = self.scene_depth.as_ref() {
                material_bindings.push(ResourceBinding::texture(scene_depth, location));
            }
        }

        if let Some(location) = &built_in_uniforms[BuiltInUniform::BlendShapesStorage as usize] {
            if let Some(texture) = blend_shapes_storage
                .as_ref()
                .and_then(|blend_shapes_storage| {
                    self.texture_cache.get(server, blend_shapes_storage)
                })
            {
                material_bindings.push(ResourceBinding::texture(texture, location));
            } else {
                material_bindings.push(ResourceBinding::texture(self.volume_dummy, location));
            }
        }

        let shader = material.shader().data_ref();
        for property in shader.definition.properties.iter() {
            if let Some(PropertyValue::Sampler { value, fallback }) =
                material.properties().get(&property.name)
            {
                let texture = value
                    .as_ref()
                    .and_then(|t| self.texture_cache.get(server, t))
                    .unwrap_or(match fallback {
                        SamplerFallback::White => self.white_dummy,
                        SamplerFallback::Normal => self.normal_dummy,
                        SamplerFallback::Black => self.black_dummy,
                    });

                if let Ok(uniform) = program.uniform_location(&property.name) {
                    material_bindings.push(ResourceBinding::texture(texture, &uniform));
                }
            }
        }

        Ok(())
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

/// Describes where to the actual uniform data is located in the memory backed by the uniform
/// memory allocator on per-instance basis.
pub struct InstanceUniformData {
    /// Instance info block location.
    pub instance_block: UniformBlockLocation,
    /// Bone matrices block location. Could be [`None`], if there's no bone matrices.
    pub bone_matrices_block: Option<UniformBlockLocation>,
}

/// Describes where to the actual uniform data is located in the memory backed by the uniform
/// memory allocator on per-bundle basis.
pub struct BundleUniformData {
    /// Material info block location. Optional, because material properties could be empty.
    pub material_block: Option<UniformBlockLocation>,
    /// Camera info block location.
    pub camera_block: UniformBlockLocation,
    /// Lights info block location.
    pub lights_block: Option<UniformBlockLocation>,
    /// Light source data info block location.
    pub light_data_block: UniformBlockLocation,
    /// Graphics settings block location.
    pub graphics_settings_block: UniformBlockLocation,
    /// Block locations for each instance in a bundle.
    pub instance_blocks: Vec<InstanceUniformData>,
}

impl RenderDataBundle {
    /// Writes all the required uniform data of the bundle to uniform memory allocator.
    pub fn write_uniforms(
        &self,
        render_context: &mut BundleRenderContext,
    ) -> Option<BundleUniformData> {
        let mut material_state = self.material.state();

        let material = material_state.data()?;

        // Upload material uniforms.
        let mut material_uniforms = StaticUniformBuffer::<16384>::new();
        let shader = material.shader().data_ref();
        for property in shader.definition.properties.iter() {
            if let Some(value) = material.properties().get(&property.name) {
                match value {
                    PropertyValue::Float(value) => material_uniforms.push(value),
                    PropertyValue::FloatArray(array) => material_uniforms.push_slice(array),
                    PropertyValue::Int(value) => material_uniforms.push(value),
                    PropertyValue::IntArray(array) => material_uniforms.push_slice(array),
                    PropertyValue::UInt(value) => material_uniforms.push(value),
                    PropertyValue::UIntArray(array) => material_uniforms.push_slice(array),
                    PropertyValue::Vector2(value) => material_uniforms.push(value),
                    PropertyValue::Vector2Array(array) => material_uniforms.push_slice(array),
                    PropertyValue::Vector3(value) => material_uniforms.push(value),
                    PropertyValue::Vector3Array(array) => material_uniforms.push_slice(array),
                    PropertyValue::Vector4(value) => material_uniforms.push(value),
                    PropertyValue::Vector4Array(array) => material_uniforms.push_slice(array),
                    PropertyValue::Matrix2(value) => material_uniforms.push(value),
                    PropertyValue::Matrix2Array(array) => material_uniforms.push_slice(array),
                    PropertyValue::Matrix3(value) => material_uniforms.push(value),
                    PropertyValue::Matrix3Array(array) => material_uniforms.push_slice(array),
                    PropertyValue::Matrix4(value) => material_uniforms.push(value),
                    PropertyValue::Matrix4Array(array) => material_uniforms.push_slice(array),
                    PropertyValue::Bool(value) => material_uniforms.push(value),
                    PropertyValue::Color(color) => material_uniforms.push(color),
                    PropertyValue::Sampler { .. } => &mut material_uniforms,
                };
            } else {
                // TODO: Fallback to shader's defaults.
            }
        }
        let material_block = if material_uniforms.is_empty() {
            None
        } else {
            Some(
                render_context
                    .uniform_memory_allocator
                    .allocate(material_uniforms),
            )
        };

        // Upload camera uniforms.
        let camera_uniforms = StaticUniformBuffer::<512>::new()
            .with(render_context.view_projection_matrix)
            .with(render_context.camera_position)
            .with(render_context.camera_up_vector)
            .with(render_context.camera_side_vector)
            .with(&render_context.z_near)
            .with(&render_context.z_far)
            .with(&(render_context.z_far - render_context.z_near));
        let camera_block = render_context
            .uniform_memory_allocator
            .allocate(camera_uniforms);

        let light_data = StaticUniformBuffer::<256>::new()
            .with(render_context.light_position)
            .with(&render_context.ambient_light.as_frgba());
        let light_data_block = render_context.uniform_memory_allocator.allocate(light_data);

        let graphics_settings = StaticUniformBuffer::<256>::new().with(&render_context.use_pom);
        let graphics_settings_block = render_context
            .uniform_memory_allocator
            .allocate(graphics_settings);

        let lights_block = if let Some(light_data) = render_context.light_data {
            let lights_data = StaticUniformBuffer::<2048>::new()
                .with(&(light_data.count as i32))
                .with_slice(&light_data.color_radius)
                .with_slice(&light_data.parameters)
                .with_slice(&light_data.position)
                .with_slice(&light_data.direction);
            Some(
                render_context
                    .uniform_memory_allocator
                    .allocate(lights_data),
            )
        } else {
            None
        };

        // Upload instance uniforms.
        let mut instance_blocks = Vec::with_capacity(self.instances.len());
        for instance in self.instances.iter() {
            let mut blend_shapes_weights = [Vector4::new(0.0, 0.0, 0.0, 0.0); 32];
            // SAFETY: This is safe to copy PODs from one array to another with type erasure.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    instance.blend_shapes_weights.as_ptr(),
                    blend_shapes_weights.as_mut_ptr() as *mut _,
                    // Copy at max the amount of blend shape weights supported by the shader.
                    instance
                        .blend_shapes_weights
                        .len()
                        .min(blend_shapes_weights.len() * 4),
                );
            }
            let instance_buffer = StaticUniformBuffer::<1024>::new()
                .with(&instance.world_transform)
                .with(&(render_context.view_projection_matrix * instance.world_transform))
                .with(&(instance.blend_shapes_weights.len() as i32))
                .with(&(!instance.bone_matrices.is_empty()))
                .with_slice(&blend_shapes_weights);

            let mut instance_uniform_data = InstanceUniformData {
                instance_block: render_context
                    .uniform_memory_allocator
                    .allocate(instance_buffer),
                bone_matrices_block: None,
            };

            if !instance.bone_matrices.is_empty() {
                const INIT: Matrix4<f32> = Matrix4::new(
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                );
                let mut matrices = [INIT; MAX_BONE_MATRICES];
                const SIZE: usize = MAX_BONE_MATRICES * size_of::<Matrix4<f32>>();
                matrices[0..instance.bone_matrices.len()].copy_from_slice(&instance.bone_matrices);

                let bone_matrices_block = render_context
                    .uniform_memory_allocator
                    .allocate(StaticUniformBuffer::<SIZE>::new().with_slice(&matrices));
                instance_uniform_data.bone_matrices_block = Some(bone_matrices_block);
            }

            instance_blocks.push(instance_uniform_data);
        }

        Some(BundleUniformData {
            material_block,
            camera_block,
            lights_block,
            light_data_block,
            graphics_settings_block,
            instance_blocks,
        })
    }

    /// Draws the entire bundle to the specified frame buffer with the specified rendering environment.
    pub fn render_to_frame_buffer<F>(
        &self,
        server: &dyn GraphicsServer,
        geometry_cache: &mut GeometryCache,
        shader_cache: &mut ShaderCache,
        instance_filter: &mut F,
        render_context: &mut BundleRenderContext,
        bundle_uniform_data: BundleUniformData,
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

        let mut material_bindings = ArrayVec::<ResourceBinding, 32>::new();
        render_context.apply_material(
            server,
            material,
            &*render_pass.program,
            blend_shapes_storage,
            &mut material_bindings,
        )?;

        let block_locations = render_pass.program.built_in_uniform_blocks();

        if let Some(location) = &block_locations[BuiltInUniformBlock::MaterialProperties as usize] {
            if let Some(material_block) = bundle_uniform_data.material_block {
                material_bindings.push(
                    render_context
                        .uniform_memory_allocator
                        .block_to_binding(material_block, *location),
                );
            }
        }

        if let Some(location) = &block_locations[BuiltInUniformBlock::CameraData as usize] {
            material_bindings.push(
                render_context
                    .uniform_memory_allocator
                    .block_to_binding(bundle_uniform_data.camera_block, *location),
            );
        }

        if let Some(location) = &block_locations[BuiltInUniformBlock::LightData as usize] {
            material_bindings.push(
                render_context
                    .uniform_memory_allocator
                    .block_to_binding(bundle_uniform_data.light_data_block, *location),
            );
        }

        if let Some(location) = &block_locations[BuiltInUniformBlock::GraphicsSettings as usize] {
            material_bindings.push(
                render_context
                    .uniform_memory_allocator
                    .block_to_binding(bundle_uniform_data.graphics_settings_block, *location),
            );
        }

        if let Some(location) = &block_locations[BuiltInUniformBlock::LightsBlock as usize] {
            if let Some(lights_block) = bundle_uniform_data.lights_block {
                material_bindings.push(
                    render_context
                        .uniform_memory_allocator
                        .block_to_binding(lights_block, *location),
                );
            }
        }

        for (instance, uniform_data) in self
            .instances
            .iter()
            .zip(bundle_uniform_data.instance_blocks)
        {
            if !instance_filter(instance) {
                continue;
            }
            let mut instance_bindings = ArrayVec::<ResourceBinding, 32>::new();

            if let Some(location) = &block_locations[BuiltInUniformBlock::BoneMatrices as usize] {
                match uniform_data.bone_matrices_block {
                    Some(block) => {
                        instance_bindings.push(
                            render_context
                                .uniform_memory_allocator
                                .block_to_binding(block, *location),
                        );
                    }
                    None => {
                        // Bind stub buffer, instead of creating and uploading 16kb with zeros per draw
                        // call.
                        instance_bindings.push(ResourceBinding::Buffer {
                            buffer: render_context.bone_matrices_stub_uniform_buffer,
                            shader_location: *location,
                            data_usage: Default::default(),
                        });
                    }
                }
            }

            if let Some(location) = &block_locations[BuiltInUniformBlock::InstanceData as usize] {
                instance_bindings.push(
                    render_context
                        .uniform_memory_allocator
                        .block_to_binding(uniform_data.instance_block, *location),
                );
            }

            stats += render_context.frame_buffer.draw(
                geometry,
                render_context.viewport,
                &*render_pass.program,
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

    /// Draws the entire bundle set to the specified frame buffer with the specified rendering environment.
    pub fn render_to_frame_buffer<BundleFilter, InstanceFilter>(
        &self,
        server: &dyn GraphicsServer,
        geometry_cache: &mut GeometryCache,
        shader_cache: &mut ShaderCache,
        mut bundle_filter: BundleFilter,
        mut instance_filter: InstanceFilter,
        mut render_context: BundleRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError>
    where
        BundleFilter: FnMut(&RenderDataBundle) -> bool,
        InstanceFilter: FnMut(&SurfaceInstanceData) -> bool,
    {
        let mut bundle_uniform_data_set = Vec::with_capacity(self.bundles.len());
        for bundle in self.bundles.iter() {
            if !bundle_filter(bundle) {
                continue;
            }
            bundle_uniform_data_set.push(bundle.write_uniforms(&mut render_context));
        }
        render_context.uniform_memory_allocator.upload(server)?;

        let mut stats = RenderPassStatistics::default();
        for (bundle, bundle_uniform_data) in self
            .bundles
            .iter()
            .filter(|bundle| bundle_filter(bundle))
            .zip(bundle_uniform_data_set)
        {
            if let Some(bundle_uniform_data) = bundle_uniform_data {
                stats += bundle.render_to_frame_buffer(
                    server,
                    geometry_cache,
                    shader_cache,
                    &mut instance_filter,
                    &mut render_context,
                    bundle_uniform_data,
                )?
            }
        }
        Ok(stats)
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
