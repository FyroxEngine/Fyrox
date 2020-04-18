//! Contains all structures and methods to create and manage navigation meshes (navmesh).
//!
//! Navigation mesh is a set of convex polygons which is used for path finding in complex
//! environment.
//!
//! # Limitations
//!
//! Current implementation can only build paths from vertex to vertex in mesh, it can't
//! search path from arbitrary point in polygon to other point in other polygon. It can
//! be added pretty easily, but requires some extensive tests. This is still TODO.

#![warn(missing_docs)]

use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};
use crate::{
    scene::mesh::Mesh,
    utils::{
        astar::{
            PathFinder,
            PathVertex,
            PathKind,
            PathError,
        },
        raw_mesh::RawMeshBuilder,
    },
    core::{
        octree::Octree,
        math::{
            TriangleDefinition,
            vec3::Vec3,
            self,
        },
    },
};

/// See module docs.
pub struct Navmesh {
    octree: Octree,
    triangles: Vec<TriangleDefinition>,
    pathfinder: PathFinder,
    query_buffer: Vec<u32>,
}

#[derive(Copy, Clone)]
struct Edge {
    a: u32,
    b: u32,
}

impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        // Direction-agnostic compare.
        (self.a == other.a && self.b == other.b) ||
            (self.a == other.b && self.b == other.a)
    }
}

impl Eq for Edge {}

impl Hash for Edge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Direction-agnostic hash.
        (self.a as u64 + self.b as u64).hash(state)
    }
}

impl Default for Navmesh {
    fn default() -> Self {
        Self {
            octree: Default::default(),
            triangles: Default::default(),
            pathfinder: Default::default(),
            query_buffer: Default::default(),
        }
    }
}

impl Navmesh {
    /// Creates new navigation mesh from given set of triangles and vertices. This is
    /// low level method that allows to specify triangles and vertices directly. In
    /// most cases you should use `from_mesh` method.
    pub fn new(triangles: &[TriangleDefinition], vertices: &[Vec3]) -> Self {
        // Build triangles for octree.
        let raw_triangles = triangles.iter().map(|t| {
            [
                vertices[t[0] as usize],
                vertices[t[1] as usize],
                vertices[t[2] as usize]
            ]
        }).collect::<Vec<[Vec3; 3]>>();

        // Fill in pathfinder.
        let mut pathfinder = PathFinder::new();
        pathfinder.set_vertices(vertices.iter().map(|v| PathVertex::new(*v)).collect());

        let mut edges = HashSet::new();
        for triangle in triangles {
            edges.insert(Edge { a: triangle[0], b: triangle[1] });
            edges.insert(Edge { a: triangle[1], b: triangle[2] });
            edges.insert(Edge { a: triangle[2], b: triangle[0] });
        }

        for edge in edges {
            pathfinder.link_bidirect(edge.a as usize, edge.b as usize);
        }

        Self {
            triangles: triangles.to_vec(),
            octree: Octree::new(&raw_triangles, 32),
            pathfinder,
            query_buffer: Default::default(),
        }
    }

    /// Creates new navigation mesh (navmesh) from given mesh. It is most simple way to create complex
    /// navigation mesh, it should be used in pair with model loading functionality - you can
    /// load model from file and turn it into navigation mesh, or even build navigation mesh
    /// from a model in existing scene. This method "eats" any kind of meshes with any amount
    /// of surfaces - it joins all surfaces into single mesh and creates navmesh from it.
    ///
    /// Example:
    /// ```
    /// use rg3d::scene::Scene;
    /// use rg3d::utils::navmesh::Navmesh;
    ///
    /// fn make_navmesh(scene: &Scene, navmesh_name: &str) -> Navmesh {
    ///     // Find mesh node in existing scene and create navigation mesh from it.
    ///     let navmesh_node_handle = scene.graph.find_by_name_from_root(navmesh_name);
    ///     Navmesh::from_mesh(scene.graph[navmesh_node_handle].as_mesh())
    /// }
    /// ```
    pub fn from_mesh(mesh: &Mesh) -> Self {
        // Join surfaces into one simple mesh.
        let mut builder = RawMeshBuilder::<Vec3>::default();
        let global_transform = mesh.global_transform();
        for surface in mesh.surfaces() {
            let shared_data = surface.get_data();
            let shared_data = shared_data.lock().unwrap();

            let vertices = shared_data.get_vertices();
            for triangle in shared_data.triangles() {
                builder.insert(global_transform.transform_vector(vertices[triangle[0] as usize].position));
                builder.insert(global_transform.transform_vector(vertices[triangle[1] as usize].position));
                builder.insert(global_transform.transform_vector(vertices[triangle[2] as usize].position));
            }
        }

        let mesh = builder.build();
        Navmesh::new(&mesh.triangles, &mesh.vertices)
    }

    /// Searches closest graph vertex to given point. Returns Some(index), or None
    /// if navmesh was empty.
    pub fn query_closest(&mut self, point: Vec3) -> Option<usize> {
        self.octree.point_query(point, &mut self.query_buffer);
        if self.query_buffer.is_empty() {
            // TODO: This is not optimal. It is better to trace ray down from given point
            //  and pick closest triangle.
            math::get_closest_point(self.pathfinder.vertices(), point)
        } else {
            math::get_closest_point_triangles(
                self.pathfinder.vertices(),
                &self.triangles,
                &self.query_buffer,
                point,
            )
        }
    }

    /// Returns reference to array of triangles.
    pub fn triangles(&self) -> &[TriangleDefinition] {
        &self.triangles
    }

    /// Returns reference to array of vertices.
    pub fn vertices(&self) -> &[PathVertex] {
        self.pathfinder.vertices()
    }

    /// Tries to build path using indices of begin and end points.
    ///
    /// Example:
    ///
    /// ```
    /// use rg3d::utils::navmesh::Navmesh;
    /// use rg3d::core::math::vec3::Vec3;
    /// use rg3d::utils::astar::{PathKind, PathError};
    ///
    /// fn find_path(navmesh: &mut Navmesh, begin: Vec3, end: Vec3, path: &mut Vec<Vec3>) -> Result<PathKind, PathError> {
    ///     if let Some(begin_index) = navmesh.query_closest(begin) {
    ///         if let Some(end_index) = navmesh.query_closest(end) {
    ///             return navmesh.build_path(begin_index, end_index, path);
    ///         }
    ///     }
    ///     Ok(PathKind::Empty)
    /// }
    /// ```
    pub fn build_path(&mut self, from: usize, to: usize, path: &mut Vec<Vec3>) -> Result<PathKind, PathError> {
        self.pathfinder.build(from, to, path)
    }
}