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

//! Contains all structures and methods to create and manage mesh scene graph nodes. See [`Mesh`] docs for more info
//! and usage examples.

use crate::material::{
    Material, MaterialResourceBinding, MaterialResourceExtension, MaterialTextureBinding,
};
use crate::renderer::DynamicSurfaceCache;
use crate::resource::texture::PLACEHOLDER;
use crate::scene::mesh::surface::SurfaceBuilder;
use crate::scene::node::constructor::NodeConstructor;
use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector3, Vector4},
        color::Color,
        math::aabb::AxisAlignedBoundingBox,
        parking_lot::Mutex,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    graph::{BaseSceneGraph, SceneGraph},
    material::MaterialResource,
    renderer::{
        self,
        bundle::{RenderContext, RenderDataBundleStorageTrait, SurfaceInstanceData},
        framework::ElementRange,
    },
    scene::{
        base::{Base, BaseBuilder},
        debug::{Line, SceneDrawingContext},
        graph::Graph,
        mesh::{
            buffer::{
                TriangleBuffer, TriangleBufferRefMut, VertexAttributeDescriptor,
                VertexAttributeUsage, VertexBuffer, VertexBufferRefMut, VertexReadTrait,
                VertexViewMut, VertexWriteTrait,
            },
            surface::{BlendShape, Surface, SurfaceData, SurfaceResource},
        },
        node::{Node, NodeTrait, RdcControlFlow, SyncContext},
    },
};
use fxhash::{FxHashMap, FxHasher};
use fyrox_graph::constructor::ConstructorProvider;
use fyrox_resource::untyped::ResourceKind;
use std::{
    cell::Cell,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

pub mod buffer;
pub mod surface;
pub mod vertex;

/// Defines a path that should be used to render a mesh.
#[derive(
    Default,
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Hash,
    Debug,
    Visit,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    TypeUuidProvider,
)]
#[type_uuid(id = "009bccb6-42e4-4dc6-bb26-6a8a70b3fab9")]
#[repr(u32)]
pub enum RenderPath {
    /// Deferred rendering has much better performance than Forward, but it does not support transparent
    /// objects and there is no way to change blending. Deferred rendering is default rendering path.
    #[default]
    Deferred = 0,

    /// Forward rendering path supports translucency and custom blending. However current support
    /// of forward rendering is very little. It is ideal for transparent objects like glass.
    Forward = 1,
}

fn transform_vertex(mut vertex: VertexViewMut, world: &Matrix4<f32>) {
    if let Ok(position) = vertex.cast_attribute::<Vector3<f32>>(VertexAttributeUsage::Position) {
        *position = world.transform_point(&(*position).into()).coords;
    }
    if let Ok(normal) = vertex.cast_attribute::<Vector3<f32>>(VertexAttributeUsage::Normal) {
        *normal = world.transform_vector(normal);
    }
    if let Ok(tangent) = vertex.cast_attribute::<Vector4<f32>>(VertexAttributeUsage::Tangent) {
        let new_tangent = world.transform_vector(&tangent.xyz());
        *tangent = Vector4::new(
            new_tangent.x,
            new_tangent.y,
            new_tangent.z,
            // Keep handedness.
            tangent.w,
        );
    }
}

/// Batching mode defines how the mesh data will be grouped before rendering.
#[derive(
    Default,
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Hash,
    Debug,
    Visit,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    TypeUuidProvider,
)]
#[type_uuid(id = "745e6f32-63f5-46fe-8edb-9708699ae328")]
#[repr(u32)]
pub enum BatchingMode {
    /// No batching. The mesh will be drawn in a separate draw call.
    #[default]
    None,
    /// Static batching. Render data of all **descendant** nodes will be baked into a static buffer
    /// and it will be drawn. This mode "bakes" world transform of a node into vertices, thus making
    /// them immovable.
    Static,
    /// Dynamic batching. Render data of the mesh will be merged with the same meshes dynamically on
    /// each frame, thus allowing the meshes to be movable. This could be slow if used incorrectly!
    Dynamic,
}

#[derive(Debug, Clone)]
struct Batch {
    data: SurfaceResource,
    material: MaterialResource,
}

