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

#![allow(missing_docs)] // TODO

use crate::{
    core::{
        algebra::{Matrix4, Vector3, Vector4},
        arrayvec::ArrayVec,
        color,
        color::Color,
        err_once,
        log::Log,
        math::{frustum::Frustum, Matrix4Ext, Rect},
        pool::Handle,
        sstorage::ImmutableString,
    },
    graph::BaseSceneGraph,
    material::{self, shader::ShaderDefinition, MaterialPropertyRef, MaterialResource},
    renderer::{
        cache::{
            geometry::GeometryCache,
            shader::ShaderCache,
            texture::TextureCache,
            uniform::{UniformBlockLocation, UniformMemoryAllocator},
            TimeToLive,
        },
        framework::{
            error::FrameworkError,
            framebuffer::{GpuFrameBuffer, ResourceBindGroup, ResourceBinding},
            gpu_program::{ShaderProperty, ShaderPropertyKind, ShaderResourceKind},
            gpu_texture::GpuTexture,
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            uniform::{ByteStorage, UniformBuffer},
            ElementRange,
        },
        DynamicSurfaceCache, FallbackResources, LightData, RenderPassStatistics,
    },
    resource::texture::TextureResource,
    scene::{
        graph::Graph,
        light::{
            directional::{CsmOptions, DirectionalLight},
            point::PointLight,
            spot::SpotLight,
            BaseLight,
        },
        mesh::{
            buffer::{TriangleBufferRefMut, VertexAttributeDescriptor, VertexBufferRefMut},
            surface::SurfaceResource,
            RenderPath,
        },
        node::{Node, RdcControlFlow},
    },
};
use fxhash::{FxBuildHasher, FxHashMap, FxHasher};
use fyrox_graph::{SceneGraph, SceneGraphNode};
use std::{
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
};

/// Observer info contains all the data, that describes an observer. It could be a real camera, light source's
/// "virtual camera" that is used for shadow mapping, etc.
#[derive(Clone, Default)]
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
    /// Amount of time (in seconds) that passed from creation of the engine. Keep in mind, that
    /// this value is **not** guaranteed to match real time. A user can change delta time with
    /// which the engine "ticks" and this delta time affects elapsed time.
    pub elapsed_time: f32,
    pub observer_info: &'a ObserverInfo,
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
    pub dynamic_surface_cache: &'a mut DynamicSurfaceCache,
}

