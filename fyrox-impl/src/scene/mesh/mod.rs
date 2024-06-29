//! Contains all structures and methods to create and manage mesh scene graph nodes. See [`Mesh`] docs for more info
//! and usage examples.

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
        TypeUuidProvider,
    },
    graph::{BaseSceneGraph, SceneGraph},
    material::MaterialResource,
    renderer::{
        self,
        bundle::{
            PersistentIdentifier, RenderContext, RenderDataBundleStorageTrait, SurfaceInstanceData,
        },
        framework::geometry_buffer::ElementRange,
    },
    scene::{
        base::{Base, BaseBuilder},
        debug::{Line, SceneDrawingContext},
        graph::Graph,
        mesh::{
            buffer::{
                BytesStorage, TriangleBuffer, TriangleBufferRefMut, VertexAttributeDescriptor,
                VertexAttributeUsage, VertexBuffer, VertexBufferRefMut, VertexReadTrait,
                VertexViewMut, VertexWriteTrait,
            },
            surface::{BlendShape, Surface, SurfaceData, SurfaceResource},
        },
        node::{Node, NodeTrait, RdcControlFlow, SyncContext},
    },
};
use fxhash::{FxHashMap, FxHasher};
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
        for descendant_handle in ctx.graph.traverse_handle_iter(from) {
            if descendant_handle == from {
                continue;
            }

            let descendant = &ctx.graph[descendant_handle];
            descendant.collect_render_data(&mut RenderContext {
                observer_position: ctx.observer_position,
                z_near: ctx.z_near,
                z_far: ctx.z_far,
                view_matrix: ctx.view_matrix,
                projection_matrix: ctx.projection_matrix,
                frustum: None,
                storage: self,
                graph: ctx.graph,
                render_pass_name: ctx.render_pass_name,
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
        layout: &[VertexAttributeDescriptor],
        material: &MaterialResource,
        _render_path: RenderPath,
        _decal_layer_index: u8,
        _sort_index: u64,
        _is_skinned: bool,
        _node_handle: Handle<Node>,
        func: &mut dyn FnMut(VertexBufferRefMut, TriangleBufferRefMut),
    ) {
        let mut hasher = FxHasher::default();
        layout.hash(&mut hasher);
        hasher.write_u64(material.key());
        let batch_hash = hasher.finish();

        let batch = self.batches.entry(batch_hash).or_insert_with(|| Batch {
            data: SurfaceResource::new_ok(
                ResourceKind::Embedded,
                SurfaceData::new(
                    VertexBuffer::new_with_layout(layout, 0, BytesStorage::with_capacity(4096))
                        .unwrap(),
                    TriangleBuffer::new(Vec::with_capacity(4096)),
                ),
            ),
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
        _decal_layer_index: u8,
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
///     let cube_surface = SurfaceBuilder::new(SurfaceResource::new_ok(ResourceKind::Embedded, cube_surface_data)).build();
///
///     MeshBuilder::new(BaseBuilder::new())
///         .with_surfaces(vec![cube_surface])
///         .build(graph)
/// }
/// ```
///
/// This example creates a unit cube surface with default material and then creates a mesh with this surface. If you need to create
/// custom surface, see [`crate::scene::mesh::surface::SurfaceData`] docs for more info.
#[derive(Debug, Reflect, Clone, Visit)]
pub struct Mesh {
    #[visit(rename = "Common")]
    base: Base,

    #[reflect(setter = "set_surfaces")]
    surfaces: InheritableVariable<Vec<Surface>>,

    #[reflect(setter = "set_render_path")]
    render_path: InheritableVariable<RenderPath>,

    #[reflect(setter = "set_decal_layer_index")]
    decal_layer_index: InheritableVariable<u8>,

    #[visit(optional)]
    #[reflect(
        setter = "set_batching_mode",
        description = "Enable or disable dynamic batching. It could be useful to reduce amount \
    of draw calls per frame if you have lots of meshes with small vertex count. Does not work with \
    meshes, that have skin or blend shapes. Such meshes will be drawn in a separate draw call."
    )]
    batching_mode: InheritableVariable<BatchingMode>,

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
            decal_layer_index: InheritableVariable::new_modified(0),
            batching_mode: Default::default(),
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
                    bounding_box.add_point(
                        self.global_transform()
                            .transform_point(&Point3::from(
                                view.read_3_f32(VertexAttributeUsage::Position).unwrap(),
                            ))
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
                    for (&bone_index, &weight) in view
                        .read_4_u8(VertexAttributeUsage::BoneIndices)
                        .unwrap()
                        .iter()
                        .zip(
                            view.read_4_f32(VertexAttributeUsage::BoneWeight)
                                .unwrap()
                                .iter(),
                        )
                    {
                        position += bone_matrices[bone_index as usize]
                            .transform_point(&Point3::from(
                                view.read_3_f32(VertexAttributeUsage::Position).unwrap(),
                            ))
                            .coords
                            .scale(weight);
                    }

                    bounding_box.add_point(position);
                }
            }
        }
        bounding_box
    }

    /// Sets new decal layer index. It defines which decals will be applies to the mesh,
    /// for example iff a decal has index == 0 and a mesh has index == 0, then decals will
    /// be applied. This allows you to apply decals only on needed surfaces.
    pub fn set_decal_layer_index(&mut self, index: u8) -> u8 {
        self.decal_layer_index.set_value_and_mark_modified(index)
    }

    /// Returns current decal index.
    pub fn decal_layer_index(&self) -> u8 {
        *self.decal_layer_index
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

impl NodeTrait for Mesh {
    crate::impl_query_component!();

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

    fn sync_transform(&self, _new_global_transform: &Matrix4<f32>, context: &mut SyncContext) {
        if self.surfaces.iter().any(|s| !s.bones.is_empty()) {
            let mut world_aabb = self
                .local_bounding_box()
                .transform(&self.global_transform());

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
            self.world_bounding_box.set(
                self.local_bounding_box()
                    .transform(&self.global_transform()),
            );
        }
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.global_visibility()
            || !self.is_globally_enabled()
            || (self.frustum_culling()
                && !ctx
                    .frustum
                    .map_or(true, |f| f.is_intersects_aabb(&self.world_bounding_box())))
        {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) && !self.cast_shadows() {
            return RdcControlFlow::Continue;
        }

        if let BatchingMode::Static = *self.batching_mode {
            let mut container = self.batch_container.0.lock();

            if container.batches.is_empty() {
                container.fill(self.self_handle, ctx);
            }

            for (index, batch) in container.batches.values().enumerate() {
                ctx.storage.push(
                    &batch.data,
                    &batch.material,
                    self.render_path(),
                    self.decal_layer_index(),
                    batch.material.key(),
                    SurfaceInstanceData {
                        world_transform: Matrix4::identity(),
                        bone_matrices: Default::default(),
                        depth_offset: self.depth_offset_factor(),
                        blend_shapes_weights: Default::default(),
                        element_range: ElementRange::Full,
                        persistent_identifier: PersistentIdentifier::new_combined(
                            &batch.data,
                            self.self_handle,
                            index,
                        ),
                        node_handle: self.self_handle,
                    },
                );
            }

            RdcControlFlow::Break
        } else {
            for (index, surface) in self.surfaces().iter().enumerate() {
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
                        ctx.storage.push(
                            surface.data_ref(),
                            surface.material(),
                            self.render_path(),
                            self.decal_layer_index(),
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
                                depth_offset: self.depth_offset_factor(),
                                blend_shapes_weights: self
                                    .blend_shapes()
                                    .iter()
                                    .map(|bs| bs.weight / 100.0)
                                    .collect(),
                                element_range: ElementRange::Full,
                                persistent_identifier: PersistentIdentifier::new_combined(
                                    surface.data_ref(),
                                    self.self_handle,
                                    index,
                                ),
                                node_handle: self.self_handle,
                            },
                        );
                    }
                    BatchingMode::Dynamic => {
                        let surface_data_guard = surface.data_ref().data_ref();

                        ctx.storage.push_triangles(
                            &surface_data_guard
                                .vertex_buffer
                                .layout_descriptor()
                                .collect::<Vec<_>>(),
                            surface.material(),
                            *self.render_path,
                            self.decal_layer_index(),
                            0,
                            false,
                            self.self_handle,
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
    decal_layer_index: u8,
    blend_shapes: Vec<BlendShape>,
    batching_mode: BatchingMode,
}

impl MeshBuilder {
    /// Creates new instance of mesh builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            surfaces: Default::default(),
            render_path: RenderPath::Deferred,
            decal_layer_index: 0,
            blend_shapes: Default::default(),
            batching_mode: BatchingMode::None,
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

    /// Sets desired decal layer index.
    pub fn with_decal_layer_index(mut self, decal_layer_index: u8) -> Self {
        self.decal_layer_index = decal_layer_index;
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

    /// Creates new mesh.
    pub fn build_node(self) -> Node {
        Node::new(Mesh {
            blend_shapes: self.blend_shapes.into(),
            base: self.base_builder.build_base(),
            surfaces: self.surfaces.into(),
            local_bounding_box: Default::default(),
            local_bounding_box_dirty: Cell::new(true),
            render_path: self.render_path.into(),
            decal_layer_index: self.decal_layer_index.into(),
            world_bounding_box: Default::default(),
            batching_mode: self.batching_mode.into(),
            batch_container: Default::default(),
        })
    }

    /// Creates new mesh and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