#[derive(Debug, Default, Clone)]
struct BatchContainer {
    batches: FxHashMap<u64, Batch>,
}

impl BatchContainer {
    fn fill(&mut self, from: Handle<Node>, ctx: &mut RenderContext) {
        for (descendant_handle, descendant) in ctx.graph.traverse_iter(from) {
            if descendant_handle == from {
                continue;
            }

            descendant.collect_render_data(&mut RenderContext {
                elapsed_time: ctx.elapsed_time,
                observer_info: ctx.observer_info,
                frustum: None,
                storage: self,
                graph: ctx.graph,
                render_pass_name: ctx.render_pass_name,
                dynamic_surface_cache: ctx.dynamic_surface_cache,
            });
        }
    }
}

#[derive(Debug, Default)]
struct BatchContainerWrapper(Mutex<BatchContainer>);

impl Clone for BatchContainerWrapper {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.0.lock().clone()))
    }
}

impl RenderDataBundleStorageTrait for BatchContainer {
    fn push_triangles(
        &mut self,
        dynamic_surface_cache: &mut DynamicSurfaceCache,
        layout: &[VertexAttributeDescriptor],
        material: &MaterialResource,
        _render_path: RenderPath,
        _sort_index: u64,
        _node_handle: Handle<Node>,
        func: &mut dyn FnMut(VertexBufferRefMut, TriangleBufferRefMut),
    ) {
        let mut hasher = FxHasher::default();
        layout.hash(&mut hasher);
        hasher.write_u64(material.key());
        let batch_hash = hasher.finish();

        let batch = self.batches.entry(batch_hash).or_insert_with(|| Batch {
            data: dynamic_surface_cache.get_or_create(batch_hash, layout),
            material: material.clone(),
        });

        let mut batch_data_guard = batch.data.data_ref();
        let batch_data = &mut *batch_data_guard;

        func(
            batch_data.vertex_buffer.modify(),
            batch_data.geometry_buffer.modify(),
        );
    }

    fn push(
        &mut self,
        data: &SurfaceResource,
        material: &MaterialResource,
        _render_path: RenderPath,
        _sort_index: u64,
        instance_data: SurfaceInstanceData,
    ) {
        let src_data = data.data_ref();

        let mut hasher = FxHasher::default();
        src_data.vertex_buffer.layout().hash(&mut hasher);
        hasher.write_u64(material.key());
        let batch_hash = hasher.finish();

        let batch = self.batches.entry(batch_hash).or_insert_with(|| Batch {
            data: SurfaceResource::new_ok(
                Uuid::new_v4(),
                ResourceKind::Embedded,
                SurfaceData::new(
                    src_data.vertex_buffer.clone_empty(4096),
                    TriangleBuffer::new(Vec::with_capacity(4096)),
                ),
            ),
            material: material.clone(),
        });

        let mut batch_data_guard = batch.data.data_ref();
        let batch_data = &mut *batch_data_guard;
        let start_vertex_index = batch_data.vertex_buffer.vertex_count();
        let mut batch_vertex_buffer = batch_data.vertex_buffer.modify();
        for src_vertex in src_data.vertex_buffer.iter() {
            batch_vertex_buffer
                .push_vertex_raw(&src_vertex.transform(&mut |vertex| {
                    transform_vertex(vertex, &instance_data.world_transform)
                }))
                .expect("Vertex size must match!");
        }

        let mut batch_geometry_buffer = batch_data.geometry_buffer.modify();
        batch_geometry_buffer.push_triangles_with_offset(
            start_vertex_index,
            src_data.geometry_buffer.triangles_ref(),
        );
    }
}

