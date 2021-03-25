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
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    renderer::surface::Surface,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};

/// Defines a path that should be used to render a mesh.
#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash, Debug)]
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
#[derive(Debug)]
pub struct Mesh {
    base: Base,
    surfaces: Vec<Surface>,
    bounding_box: Cell<AxisAlignedBoundingBox>,
    bounding_box_dirty: Cell<bool>,
    cast_shadows: bool,
    render_path: RenderPath,
}

impl Default for Mesh {
    fn default() -> Self {
        Self {
            base: Default::default(),
            surfaces: Default::default(),
            bounding_box: Default::default(),
            bounding_box_dirty: Cell::new(true),
            cast_shadows: true,
            render_path: RenderPath::Deferred,
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

impl Visit for Mesh {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base.visit("Common", visitor)?;
        let _ = self.cast_shadows.visit("CastShadows", visitor);

        let mut render_path = self.render_path as u32;
        let _ = render_path.visit("RenderPath", visitor);
        if visitor.is_reading() {
            self.render_path = RenderPath::from_id(render_path)?;
        }

        // Serialize surfaces, but keep in mind that surfaces from resources will be automatically
        // recreated on resolve stage! Serialization of surfaces needed for procedural surfaces.
        self.surfaces.visit("Surfaces", visitor)?;

        visitor.leave_region()
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
        &mut self.surfaces
    }

    /// Removes all surfaces from mesh.
    #[inline]
    pub fn clear_surfaces(&mut self) {
        self.surfaces.clear();
        self.bounding_box_dirty.set(true);
    }

    /// Adds new surface into mesh, can be used to procedurally generate meshes.
    #[inline]
    pub fn add_surface(&mut self, surface: Surface) {
        self.surfaces.push(surface);
        self.bounding_box_dirty.set(true);
    }

    /// Applies given color to all surfaces.
    #[inline]
    pub fn set_color(&mut self, color: Color) {
        for surface in self.surfaces.iter_mut() {
            surface.set_color(color);
        }
    }

    /// Returns true if mesh should cast shadows, false - otherwise.
    #[inline]
    pub fn cast_shadows(&self) -> bool {
        self.cast_shadows
    }

    /// Sets whether mesh should cast shadows or not.
    #[inline]
    pub fn set_cast_shadows(&mut self, cast_shadows: bool) {
        self.cast_shadows = cast_shadows;
    }

    /// Performs lazy bounding box evaluation. Bounding box presented in *local coordinates*
    /// WARNING: This method does *not* includes bounds of bones!
    pub fn bounding_box(&self) -> AxisAlignedBoundingBox {
        if self.bounding_box_dirty.get() {
            let mut bounding_box = AxisAlignedBoundingBox::default();
            for surface in self.surfaces.iter() {
                let data = surface.data();
                let data = data.read().unwrap();
                for vertex in data.get_vertices() {
                    bounding_box.add_point(vertex.position);
                }
            }
            self.bounding_box.set(bounding_box);
            self.bounding_box_dirty.set(false);
        }
        self.bounding_box.get()
    }

    /// Sets new render path for the mesh.
    pub fn set_render_path(&mut self, render_path: RenderPath) {
        self.render_path = render_path;
    }

    /// Returns current render path of the mesh.
    pub fn render_path(&self) -> RenderPath {
        self.render_path
    }

    /// Calculate bounding box in *world coordinates*. This method is very heavy and not
    /// intended to use every frame! WARNING: This method does *not* includes bounds of bones!
    pub fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        let mut bounding_box = AxisAlignedBoundingBox::default();
        for surface in self.surfaces.iter() {
            let data = surface.data();
            let data = data.read().unwrap();
            for vertex in data.get_vertices() {
                bounding_box.add_point(
                    self.global_transform()
                        .transform_point(&Point3::from(vertex.position))
                        .coords,
                );
            }
        }
        bounding_box
    }

    /// Calculate bounding box in *world coordinates* including influence of bones. This method
    /// is very heavy and not intended to use every frame!
    pub fn full_world_bounding_box(&self, graph: &Graph) -> AxisAlignedBoundingBox {
        let mut bounding_box = AxisAlignedBoundingBox::default();
        for surface in self.surfaces.iter() {
            let data = surface.data();
            let data = data.read().unwrap();
            if surface.bones().is_empty() {
                for vertex in data.get_vertices() {
                    bounding_box.add_point(
                        self.global_transform()
                            .transform_point(&Point3::from(vertex.position))
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

                for vertex in data.get_vertices() {
                    let mut position = Vector3::default();
                    for (&bone_index, &weight) in
                        vertex.bone_indices.iter().zip(vertex.bone_weights.iter())
                    {
                        position += bone_matrices[bone_index as usize]
                            .transform_point(&Point3::from(vertex.position))
                            .coords
                            .scale(weight);
                    }

                    bounding_box.add_point(position);
                }
            }
        }
        bounding_box
    }

    /// Performs frustum visibility test. It uses mesh bounding box *and* positions of bones.
    /// Mesh is considered visible if its bounding box visible by frustum, or if any bones
    /// position is inside frustum.
    pub fn is_intersect_frustum(&self, graph: &Graph, frustum: &Frustum) -> bool {
        if frustum.is_intersects_aabb_transform(&self.bounding_box(), &self.global_transform.get())
        {
            return true;
        }

        for surface in self.surfaces.iter() {
            for &bone in surface.bones.iter() {
                if frustum.is_contains_point(graph[bone].global_position()) {
                    return true;
                }
            }
        }

        false
    }

    /// Creates a raw copy of a mesh node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            surfaces: self.surfaces.clone(),
            bounding_box: self.bounding_box.clone(),
            bounding_box_dirty: self.bounding_box_dirty.clone(),
            cast_shadows: self.cast_shadows,
            render_path: self.render_path,
        }
    }
}

/// Mesh builder allows you to construct mesh in declarative manner.
pub struct MeshBuilder {
    base_builder: BaseBuilder,
    surfaces: Vec<Surface>,
    cast_shadows: bool,
    render_path: RenderPath,
}

impl MeshBuilder {
    /// Creates new instance of mesh builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            surfaces: Default::default(),
            cast_shadows: true,
            render_path: RenderPath::Deferred,
        }
    }

    /// Sets desired surfaces for mesh.
    pub fn with_surfaces(mut self, surfaces: Vec<Surface>) -> Self {
        self.surfaces = surfaces;
        self
    }

    /// Sets whether mesh should cast shadows or not.
    pub fn with_cast_shadows(mut self, cast_shadows: bool) -> Self {
        self.cast_shadows = cast_shadows;
        self
    }

    /// Sets desired render path. Keep in mind that RenderPath::Forward is not fully
    /// implemented and only used to render transparent objects!
    pub fn with_render_path(mut self, render_path: RenderPath) -> Self {
        self.render_path = render_path;
        self
    }

    /// Creates new mesh.
    pub fn build_node(self) -> Node {
        Node::Mesh(Mesh {
            base: self.base_builder.build_base(),
            cast_shadows: self.cast_shadows,
            surfaces: self.surfaces,
            bounding_box: Default::default(),
            bounding_box_dirty: Cell::new(true),
            render_path: self.render_path,
        })
    }

    /// Creates new mesh and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