impl RenderContext<'_> {
    /// Calculates sorting index using of the given point by transforming it in the view space and
    /// using Z coordinate. This index could be used for back-to-front sorting to prevent blending
    /// issues.
    pub fn calculate_sorting_index(&self, global_position: Vector3<f32>) -> u64 {
        let granularity = 1000.0;
        u64::MAX
            - (self
                .observer_info
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
    pub frame_buffer: &'a GpuFrameBuffer,
    pub viewport: Rect<i32>,
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,

    // Built-in uniforms.
    pub use_pom: bool,
    pub light_position: &'a Vector3<f32>,
    pub ambient_light: Color,
    // TODO: Add depth pre-pass to remove Option here. Current architecture allows only forward
    // renderer to have access to depth buffer that is available from G-Buffer.
    pub scene_depth: Option<&'a GpuTexture>,
    pub fallback_resources: &'a FallbackResources,
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
    /// A handle of a node that emitted this surface data. Could be none, if there's no info about scene node.
    pub node_handle: Handle<Node>,
}

impl Default for SurfaceInstanceData {
    fn default() -> Self {
        Self {
            world_transform: Matrix4::identity(),
            bone_matrices: Default::default(),
            blend_shapes_weights: Default::default(),
            element_range: Default::default(),
            node_handle: Default::default(),
        }
    }
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
    /// Material info block location.
    pub material_property_group_blocks: Vec<(usize, UniformBlockLocation)>,
    /// Lights info block location.
    pub light_data_block: UniformBlockLocation,
    /// Block locations for each instance in a bundle.
    pub instance_blocks: Vec<InstanceUniformData>,
}

pub struct GlobalUniformData {
    /// Camera info block location.
    pub camera_block: UniformBlockLocation,
    /// Light source data info block location.
    pub lights_block: UniformBlockLocation,
    /// Graphics settings block location.
    pub graphics_settings_block: UniformBlockLocation,
}

pub fn write_with_material<T, C, G>(
    shader_property_group: &[ShaderProperty],
    material_property_group: &C,
    getter: G,
    buf: &mut UniformBuffer<T>,
) where
    T: ByteStorage,
    G: for<'a> Fn(&'a C, &ImmutableString) -> Option<MaterialPropertyRef<'a>>,
{
    // The order of fields is strictly defined in shader, so we must iterate over shader definition
    // of a structure and look for respective values in the material.
    for shader_property in shader_property_group {
        let material_property = getter(material_property_group, &shader_property.name);

        macro_rules! push_value {
            ($variant:ident, $shader_value:ident) => {
                if let Some(property) = material_property {
                    if let MaterialPropertyRef::$variant(material_value) = property {
                        buf.push(material_value);
                    } else {
                        buf.push($shader_value);
                        Log::err(format!(
                            "Unable to use material property {} because of mismatching types.\
                            Expected {:?} got {:?}. Fallback to shader default value.",
                            shader_property.name, shader_property, property
                        ));
                    }
                } else {
                    buf.push($shader_value);
                }
            };
        }

        macro_rules! push_slice {
            ($variant:ident, $shader_value:ident, $max_size:ident) => {
                if let Some(property) = material_property {
                    if let MaterialPropertyRef::$variant(material_value) = property {
                        buf.push_slice_with_max_size(material_value, *$max_size);
                    } else {
                        buf.push_slice_with_max_size($shader_value, *$max_size);
                        Log::err(format!(
                            "Unable to use material property {} because of mismatching types.\
                            Expected {:?} got {:?}. Fallback to shader default value.",
                            shader_property.name, shader_property, property
                        ))
                    }
                } else {
                    buf.push_slice_with_max_size($shader_value, *$max_size);
                }
            };
        }

        use ShaderPropertyKind::*;
        match &shader_property.kind {
            Float { value } => push_value!(Float, value),
            FloatArray { value, max_len } => push_slice!(FloatArray, value, max_len),
            Int { value } => push_value!(Int, value),
            IntArray { value, max_len } => push_slice!(IntArray, value, max_len),
            UInt { value } => push_value!(UInt, value),
            UIntArray { value, max_len } => push_slice!(UIntArray, value, max_len),
            Vector2 { value } => push_value!(Vector2, value),
            Vector2Array { value, max_len } => push_slice!(Vector2Array, value, max_len),
            Vector3 { value } => push_value!(Vector3, value),
            Vector3Array { value, max_len } => push_slice!(Vector3Array, value, max_len),
            Vector4 { value: default } => push_value!(Vector4, default),
            Vector4Array { value, max_len } => push_slice!(Vector4Array, value, max_len),
            Matrix2 { value: default } => push_value!(Matrix2, default),
            Matrix2Array { value, max_len } => push_slice!(Matrix2Array, value, max_len),
            Matrix3 { value: default } => push_value!(Matrix3, default),
            Matrix3Array { value, max_len } => push_slice!(Matrix3Array, value, max_len),
            Matrix4 { value: default } => push_value!(Matrix4, default),
            Matrix4Array { value, max_len } => push_slice!(Matrix4Array, value, max_len),
            Bool { value } => push_value!(Bool, value),
            Color { r, g, b, a } => {
                let value = &color::Color::from_rgba(*r, *g, *b, *a);
                push_value!(Color, value)
            }
        };
    }
}

