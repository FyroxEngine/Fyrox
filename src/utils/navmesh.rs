use crate::{
    utils::astar::{
        PathFinder,
        PathVertex,
        PathKind,
        PathError,
    },
    core::{
        octree::Octree,
        math::{
            TriangleDefinition,
            vec3::Vec3,
            self,
        },
    }
};
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

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
        // Direction-agnostic compare
        (self.a == other.a && self.b == other.b) || (self.a == other.b && self.b == other.a)
    }
}

impl Eq for Edge {}

impl Hash for Edge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Intentionally make hash collision so Eq will be called.
        state.write_i8(0);
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

    pub fn triangles(&self) -> &[TriangleDefinition] {
        &self.triangles
    }

    pub fn vertices(&self) -> &[PathVertex] {
        self.pathfinder.vertices()
    }

    pub fn build_path(&mut self, from: usize, to: usize, path: &mut Vec<Vec3>) -> Result<PathKind, PathError> {
        self.pathfinder.build(from, to, path)
    }
}