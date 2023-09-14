//! Contains classic A* (A-star) path finding algorithms.
//!
//! A* is one of fastest graph search algorithms, it is used to construct shortest
//! possible path from vertex to vertex. In vast majority of games it is used in pair
//! with navigation meshes (navmesh). Check navmesh module docs for more info.

#![warn(missing_docs)]

use crate::core::{
    algebra::Vector3,
    math::{self, PositionProvider},
    visitor::prelude::*,
};
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum PathVertexState {
    NonVisited,
    Open,
    Closed,
}

/// Graph vertex that contains position in world and list of indices of neighbour
/// vertices.
#[derive(Clone, Debug, Visit, PartialEq)]
pub struct PathVertex {
    /// Position in the world coordinates
    pub position: Vector3<f32>,
    pub(crate) neighbours: Vec<u32>,
    #[visit(skip)]
    state: PathVertexState,
    #[visit(skip)]
    g_penalty: f32,
    #[visit(skip)]
    g_score: f32,
    #[visit(skip)]
    f_score: f32,
    #[visit(skip)]
    parent: Option<usize>,
}

impl Default for PathVertex {
    fn default() -> Self {
        Self {
            position: Default::default(),
            parent: None,
            g_penalty: 1f32,
            g_score: f32::MAX,
            f_score: f32::MAX,
            state: PathVertexState::NonVisited,
            neighbours: Default::default(),
        }
    }
}

impl PathVertex {
    /// Creates new vertex at given position.
    pub fn new(position: Vector3<f32>) -> Self {
        Self {
            position,
            parent: None,
            g_penalty: 1f32,
            g_score: f32::MAX,
            f_score: f32::MAX,
            state: PathVertexState::NonVisited,
            neighbours: Default::default(),
        }
    }

    /// Returns reference to array of indices of neighbour vertices.
    pub fn neighbours(&self) -> &[u32] {
        &self.neighbours
    }

    /// Sets penalty for vertex g_score calculation
    /// Penalty can be interpreted as measure, how harder is to travel
    /// to this vertex.
    pub fn set_penalty(&mut self, new_penalty: f32) {
        self.g_penalty = new_penalty;
    }

    fn clear(&mut self) {
        self.g_penalty = 1f32;
        self.g_score = f32::MAX;
        self.f_score = f32::MAX;
        self.state = PathVertexState::NonVisited;
        self.parent = None;
    }
}