/// Mesh is a 3D model, each mesh split into multiple surfaces, each surface represents a patch of the mesh with a single material
/// assigned to each face. See [`Surface`] docs for more info.
///
/// ## How to create
///
/// Usually there is no need to manually create meshes, it is much easier to make one in a 3d modelling software or just download
/// some model you like and load it in engine. See [`crate::resource::model::Model`] docs for more info about model resources.
///
/// However, sometimes there's a need to create meshes manually (for example - in games with procedurally-generated content). You
/// can do it like so:
///
/// ```rust
/// # use fyrox_impl::{
/// #     core::{algebra::Matrix4, pool::Handle},
/// #     scene::{
/// #         base::BaseBuilder,
/// #         graph::Graph,
/// #         mesh::{
/// #             surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
/// #             MeshBuilder,
/// #         },
/// #         node::Node,
/// #     },
/// # };
/// use fyrox_resource::untyped::ResourceKind;
/// fn create_cube_mesh(graph: &mut Graph) -> Handle<Node> {
///     let cube_surface_data = SurfaceData::make_cube(Matrix4::identity());
///
///     let cube_surface = SurfaceBuilder::new(SurfaceResource::new_embedded(cube_surface_data)).build();
///
///     MeshBuilder::new(BaseBuilder::new())
///         .with_surfaces(vec![cube_surface])
///         .build(graph)
/// }
/// ```
///
/// This example creates a unit cube surface with default material and then creates a mesh with this surface. If you need to create
/// custom surface, see [`crate::scene::mesh::surface::SurfaceData`] docs for more info.
#[derive(Debug, Reflect, Clone, Visit, ComponentProvider)]
#[reflect(derived_type = "Node")]
pub struct Mesh {
    #[visit(rename = "Common")]
    base: Base,

    #[reflect(setter = "set_surfaces")]
    surfaces: InheritableVariable<Vec<Surface>>,

    #[reflect(setter = "set_render_path")]
    render_path: InheritableVariable<RenderPath>,

    #[visit(optional)]
    #[reflect(
        setter = "set_batching_mode",
        description = "Enable or disable dynamic batching. It could be useful to reduce amount \
    of draw calls per frame if you have lots of meshes with small vertex count. Does not work with \
    meshes, that have skin or blend shapes. Such meshes will be drawn in a separate draw call."
    )]
    batching_mode: InheritableVariable<BatchingMode>,

    #[visit(optional)]
    blend_shapes_property_name: String,

    #[visit(optional)]
    blend_shapes: InheritableVariable<Vec<BlendShape>>,

    #[reflect(hidden)]
    #[visit(skip)]
    local_bounding_box: Cell<AxisAlignedBoundingBox>,

    #[reflect(hidden)]
    #[visit(skip)]
    local_bounding_box_dirty: Cell<bool>,

    #[reflect(hidden)]
    #[visit(skip)]
    world_bounding_box: Cell<AxisAlignedBoundingBox>,

    #[reflect(hidden)]
    #[visit(skip)]
    batch_container: BatchContainerWrapper,
}

impl Default for Mesh {
    fn default() -> Self {
        Self {
            base: Default::default(),
            surfaces: Default::default(),
            local_bounding_box: Default::default(),
            world_bounding_box: Default::default(),
            local_bounding_box_dirty: Cell::new(true),
            render_path: InheritableVariable::new_modified(RenderPath::Deferred),
            batching_mode: Default::default(),
            blend_shapes_property_name: Mesh::DEFAULT_BLEND_SHAPES_PROPERTY_NAME.to_string(),
            blend_shapes: Default::default(),
            batch_container: Default::default(),
        }
    }
}

impl Deref for Mesh {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Mesh {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl TypeUuidProvider for Mesh {
    fn type_uuid() -> Uuid {
        uuid!("caaf9d7b-bd74-48ce-b7cc-57e9dc65c2e6")
    }
}

impl Mesh {
    /// Default name of the blend shapes storage property in a shader.
    pub const DEFAULT_BLEND_SHAPES_PROPERTY_NAME: &'static str = "blendShapesStorage";

    /// Sets surfaces for the mesh.
    pub fn set_surfaces(&mut self, surfaces: Vec<Surface>) -> Vec<Surface> {
        self.surfaces.set_value_and_mark_modified(surfaces)
    }

    /// Returns shared reference to array of surfaces.
    #[inline]
    pub fn surfaces(&self) -> &[Surface] {
        &self.surfaces
    }

