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

use crate::{
    core::{
        algebra::{Point3, Vector3},
        arrayvec::ArrayVec,
        math::{self, ray::Ray, TriangleDefinition},
        octree::{Octree, OctreeNode},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::mesh::Mesh,
    utils::{
        astar::{PathError, PathFinder, PathKind, PathVertex},
        raw_mesh::{RawMeshBuilder, RawVertex},
    },
};
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

/// See module docs.
#[derive(Clone, Debug)]
pub struct Navmesh {
    octree: Octree,
    triangles: Vec<TriangleDefinition>,
    pathfinder: PathFinder,
    query_buffer: Vec<u32>,
}

impl Visit for Navmesh {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pathfinder.visit("PathFinder", visitor)?;
        self.triangles.visit("Triangles", visitor)?;

        // No need to save octree, we can restore it on load.
        if visitor.is_reading() {
            let vertices = self.pathfinder.vertices();
            let raw_triangles = self
                .triangles
                .iter()
                .map(|t| {
                    [
                        vertices[t[0] as usize].position,
                        vertices[t[1] as usize].position,
                        vertices[t[2] as usize].position,
                    ]
                })
                .collect::<Vec<[Vector3<f32>; 3]>>();

            self.octree = Octree::new(&raw_triangles, 32);
        }

        visitor.leave_region()
    }
}

#[derive(Copy, Clone)]
struct Edge {
    a: u32,
    b: u32,
}

impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        // Direction-agnostic compare.
        (self.a == other.a && self.b == other.b) || (self.a == other.b && self.b == other.a)
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
    pub fn new(triangles: &[TriangleDefinition], vertices: &[Vector3<f32>]) -> Self {
        // Build triangles for octree.
        let raw_triangles = triangles
            .iter()
            .map(|t| {
                [
                    vertices[t[0] as usize],
                    vertices[t[1] as usize],
                    vertices[t[2] as usize],
                ]
            })
            .collect::<Vec<[Vector3<f32>; 3]>>();

        // Fill in pathfinder.
        let mut pathfinder = PathFinder::new();
        pathfinder.set_vertices(vertices.iter().map(|v| PathVertex::new(*v)).collect());

        let mut edges = HashSet::new();
        for triangle in triangles {
            edges.insert(Edge {
                a: triangle[0],
                b: triangle[1],
            });
            edges.insert(Edge {
                a: triangle[1],
                b: triangle[2],
            });
            edges.insert(Edge {
                a: triangle[2],
                b: triangle[0],
            });
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
        let mut builder = RawMeshBuilder::<RawVertex>::default();
        let global_transform = mesh.global_transform();
        for surface in mesh.surfaces() {
            let shared_data = surface.data();
            let shared_data = shared_data.read().unwrap();

            let vertices = shared_data.get_vertices();
            for triangle in shared_data.triangles() {
                builder.insert(RawVertex::from(
                    global_transform
                        .transform_point(&Point3::from(vertices[triangle[0] as usize].position))
                        .coords,
                ));
                builder.insert(RawVertex::from(
                    global_transform
                        .transform_point(&Point3::from(vertices[triangle[1] as usize].position))
                        .coords,
                ));
                builder.insert(RawVertex::from(
                    global_transform
                        .transform_point(&Point3::from(vertices[triangle[2] as usize].position))
                        .coords,
                ));
            }
        }

        let mesh = builder.build();

        Navmesh::new(
            &mesh.triangles,
            &mesh
                .vertices
                .into_iter()
                .map(|v| Vector3::new(v.x, v.y, v.z))
                .collect::<Vec<_>>(),
        )
    }

    /// Searches closest graph vertex to given point. Returns Some(index), or None
    /// if navmesh was empty.
    pub fn query_closest(&mut self, point: Vector3<f32>) -> Option<usize> {
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

    /// Returns shared reference to inner octree.
    pub fn octree(&self) -> &Octree {
        &self.octree
    }

    /// Tries to build path using indices of begin and end points.
    ///
    /// Example:
    ///
    /// ```
    /// use rg3d::utils::navmesh::Navmesh;
    /// use rg3d::core::algebra::Vector3;
    /// use rg3d::utils::astar::{PathKind, PathError};
    ///
    /// fn find_path(navmesh: &mut Navmesh, begin: Vector3<f32>, end: Vector3<f32>, path: &mut Vec<Vector3<f32>>) -> Result<PathKind, PathError> {
    ///     if let Some(begin_index) = navmesh.query_closest(begin) {
    ///         if let Some(end_index) = navmesh.query_closest(end) {
    ///             return navmesh.build_path(begin_index, end_index, path);
    ///         }
    ///     }
    ///     Ok(PathKind::Empty)
    /// }
    /// ```
    pub fn build_path(
        &mut self,
        from: usize,
        to: usize,
        path: &mut Vec<Vector3<f32>>,
    ) -> Result<PathKind, PathError> {
        self.pathfinder.build(from, to, path)
    }

    /// Tries to pick a triangle by given ray.
    pub fn ray_cast(&self, ray: Ray) -> Option<(Vector3<f32>, usize, TriangleDefinition)> {
        let mut buffer = ArrayVec::<[Handle<OctreeNode>; 128]>::new();

        self.octree.ray_query_static(&ray, &mut buffer);

        for node in buffer.into_iter() {
            if let OctreeNode::Leaf { indices, .. } = self.octree.node(node) {
                for &index in indices {
                    let triangle = self.triangles[index as usize].clone();
                    let a = self.pathfinder.vertices()[triangle[0] as usize].position;
                    let b = self.pathfinder.vertices()[triangle[1] as usize].position;
                    let c = self.pathfinder.vertices()[triangle[2] as usize].position;

                    if let Some(intersection) = ray.triangle_intersection(&[a, b, c]) {
                        return Some((intersection, index as usize, triangle));
                    }
                }
            } else {
                unreachable!()
            }
        }

        None
    }
}

/// Navmesh agent is a "pathfinding unit" that performs navigation on a mesh. It is designed to
/// cover most of simple use cases when you need to build and follow some path from point A to point B.
pub struct NavmeshAgent {
    path: Vec<Vector3<f32>>,
    current: u32,
    position: Vector3<f32>,
    last_target_position: Vector3<f32>,
    recalculation_threshold: f32,
    speed: f32,
}

impl Visit for NavmeshAgent {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.path.visit("Path", visitor)?;
        self.current.visit("Current", visitor)?;
        self.position.visit("Position", visitor)?;
        self.last_target_position
            .visit("LastTargetPosition", visitor)?;
        self.recalculation_threshold
            .visit("RecalculationThreshold", visitor)?;
        self.speed.visit("Speed", visitor)?;

        visitor.leave_region()
    }
}

