//! Contains all structures and methods to create and manage mesh scene graph nodes.
//!
//! Mesh is a 3D model, each mesh split into multiple surfaces, each surface holds single
//! part of 3D model that have same textures assigned to each face. Such separation allows
//! us to efficiently render geometry, thus reducing amount of draw calls.
//!
//! Usually there is no need to manually create meshes, it is much easier to make one in 3d
//! modelling software or just download some model you like and load it in engine. But since
//! 3d model can contain multiple nodes, 3d model loading discussed in model resource section.

use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector3},
        inspect::{Inspect, PropertyInfo},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        uuid::{uuid, Uuid},
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    impl_directly_inheritable_entity_trait,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::{
            buffer::{VertexAttributeUsage, VertexReadTrait},
            surface::Surface,
        },
        node::{Node, NodeTrait, TypeUuidProvider, UpdateContext},
        variable::{InheritError, TemplateVariable, VariableFlags},
        DirectlyInheritableEntity,
    },
};
use fxhash::FxHashMap;
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
    Inspect,
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
#[derive(Debug, Inspect, Clone, Visit)]
pub struct Mesh {
    #[visit(rename = "Common")]
    base: Base,

    #[inspect(getter = "Deref::deref")]
    surfaces: TemplateVariable<Vec<Surface>>,

    #[inspect(getter = "Deref::deref")]
    render_path: TemplateVariable<RenderPath>,

    #[inspect(getter = "Deref::deref")]
    decal_layer_index: TemplateVariable<u8>,

    #[inspect(skip)]
    #[visit(skip)]
    local_bounding_box: Cell<AxisAlignedBoundingBox>,

    #[inspect(skip)]
    #[visit(skip)]
    local_bounding_box_dirty: Cell<bool>,

    #[inspect(skip)]
    #[visit(skip)]
    world_bounding_box: Cell<AxisAlignedBoundingBox>,
}

impl_directly_inheritable_entity_trait!(Mesh;
    surfaces,
    render_path,
    decal_layer_index
);

impl Default for Mesh {
    fn default() -> Self {
        Self {
            base: Default::default(),
            surfaces: TemplateVariable::new_with_flags(
                Default::default(),
                VariableFlags::DONT_MARK_AS_MODIFIED_IF_MISSING,
            ),
            local_bounding_box: Default::default(),
            world_bounding_box: Default::default(),
            local_bounding_box_dirty: Cell::new(true),
            render_path: TemplateVariable::new(RenderPath::Deferred),
            decal_layer_index: TemplateVariable::new(0),
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
    /// Returns shared reference to array of surfaces.
    #[inline]
    pub fn surfaces(&self) -> &[Surface] {
        &self.surfaces
    }

    /// Returns mutable reference to array of surfaces.
    #[inline]
    pub fn surfaces_mut(&mut self) -> &mut [Surface] {
        self.surfaces.get_mut_silent()
    }

    /// Removes all surfaces from mesh.
    #[inline]
    pub fn clear_surfaces(&mut self) {
        self.surfaces.get_mut().clear();
        self.local_bounding_box_dirty.set(true);
    }

    /// Adds new surface into mesh, can be used to procedurally generate meshes.
    #[inline]
    pub fn add_surface(&mut self, surface: Surface) {
        self.surfaces.get_mut().push(surface);
        self.local_bounding_box_dirty.set(true);
    }

    /// Sets new render path for the mesh.
    pub fn set_render_path(&mut self, render_path: RenderPath) {
        self.render_path.set(render_path);
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
    pub fn set_decal_layer_index(&mut self, index: u8) {
        self.decal_layer_index.set(index);
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
        self.local_bounding_box.get()
    }

    /// Returns current **world-space** bounding box.
    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.world_bounding_box.get()
    }

    // Prefab inheritance resolving.
    fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        self.base.inherit_properties(parent)?;
        if let Some(parent) = parent.cast::<Self>() {
            self.try_inherit_self_properties(parent)?;
        }
        Ok(())
    }

    fn reset_inheritable_properties(&mut self) {
        self.base.reset_inheritable_properties();
        self.reset_self_inheritable_properties();
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base.restore_resources(resource_manager.clone());

        for surface in self.surfaces_mut() {
            surface.material().lock().resolve(resource_manager.clone());
        }
    }

    fn remap_handles(&mut self, old_new_mapping: &FxHashMap<Handle<Node>, Handle<Node>>) {
        self.base.remap_handles(old_new_mapping);

        for surface in self.surfaces.get_mut_silent() {
            for bone_handle in surface.bones.iter_mut() {
                if let Some(entry) = old_new_mapping.get(bone_handle) {
                    *bone_handle = *entry;
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn update(&mut self, context: &mut UpdateContext) -> bool {
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

        if self.surfaces.iter().any(|s| !s.bones.is_empty()) {
            let mut world_aabb = self
                .local_bounding_box()
                .transform(&self.global_transform());

            // Special case for skinned meshes.
            for surface in self.surfaces.iter() {
                for &bone in surface.bones() {
                    world_aabb.add_point(context.nodes[bone].global_position())
                }
            }

            self.world_bounding_box.set(world_aabb)
        } else {
            self.world_bounding_box.set(
                self.local_bounding_box()
                    .transform(&self.global_transform()),
            );
        }

        self.base.update_lifetime(context.dt)
    }
}

/// Mesh builder allows you to construct mesh in declarative manner.
pub struct MeshBuilder {
    base_builder: BaseBuilder,
    surfaces: Vec<Surface>,
    render_path: RenderPath,
    decal_layer_index: u8,
}

impl MeshBuilder {
    /// Creates new instance of mesh builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            surfaces: Default::default(),
            render_path: RenderPath::Deferred,
            decal_layer_index: 0,
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

    /// Creates new mesh.
    pub fn build_node(self) -> Node {
        Node::new(Mesh {
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