    /// Returns mutable reference to array of surfaces.
    #[inline]
    pub fn surfaces_mut(&mut self) -> &mut [Surface] {
        self.local_bounding_box_dirty.set(true);
        self.surfaces.get_value_mut_silent()
    }

    /// Removes all surfaces from mesh.
    #[inline]
    pub fn clear_surfaces(&mut self) {
        self.surfaces.get_value_mut_and_mark_modified().clear();
        self.local_bounding_box_dirty.set(true);
    }

    /// Adds new surface into mesh, can be used to procedurally generate meshes.
    #[inline]
    pub fn add_surface(&mut self, surface: Surface) {
        self.surfaces
            .get_value_mut_and_mark_modified()
            .push(surface);
        self.local_bounding_box_dirty.set(true);
    }

    /// Returns a list of blend shapes.
    pub fn blend_shapes(&self) -> &[BlendShape] {
        &self.blend_shapes
    }

    /// Returns a list of blend shapes.
    pub fn blend_shapes_mut(&mut self) -> &mut [BlendShape] {
        self.blend_shapes.get_value_mut_and_mark_modified()
    }

    /// Sets new render path for the mesh.
    pub fn set_render_path(&mut self, render_path: RenderPath) -> RenderPath {
        self.render_path.set_value_and_mark_modified(render_path)
    }

    /// Returns current render path of the mesh.
    pub fn render_path(&self) -> RenderPath {
        *self.render_path
    }

    /// Calculate very accurate bounding box in *world coordinates* including influence of bones.
    /// This method is very heavy and not intended to use every frame!
    pub fn accurate_world_bounding_box(&self, graph: &Graph) -> AxisAlignedBoundingBox {
        let mut bounding_box = AxisAlignedBoundingBox::default();
        for surface in self.surfaces.iter() {
            let data = surface.data();
            let data = data.data_ref();
            if surface.bones().is_empty() {
                for view in data.vertex_buffer.iter() {
                    let Ok(vertex_pos) = view.read_3_f32(VertexAttributeUsage::Position) else {
                        break;
                    };

                    bounding_box.add_point(
                        self.global_transform()
                            .transform_point(&Point3::from(vertex_pos))
                            .coords,
                    );
                }
            } else {
                // Special case for skinned surface. Its actual bounds defined only by bones
                // influence.

                // Precalculate bone matrices first to speed up calculations.
                let bone_matrices = surface
                    .bones()
                    .iter()
                    .map(|&b| {
                        let bone_node = &graph[b];
                        bone_node.global_transform() * bone_node.inv_bind_pose_transform()
                    })
                    .collect::<Vec<Matrix4<f32>>>();

                for view in data.vertex_buffer.iter() {
                    let mut position = Vector3::default();

                    let Ok(vertex_pos) = view.read_3_f32(VertexAttributeUsage::Position) else {
                        break;
                    };
                    let Ok(bone_indices) = view.read_4_u8(VertexAttributeUsage::BoneIndices) else {
                        break;
                    };
                    let Ok(bone_weights) = view.read_4_f32(VertexAttributeUsage::BoneWeight) else {
                        break;
                    };

                    for (&bone_index, &weight) in bone_indices.iter().zip(bone_weights.iter()) {
                        position += bone_matrices[bone_index as usize]
                            .transform_point(&Point3::from(vertex_pos))
                            .coords
                            .scale(weight);
                    }

                    bounding_box.add_point(position);
                }
            }
        }
        bounding_box
    }

    /// Enable or disable dynamic batching. It could be useful to reduce amount of draw calls per
    /// frame if you have lots of meshes with small vertex count. Does not work with meshes, that
    /// have skin or blend shapes. Such meshes will be drawn in a separate draw call.
    pub fn set_batching_mode(&mut self, mode: BatchingMode) -> BatchingMode {
        if let BatchingMode::None | BatchingMode::Dynamic = mode {
            // Destroy batched data.
            std::mem::take(&mut self.batch_container);
        }

        self.batching_mode.set_value_and_mark_modified(mode)
    }