pub fn write_shader_values<T: ByteStorage>(
    shader_property_group: &[ShaderProperty],
    buf: &mut UniformBuffer<T>,
) {
    for property in shader_property_group {
        use ShaderPropertyKind::*;
        match &property.kind {
            Float { value } => buf.push(value),
            FloatArray { value, max_len } => buf.push_slice_with_max_size(value, *max_len),
            Int { value } => buf.push(value),
            IntArray { value, max_len } => buf.push_slice_with_max_size(value, *max_len),
            UInt { value } => buf.push(value),
            UIntArray { value, max_len } => buf.push_slice_with_max_size(value, *max_len),
            Vector2 { value } => buf.push(value),
            Vector2Array { value, max_len } => buf.push_slice_with_max_size(value, *max_len),
            Vector3 { value } => buf.push(value),
            Vector3Array { value, max_len } => buf.push_slice_with_max_size(value, *max_len),
            Vector4 { value: default } => buf.push(default),
            Vector4Array { value, max_len } => buf.push_slice_with_max_size(value, *max_len),
            Matrix2 { value: default } => buf.push(default),
            Matrix2Array { value, max_len } => buf.push_slice_with_max_size(value, *max_len),
            Matrix3 { value: default } => buf.push(default),
            Matrix3Array { value, max_len } => buf.push_slice_with_max_size(value, *max_len),
            Matrix4 { value: default } => buf.push(default),
            Matrix4Array { value, max_len } => buf.push_slice_with_max_size(value, *max_len),
            Bool { value } => buf.push(value),
            Color { r, g, b, a } => buf.push(&color::Color::from_rgba(*r, *g, *b, *a)),
        };
    }
}

impl RenderDataBundle {
    /// Writes all the required uniform data of the bundle to uniform memory allocator.
    pub fn write_uniforms(
        &self,
        view_projection_matrix: &Matrix4<f32>,
        render_context: &mut BundleRenderContext,
    ) -> Option<BundleUniformData> {
        let mut material_state = self.material.state();
        let material = material_state.data()?;

        // Upload material property groups.
        let mut material_property_group_blocks = Vec::new();
        let shader_state = material.shader().state();
        let shader = shader_state.data_ref()?;
        for resource_definition in shader.definition.resources.iter() {
            // Ignore built-in groups.
            if resource_definition.is_built_in() {
                continue;
            }

            let ShaderResourceKind::PropertyGroup(ref shader_property_group) =
                resource_definition.kind
            else {
                continue;
            };

            let mut buf = StaticUniformBuffer::<16384>::new();

            if let Some(material_property_group) =
                material.property_group_ref(resource_definition.name.clone())
            {
                write_with_material(
                    shader_property_group,
                    material_property_group,
                    |c, n| c.property_ref(n.clone()).map(|p| p.as_ref()),
                    &mut buf,
                );
            } else {
                // No respective resource bound in the material, use shader defaults. This is very
                // important, because some drivers will crash if uniform buffer has insufficient
                // data.
                write_shader_values(shader_property_group, &mut buf)
            }

            material_property_group_blocks.push((
                resource_definition.binding,
                render_context.uniform_memory_allocator.allocate(buf),
            ))
        }

        let light_data = StaticUniformBuffer::<256>::new()
            .with(render_context.light_position)
            .with(&render_context.ambient_light.as_frgba());
        let light_data_block = render_context.uniform_memory_allocator.allocate(light_data);

        // Upload instance uniforms.
        let mut instance_blocks = Vec::with_capacity(self.instances.len());
        for instance in self.instances.iter() {
            let mut packed_blend_shape_weights =
                [Vector4::<f32>::default(); ShaderDefinition::MAX_BLEND_SHAPE_WEIGHT_GROUPS];

            for (i, blend_shape_weight) in instance.blend_shapes_weights.iter().enumerate() {
                let n = i / 4;
                let c = i % 4;
                packed_blend_shape_weights[n][c] = *blend_shape_weight;
            }

            let instance_buffer = StaticUniformBuffer::<1024>::new()
                .with(&instance.world_transform)
                .with(&(view_projection_matrix * instance.world_transform))
                .with(&(instance.blend_shapes_weights.len() as i32))
                .with(&(!instance.bone_matrices.is_empty()))
                .with_slice_with_max_size(
                    &packed_blend_shape_weights,
                    ShaderDefinition::MAX_BLEND_SHAPE_WEIGHT_GROUPS,
                );

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
                let mut matrices = [INIT; ShaderDefinition::MAX_BONE_MATRICES];
                const SIZE: usize = ShaderDefinition::MAX_BONE_MATRICES * size_of::<Matrix4<f32>>();
                matrices[0..instance.bone_matrices.len()].copy_from_slice(&instance.bone_matrices);

                let bone_matrices_block = render_context
                    .uniform_memory_allocator
                    .allocate(StaticUniformBuffer::<SIZE>::new().with(&matrices));
                instance_uniform_data.bone_matrices_block = Some(bone_matrices_block);
            }

            instance_blocks.push(instance_uniform_data);
        }

