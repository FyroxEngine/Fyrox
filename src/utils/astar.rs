//! Contains classic A* (A-star) path finding algorithms.
//!
//! A* is one of fastest graph search algorithms, it is used to construct shortest
//! possible path from vertex to vertex. In vast majority of games it is used in pair
//! with navigation meshes (navmesh). Check navmesh module docs for more info.

#![warn(missing_docs)]

use crate::core::algebra::Vector3;
use crate::core::math::{self, PositionProvider};
use crate::core::visitor::Visit;
use rg3d_core::visitor::{VisitResult, Visitor};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum PathVertexState {
    NonVisited,
    Open,
    Closed,
}

/// Graph vertex that contains position in world and list of indices of neighbour
/// vertices.
#[derive(Clone, Debug)]
pub struct PathVertex {
    /// Position in world.
    pub position: Vector3<f32>,
    state: PathVertexState,
    g_score: f32,
    f_score: f32,
    parent: Option<usize>,
    neighbours: Vec<u32>,
}

impl Default for PathVertex {
    fn default() -> Self {
        Self {
            position: Default::default(),
            parent: None,
            g_score: std::f32::MAX,
            f_score: std::f32::MAX,
            state: PathVertexState::NonVisited,
            neighbours: Default::default(),
        }
    }
}

impl Visit for PathVertex {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;
        self.neighbours.visit("Neighbours", visitor)?;
        // Rest of fields are runtime state and valid only while in a build method of pathfinder.

        visitor.leave_region()
    }
}

impl PathVertex {
    /// Creates new vertex at given position.
    pub fn new(position: Vector3<f32>) -> Self {
        Self {
            position,
            parent: None,
            g_score: std::f32::MAX,
            f_score: std::f32::MAX,
            state: PathVertexState::NonVisited,
            neighbours: Default::default(),
        }
    }

    /// Returns reference to array of indices of neighbour vertices.
    pub fn neighbours(&self) -> &[u32] {
        &self.neighbours
    }

    fn clear(&mut self) {
        self.g_score = std::f32::MAX;
        self.f_score = std::f32::MAX;
        self.state = PathVertexState::NonVisited;
        self.parent = None;
    }
}

/// See module docs.
#[derive(Clone, Debug)]
pub struct PathFinder {
    vertices: Vec<PathVertex>,
}

impl Visit for PathFinder {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.vertices.visit("Vertices", visitor)?;

        visitor.leave_region()
    }
}

/// Shows path status.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PathKind {
    /// There is direct path from begin to end.
    Full,
    /// No direct path, only partial to closest reachable vertex to destination. Can
    /// happen if there are isolated "islands" of graph vertices with no links between
    /// them and you trying to find path from one "island" to other.
    Partial,
    /// Either array of vertices to search on was empty, or search was started from
    /// isolated vertex.
    Empty,
}

fn heuristic(a: Vector3<f32>, b: Vector3<f32>) -> f32 {
    (a - b).norm_squared()
}

impl Default for PathFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionProvider for PathVertex {
    fn position(&self) -> Vector3<f32> {
        self.position
    }
}

/// Path search can be interrupted by errors, this enum stores all possible
/// kinds of errors.
#[derive(Clone, Debug)]
pub enum PathError {
    /// Out-of-bounds vertex index has found, it can be either index of begin/end
    /// points, or some index of neighbour vertices in list of neighbours in vertex.
    InvalidIndex(usize),
    /// There is a vertex that has itself as neighbour.
    CyclicReferenceFound(usize),
    /// User-defined error.
    Custom(String),
}

impl PathFinder {
    /// Creates new empty path finder.
    pub fn new() -> Self {
        Self {
            vertices: Default::default(),
        }
    }

    /// Sets active set of vertices. Links between vertices must contain
    /// valid indices (which are not out-of-bounds), otherwise path from/to
    /// such vertices won't be built.
    pub fn set_vertices(&mut self, vertices: Vec<PathVertex>) {
        self.vertices = vertices;
    }

    /// Tries to find a vertex closest to given point.
    ///
    /// # Notes
    ///
    /// O(n) complexity.
    pub fn get_closest_vertex_to(&self, point: Vector3<f32>) -> Option<usize> {
        math::get_closest_point(&self.vertices, point)
    }

    /// Creates bidirectional link between two vertices. Bidirectional means
    /// that point `a` can be reached from point `b` as well as point `b`
    /// can be reached from point `a`.
    pub fn link_bidirect(&mut self, a: usize, b: usize) {
        self.link_unidirect(a, b);
        self.link_unidirect(b, a);
    }

    /// Creates unidirectional link between vertex `a` and vertex `b`. Unidirectional
    /// means that there is no direct link between `b` to `a`, only from `a` to `b`.
    pub fn link_unidirect(&mut self, a: usize, b: usize) {
        if let Some(vertex_a) = self.vertices.get_mut(a) {
            vertex_a.neighbours.push(b as u32);
        }
    }

    /// Returns shared reference to path vertex at given index.
    pub fn get_vertex(&self, index: usize) -> Option<&PathVertex> {
        self.vertices.get(index)
    }

    /// Returns reference to array of vertices.
    pub fn vertices(&self) -> &[PathVertex] {
        &self.vertices
    }

