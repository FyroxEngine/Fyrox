//! Contains all structures and methods to create and manage navigation meshes (navmesh).
//!
//! Navigation mesh is a set of convex polygons which is used for path finding in complex
//! environment.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::{Point3, Vector3},
        arrayvec::ArrayVec,
        math::{self, ray::Ray, PositionProvider, TriangleDefinition},
        octree::{Octree, OctreeNode},
        pool::Handle,
        reflect::prelude::*,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::mesh::{
        buffer::{VertexAttributeUsage, VertexReadTrait},
        Mesh,
    },
    utils::{
        astar::{Graph, GraphVertex, PathError, PathKind, VertexData, VertexDataProvider},
        raw_mesh::{RawMeshBuilder, RawVertex},
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, Default, Visit)]
struct Vertex {
    triangle_index: usize,
    data: VertexData,
}

impl Deref for Vertex {
    type Target = VertexData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Vertex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl PositionProvider for Vertex {
    fn position(&self) -> Vector3<f32> {
        self.data.position
    }
}

impl VertexDataProvider for Vertex {}

/// See module docs.
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(hide_all)]
pub struct Navmesh {
    octree: Octree,
    triangles: Vec<TriangleDefinition>,
    vertices: Vec<Vector3<f32>>,
    graph: Option<Graph<Vertex>>,
    query_buffer: Vec<u32>,
}

impl PartialEq for Navmesh {
    fn eq(&self, other: &Self) -> bool {
        self.triangles == other.triangles && self.vertices == other.vertices
    }
}

impl Visit for Navmesh {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        // Backward compatibility.
        if region.is_reading() {
            let mut pathfinder = Graph::<GraphVertex>::new();
            if pathfinder.visit("PathFinder", &mut region).is_ok() {
                self.vertices = pathfinder
                    .vertices
                    .iter()
                    .map(|v| v.position)
                    .collect::<Vec<_>>();
            } else {
                self.vertices.visit("Vertices", &mut region)?;
            }
        } else {
            self.vertices.visit("Vertices", &mut region)?;
        }

        self.triangles.visit("Triangles", &mut region)?;

        drop(region);

        // No need to save octree, we can restore it on load.
        if visitor.is_reading() {
            let raw_triangles = self
                .triangles
                .iter()
                .map(|t| {
                    [
                        self.vertices[t[0] as usize],
                        self.vertices[t[1] as usize],
                        self.vertices[t[2] as usize],
                    ]
                })
                .collect::<Vec<[Vector3<f32>; 3]>>();

            self.octree = Octree::new(&raw_triangles, 32);
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
struct Portal {
    left: usize,
    right: usize,
}
fn signed_triangle_area_2d(a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>) -> f32 {
    let abx = b[0] - a[0];
    let abz = b[2] - a[2];
    let acx = c[0] - a[0];
    let acz = c[2] - a[2];
    acx * abz - abx * acz
}

#[derive(PartialEq, Clone, Copy, Eq)]
enum Winding {
    Clockwise,
    CounterClockwise,
}

fn winding(a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>) -> Winding {
    if signed_triangle_area_2d(a, b, c) > 0.0 {
        Winding::Clockwise
    } else {
        Winding::CounterClockwise
    }
}

fn make_graph(triangles: &[TriangleDefinition], vertices: &[Vector3<f32>]) -> Graph<Vertex> {
    let mut graph = Graph::new();

    // Add vertices at the center of each triangle.
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        let a = vertices[triangle[0] as usize];
        let b = vertices[triangle[1] as usize];
        let c = vertices[triangle[2] as usize];

        let center = (a + b + c).scale(1.0 / 3.0);
        graph.add_vertex(Vertex {
            triangle_index,
            data: VertexData::new(center),
        });
    }

    // Link vertices.
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        // Vertex index in the graph matches triangle index.
        let vertex_index = triangle_index;

        for edge in triangle.edges() {
            for (other_triangle_index, other_triangle) in triangles.iter().enumerate() {
                if triangle_index != other_triangle_index {
                    // Vertex index in the graph matches triangle index.
                    let other_vertex_index = other_triangle_index;

                    'inner_edge_loop: for other_edge in other_triangle.edges() {
                        if edge == other_edge {
                            graph.link_bidirect(vertex_index, other_vertex_index);
                            break 'inner_edge_loop;
                        }
                    }
                }
            }
        }
    }

    graph
}