/// See module docs.
#[derive(Clone, Debug, Visit, PartialEq)]
pub struct PathFinder {
    vertices: Vec<PathVertex>,
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

impl Display for PathError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::InvalidIndex(v) => {
                write!(f, "Invalid vertex index {v}.")
            }
            PathError::CyclicReferenceFound(v) => {
                write!(f, "Cyclical reference was found {v}.")
            }
            PathError::Custom(v) => {
                write!(f, "An error has occurred {v}")
            }
        }
    }
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
            if vertex_a.neighbours.iter().all(|n| *n != b as u32) {
                vertex_a.neighbours.push(b as u32);
            }
        }
    }

    /// Returns shared reference to a path vertex at the given index.
    pub fn vertex(&self, index: usize) -> Option<&PathVertex> {
        self.vertices.get(index)
    }

    /// Returns mutable reference to a path vertex at the given index.
    pub fn vertex_mut(&mut self, index: usize) -> Option<&mut PathVertex> {
        self.vertices.get_mut(index)
    }

    /// Returns reference to the array of vertices.
    pub fn vertices(&self) -> &[PathVertex] {
        &self.vertices
    }

    /// Returns reference to the array of vertices.
    pub fn vertices_mut(&mut self) -> &mut [PathVertex] {
        &mut self.vertices
    }

    /// Adds a new vertex to the path finder.
    pub fn add_vertex(&mut self, vertex: PathVertex) -> u32 {
        let index = self.vertices.len();
        // Since we're adding the vertex to the end of the array, we don't need to
        // shift indices of neighbours (like `insert_vertex`)
        self.vertices.push(vertex);
        index as u32
    }

    /// Removes last vertex from the graph. Automatically cleans "dangling" references to the deleted vertex
    /// from every other vertex in the graph and shifts indices of neighbour vertices, to preserve graph
    /// structure.
    pub fn pop_vertex(&mut self) -> Option<PathVertex> {
        if self.vertices.is_empty() {
            None
        } else {
            Some(self.remove_vertex(self.vertices.len() - 1))
        }
    }

    /// Removes a vertex at the given index from the graph. Automatically cleans "dangling" references to the
    /// deleted vertex from every other vertex in the graph and shifts indices of neighbour vertices, to
    /// preserve graph structure.
    pub fn remove_vertex(&mut self, index: usize) -> PathVertex {
        for other_vertex in self.vertices.iter_mut() {
            // Remove "references" to the vertex, that will be deleted.
            if let Some(position) = other_vertex
                .neighbours
                .iter()
                .position(|n| *n == index as u32)
            {
                other_vertex.neighbours.remove(position);
            }

            // Shift neighbour indices to preserve vertex indexation.
            for neighbour_index in other_vertex.neighbours.iter_mut() {
                if *neighbour_index > index as u32 {
                    *neighbour_index -= 1;
                }
            }
        }

        self.vertices.remove(index)
    }

    /// Inserts the vertex at the given index. Automatically shifts neighbour indices of every other vertex
    /// in the graph to preserve graph structure.
    pub fn insert_vertex(&mut self, index: u32, vertex: PathVertex) {
        self.vertices.insert(index as usize, vertex);

        // Shift neighbour indices to preserve vertex indexation.
        for other_vertex in self.vertices.iter_mut() {
            for neighbour_index in other_vertex.neighbours.iter_mut() {
                if *neighbour_index >= index {
                    *neighbour_index += 1;
                }
            }
        }
    }

    /// Tries to build path from begin point to end point. Returns path kind:
    ///
    /// - Full: there are direct path from begin to end.
    /// - Partial: there are not direct path from begin to end, but it is closest.
    /// - Empty: no path available - in most cases indicates some error in input params.
    ///
    /// # Notes
    ///
    /// This is more or less naive implementation, it most certainly will be slower than specialized solutions.
    pub fn build(
        &mut self,
        from: usize,
        to: usize,
        path: &mut Vec<Vector3<f32>>,
    ) -> Result<PathKind, PathError> {
        self.build_and_convert(from, to, path, |_, v| v.position)
    }

    /// Tries to build path from begin point to end point. Returns path kind:
    ///
    /// - Full: there are direct path from begin to end.
    /// - Partial: there are not direct path from begin to end, but it is closest.
    /// - Empty: no path available - in most cases indicates some error in input params.
    ///
    /// # Notes
    ///
    /// This is more or less naive implementation, it most certainly will be slower than specialized solutions.
    pub fn build_and_convert<F, T>(
        &mut self,
        from: usize,
        to: usize,
        path: &mut Vec<T>,
        func: F,
    ) -> Result<PathKind, PathError>
    where
        F: FnMut(usize, &PathVertex) -> T,
    {
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
            let mut lowest_f_score = f32::MAX;
            for (i, vertex) in self.vertices.iter().enumerate() {
                if vertex.state == PathVertexState::Open && vertex.f_score < lowest_f_score {
                    current_index = i;
                    lowest_f_score = vertex.f_score;
                }
            }

            if current_index == to {
                self.reconstruct_path(current_index, path, func);
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
                    + ((current_vertex.position - neighbour.position).norm_squared()
                        * neighbour.g_penalty);
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

        self.reconstruct_path(closest_index, path, func);

        if path.is_empty() {
            Ok(PathKind::Empty)
        } else {
            Ok(PathKind::Partial)
        }
    }

    fn reconstruct_path<F, T>(&self, mut current: usize, path: &mut Vec<T>, mut func: F)
    where
        F: FnMut(usize, &PathVertex) -> T,
    {
        while let Some(vertex) = self.vertices.get(current) {
            path.push(func(current, vertex));
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
    use crate::rand::Rng;
    use crate::{
        core::{algebra::Vector3, rand},
        utils::astar::{PathFinder, PathVertex},
    };

    #[test]
    fn astar_random_points() {
        let mut pathfinder = PathFinder::new();

        let mut path = vec![];
        assert!(pathfinder.build(0, 0, &mut path).is_ok());
        assert!(path.is_empty());

        let size = 40;

        // Create vertices.
        let mut vertices = vec![];
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
                    pathfinder.vertex(to).unwrap().position
                );
                assert_eq!(
                    *path.last().unwrap(),
                    pathfinder.vertex(from).unwrap().position
                );
            } else {
                let point = *path.first().unwrap();
                assert_eq!(point, pathfinder.vertex(to).unwrap().position);
                assert_eq!(point, pathfinder.vertex(from).unwrap().position);
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

    #[test]
    fn test_remove_vertex() {
        let mut pathfinder = PathFinder::new();

        pathfinder.add_vertex(PathVertex::new(Vector3::zeros()));
        pathfinder.add_vertex(PathVertex::new(Vector3::x()));
        pathfinder.add_vertex(PathVertex::new(Vector3::new(1.0, 1.0, 0.0)));

        pathfinder.link_bidirect(0, 1);
        pathfinder.link_bidirect(1, 2);
        pathfinder.link_bidirect(2, 0);

        pathfinder.remove_vertex(0);

        assert_eq!(pathfinder.vertex(0).unwrap().neighbours, vec![1]);
        assert_eq!(pathfinder.vertex(1).unwrap().neighbours, vec![0]);
        assert_eq!(pathfinder.vertex(2), None);

        pathfinder.remove_vertex(0);

        assert_eq!(pathfinder.vertex(0).unwrap().neighbours, vec![]);
        assert_eq!(pathfinder.vertex(1), None);
        assert_eq!(pathfinder.vertex(2), None);
    }

    #[test]
    fn test_insert_vertex() {
        let mut pathfinder = PathFinder::new();

        pathfinder.add_vertex(PathVertex::new(Vector3::zeros()));
        pathfinder.add_vertex(PathVertex::new(Vector3::x()));
        pathfinder.add_vertex(PathVertex::new(Vector3::new(1.0, 1.0, 0.0)));

        pathfinder.link_bidirect(0, 1);
        pathfinder.link_bidirect(1, 2);
        pathfinder.link_bidirect(2, 0);

        assert_eq!(pathfinder.vertex(0).unwrap().neighbours, vec![1, 2]);
        assert_eq!(pathfinder.vertex(1).unwrap().neighbours, vec![0, 2]);
        assert_eq!(pathfinder.vertex(2).unwrap().neighbours, vec![1, 0]);

        pathfinder.insert_vertex(0, PathVertex::new(Vector3::new(1.0, 1.0, 1.0)));

        assert_eq!(pathfinder.vertex(0).unwrap().neighbours, vec![]);
        assert_eq!(pathfinder.vertex(1).unwrap().neighbours, vec![2, 3]);
        assert_eq!(pathfinder.vertex(2).unwrap().neighbours, vec![1, 3]);
        assert_eq!(pathfinder.vertex(3).unwrap().neighbours, vec![2, 1]);
    }
}