    /// Returns `true` if the dynamic batching is enabled, `false` otherwise.
    pub fn batching_mode(&self) -> BatchingMode {
        *self.batching_mode
    }
}

fn extend_aabb_from_vertex_buffer(
    vertex_buffer: &VertexBuffer,
    bounding_box: &mut AxisAlignedBoundingBox,
) {
    if let Some(position_attribute_view) =
        vertex_buffer.attribute_view::<Vector3<f32>>(VertexAttributeUsage::Position)
    {
        for i in 0..vertex_buffer.vertex_count() as usize {
            bounding_box.add_point(*position_attribute_view.get(i).unwrap());
        }
    }
}

fn placeholder_material() -> MaterialResource {
    let mut material = Material::standard();
    material.bind("diffuseTexture", PLACEHOLDER.resource());
    MaterialResource::new_ok(Uuid::new_v4(), ResourceKind::Embedded, material)
}

impl ConstructorProvider<Node, Graph> for Mesh {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_variant("Empty", |_| {
                MeshBuilder::new(BaseBuilder::new()).build_node().into()
            })
            .with_variant("Cube", |_| {
                MeshBuilder::new(BaseBuilder::new().with_name("Cube"))
                    .with_surfaces(vec![SurfaceBuilder::new(surface::CUBE.resource.clone())
                        .with_material(placeholder_material())
                        .build()])
                    .build_node()
                    .into()
            })
            .with_variant("Cone", |_| {
                MeshBuilder::new(BaseBuilder::new().with_name("Cone"))
                    .with_surfaces(vec![SurfaceBuilder::new(surface::CONE.resource.clone())
                        .with_material(placeholder_material())
                        .build()])
                    .build_node()
                    .into()
            })
            .with_variant("Cylinder", |_| {
                MeshBuilder::new(BaseBuilder::new().with_name("Cylinder"))
                    .with_surfaces(vec![SurfaceBuilder::new(
                        surface::CYLINDER.resource.clone(),
                    )
                    .with_material(placeholder_material())
                    .build()])
                    .build_node()
                    .into()
            })
            .with_variant("Sphere", |_| {
                MeshBuilder::new(BaseBuilder::new().with_name("Sphere"))
                    .with_surfaces(vec![SurfaceBuilder::new(surface::SPHERE.resource.clone())
                        .with_material(placeholder_material())
                        .build()])
                    .build_node()
                    .into()
            })
            .with_variant("Quad", |_| {
                MeshBuilder::new(BaseBuilder::new().with_name("Quad"))
                    .with_surfaces(vec![SurfaceBuilder::new(surface::QUAD.resource.clone())
                        .with_material(placeholder_material())
                        .build()])
                    .build_node()
                    .into()
            })
            .with_group("Mesh")
    }
}

impl NodeTrait for Mesh {
    /// Returns current bounding box. Bounding box presented in *local coordinates*
    /// WARNING: This method does *not* includes bounds of bones!
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        if self.local_bounding_box_dirty.get() {
            let mut bounding_box = AxisAlignedBoundingBox::default();

            if let BatchingMode::Static = *self.batching_mode {
                let container = self.batch_container.0.lock();
                for batch in container.batches.values() {
                    let data = batch.data.data_ref();
                    extend_aabb_from_vertex_buffer(&data.vertex_buffer, &mut bounding_box);
                }
            } else {
                for surface in self.surfaces.iter() {
                    let data = surface.data();
                    let data = data.data_ref();
                    extend_aabb_from_vertex_buffer(&data.vertex_buffer, &mut bounding_box);
                }
            }

            self.local_bounding_box.set(bounding_box);
            self.local_bounding_box_dirty.set(false);
        }

        self.local_bounding_box.get()
    }