impl Navmesh {
    /// Creates new navigation mesh from given set of triangles and vertices. This is
    /// low level method that allows to specify triangles and vertices directly. In
    /// most cases you should use `from_mesh` method.
    pub fn new(triangles: Vec<TriangleDefinition>, vertices: Vec<Vector3<f32>>) -> Self {
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

        Self {
            graph: None,
            triangles,
            vertices,
            octree: Octree::new(&raw_triangles, 32),
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
    /// use fyrox::scene::Scene;
    /// use fyrox::utils::navmesh::Navmesh;
    ///
    /// fn make_navmesh(scene: &Scene, navmesh_name: &str) -> Navmesh {
    ///     // Find mesh node in existing scene and create navigation mesh from it.
    ///     let navmesh_node_handle = scene.graph.find_by_name_from_root(navmesh_name).unwrap().0;
    ///     Navmesh::from_mesh(scene.graph[navmesh_node_handle].as_mesh())
    /// }
    /// ```
    pub fn from_mesh(mesh: &Mesh) -> Self {
        // Join surfaces into one simple mesh.
        let mut builder = RawMeshBuilder::<RawVertex>::default();
        let global_transform = mesh.global_transform();
        for surface in mesh.surfaces() {
            let shared_data = surface.data();
            let shared_data = shared_data.lock();

            let vertex_buffer = &shared_data.vertex_buffer;
            for triangle in shared_data.geometry_buffer.iter() {
                builder.insert(RawVertex::from(
                    global_transform
                        .transform_point(&Point3::from(
                            vertex_buffer
                                .get(triangle[0] as usize)
                                .unwrap()
                                .read_3_f32(VertexAttributeUsage::Position)
                                .unwrap(),
                        ))
                        .coords,
                ));
                builder.insert(RawVertex::from(
                    global_transform
                        .transform_point(&Point3::from(
                            vertex_buffer
                                .get(triangle[1] as usize)
                                .unwrap()
                                .read_3_f32(VertexAttributeUsage::Position)
                                .unwrap(),
                        ))
                        .coords,
                ));
                builder.insert(RawVertex::from(
                    global_transform
                        .transform_point(&Point3::from(
                            vertex_buffer
                                .get(triangle[2] as usize)
                                .unwrap()
                                .read_3_f32(VertexAttributeUsage::Position)
                                .unwrap(),
                        ))
                        .coords,
                ));
            }
        }

        let mesh = builder.build();

        Navmesh::new(
            mesh.triangles,
            mesh.vertices
                .into_iter()
                .map(|v| Vector3::new(v.x, v.y, v.z))
                .collect::<Vec<_>>(),
        )
    }

    /// Searches for a closest graph vertex to the given point. Returns Some((vertex_index, triangle_index)), or None
    /// if navmesh was empty.
    pub fn query_closest(&mut self, point: Vector3<f32>) -> Option<(usize, usize)> {
        self.octree.point_query(point, &mut self.query_buffer);
        if self.query_buffer.is_empty() {
            math::get_closest_point_triangle_set(&self.vertices, &self.triangles, point)
        } else {
            math::get_closest_point_triangles(
                &self.vertices,
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

    /// Adds the triangle to the navigational mesh and returns its index in the internal array. Vertex indices in
    /// the triangle must be valid!
    pub fn add_triangle(&mut self, triangle: TriangleDefinition) -> u32 {
        let index = self.triangles.len();
        self.graph = None;
        self.triangles.push(triangle);
        index as u32
    }

    /// Removes a triangle at the given index from the navigational mesh.
    pub fn remove_triangle(&mut self, index: usize) -> TriangleDefinition {
        let triangle = self.triangles.remove(index);
        self.graph = None;
        triangle
    }

    /// Removes last triangle from the navigational mesh. Automatically fixes vertex links in the internal
    /// navigational graph.
    pub fn pop_triangle(&mut self) -> Option<TriangleDefinition> {
        if self.triangles.is_empty() {
            None
        } else {
            Some(self.remove_triangle(self.triangles.len() - 1))
        }
    }

    /// Removes a vertex at the given index from the navigational mesh. All triangles that share the vertex will
    /// be also removed.
    pub fn remove_vertex(&mut self, index: usize) -> Vector3<f32> {
        // Remove triangles that sharing the vertex first.
        let mut i = 0;
        while i < self.triangles.len() {
            if self.triangles[i].indices().contains(&(index as u32)) {
                self.remove_triangle(i);
            } else {
                i += 1;
            }
        }

        // Shift vertex indices in triangles. Example:
        //
        // 0:A 1:B 2:C 3:D 4:E
        // [A,B,C], [A,C,D], [A,D,E], [D,C,E]
        // [0,1,2], [0,2,3], [0,3,4], [3,2,4]
        //
        // Remove B.
        //
        // 0:A 1:C 2:D 3:E
        // [A,C,D], [A,D,E], [D,C,E]
        // [0,1,2], [0,2,3], [2,1,3]
        for triangle in self.triangles.iter_mut() {
            for other_vertex_index in triangle.indices_mut() {
                if *other_vertex_index > index as u32 {
                    *other_vertex_index -= 1;
                }
            }
        }

        self.vertices.remove(index)
    }

    /// Returns reference to the internal array of vertices.
    pub fn vertices(&self) -> &[Vector3<f32>] {
        &self.vertices
    }

    /// Returns a mutable reference to the internal array of vertices.
    pub fn vertices_mut(&mut self) -> &mut [Vector3<f32>] {
        self.graph = None;
        &mut self.vertices
    }

    /// Adds the vertex to the navigational mesh. The vertex will **not** be connected with any other vertex.
    pub fn add_vertex(&mut self, vertex: Vector3<f32>) -> u32 {
        let index = self.vertices.len();
        self.vertices.push(vertex);
        self.graph = None;
        index as u32
    }

    /// Removes last vertex from the navigational mesh. All triangles that share the vertex will be also removed.
    pub fn pop_vertex(&mut self) -> Option<Vector3<f32>> {
        if self.vertices.is_empty() {
            None
        } else {
            Some(self.remove_vertex(self.vertices.len() - 1))
        }
    }

    /// Inserts the vertex at the given index. Automatically shift indices in triangles to preserve mesh structure.
    pub fn insert_vertex(&mut self, index: u32, vertex: Vector3<f32>) {
        self.vertices.insert(index as usize, vertex);

        // Shift vertex indices in triangles. Example:
        //
        // 0:A 1:C 2:D 3:E
        // [A,C,D], [A,D,E], [D,C,E]
        // [0,1,2], [0,2,3], [2,1,3]
        //
        // Insert B.
        //
        // 0:A 1:B 2:C 3:D 4:E
        // [A,C,D], [A,D,E], [D,C,E]
        // [0,2,3], [0,3,4], [3,2,4]
        for triangle in self.triangles.iter_mut() {
            for other_vertex_index in triangle.indices_mut() {
                if *other_vertex_index >= index {
                    *other_vertex_index += 1;
                }
            }
        }

        self.graph = None;
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
    /// use fyrox::utils::navmesh::Navmesh;
    /// use fyrox::core::algebra::Vector3;
    /// use fyrox::utils::astar::{PathKind, PathError};
    ///
    /// fn find_path(navmesh: &mut Navmesh, begin: Vector3<f32>, end: Vector3<f32>, path: &mut Vec<Vector3<f32>>) -> Result<PathKind, PathError> {
    ///     if let Some(begin_index) = navmesh.query_closest(begin) {
    ///         if let Some(end_index) = navmesh.query_closest(end) {
    ///             return navmesh.build_path(begin_index.0, end_index.0, path);
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
        let graph = self
            .graph
            .get_or_insert_with(|| make_graph(&self.triangles, &self.vertices));
        graph.build(from, to, path)
    }

    /// Tries to pick a triangle by given ray. Returns closest result.
    pub fn ray_cast(&self, ray: Ray) -> Option<(Vector3<f32>, usize)> {
        let mut buffer = ArrayVec::<Handle<OctreeNode>, 128>::new();

        self.octree.ray_query_static(&ray, &mut buffer);

        let mut closest_distance = f32::MAX;
        let mut result = None;
        for node in buffer.into_iter() {
            if let OctreeNode::Leaf { indices, .. } = self.octree.node(node) {
                for &index in indices {
                    let triangle = self.triangles[index as usize];
                    let a = self.vertices()[triangle[0] as usize];
                    let b = self.vertices()[triangle[1] as usize];
                    let c = self.vertices()[triangle[2] as usize];

                    if let Some(intersection) = ray.triangle_intersection_point(&[a, b, c]) {
                        let distance = intersection.metric_distance(&ray.origin);
                        if distance < closest_distance {
                            closest_distance = distance;
                            result = Some((intersection, index as usize));
                        }
                    }
                }
            } else {
                unreachable!()
            }
        }

        result
    }

    fn portal_between(&self, src_triangle: usize, dest_triangle: usize) -> Option<Portal> {
        let src_triangle = self.triangles.get(src_triangle)?;
        let dest_triangle = self.triangles.get(dest_triangle)?;
        for src_triangle_edge in src_triangle.edges() {
            for dest_triangle_edge in dest_triangle.edges() {
                if src_triangle_edge == dest_triangle_edge {
                    let a = self.vertices[src_triangle[0] as usize];
                    let b = self.vertices[src_triangle[1] as usize];
                    let c = self.vertices[src_triangle[2] as usize];

                    return if winding(a, b, c) == Winding::Clockwise {
                        Some(Portal {
                            left: src_triangle_edge.a as usize,
                            right: src_triangle_edge.b as usize,
                        })
                    } else {
                        Some(Portal {
                            left: src_triangle_edge.b as usize,
                            right: src_triangle_edge.a as usize,
                        })
                    };
                }
            }
        }
        None
    }
}

/// Navmesh agent is a "pathfinding unit" that performs navigation on a mesh. It is designed to
/// cover most of simple use cases when you need to build and follow some path from point A to point B.
#[derive(Visit, Clone, Debug)]
pub struct NavmeshAgent {
    path: Vec<Vector3<f32>>,
    current: u32,
    position: Vector3<f32>,
    last_warp_position: Vector3<f32>,
    target: Vector3<f32>,
    last_target_position: Vector3<f32>,
    recalculation_threshold: f32,
    speed: f32,
    path_dirty: bool,
    #[visit(optional)]
    radius: f32,
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
            last_warp_position: Default::default(),
            target: Default::default(),
            last_target_position: Default::default(),
            recalculation_threshold: 0.25,
            speed: 1.5,
            path_dirty: true,
            radius: 0.2,
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

    /// Sets a new radius for the navmesh agent. The agent will use this radius to walk around
    /// corners with the distance equal to the radius. This could help to prevent the agent from
    /// being stuck in the corners. The default value is 0.2 meters.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    /// Returns the current radius of the navmesh agent. See [`Self::set_radius`] for more info
    /// about radius parameter.
    pub fn radius(&self) -> f32 {
        self.radius
    }
}

fn query_data(navmesh: &mut Navmesh, query_point: Vector3<f32>) -> Option<(Vector3<f32>, usize)> {
    if let Some((point, triangle_index)) = navmesh.ray_cast(Ray::new(
        query_point + Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, -10.0, 0.0),
    )) {
        Some((point, triangle_index))
    } else if let Some((point_index, triangle_index)) = navmesh.query_closest(query_point) {
        Some((navmesh.vertices()[point_index], triangle_index))
    } else {
        None
    }
}

impl NavmeshAgent {
    /// Calculates path from point A to point B. In most cases there is no need to use this method
    /// directly, because `update` will call it anyway if target position has moved.
    pub fn calculate_path(
        &mut self,
        navmesh: &mut Navmesh,
        src_point: Vector3<f32>,
        dest_point: Vector3<f32>,
    ) -> Result<PathKind, PathError> {
        self.path.clear();

        self.current = 0;

        if let Some((src_point_on_navmesh, src_triangle)) = query_data(navmesh, src_point) {
            if let Some((dest_point_on_navmesh, dest_triangle)) = query_data(navmesh, dest_point) {
                if src_triangle == dest_triangle {
                    self.path.push(src_point_on_navmesh);
                    self.path.push(dest_point_on_navmesh);

                    return Ok(PathKind::Full);
                }

                let mut path_triangle_indices = Vec::new();
                let graph = navmesh
                    .graph
                    .get_or_insert_with(|| make_graph(&navmesh.triangles, &navmesh.vertices));
                let path_kind = graph.build_and_convert(
                    src_triangle,
                    dest_triangle,
                    &mut self.path,
                    |idx, v| {
                        path_triangle_indices.push(idx);
                        v.position
                    },
                )?;

                self.path.insert(0, dest_point_on_navmesh);
                self.path.push(src_point_on_navmesh);

                self.path.reverse();
                path_triangle_indices.reverse();

                self.straighten_path(
                    navmesh,
                    src_point_on_navmesh,
                    dest_point_on_navmesh,
                    &path_triangle_indices,
                );

                return Ok(path_kind);
            }
        }

        Err(PathError::Custom("Empty navmesh!".to_owned()))
    }

    fn straighten_path(
        &mut self,
        navmesh: &Navmesh,
        src_position: Vector3<f32>,
        dest_position: Vector3<f32>,
        path_triangles: &[usize],
    ) {
        self.path.clear();
        self.path.push(src_position);

        if path_triangles.len() > 1 {
            let mut portal_apex = src_position;
            let mut portal_left = src_position;
            let mut portal_right = src_position;

            let mut left_index = 0;
            let mut right_index = 0;

            let mut i = 0;
            while i < path_triangles.len() {
                let (left, right) = if i + 1 < path_triangles.len() {
                    let portal = navmesh
                        .portal_between(path_triangles[i], path_triangles[i + 1])
                        .unwrap();

                    let mut left = navmesh.vertices[portal.left];
                    let mut right = navmesh.vertices[portal.right];

                    if self.radius > 0.0 {
                        let delta = right - left;
                        let len = delta.norm();
                        let offset = delta.scale(self.radius.min(len * 0.5) / len);

                        left += offset;
                        right -= offset;
                    }

                    (left, right)
                } else {
                    (dest_position, dest_position)
                };

                // Update right vertex.
                if signed_triangle_area_2d(portal_apex, portal_right, right) <= 0.0 {
                    if portal_apex == portal_right
                        || signed_triangle_area_2d(portal_apex, portal_left, right) > 0.0
                    {
                        // Tighten the funnel.
                        portal_right = right;
                        right_index = i;
                    } else {
                        // Right over left, insert left to path and restart scan from portal left point.
                        self.path.push(portal_left);
                        // Make current left the new apex.
                        portal_apex = portal_left;
                        // Reset portal
                        portal_left = portal_apex;
                        portal_right = portal_apex;
                        right_index = left_index;
                        // Restart scan
                        i = left_index + 1;
                        continue;
                    }
                }

                // Update left vertex.
                if signed_triangle_area_2d(portal_apex, portal_left, left) >= 0.0 {
                    if portal_apex == portal_left
                        || signed_triangle_area_2d(portal_apex, portal_right, left) < 0.0
                    {
                        // Tighten the funnel.
                        portal_left = left;
                        left_index = i;
                    } else {
                        // Left over right, insert right to path and restart scan from portal right point.
                        self.path.push(portal_right);
                        // Make current right the new apex.
                        portal_apex = portal_right;
                        // Reset portal
                        portal_left = portal_apex;
                        portal_right = portal_apex;
                        left_index = right_index;
                        // Restart scan
                        i = right_index + 1;
                        continue;
                    }
                }

                i += 1;
            }
        }

        self.path.push(dest_position);
    }

    /// Performs single update tick that moves agent to the target along the path (which is automatically
    /// recalculated if target's position has changed).
    pub fn update(&mut self, dt: f32, navmesh: &mut Navmesh) -> Result<PathKind, PathError> {
        if self.path_dirty {
            self.calculate_path(navmesh, self.position, self.target)?;
            self.path_dirty = false;
        }

        if let Some(source) = self.path.get(self.current as usize) {
            if let Some(destination) = self.path.get((self.current + 1) as usize) {
                let ray = Ray::from_two_points(*source, *destination);
                let d = ray.dir.try_normalize(f32::EPSILON).unwrap_or_default();
                self.position += d.scale(self.speed * dt);
                if ray.project_point(&self.position) >= 1.0 {
                    self.current += 1;
                }
            }
        }

        Ok(PathKind::Full)
    }

    /// Returns current steering target which in most cases next path point from which
    /// agent is close to.
    pub fn steering_target(&self) -> Option<Vector3<f32>> {
        self.path
            .get(self.current as usize + 1)
            .or_else(|| self.path.last())
            .cloned()
    }

    /// Sets new target for the agent.
    pub fn set_target(&mut self, new_target: Vector3<f32>) {
        if new_target.metric_distance(&self.last_target_position) >= self.recalculation_threshold {
            self.path_dirty = true;
            self.last_target_position = new_target;
        }

        self.target = new_target;
    }

    /// Returns current target of the agent.
    pub fn target(&self) -> Vector3<f32> {
        self.target
    }

    /// Sets new position of the agent.
    pub fn set_position(&mut self, new_position: Vector3<f32>) {
        if new_position.metric_distance(&self.last_warp_position) >= self.recalculation_threshold {
            self.path_dirty = true;
            self.last_warp_position = new_position;
        }

        self.position = new_position;
    }
}

/// Allows you to build agent in declarative manner.
pub struct NavmeshAgentBuilder {
    position: Vector3<f32>,
    target: Vector3<f32>,
    recalculation_threshold: f32,
    speed: f32,
}

impl Default for NavmeshAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NavmeshAgentBuilder {
    /// Creates new builder instance.
    pub fn new() -> Self {
        Self {
            position: Default::default(),
            target: Default::default(),
            recalculation_threshold: 0.25,
            speed: 1.5,
        }
    }

    /// Sets new desired position of the agent being built.
    pub fn with_position(mut self, position: Vector3<f32>) -> Self {
        self.position = position;
        self
    }

    /// Sets new desired target of the agent being built.
    pub fn with_target(mut self, position: Vector3<f32>) -> Self {
        self.target = position;
        self
    }

    /// Sets new desired recalculation threshold (in meters) of the agent being built.
    pub fn with_recalculation_threshold(mut self, threshold: f32) -> Self {
        self.recalculation_threshold = threshold;
        self
    }

    /// Sets new desired movement speed of the agent being built.
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Build the agent.
    pub fn build(self) -> NavmeshAgent {
        NavmeshAgent {
            position: self.position,
            last_warp_position: self.position,
            target: self.target,
            last_target_position: self.target,
            recalculation_threshold: self.recalculation_threshold,
            speed: self.speed,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::{algebra::Vector3, math::TriangleDefinition},
        utils::navmesh::{Navmesh, NavmeshAgent},
    };

    #[test]
    fn test_navmesh() {
        let mut navmesh = Navmesh::new(
            vec![
                TriangleDefinition([0, 1, 3]),
                TriangleDefinition([1, 2, 3]),
                TriangleDefinition([2, 5, 3]),
                TriangleDefinition([2, 4, 5]),
                TriangleDefinition([4, 7, 5]),
                TriangleDefinition([4, 6, 7]),
            ],
            vec![
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, 1.0),
                Vector3::new(1.0, 0.0, 1.0),
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(2.0, 0.0, 1.0),
                Vector3::new(2.0, 0.0, 0.0),
                Vector3::new(3.0, 0.0, 1.0),
                Vector3::new(3.0, 0.0, 0.0),
            ],
        );

        let mut agent = NavmeshAgent::new();

        agent.set_target(Vector3::new(3.0, 0.0, 1.0));
        agent.update(1.0 / 60.0, &mut navmesh).unwrap();

        let graph = navmesh.graph.as_ref().unwrap();

        assert_eq!(graph.vertices.len(), 6);
        assert_eq!(graph.vertices[0].neighbours[0], 1);

        assert_eq!(graph.vertices[1].neighbours[0], 0);
        assert_eq!(graph.vertices[1].neighbours[1], 2);

        assert_eq!(graph.vertices[2].neighbours[0], 1);
        assert_eq!(graph.vertices[2].neighbours[1], 3);

        assert_eq!(graph.vertices[3].neighbours[0], 2);
        assert_eq!(graph.vertices[3].neighbours[1], 4);

        assert_eq!(graph.vertices[4].neighbours[0], 3);
        assert_eq!(graph.vertices[4].neighbours[1], 5);

        assert_eq!(graph.vertices[5].neighbours[0], 4);

        assert_eq!(
            agent.path,
            vec![
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(3.0, 0.0, 1.0),
                Vector3::new(3.0, 0.0, 1.0)
            ]
        );
    }
}
