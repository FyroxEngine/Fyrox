//! Contains all structures and methods to create and manage mesh scene graph nodes.
//!
//! Mesh is a 3D model, each mesh split into multiple surfaces, each surface holds single
//! part of 3D model that have same textures assigned to each face. Such separation allows
//! us to efficiently render geometry, thus reducing amount of draw calls.
//!
//! Usually there is no need to manually create meshes, it is much easier to make one in 3d
//! modelling software or just download some model you like and load it in engine. But since
//! 3d model can contain multiple nodes, 3d model loading discussed in model resource section.

use crate::scene::mesh::surface::BlendShape;
use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector3},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::{
            buffer::{VertexAttributeUsage, VertexReadTrait},
            surface::Surface,
        },
        node::{Node, NodeTrait, TypeUuidProvider, UpdateContext},
    },
};
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

pub mod buffer;
pub mod surface;
pub mod vertex;

/// Defines a path that should be used to render a mesh.
#[derive(
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
    EnumVariantNames,
)]
#[repr(u32)]
pub enum RenderPath {
    /// Deferred rendering has much better performance than Forward, but it does not support transparent
    /// objects and there is no way to change blending. Deferred rendering is default rendering path.
    Deferred = 0,

    /// Forward rendering path supports translucency and custom blending. However current support
    /// of forward rendering is very little. It is ideal for transparent objects like glass.
    Forward = 1,
}

impl Default for RenderPath {
    fn default() -> Self {
        Self::Deferred
    }
}

impl RenderPath {
    /// Creates render path instance from its id.
    pub fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Deferred),
            1 => Ok(Self::Forward),
            _ => Err(format!("Invalid render path id {}!", id)),
        }
    }
}

/// See module docs.
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
}

impl Default for Mesh {
    fn default() -> Self {
        Self {
            base: Default::default(),
            surfaces: Default::default(),
            local_bounding_box: Default::default(),
            world_bounding_box: Default::default(),
            local_bounding_box_dirty: Cell::new(true),
            render_path: InheritableVariable::new(RenderPath::Deferred),
            decal_layer_index: InheritableVariable::new(0),
            blend_shapes: Default::default(),
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
            let data = data.lock();
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
}

impl NodeTrait for Mesh {
    crate::impl_query_component!();

    /// Returns current bounding box. Bounding box presented in *local coordinates*
    /// WARNING: This method does *not* includes bounds of bones!
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        if self.local_bounding_box_dirty.get() {
            let mut bounding_box = AxisAlignedBoundingBox::default();
            for surface in self.surfaces.iter() {
                let data = surface.data();
                let data = data.lock();
                for view in data.vertex_buffer.iter() {
                    bounding_box
                        .add_point(view.read_3_f32(VertexAttributeUsage::Position).unwrap());
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

    fn update(&mut self, context: &mut UpdateContext) {
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
}

/// Mesh builder allows you to construct mesh in declarative manner.
pub struct MeshBuilder {
    base_builder: BaseBuilder,
    surfaces: Vec<Surface>,
    render_path: RenderPath,
    decal_layer_index: u8,
    blend_shapes: Vec<BlendShape>,
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
        })
    }

    /// Creates new mesh and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