impl Default for NavmeshAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl NavmeshAgent {
    /// Creates new navigation mesh agent.
    pub fn new() -> Self {
        Self {
            path: vec![],
            current: 0,
            position: Default::default(),
            last_target_position: Default::default(),
            recalculation_threshold: 0.25,
            speed: 1.5,
        }
    }

    /// Returns agent's position.
    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    /// Returns agent's path that will be followed.
    pub fn path(&self) -> &[Vector3<f32>] {
        &self.path
    }

    /// Sets new speed of agent's movement.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    /// Returns current agent's movement speed.
    pub fn speed(&self) -> f32 {
        self.speed
    }
}

fn closest_point_index_in_triangle_and_adjacent(
    triangle: TriangleDefinition,
    navmesh: &Navmesh,
    to: Vector3<f32>,
) -> Option<usize> {
    let mut triangles = ArrayVec::<[TriangleDefinition; 4]>::new();
    triangles.push(triangle);
    math::get_closest_point_triangle_set(&navmesh.pathfinder.vertices(), &triangles, to)
}

/// Projects a point on a plane and returns a projection and distance from the point to the plane.
fn project(
    point: Vector3<f32>,
    plane_point: Vector3<f32>,
    plane_normal: Vector3<f32>,
) -> (Vector3<f32>, f32) {
    if let Some(normal) = plane_normal.try_normalize(std::f32::EPSILON) {
        let distance = (point - plane_point).dot(&normal);
        (point - normal.scale(distance), distance)
    } else {
        (point, 0.0)
    }
}