    /// Returns current **world-space** bounding box.
    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.world_bounding_box.get()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn on_global_transform_changed(
        &self,
        new_global_transform: &Matrix4<f32>,
        context: &mut SyncContext,
    ) {
        if self.surfaces.iter().any(|s| !s.bones.is_empty()) {
            let mut world_aabb = self.local_bounding_box().transform(new_global_transform);

            // Special case for skinned meshes.
            for surface in self.surfaces.iter() {
                for &bone in surface.bones() {
                    if let Some(node) = context.nodes.try_borrow(bone) {
                        world_aabb.add_point(node.global_position())
                    }
                }
            }

            self.world_bounding_box.set(world_aabb)
        } else {
            self.world_bounding_box
                .set(self.local_bounding_box().transform(new_global_transform));
        }
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.should_be_rendered(ctx.frustum) {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) && !self.cast_shadows() {
            return RdcControlFlow::Continue;
        }

        if let BatchingMode::Static = *self.batching_mode {
            let mut container = self.batch_container.0.lock();

            if container.batches.is_empty() {
                container.fill(self.handle(), ctx);
            }

            for batch in container.batches.values() {
                ctx.storage.push(
                    &batch.data,
                    &batch.material,
                    self.render_path(),
                    batch.material.key(),
                    SurfaceInstanceData {
                        world_transform: Matrix4::identity(),
                        bone_matrices: Default::default(),
                        blend_shapes_weights: Default::default(),
                        element_range: ElementRange::Full,
                        node_handle: self.handle(),
                    },
                );
            }

            RdcControlFlow::Break
        } else {
            for surface in self.surfaces().iter() {
                let is_skinned = !surface.bones.is_empty();

                let world = if is_skinned {
                    Matrix4::identity()
                } else {
                    self.global_transform()
                };

                let batching_mode = match *self.batching_mode {
                    BatchingMode::None => BatchingMode::None,
                    BatchingMode::Static => BatchingMode::Static,
                    BatchingMode::Dynamic => {
                        let surface_data_guard = surface.data_ref().data_ref();
                        if self.blend_shapes().is_empty()
                            && surface.bones().is_empty()
                            && surface_data_guard.vertex_buffer.vertex_count() < 256
                        {
                            BatchingMode::Dynamic
                        } else {
                            BatchingMode::None
                        }
                    }
                };

                match batching_mode {
                    BatchingMode::None => {
                        let surface_data = surface.data_ref();
                        let substitute_material = surface_data
                            .data_ref()
                            .blend_shapes_container
                            .as_ref()
                            .and_then(|c| c.blend_shape_storage.as_ref())
                            .map(|texture| {
                                let material_copy = surface.material().deep_copy();
                                material_copy.data_ref().bind(
                                    &self.blend_shapes_property_name,
                                    MaterialResourceBinding::Texture(MaterialTextureBinding {
                                        value: Some(texture.clone()),
                                    }),
                                );
                                material_copy
                            });

                        ctx.storage.push(
                            surface_data,
                            substitute_material.as_ref().unwrap_or(surface.material()),
                            self.render_path(),
                            surface.material().key(),
                            SurfaceInstanceData {
                                world_transform: world,
                                bone_matrices: surface
                                    .bones
                                    .iter()
                                    .map(|bone_handle| {
                                        if let Some(bone_node) = ctx.graph.try_get(*bone_handle) {
                                            bone_node.global_transform()
                                                * bone_node.inv_bind_pose_transform()
                                        } else {
                                            Matrix4::identity()
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                                blend_shapes_weights: self
                                    .blend_shapes()
                                    .iter()
                                    .map(|bs| bs.weight / 100.0)
                                    .collect(),
                                element_range: ElementRange::Full,
                                node_handle: self.handle(),
                            },
                        );
                    }
                    BatchingMode::Dynamic => {
                        let surface_data_guard = surface.data_ref().data_ref();

                        ctx.storage.push_triangles(
                            ctx.dynamic_surface_cache,
                            &surface_data_guard
                                .vertex_buffer
                                .layout_descriptor()
                                .collect::<Vec<_>>(),
                            surface.material(),
                            *self.render_path,
                            0,
                            self.handle(),
                            &mut move |mut vertex_buffer, mut triangle_buffer| {
                                let start_vertex_index = vertex_buffer.vertex_count();

                                for vertex in surface_data_guard.vertex_buffer.iter() {
                                    vertex_buffer
                                        .push_vertex_raw(&vertex.transform(&mut |vertex| {
                                            transform_vertex(vertex, &world)
                                        }))
                                        .unwrap();
                                }

                                triangle_buffer.push_triangles_with_offset(
                                    start_vertex_index,
                                    surface_data_guard.geometry_buffer.triangles_ref(),
                                )
                            },
                        );
                    }
                    _ => (),
                }
            }

            RdcControlFlow::Continue
        }
    }

    fn debug_draw(&self, ctx: &mut SceneDrawingContext) {
        let transform = self.global_transform();

        for surface in self.surfaces() {
            for vertex in surface.data().data_ref().vertex_buffer.iter() {
                let len = 0.025;
                let position = transform
                    .transform_point(&Point3::from(
                        vertex.read_3_f32(VertexAttributeUsage::Position).unwrap(),
                    ))
                    .coords;
                let vertex_tangent = vertex.read_4_f32(VertexAttributeUsage::Tangent).unwrap();
                let tangent = transform
                    .transform_vector(&vertex_tangent.xyz())
                    .normalize()
                    .scale(len);
                let normal = transform
                    .transform_vector(
                        &vertex
                            .read_3_f32(VertexAttributeUsage::Normal)
                            .unwrap()
                            .xyz(),
                    )
                    .normalize()
                    .scale(len);
                let binormal = normal
                    .xyz()
                    .cross(&tangent)
                    .scale(vertex_tangent.w)
                    .normalize()
                    .scale(len);

                ctx.add_line(Line {
                    begin: position,
                    end: position + tangent,
                    color: Color::RED,
                });

                ctx.add_line(Line {
                    begin: position,
                    end: position + normal,
                    color: Color::BLUE,
                });

                ctx.add_line(Line {
                    begin: position,
                    end: position + binormal,
                    color: Color::GREEN,
                });
            }
        }
    }
}

/// Mesh builder allows you to construct mesh in declarative manner.
pub struct MeshBuilder {
    base_builder: BaseBuilder,
    surfaces: Vec<Surface>,
    render_path: RenderPath,
    blend_shapes: Vec<BlendShape>,
    batching_mode: BatchingMode,
    blend_shapes_property_name: String,
}

impl MeshBuilder {
    /// Creates new instance of mesh builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            surfaces: Default::default(),
            render_path: RenderPath::Deferred,
            blend_shapes: Default::default(),
            batching_mode: BatchingMode::None,
            blend_shapes_property_name: Mesh::DEFAULT_BLEND_SHAPES_PROPERTY_NAME.to_string(),
        }
    }

    /// Sets desired surfaces for mesh.
    pub fn with_surfaces(mut self, surfaces: Vec<Surface>) -> Self {
        self.surfaces = surfaces;
        self
    }

    /// Sets desired render path. Keep in mind that RenderPath::Forward is not fully
    /// implemented and only used to render transparent objects!
    pub fn with_render_path(mut self, render_path: RenderPath) -> Self {
        self.render_path = render_path;
        self
    }

    /// Sets the list of blend shapes. Keep in mind that actual blend shape data must be baked in surface data
    /// of every surface used by the mesh. Blend shapes are shared across all surfaces.
    pub fn with_blend_shapes(mut self, blend_shapes: Vec<BlendShape>) -> Self {
        self.blend_shapes = blend_shapes;
        self
    }

    /// Sets the desired batching mode. See [`BatchingMode`] docs for more info.
    pub fn with_batching_mode(mut self, mode: BatchingMode) -> Self {
        self.batching_mode = mode;
        self
    }

    /// Sets a name of the blend shapes property in a material used by this mesh.
    pub fn with_blend_shapes_property_name(mut self, name: String) -> Self {
        self.blend_shapes_property_name = name;
        self
    }

    /// Creates new mesh.
    pub fn build_node(self) -> Node {
        Node::new(Mesh {
            blend_shapes: self.blend_shapes.into(),
            base: self.base_builder.build_base(),
            surfaces: self.surfaces.into(),
            local_bounding_box: Default::default(),
            local_bounding_box_dirty: Cell::new(true),
            render_path: self.render_path.into(),
            world_bounding_box: Default::default(),
            batching_mode: self.batching_mode.into(),
            batch_container: Default::default(),
            blend_shapes_property_name: self.blend_shapes_property_name,
        })
    }

    /// Creates new mesh and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
