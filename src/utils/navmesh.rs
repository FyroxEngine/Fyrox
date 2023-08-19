//! Contains all structures and methods to create and manage navigation meshes (navmesh).
//!
//! Navigation mesh is a set of convex polygons which is used for path finding in complex
//! environment.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::{Point3, Vector3},
        arrayvec::ArrayVec,
        math::{self, ray::Ray, TriangleDefinition},
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
        astar::{PathError, PathFinder, PathKind, PathVertex},
        raw_mesh::{RawMeshBuilder, RawVertex},
    },
};
use fxhash::FxHashSet;

/// See module docs.
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(hide_all)]
pub struct Navmesh {
    octree: Octree,
    triangles: Vec<TriangleDefinition>,
    pathfinder: PathFinder,
    query_buffer: Vec<u32>,
}

impl PartialEq for Navmesh {
    fn eq(&self, other: &Self) -> bool {
        self.triangles == other.triangles && self.pathfinder == other.pathfinder
    }
}

impl Visit for Navmesh {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.pathfinder.visit("PathFinder", &mut region)?;
        self.triangles.visit("Triangles", &mut region)?;

        drop(region);

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

        Ok(())
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

        let mut edges = FxHashSet::default();
        for triangle in triangles {
            for edge in triangle.edges() {
                edges.insert(edge);
            }
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

    /// Adds the triangle to the navigational mesh and returns its index in the internal array. Vertex indices in
    /// the triangle must be valid!
    pub fn add_triangle(&mut self, triangle: TriangleDefinition) -> u32 {
        let index = self.triangles.len();
        for edge in triangle.edges() {
            self.pathfinder
                .link_bidirect(edge.a as usize, edge.b as usize);
        }
        self.triangles.push(triangle);
        index as u32
    }

    /// Removes a triangle at the given index from the navigational mesh. Automatically fixes vertex links in the
    /// internal navigational graph.
    pub fn remove_triangle(&mut self, index: usize) -> TriangleDefinition {
        let triangle = self.triangles.remove(index);
        for &vertex_index in triangle.indices() {
            let mut isolated = true;
            for other_triangle in self.triangles.iter() {
                if other_triangle.indices().contains(&vertex_index) {
                    isolated = false;
                    break;
                }
            }

            if isolated {
                if let Some(vertex) = self.pathfinder.vertex_mut(vertex_index as usize) {
                    let neighbour_indices = vertex.neighbours.clone();
                    vertex.neighbours.clear();

                    for neighbour_index in neighbour_indices {
                        if let Some(neighbour_vertex) =
                            self.pathfinder.vertex_mut(neighbour_index as usize)
                        {
                            if let Some(position) = neighbour_vertex
                                .neighbours
                                .iter()
                                .position(|n| *n == vertex_index)
                            {
                                neighbour_vertex.neighbours.remove(position);
                            }
                        }
                    }
                }
            }
        }
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
    pub fn remove_vertex(&mut self, index: usize) -> PathVertex {
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

        self.pathfinder.remove_vertex(index)
    }

    /// Returns reference to the internal array of vertices.
    pub fn vertices(&self) -> &[PathVertex] {
        self.pathfinder.vertices()
    }

    /// Returns a mutable reference to the internal array of vertices.
    pub fn vertices_mut(&mut self) -> &mut [PathVertex] {
        self.pathfinder.vertices_mut()
    }

    /// Adds the vertex to the navigational mesh. The vertex will **not** be connected with any other vertex.
    pub fn add_vertex(&mut self, vertex: PathVertex) -> u32 {
        self.pathfinder.add_vertex(vertex)
    }

    /// Removes last vertex from the navigational mesh. All triangles that share the vertex will be also removed.
    pub fn pop_vertex(&mut self) -> Option<PathVertex> {
        if self.pathfinder.vertices().is_empty() {
            None
        } else {
            Some(self.remove_vertex(self.pathfinder.vertices().len() - 1))
        }
    }

    /// Inserts the vertex at the given index. Automatically shift indices in triangles to preserve mesh structure.
    pub fn insert_vertex(&mut self, index: u32, vertex: PathVertex) {
        self.pathfinder.insert_vertex(index, vertex);

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

    /// Tries to pick a triangle by given ray. Returns closest result.
    pub fn ray_cast(&self, ray: Ray) -> Option<(Vector3<f32>, usize, TriangleDefinition)> {
        let mut buffer = ArrayVec::<Handle<OctreeNode>, 128>::new();

        self.octree.ray_query_static(&ray, &mut buffer);

        let mut closest_distance = f32::MAX;
        let mut result = None;
        for node in buffer.into_iter() {
            if let OctreeNode::Leaf { indices, .. } = self.octree.node(node) {
                for &index in indices {
                    let triangle = self.triangles[index as usize].clone();
                    let a = self.pathfinder.vertices()[triangle[0] as usize].position;
                    let b = self.pathfinder.vertices()[triangle[1] as usize].position;
                    let c = self.pathfinder.vertices()[triangle[2] as usize].position;

                    if let Some(intersection) = ray.triangle_intersection_point(&[a, b, c]) {
                        let distance = intersection.metric_distance(&ray.origin);
                        if distance < closest_distance {
                            closest_distance = distance;
                            result = Some((intersection, index as usize, triangle));
                        }
                    }
                }
            } else {
                unreachable!()
            }
        }

        result
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
    let mut triangles = ArrayVec::<TriangleDefinition, 4>::new();
    triangles.push(triangle);
    math::get_closest_point_triangle_set(navmesh.pathfinder.vertices(), &triangles, to)
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
            let mut path_vertex_indices = Vec::new();
            let result =
                navmesh
                    .pathfinder
                    .build_and_convert(n_from, n_to, &mut self.path, |idx, v| {
                        path_vertex_indices.push(idx);
                        v.position
                    });

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
            path_vertex_indices.reverse();

            // Perform few smoothing passes to straighten computed path.
            for _ in 0..2 {
                self.smooth_path(navmesh, &path_vertex_indices);
            }

            result
        } else {
            Err(PathError::Custom("Empty navmesh!".to_owned()))
        }
    }

    fn smooth_path(&mut self, navmesh: &Navmesh, path_vertex_indices: &[usize]) {
        let vertices = navmesh.vertices();

        let dn = (self.path.len() - path_vertex_indices.len()).clamp(0, 1);
        let mut i = 0;
        while i < self.path.len().saturating_sub(2) {
            let begin = self.path[i];
            let end = self.path[i + 2];
            let delta = end - begin;

            let max_delta = (delta.x.max(delta.y).max(delta.z)).abs();

            // Calculate center point between end points of each path edge.
            //     i+1
            //      ^
            //     / \
            //    /   \
            //   /     \
            //  /-  *  -\
            // i    C   i+2
            let center = (begin + end).scale(0.5);

            // Get the normal vector.
            let normal = center - self.path[i + 1];

            // Start "nudging" loop - we start from the center and nudging it towards the middle point until it
            // lies on one of the triangles along the path.
            //
            // TODO: This algorithm can cut corners for some cases, which means that the path could lie off the
            // navmesh. It is a bug which should be fixed.
            let mut k = 1.0;
            'nudge_loop: while k >= -0.1 {
                let probe = self.path[i + 1] + normal.scale(k);
                // And check if center is lying on navmesh or not. If so - replace i+1 vertex
                // with its projection on the triangle it belongs to.
                for triangle in navmesh.triangles.iter() {
                    // Check if the triangle is one of the triangles along the path starting from the beginning point
                    // of the current triple of points.
                    if triangle.0.iter().any(|idx| {
                        path_vertex_indices[i.saturating_sub(dn)..].contains(&(*idx as usize))
                    }) {
                        let a = vertices[triangle[0] as usize].position;
                        let b = vertices[triangle[1] as usize].position;
                        let c = vertices[triangle[2] as usize].position;

                        // Ignore degenerated triangles.
                        if let Some(normal) = (c - a).cross(&(b - a)).try_normalize(f32::EPSILON) {
                            // Calculate signed distance between triangle and segment's center.
                            let signed_distance = (probe - a).dot(&normal);

                            // And check "slope": If probe is too far from triangle, check next triangle.
                            if signed_distance.abs() <= max_delta {
                                // Project probe on the triangle.
                                let probe_projection = probe - normal.scale(signed_distance);

                                // And check if the projection lies inside the triangle.
                                if math::is_point_inside_triangle(&probe_projection, &[a, b, c]) {
                                    self.path[i + 1] = probe_projection;
                                    break 'nudge_loop;
                                }
                            }
                        }
                    }
                }
                k -= 0.1;
            }

            i += 1;
        }
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
        utils::navmesh::Navmesh,
    };

    fn make_navmesh() -> Navmesh {
        //             0                 1
        //              *---------------*
        //            / | \       A     |
        //           /  |     \         |
        //          /   |   B     \     |
        //         /    |             \ |
        //        /   3 *---------------* 2
        //       / C  /                /
        //      /   /    D      /
        //     /  /      /
        //    / /   /
        //   //
        //    4
        Navmesh::new(
            &[
                TriangleDefinition([0, 1, 2]),
                TriangleDefinition([0, 2, 3]),
                TriangleDefinition([0, 3, 4]),
                TriangleDefinition([3, 2, 4]),
            ],
            &[
                Vector3::new(-1.0, 0.0, 1.0),
                Vector3::new(1.0, 0.0, 1.0),
                Vector3::new(1.0, 0.0, -1.0),
                Vector3::new(-1.0, 0.0, -1.0),
                Vector3::new(-2.0, 0.0, 2.0),
            ],
        )
    }

    #[test]
    fn test_remove_triangle() {
        let mut navmesh = make_navmesh();

        assert_eq!(navmesh.vertices()[0].neighbours, vec![4, 1, 2, 3]);
        assert_eq!(navmesh.vertices()[1].neighbours, vec![2, 0]);
        assert_eq!(navmesh.vertices()[2].neighbours, vec![1, 3, 0, 4]);
        assert_eq!(navmesh.vertices()[3].neighbours, vec![4, 2, 0]);
        assert_eq!(navmesh.vertices()[4].neighbours, vec![3, 0, 2]);

        navmesh.remove_triangle(1); // B

        assert_eq!(navmesh.vertices()[0].neighbours, vec![4, 1, 2, 3]);
        assert_eq!(navmesh.vertices()[1].neighbours, vec![2, 0]);
        assert_eq!(navmesh.vertices()[2].neighbours, vec![1, 3, 0, 4]);
        assert_eq!(navmesh.vertices()[3].neighbours, vec![4, 2, 0]);
        assert_eq!(navmesh.vertices()[4].neighbours, vec![3, 0, 2]);

        navmesh.remove_triangle(0); // A

        assert_eq!(navmesh.vertices()[0].neighbours, vec![4, 2, 3]);
        assert_eq!(navmesh.vertices()[1].neighbours, vec![]);
        assert_eq!(navmesh.vertices()[2].neighbours, vec![3, 0, 4]);
        assert_eq!(navmesh.vertices()[3].neighbours, vec![4, 2, 0]);
        assert_eq!(navmesh.vertices()[4].neighbours, vec![3, 0, 2]);

        navmesh.remove_triangle(0); // C

        assert_eq!(navmesh.vertices()[0].neighbours, vec![]);
        assert_eq!(navmesh.vertices()[1].neighbours, vec![]);
        assert_eq!(navmesh.vertices()[2].neighbours, vec![3, 4]);
        assert_eq!(navmesh.vertices()[3].neighbours, vec![4, 2]);
        assert_eq!(navmesh.vertices()[4].neighbours, vec![3, 2]);

        navmesh.remove_triangle(0); // D

        assert_eq!(navmesh.vertices()[0].neighbours, vec![]);
        assert_eq!(navmesh.vertices()[1].neighbours, vec![]);
        assert_eq!(navmesh.vertices()[2].neighbours, vec![]);
        assert_eq!(navmesh.vertices()[3].neighbours, vec![]);
        assert_eq!(navmesh.vertices()[4].neighbours, vec![]);
    }

    #[test]
    fn test_remove_vertex() {
        let mut navmesh = make_navmesh();

        assert_eq!(navmesh.vertices()[0].neighbours, vec![4, 1, 2, 3]);
        assert_eq!(navmesh.vertices()[1].neighbours, vec![2, 0]);
        assert_eq!(navmesh.vertices()[2].neighbours, vec![1, 3, 0, 4]);
        assert_eq!(navmesh.vertices()[3].neighbours, vec![4, 2, 0]);
        assert_eq!(navmesh.vertices()[4].neighbours, vec![3, 0, 2]);

        navmesh.remove_vertex(4);

        assert_eq!(navmesh.triangles().len(), 2);

        assert_eq!(navmesh.vertices()[0].neighbours, vec![1, 2, 3]);
        assert_eq!(navmesh.vertices()[1].neighbours, vec![2, 0]);
        assert_eq!(navmesh.vertices()[2].neighbours, vec![1, 3, 0]);
        assert_eq!(navmesh.vertices()[3].neighbours, vec![2, 0]);

        navmesh.remove_vertex(3);

        assert_eq!(navmesh.triangles().len(), 1);

        assert_eq!(navmesh.vertices()[0].neighbours, vec![1, 2]);
        assert_eq!(navmesh.vertices()[1].neighbours, vec![2, 0]);
        assert_eq!(navmesh.vertices()[2].neighbours, vec![1, 0]);

        navmesh.remove_vertex(2);

        assert_eq!(navmesh.triangles().len(), 0);

        assert_eq!(navmesh.vertices()[0].neighbours, vec![]);
        assert_eq!(navmesh.vertices()[1].neighbours, vec![]);

        navmesh.remove_vertex(1);

        assert_eq!(navmesh.triangles().len(), 0);

        assert_eq!(navmesh.vertices()[0].neighbours, vec![]);

        navmesh.remove_vertex(0);

        assert_eq!(navmesh.triangles().len(), 0);
        assert_eq!(navmesh.vertices().len(), 0);
    }
}