impl NavmeshAgent {
    /// Calculates path from point A to point B. In most cases there is no need to use this method
    /// directly, because `update` will call it anyway if target position has moved.
    pub fn calculate_path(
        &mut self,
        navmesh: &mut Navmesh,
        from: Vector3<f32>,
        to: Vector3<f32>,
    ) -> Result<PathKind, PathError> {
        self.path.clear();

        self.last_target_position = to;
        self.current = 0;

        let (n_from, begin, from_triangle) = if let Some((point, index, triangle)) = navmesh
            .ray_cast(Ray::new(
                from + Vector3::new(0.0, 1.0, 0.0),
                Vector3::new(0.0, -10.0, 0.0),
            )) {
            (
                closest_point_index_in_triangle_and_adjacent(triangle, navmesh, to),
                Some(point),
                Some(index),
            )
        } else {
            (navmesh.query_closest(from), None, None)
        };

        let (n_to, end, to_triangle) = if let Some((point, index, triangle)) =
            navmesh.ray_cast(Ray::new(
                to + Vector3::new(0.0, 1.0, 0.0),
                Vector3::new(0.0, -10.0, 0.0),
            )) {
            (
                closest_point_index_in_triangle_and_adjacent(triangle, navmesh, from),
                Some(point),
                Some(index),
            )
        } else {
            (navmesh.query_closest(to), None, None)
        };

        if let (Some(from_triangle), Some(to_triangle)) = (from_triangle, to_triangle) {
            if from_triangle == to_triangle {
                self.path.push(from);
                self.path.push(to);

                return Ok(PathKind::Full);
            }
        }

        if let (Some(n_from), Some(n_to)) = (n_from, n_to) {
            let result = navmesh.build_path(n_from, n_to, &mut self.path);

            if let Some(end) = end {
                if self.path.is_empty() {
                    self.path.push(end);
                } else {
                    self.path.insert(0, end)
                }
            }

            if let Some(begin) = begin {
                self.path.push(begin);
            }

            self.path.reverse();

            // Refine path.
            // TODO: Optimize.
            for _ in 0..2 {
                let mut i = 0;
                while i < self.path.len().saturating_sub(2) {
                    let begin = self.path[i];
                    let end = self.path[i + 2];
                    let center = (begin + end).scale(0.5);
                    let delta = end - begin;

                    let max_delta = delta.x.max(delta.y).max(delta.z);

                    for triangle in navmesh.triangles.iter() {
                        let a = navmesh.pathfinder.vertices()[triangle[0] as usize].position;
                        let b = navmesh.pathfinder.vertices()[triangle[1] as usize].position;
                        let c = navmesh.pathfinder.vertices()[triangle[2] as usize].position;

                        let normal = (c - a).cross(&(b - a));

                        let (projection, distance) = project(center, a, normal);

                        if distance.abs() <= max_delta
                            && math::is_point_inside_triangle(&projection, &[a, b, c])
                        {
                            self.path[i + 1] = projection;
                            break;
                        }
                    }

                    i += 1;
                }
            }

            result
        } else {
            Err(PathError::Custom("Empty navmesh!".to_owned()))
        }
    }

    /// Performs single update tick that moves agent to the target along the path (which is automatically
    /// recalculated if target's position has changed).
    pub fn update(
        &mut self,
        dt: f32,
        navmesh: &mut Navmesh,
        target: Vector3<f32>,
    ) -> Result<PathKind, PathError> {
        if target.metric_distance(&self.last_target_position) >= self.recalculation_threshold {
            self.calculate_path(navmesh, self.position, target)?;
        }

        if let Some(source) = self.path.get(self.current as usize) {
            if let Some(destination) = self.path.get((self.current + 1) as usize) {
                let ray = Ray::from_two_points(*source, *destination);
                let d = ray.dir.try_normalize(std::f32::EPSILON).unwrap_or_default();
                self.position += d.scale(self.speed * dt);
                if ray.project_point(&self.position) >= 1.0 {
                    self.current += 1;
                }
            }
        }

        Ok(PathKind::Full)
    }
}