        Some(BundleUniformData {
            material_property_group_blocks,
            light_data_block,
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
        global_uniform_data: &GlobalUniformData,
    ) -> Result<RenderPassStatistics, FrameworkError>
    where
        F: FnMut(&SurfaceInstanceData) -> bool,
    {
        let mut stats = RenderPassStatistics::default();

        let mut material_state = self.material.state();

        let Some(material) = material_state.data() else {
            err_once!(
                self.data.key() as usize,
                "Unable to use material {}, because it is in invalid state \
                (failed to load or still loading)!",
                material_state.kind()
            );
            return Ok(stats);
        };

        let geometry = match geometry_cache.get(server, &self.data, self.time_to_live) {
            Ok(geometry) => geometry,
            Err(err) => {
                err_once!(
                    self.data.key() as usize,
                    "Unable to get geometry for rendering! Reason: {err:?}"
                );
                return Ok(stats);
            }
        };

        let Some(shader_set) = shader_cache.get(server, material.shader()) else {
            err_once!(
                self.data.key() as usize,
                "Unable to get a compiled shader set for material {:?}!",
                material.shader().resource_uuid()
            );
            return Ok(stats);
        };

        let Some(render_pass) = shader_set
            .render_passes
            .get(render_context.render_pass_name)
        else {
            let shader_state = material.shader().state();
            if let Some(shader_data) = shader_state.data_ref() {
                if !shader_data
                    .definition
                    .disabled_passes
                    .iter()
                    .any(|pass_name| pass_name.as_str() == render_context.render_pass_name.as_str())
                {
                    err_once!(
                        self.data.key() as usize,
                        "There's no render pass {} in {} shader! \
                        If it is not needed, add it to disabled passes.",
                        render_context.render_pass_name,
                        shader_state.kind()
                    );
                }
            }
            return Ok(stats);
        };

        let mut material_bindings = ArrayVec::<ResourceBinding, 32>::new();
        let shader_state = material.shader().state();
        let shader = shader_state
            .data_ref()
            .ok_or_else(|| FrameworkError::Custom("Invalid shader!".to_string()))?;
        for resource_definition in shader.definition.resources.iter() {
            let name = resource_definition.name.as_str();

            match name {
                "fyrox_sceneDepth" => {
                    material_bindings.push(ResourceBinding::texture(
                        if let Some(scene_depth) = render_context.scene_depth.as_ref() {
                            scene_depth
                        } else {
                            &render_context.fallback_resources.black_dummy
                        },
                        &render_context.fallback_resources.nearest_clamp_sampler,
                        resource_definition.binding,
                    ));
                }
                "fyrox_cameraData" => {
                    material_bindings.push(
                        render_context.uniform_memory_allocator.block_to_binding(
                            global_uniform_data.camera_block,
                            resource_definition.binding,
                        ),
                    );
                }
                "fyrox_lightData" => {
                    material_bindings.push(
                        render_context.uniform_memory_allocator.block_to_binding(
                            bundle_uniform_data.light_data_block,
                            resource_definition.binding,
                        ),
                    );
                }
                "fyrox_graphicsSettings" => {
                    material_bindings.push(
                        render_context.uniform_memory_allocator.block_to_binding(
                            global_uniform_data.graphics_settings_block,
                            resource_definition.binding,
                        ),
                    );
                }
                "fyrox_lightsBlock" => {
                    material_bindings.push(
                        render_context.uniform_memory_allocator.block_to_binding(
                            global_uniform_data.lights_block,
                            resource_definition.binding,
                        ),
                    );
                }
                _ => match resource_definition.kind {
                    ShaderResourceKind::Texture { fallback, .. } => {
                        let fallback = render_context.fallback_resources.sampler_fallback(fallback);
                        let fallback = (
                            fallback,
                            &render_context.fallback_resources.linear_wrap_sampler,
                        );

                        let texture_sampler_pair = if let Some(binding) =
                            material.binding_ref(resource_definition.name.clone())
                        {
                            if let material::MaterialResourceBinding::Texture(binding) = binding {
                                binding
                                    .value
                                    .as_ref()
                                    .and_then(|t| {
                                        render_context
                                            .texture_cache
                                            .get(server, t)
                                            .map(|t| (&t.gpu_texture, &t.gpu_sampler))
                                    })
                                    .unwrap_or(fallback)
                            } else {
                                Log::err(format!(
                                    "Unable to use texture binding {}, types mismatch! Expected \
                                {:?} got {:?}",
                                    resource_definition.name, resource_definition.kind, binding
                                ));

                                fallback
                            }
                        } else {
                            fallback
                        };

                        material_bindings.push(ResourceBinding::texture(
                            texture_sampler_pair.0,
                            texture_sampler_pair.1,
                            resource_definition.binding,
                        ));
                    }
                    ShaderResourceKind::PropertyGroup(_) => {
                        // No validation here, it is done in uniform variables collection step.
                        if let Some((_, block_location)) = bundle_uniform_data
                            .material_property_group_blocks
                            .iter()
                            .find(|(binding, _)| *binding == resource_definition.binding)
                        {
                            material_bindings.push(
                                render_context
                                    .uniform_memory_allocator
                                    .block_to_binding(*block_location, resource_definition.binding),
                            );
                        }
                    }
                },
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

            for resource_definition in shader.definition.resources.iter() {
                let name = resource_definition.name.as_str();
                match name {
                    "fyrox_instanceData" => {
                        instance_bindings.push(
                            render_context.uniform_memory_allocator.block_to_binding(
                                uniform_data.instance_block,
                                resource_definition.binding,
                            ),
                        );
                    }
                    "fyrox_boneMatrices" => {
                        match uniform_data.bone_matrices_block {
                            Some(block) => {
                                instance_bindings.push(
                                    render_context
                                        .uniform_memory_allocator
                                        .block_to_binding(block, resource_definition.binding),
                                );
                            }
                            None => {
                                // Bind stub buffer, instead of creating and uploading 16kb with zeros per draw
                                // call.
                                instance_bindings.push(ResourceBinding::Buffer {
                                    buffer: render_context
                                        .fallback_resources
                                        .bone_matrices_stub_uniform_buffer
                                        .clone(),
                                    binding: resource_definition.binding,
                                    data_usage: Default::default(),
                                });
                            }
                        }
                    }
                    _ => (),
                };
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
        dynamic_surface_cache: &mut DynamicSurfaceCache,
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

pub enum LightSourceKind {
    Spot {
        full_cone_angle: f32,
        hotspot_cone_angle: f32,
        distance: f32,
        shadow_bias: f32,
        cookie_texture: Option<TextureResource>,
    },
    Point {
        radius: f32,
        shadow_bias: f32,
    },
    Directional {
        csm_options: CsmOptions,
    },
    Unknown,
}

pub struct LightSource {
    pub handle: Handle<Node>,
    pub global_transform: Matrix4<f32>,
    pub kind: LightSourceKind,
    pub position: Vector3<f32>,
    pub up_vector: Vector3<f32>,
    pub side_vector: Vector3<f32>,
    pub look_vector: Vector3<f32>,
    pub cast_shadows: bool,
    pub local_scale: Vector3<f32>,
    pub color: Color,
    pub intensity: f32,
    pub scatter_enabled: bool,
    pub scatter: Vector3<f32>,
}

/// Bundle storage handles bundle generation for a scene before rendering. It is used to optimize
/// rendering by reducing amount of state changes of OpenGL context.
pub struct RenderDataBundleStorage {
    bundle_map: FxHashMap<u64, usize>,
    pub observer_info: ObserverInfo,
    /// A sorted list of bundles.
    pub bundles: Vec<RenderDataBundle>,
    pub light_sources: Vec<LightSource>,
}

pub struct RenderDataBundleStorageOptions {
    pub collect_lights: bool,
}

impl Default for RenderDataBundleStorageOptions {
    fn default() -> Self {
        Self {
            collect_lights: true,
        }
    }
}

impl RenderDataBundleStorage {
    pub fn new_empty(observer_info: ObserverInfo) -> Self {
        Self {
            bundle_map: Default::default(),
            observer_info,
            bundles: Default::default(),
            light_sources: Default::default(),
        }
    }

    /// Creates a new render bundle storage from the given graph and observer info. It "asks" every node in the
    /// graph one-by-one to give render data which is then put in the storage, sorted and ready for rendering.
    /// Frustum culling is done on scene node side ([`crate::scene::node::NodeTrait::collect_render_data`]).
    pub fn from_graph(
        graph: &Graph,
        elapsed_time: f32,
        observer_info: ObserverInfo,
        render_pass_name: ImmutableString,
        options: RenderDataBundleStorageOptions,
        dynamic_surface_cache: &mut DynamicSurfaceCache,
    ) -> Self {
        // Aim for the worst-case scenario when every node has unique render data.
        let capacity = graph.node_count() as usize;
        let mut storage = Self {
            bundle_map: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher::default()),
            observer_info: observer_info.clone(),
            bundles: Vec::with_capacity(capacity),
            light_sources: Default::default(),
        };

        let frustum = Frustum::from_view_projection_matrix(
            observer_info.projection_matrix * observer_info.view_matrix,
        )
        .unwrap_or_default();

        let mut lod_filter = vec![true; graph.capacity() as usize];
        for (node_handle, node) in graph.pair_iter() {
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

            if options.collect_lights {
                if let Some(base_light) = node.component_ref::<BaseLight>() {
                    if frustum.is_intersects_aabb(&node.world_bounding_box())
                        && base_light.global_visibility()
                        && base_light.is_globally_enabled()
                    {
                        let kind = if let Some(spot_light) = node.cast::<SpotLight>() {
                            LightSourceKind::Spot {
                                full_cone_angle: spot_light.full_cone_angle(),
                                hotspot_cone_angle: spot_light.hotspot_cone_angle(),
                                distance: spot_light.distance(),
                                shadow_bias: spot_light.shadow_bias(),
                                cookie_texture: spot_light.cookie_texture(),
                            }
                        } else if let Some(point_light) = node.cast::<PointLight>() {
                            LightSourceKind::Point {
                                radius: point_light.radius(),
                                shadow_bias: point_light.shadow_bias(),
                            }
                        } else if let Some(directional_light) = node.cast::<DirectionalLight>() {
                            LightSourceKind::Directional {
                                csm_options: (*directional_light.csm_options).clone(),
                            }
                        } else {
                            LightSourceKind::Unknown
                        };

                        let source = LightSource {
                            handle: node_handle,
                            global_transform: base_light.global_transform(),
                            kind,
                            position: base_light.global_position(),
                            up_vector: base_light.up_vector(),
                            side_vector: base_light.side_vector(),
                            look_vector: base_light.look_vector(),
                            cast_shadows: base_light.cast_shadows(),
                            local_scale: **base_light.local_transform().scale(),
                            color: base_light.color(),
                            intensity: base_light.intensity(),
                            scatter_enabled: base_light.is_scatter_enabled(),
                            scatter: base_light.scatter(),
                        };

                        storage.light_sources.push(source);
                    }
                }
            }
        }

        let mut ctx = RenderContext {
            elapsed_time,
            observer_info: &observer_info,
            frustum: Some(&frustum),
            storage: &mut storage,
            graph,
            render_pass_name: &render_pass_name,
            dynamic_surface_cache,
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

    pub fn write_global_uniform_blocks(
        &self,
        render_context: &mut BundleRenderContext,
    ) -> GlobalUniformData {
        let mut light_data = LightData::<{ ShaderDefinition::MAX_LIGHTS }>::default();

        for (i, light) in self
            .light_sources
            .iter()
            .enumerate()
            .take(ShaderDefinition::MAX_LIGHTS)
        {
            let color = light.color.as_frgb();

            light_data.color_radius[i] = Vector4::new(color.x, color.y, color.z, 0.0);
            light_data.position[i] = light.position;
            light_data.direction[i] = light.up_vector;

            match light.kind {
                LightSourceKind::Spot {
                    full_cone_angle,
                    hotspot_cone_angle,
                    distance,
                    ..
                } => {
                    light_data.color_radius[i].w = distance;
                    light_data.parameters[i].x = (hotspot_cone_angle * 0.5).cos();
                    light_data.parameters[i].y = (full_cone_angle * 0.5).cos();
                }
                LightSourceKind::Point { radius, .. } => {
                    light_data.color_radius[i].w = radius;
                    light_data.parameters[i].x = std::f32::consts::PI.cos();
                    light_data.parameters[i].y = std::f32::consts::PI.cos();
                }
                LightSourceKind::Directional { .. } => {
                    light_data.color_radius[i].w = f32::INFINITY;
                    light_data.parameters[i].x = std::f32::consts::PI.cos();
                    light_data.parameters[i].y = std::f32::consts::PI.cos();
                }
                LightSourceKind::Unknown => {}
            }

            light_data.count += 1;
        }

        let lights_data = StaticUniformBuffer::<2048>::new()
            .with(&(light_data.count as i32))
            .with(&light_data.color_radius)
            .with(&light_data.parameters)
            .with(&light_data.position)
            .with(&light_data.direction);
        let lights_block = render_context
            .uniform_memory_allocator
            .allocate(lights_data);

        // Upload camera uniforms.
        let inv_view = self
            .observer_info
            .view_matrix
            .try_inverse()
            .unwrap_or_default();
        let view_projection = self.observer_info.projection_matrix * self.observer_info.view_matrix;
        let camera_up = inv_view.up();
        let camera_side = inv_view.side();
        let camera_uniforms = StaticUniformBuffer::<512>::new()
            .with(&view_projection)
            .with(&self.observer_info.observer_position)
            .with(&camera_up)
            .with(&camera_side)
            .with(&self.observer_info.z_near)
            .with(&self.observer_info.z_far)
            .with(&(self.observer_info.z_far - self.observer_info.z_near));
        let camera_block = render_context
            .uniform_memory_allocator
            .allocate(camera_uniforms);

        let graphics_settings = StaticUniformBuffer::<256>::new().with(&render_context.use_pom);
        let graphics_settings_block = render_context
            .uniform_memory_allocator
            .allocate(graphics_settings);

        GlobalUniformData {
            camera_block,
            lights_block,
            graphics_settings_block,
        }
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
        let global_uniforms = self.write_global_uniform_blocks(&mut render_context);

        let view_projection = self.observer_info.projection_matrix * self.observer_info.view_matrix;
        let mut bundle_uniform_data_set = Vec::with_capacity(self.bundles.len());
        for bundle in self.bundles.iter() {
            if !bundle_filter(bundle) {
                continue;
            }
            bundle_uniform_data_set
                .push(bundle.write_uniforms(&view_projection, &mut render_context));
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
                    &global_uniforms,
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
        dynamic_surface_cache: &mut DynamicSurfaceCache,
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
            self.bundle_map.insert(key, self.bundles.len());
            self.bundles.push(RenderDataBundle {
                data: dynamic_surface_cache.get_or_create(key, layout),
                sort_index,
                instances: vec![
                    // Each bundle must have at least one instance to be rendered.
                    SurfaceInstanceData {
                        node_handle,
                        ..Default::default()
                    },
                ],
                material: material.clone(),
                render_path,
                time_to_live: Default::default(),
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