    /// Tries to build path from begin point to end point. Returns path kind:
    /// - Full: there are direct path from begin to end.
    /// - Partial: there are not direct path from begin to end, but it is closest.
    /// - Empty: no path available - in most cases indicates some error in input
    ///   params.
    ///
    /// # Notes
    ///
    /// This is more or less naive implementation, it most certainly will be slower
    /// than specialized solutions. I haven't benchmarked this algorithm against any
    /// other library!
    pub fn build(
        &mut self,
        from: usize,
        to: usize,
        path: &mut Vec<Vector3<f32>>,
    ) -> Result<PathKind, PathError> {
        if self.vertices.is_empty() {
            return Ok(PathKind::Empty);
        }

        path.clear();

        for vertex in self.vertices.iter_mut() {
            vertex.clear();
        }

        let end_pos = self
            .vertices
            .get(to)
            .ok_or(PathError::InvalidIndex(to))?
            .position;

        // Put start vertex in open set.
        let start = self
            .vertices
            .get_mut(from)
            .ok_or(PathError::InvalidIndex(from))?;
        start.state = PathVertexState::Open;
        start.g_score = 0.0;
        start.f_score = heuristic(start.position, end_pos);

        let mut open_set_size = 1;
        while open_set_size > 0 {
            let mut current_index = 0;
            let mut lowest_f_score = std::f32::MAX;
            for (i, vertex) in self.vertices.iter().enumerate() {
                if vertex.state == PathVertexState::Open && vertex.f_score < lowest_f_score {
                    current_index = i;
                    lowest_f_score = vertex.f_score;
                }
            }

            if current_index == to {
                self.reconstruct_path(current_index, path);
                return Ok(PathKind::Full);
            }

            open_set_size -= 1;

            // Take second mutable reference to vertices array, we'll enforce borrowing rules
            // at runtime. It will *never* give two mutable references to same path vertex.
            let unsafe_vertices: &mut Vec<PathVertex> =
                unsafe { &mut *(&mut self.vertices as *mut _) };

            let current_vertex = self
                .vertices
                .get_mut(current_index)
                .ok_or(PathError::InvalidIndex(current_index))?;

            current_vertex.state = PathVertexState::Closed;

            for neighbour_index in current_vertex.neighbours.iter() {
                // Make sure that borrowing rules are not violated.
                if *neighbour_index as usize == current_index {
                    return Err(PathError::CyclicReferenceFound(current_index));
                }

                // Safely get mutable reference to neighbour
                let neighbour = unsafe_vertices
                    .get_mut(*neighbour_index as usize)
                    .ok_or(PathError::InvalidIndex(*neighbour_index as usize))?;

                let g_score = current_vertex.g_score
                    + (current_vertex.position - neighbour.position).norm_squared();
                if g_score < neighbour.g_score {
                    neighbour.parent = Some(current_index);
                    neighbour.g_score = g_score;
                    neighbour.f_score = g_score + heuristic(neighbour.position, end_pos);

                    if neighbour.state != PathVertexState::Open {
                        neighbour.state = PathVertexState::Open;
                        open_set_size += 1;
                    }
                }
            }
        }

        // No direct path found, then there is probably partial path exists.
        // Look for vertex with least f_score and use it as starting point to
        // reconstruct partial path.
        let mut closest_index = 0;
        for (i, vertex) in self.vertices.iter().enumerate() {
            let closest_vertex = self
                .vertices
                .get(closest_index)
                .ok_or(PathError::InvalidIndex(closest_index))?;
            if vertex.f_score < closest_vertex.f_score {
                closest_index = i;
            }
        }

        self.reconstruct_path(closest_index, path);

        if path.is_empty() {
            Ok(PathKind::Empty)
        } else {
            Ok(PathKind::Partial)
        }
    }

    fn reconstruct_path(&self, mut current: usize, path: &mut Vec<Vector3<f32>>) {
        while let Some(vertex) = self.vertices.get(current) {
            path.push(vertex.position);
            if let Some(parent) = vertex.parent {
                current = parent;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::{algebra::Vector3, rand},
        utils::astar::{PathFinder, PathVertex},
    };
    use rand::Rng;

    #[test]
    fn astar_random_points() {
        let mut pathfinder = PathFinder::new();

        let mut path = Vec::new();
        assert!(pathfinder.build(0, 0, &mut path).is_ok());
        assert!(path.is_empty());

        let size = 40;

        // Create vertices.
        let mut vertices = Vec::new();
        for y in 0..size {
            for x in 0..size {
                vertices.push(PathVertex::new(Vector3::new(x as f32, y as f32, 0.0)));
            }
        }
        pathfinder.set_vertices(vertices);

        assert!(pathfinder.build(100000, 99999, &mut path).is_err());

        // Link vertices as grid.
        for y in 0..(size - 1) {
            for x in 0..(size - 1) {
                pathfinder.link_bidirect(y * size + x, y * size + x + 1);
                pathfinder.link_bidirect(y * size + x, (y + 1) * size + x);
            }
        }

        let mut paths_count = 0;

        for _ in 0..1000 {
            let sx = rand::thread_rng().gen_range(0..(size - 1));
            let sy = rand::thread_rng().gen_range(0..(size - 1));

            let tx = rand::thread_rng().gen_range(0..(size - 1));
            let ty = rand::thread_rng().gen_range(0..(size - 1));

            let from = sy * size + sx;
            let to = ty * size + tx;

            assert!(pathfinder.build(from, to, &mut path).is_ok());
            assert!(!path.is_empty());

            if path.len() > 1 {
                paths_count += 1;

                assert_eq!(
                    *path.first().unwrap(),
                    pathfinder.get_vertex(to).unwrap().position
                );
                assert_eq!(
                    *path.last().unwrap(),
                    pathfinder.get_vertex(from).unwrap().position
                );
            } else {
                let point = *path.first().unwrap();
                assert_eq!(point, pathfinder.get_vertex(to).unwrap().position);
                assert_eq!(point, pathfinder.get_vertex(from).unwrap().position);
            }

            for pair in path.chunks(2) {
                if pair.len() == 2 {
                    let a = pair[0];
                    let b = pair[1];

                    assert!(a.metric_distance(&b) <= 2.0f32.sqrt());
                }
            }
        }

        assert!(paths_count > 0);
    }
}
