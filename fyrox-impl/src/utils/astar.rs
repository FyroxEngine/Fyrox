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

use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};

/// Graph vertex that contains position in world and list of indices of neighbour
/// vertices.
#[derive(Clone, Debug, Visit, PartialEq)]
pub struct VertexData {
    /// Position in the world coordinates
    pub position: Vector3<f32>,
    /// A set of indices of neighbour vertices.
    pub neighbours: Vec<u32>,
    /// Penalty can be interpreted as measure, how harder is to travel to this vertex.
    #[visit(skip)]
    pub g_penalty: f32,
}

impl Default for VertexData {
    fn default() -> Self {
        Self {
            position: Default::default(),
            g_penalty: 1f32,
            neighbours: Default::default(),
        }
    }
}

impl VertexData {
    /// Creates new vertex at given position.
    pub fn new(position: Vector3<f32>) -> Self {
        Self {
            position,
            g_penalty: 1f32,
            neighbours: Default::default(),
        }
    }
}

/// A trait, that describes and arbitrary vertex that could be used in a graph. It allows you to
/// use your structure to store additional info in the graph.
pub trait VertexDataProvider: Deref<Target = VertexData> + DerefMut + PositionProvider {}

/// A default graph vertex with no additional data.
#[derive(Default, PartialEq, Debug)]
pub struct GraphVertex {
    /// Data of the vertex.
    pub data: VertexData,
}

impl GraphVertex {
    /// Creates a new graph vertex.
    pub fn new(position: Vector3<f32>) -> Self {
        Self {
            data: VertexData::new(position),
        }
    }
}

impl Deref for GraphVertex {
    type Target = VertexData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for GraphVertex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl PositionProvider for GraphVertex {
    fn position(&self) -> Vector3<f32> {
        self.data.position
    }
}

impl Visit for GraphVertex {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.data.visit(name, visitor)
    }
}

impl VertexDataProvider for GraphVertex {}

/// A collection of GraphVertices for pathfinding.
///
/// See module docs
#[derive(Clone, Debug, Visit, PartialEq)]
pub struct Graph<T>
where
    T: VertexDataProvider,
{
    /// Vertices of the graph.
    pub vertices: Vec<T>,
    /// The maximum iterations A* pathfinding will attempt before giving up and returning its best path.
    ///
    /// **Default:** 1000
    ///
    /// # Notes
    ///
    /// A* is inefficient when its desired destination is isolated or it must backtrack a substantial distance before it may reach the goal.
    /// Higher max iteration numbers will be required for huge graphs and graphs with many obstacles.
    /// Whereas, lower max iterations may be desired for smaller simple graphs.
    ///
    /// **Negative numbers** disable max iterations
    pub max_search_iterations: i32,
}

/// Shows path status.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PathKind {
    /// The path is a direct path from beginning to end.
    Full,
    /// The path is not a direct path from beginning to end.
    /// Instead, it is a partial path ending at the closest reachable vertex to the desired destination.
    ///
    /// # Notes
    ///
    /// Can happen if there are isolated "islands" of graph vertices with no links between
    /// them and you trying to find a path from one "island" to another.
    Partial,
}

fn heuristic(a: Vector3<f32>, b: Vector3<f32>) -> f32 {
    (a - b).norm_squared()
}

impl<T: VertexDataProvider> Default for Graph<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionProvider for VertexData {
    fn position(&self) -> Vector3<f32> {
        self.position
    }
}

/// Path search can be interrupted by errors, this enum stores all possible
/// kinds of errors.
#[derive(Clone, Debug)]
pub enum PathError {
    /// Out-of-bounds vertex index was found, it can be either the index of begin/end
    /// points, or some index of neighbour vertices in list of neighbours in vertex.
    InvalidIndex(usize),

    /// There is a vertex that has itself as neighbour.
    CyclicReferenceFound(usize),

    /// Path vector is still valid and partial, but pathfinder hit its maximum search iterations and gave up.
    ///
    /// # Notes
    ///
    /// This most often means the desired destination is isolated, but a full path may exist. If a full path does exist you can:
    /// - increase or disable max search iterations for this graph (at the cost of time).
    /// - use a pathfinding algorithm that is better in these situations.
    ///
    /// See `Graph<T>.max_search_iterations` for more
    HitMaxSearchIterations(i32),

    /// Graph was empty.
    Empty,
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
            PathError::HitMaxSearchIterations(v) => {
                write!(
                    f,
                    "Maximum search iterations ({v}) hit, returning with partial path."
                )
            }
            PathError::Empty => {
                write!(f, "Graph was empty")
            }
        }
    }
}

#[derive(Clone)]
/// A partially complete path containing the indices of graph vertices and its A* scores
pub struct PartialPath {
    vertices: Vec<usize>,
    g_score: f32,
    f_score: f32,
}

impl Default for PartialPath {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            g_score: f32::MAX,
            f_score: f32::MAX,
        }
    }
}

impl Ord for PartialPath {
    /// Only compairs f-value and heuristic
    fn cmp(&self, other: &Self) -> Ordering {
        (self.f_score.total_cmp(&other.f_score))
            .then((self.f_score - self.g_score).total_cmp(&(other.f_score - other.g_score)))
            .reverse()
    }
}

impl PartialOrd for PartialPath {
    /// Only compairs f-value and heuristic
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PartialPath {
    /// Only determaines if scores are equal, does not evaluate vertices
    fn eq(&self, other: &Self) -> bool {
        self.f_score == other.f_score && self.g_score == other.g_score
    }
}

impl Eq for PartialPath {}

impl PartialPath {
    /// Creates a new partial path from the starting vertex index
    pub fn new(start: usize) -> Self {
        Self {
            vertices: vec![start],
            g_score: 0f32,
            f_score: f32::MAX,
        }
    }

    /// Returns a clone with the new vertex added to the end and updates scores to given new scores
    pub fn clone_and_add(
        &self,
        new_vertex: usize,
        new_g_score: f32,
        new_f_score: f32,
    ) -> PartialPath {
        let mut clone = self.clone();
        clone.vertices.push(new_vertex);
        clone.g_score = new_g_score;
        clone.f_score = new_f_score;

        clone
    }
}

impl<T: VertexDataProvider> Graph<T> {
    /// Creates new empty graph.
    pub fn new() -> Self {
        Self {
            vertices: Default::default(),
            max_search_iterations: 1000i32,
        }
    }

    /// Sets active set of vertices. Links between vertices must contain
    /// valid indices (which are not out-of-bounds), otherwise path from/to
    /// such vertices won't be built.
    pub fn set_vertices(&mut self, vertices: Vec<T>) {
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
    pub fn vertex(&self, index: usize) -> Option<&T> {
        self.vertices.get(index)
    }

    /// Returns mutable reference to a path vertex at the given index.
    pub fn vertex_mut(&mut self, index: usize) -> Option<&mut T> {
        self.vertices.get_mut(index)
    }

    /// Returns reference to the array of vertices.
    pub fn vertices(&self) -> &[T] {
        &self.vertices
    }

    /// Returns mutable reference to the array of vertices.
    pub fn vertices_mut(&mut self) -> &mut [T] {
        &mut self.vertices
    }

    /// Adds a new vertex to the path finder.
    pub fn add_vertex(&mut self, vertex: T) -> u32 {
        let index = self.vertices.len();
        // Since we're adding the vertex to the end of the array, we don't need to
        // shift indices of neighbours (like `insert_vertex`)
        self.vertices.push(vertex);
        index as u32
    }

    /// Removes last vertex from the graph. Automatically cleans "dangling" references to the deleted vertex
    /// from every other vertex in the graph and shifts indices of neighbour vertices, to preserve graph
    /// structure.
    pub fn pop_vertex(&mut self) -> Option<T> {
        if self.vertices.is_empty() {
            None
        } else {
            Some(self.remove_vertex(self.vertices.len() - 1))
        }
    }

    /// Removes a vertex at the given index from the graph. Automatically cleans "dangling" references to the
    /// deleted vertex from every other vertex in the graph and shifts indices of neighbour vertices, to
    /// preserve graph structure.
    pub fn remove_vertex(&mut self, index: usize) -> T {
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
    pub fn insert_vertex(&mut self, index: u32, vertex: T) {
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

    /// Tries to build path of vertex indices from beginning point to endpoint. Returns path kind:
    ///
    /// - Full: Path vector is a direct path from beginning to end.
    /// - Partial: Path vector is a path that ends closest to the desired end, because pathfinder could not find a full path.
    ///
    /// *See `PathKind`*
    ///
    /// # Notes
    ///
    /// This implementation is fast and allows for multiple searches in parallel, but does not attempt to find the optimal route
    ///
    /// **See `Graph<T>.max_search_iterations`** to change the maximum amount of search iterations

    pub fn build_indexed_path(
        &self,
        from: usize,
        to: usize,
        path: &mut Vec<usize>,
    ) -> Result<PathKind, PathError> {
        path.clear();

        if self.vertices.is_empty() {
            return Err(PathError::Empty);
        }

        let end_pos = self
            .vertices
            .get(to)
            .ok_or(PathError::InvalidIndex(to))?
            .position;

        // returns one point if the goal is the current postion
        if from == to {
            path.push(to);
            return Ok(PathKind::Full);
        }

        // keeps track of which vertices we've searched
        let mut searched_vertices = vec![false; self.vertices.len()];

        // creates heap for searching
        let mut search_heap: BinaryHeap<PartialPath> = BinaryHeap::new();

        // creates first partial path and adds it to heap
        search_heap.push(PartialPath::new(from));

        // stores best path found
        let mut best_path = PartialPath::default();

        // search loop
        let mut search_iteration = 0i32;

        while self.max_search_iterations < 0 || search_iteration < self.max_search_iterations {
            // breakes loop if heap is empty
            if search_heap.is_empty() {
                break;
            }

            // pops best partial path off the heap to use for this iteration
            let current_path = search_heap.pop().unwrap();

            let current_index = *current_path.vertices.last().unwrap();
            let current_vertex = self
                .vertices
                .get(current_index)
                .ok_or(PathError::InvalidIndex(current_index))?;

            // updates best path
            if current_path > best_path {
                best_path = current_path.clone();

                // breaks if end is found
                if current_index == to {
                    break;
                }
            }

            // evaluates path scores one level deeper and adds the paths to the heap
            for i in current_vertex.neighbours.iter() {
                let neighbour_index = *i as usize;

                // this error is thrown for the users sake
                // it shouldn't actually cause an issue because the next line would skip it
                if neighbour_index == current_index {
                    return Err(PathError::CyclicReferenceFound(current_index));
                }

                // avoids going in circles
                if searched_vertices[neighbour_index] {
                    continue;
                }

                let neighbour = self
                    .vertices
                    .get(neighbour_index)
                    .ok_or(PathError::InvalidIndex(neighbour_index))?;

                let neighbour_g_score = current_path.g_score
                    + ((current_vertex.position - neighbour.position).norm_squared()
                        * neighbour.g_penalty);

                let neighbour_f_score = neighbour_g_score + heuristic(neighbour.position, end_pos);

                search_heap.push(current_path.clone_and_add(
                    neighbour_index,
                    neighbour_g_score,
                    neighbour_f_score,
                ));
            }

            // marks vertex as searched
            searched_vertices[current_index] = true;

            search_iteration += 1;
        }

        // sets path to the best path of indices
        path.clone_from(&best_path.vertices);
        path.reverse();

        if *path.first().unwrap() == to {
            Ok(PathKind::Full)
        } else if search_iteration == self.max_search_iterations - 1 {
            Err(PathError::HitMaxSearchIterations(
                self.max_search_iterations,
            ))
        } else {
            Ok(PathKind::Partial)
        }
    }

    /// Tries to build path of Vector3's from beginning point to endpoint. Returns path kind:
    ///
    /// - Full: Path vector is a direct path from beginning to end.
    /// - Partial: Path vector is a path that ends closest to the desired end, because pathfinder could not find a full path.
    ///
    /// *See `PathKind`*
    ///
    /// # Notes
    ///
    /// This implementation is fast and allows for multiple searches in parallel, but does not attempt to find the optimal route
    ///
    /// **See `Graph<T>.max_search_iterations`** to change the maximum amount of search iterations
    pub fn build_positional_path(
        &self,
        from: usize,
        to: usize,
        path: &mut Vec<Vector3<f32>>,
    ) -> Result<PathKind, PathError> {
        path.clear();

        let mut indices: Vec<usize> = Vec::new();
        let path_kind = self.build_indexed_path(from, to, &mut indices)?;

        // converts from indicies to positions
        for index in indices.iter() {
            let vertex = self
                .vertices
                .get(*index)
                .ok_or(PathError::InvalidIndex(*index))?;

            path.push(vertex.position);
        }

        Ok(path_kind)
    }

    /// **Deprecated** *use **`Graph<T>.build_positional_path()`** instead*
    ///
    /// Tries to build path of Vector3's from beginning point to endpoint. Returns path kind:
    ///
    /// - Full: Path vector is a direct path from beginning to end.
    /// - Partial: Path vector is a path that ends closest to the desired end, because pathfinder could not find a full path.
    ///
    /// *See `PathKind`*
    ///
    /// # Notes
    ///
    /// This implementation is fast and allows for multiple searches in parallel, but does not attempt to find the optimal route
    ///
    /// **See `Graph<T>.max_search_iterations`** to change the maximum amount of search iterations
    #[deprecated = "name is too ambiguous use build_positional_path instead"]
    pub fn build(
        &self,
        from: usize,
        to: usize,
        path: &mut Vec<Vector3<f32>>,
    ) -> Result<PathKind, PathError> {
        self.build_positional_path(from, to, path)
    }
}

#[cfg(test)]
mod test {
    use crate::rand::Rng;
    use crate::utils::astar::PathError;
    use crate::{
        core::{algebra::Vector3, rand},
        utils::astar::{Graph, GraphVertex, PathKind},
    };
    use std::time::Instant;

    #[test]
    fn astar_random_points() {
        let mut pathfinder = Graph::<GraphVertex>::new();

        let mut path = Vec::new();
        assert!(pathfinder
            .build_positional_path(0, 0, &mut path)
            .is_err_and(|e| matches!(e, PathError::Empty)));
        assert!(path.is_empty());

        let size = 40;

        // Create vertices.
        let mut vertices = Vec::new();
        for y in 0..size {
            for x in 0..size {
                vertices.push(GraphVertex::new(Vector3::new(x as f32, y as f32, 0.0)));
            }
        }
        pathfinder.set_vertices(vertices);

        assert!(pathfinder
            .build_positional_path(100000, 99999, &mut path)
            .is_err_and(|e| matches!(e, PathError::InvalidIndex(_))));

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

            assert!(pathfinder
                .build_positional_path(from, to, &mut path)
                .is_ok());
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
        let mut pathfinder = Graph::<GraphVertex>::new();

        pathfinder.add_vertex(GraphVertex::new(Vector3::new(0.0, 0.0, 0.0)));
        pathfinder.add_vertex(GraphVertex::new(Vector3::new(1.0, 0.0, 0.0)));
        pathfinder.add_vertex(GraphVertex::new(Vector3::new(1.0, 1.0, 0.0)));

        pathfinder.link_bidirect(0, 1);
        pathfinder.link_bidirect(1, 2);
        pathfinder.link_bidirect(2, 0);

        pathfinder.remove_vertex(0);

        assert_eq!(pathfinder.vertex(0).unwrap().neighbours, vec![1]);
        assert_eq!(pathfinder.vertex(1).unwrap().neighbours, vec![0]);
        assert_eq!(pathfinder.vertex(2), None);

        pathfinder.remove_vertex(0);

        assert_eq!(pathfinder.vertex(0).unwrap().neighbours, Vec::<u32>::new());
        assert_eq!(pathfinder.vertex(1), None);
        assert_eq!(pathfinder.vertex(2), None);
    }

    #[test]
    fn test_insert_vertex() {
        let mut pathfinder = Graph::new();

        pathfinder.add_vertex(GraphVertex::new(Vector3::new(0.0, 0.0, 0.0)));
        pathfinder.add_vertex(GraphVertex::new(Vector3::new(1.0, 0.0, 0.0)));
        pathfinder.add_vertex(GraphVertex::new(Vector3::new(1.0, 1.0, 0.0)));

        pathfinder.link_bidirect(0, 1);
        pathfinder.link_bidirect(1, 2);
        pathfinder.link_bidirect(2, 0);

        assert_eq!(pathfinder.vertex(0).unwrap().neighbours, vec![1, 2]);
        assert_eq!(pathfinder.vertex(1).unwrap().neighbours, vec![0, 2]);
        assert_eq!(pathfinder.vertex(2).unwrap().neighbours, vec![1, 0]);

        pathfinder.insert_vertex(0, GraphVertex::new(Vector3::new(1.0, 1.0, 1.0)));

        assert_eq!(pathfinder.vertex(0).unwrap().neighbours, Vec::<u32>::new());
        assert_eq!(pathfinder.vertex(1).unwrap().neighbours, vec![2, 3]);
        assert_eq!(pathfinder.vertex(2).unwrap().neighbours, vec![1, 3]);
        assert_eq!(pathfinder.vertex(3).unwrap().neighbours, vec![2, 1]);
    }

    #[ignore = "takes multiple seconds to run"]
    #[test]
    /// Tests A*'s speed when finding a direct path with no obsticles
    fn astar_complete_grid_benchmark() {
        let start_time = Instant::now();
        let mut path = Vec::new();

        println!();
        for size in [10, 40, 100, 500] {
            println!("benchmarking grid size of: {}^2", size);
            let setup_start_time = Instant::now();

            let mut pathfinder = Graph::new();

            // Create vertices.
            let mut vertices = Vec::new();
            for y in 0..size {
                for x in 0..size {
                    vertices.push(GraphVertex::new(Vector3::new(x as f32, y as f32, 0.0)));
                }
            }
            pathfinder.set_vertices(vertices);

            // Link vertices as grid.
            for y in 0..(size - 1) {
                for x in 0..(size - 1) {
                    pathfinder.link_bidirect(y * size + x, y * size + x + 1);
                    pathfinder.link_bidirect(y * size + x, (y + 1) * size + x);
                }
            }

            let setup_complete_time = Instant::now();
            println!(
                "setup in: {:?}",
                setup_complete_time.duration_since(setup_start_time)
            );

            for _ in 0..1000 {
                let sx = rand::thread_rng().gen_range(0..(size - 1));
                let sy = rand::thread_rng().gen_range(0..(size - 1));

                let tx = rand::thread_rng().gen_range(0..(size - 1));
                let ty = rand::thread_rng().gen_range(0..(size - 1));

                let from = sy * size + sx;
                let to = ty * size + tx;

                assert!(pathfinder
                    .build_positional_path(from, to, &mut path)
                    .is_ok());
                assert!(!path.is_empty());

                if path.len() > 1 {
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
            println!("paths found in: {:?}", setup_complete_time.elapsed());
            println!(
                "Current size complete in: {:?}\n",
                setup_start_time.elapsed()
            );
        }
        println!("Total time: {:?}\n", start_time.elapsed());
    }

    #[ignore = "takes multiple seconds to run"]
    #[test]
    /// Tests A*'s speed when finding partial paths (no direct path available)
    fn astar_island_benchmark() {
        let start_time = Instant::now();

        let size = 100;
        let mut path = Vec::new();
        let mut pathfinder = Graph::new();

        // Create vertices.
        let mut vertices = Vec::new();
        for y in 0..size {
            for x in 0..size {
                vertices.push(GraphVertex::new(Vector3::new(x as f32, y as f32, 0.0)));
            }
        }
        pathfinder.set_vertices(vertices);

        // Link vertices as grid.
        // seperates grids half way down the y-axis
        for y in 0..(size - 1) {
            for x in 0..(size - 1) {
                if x != ((size / 2) - 1) {
                    pathfinder.link_bidirect(y * size + x, y * size + x + 1);
                }
                pathfinder.link_bidirect(y * size + x, (y + 1) * size + x);
            }
        }

        let setup_complete_time = Instant::now();

        println!(
            "\nsetup in: {:?}",
            setup_complete_time.duration_since(start_time)
        );

        for _ in 0..1000 {
            // generates a random start point on the left half of the grid
            let sx = rand::thread_rng().gen_range(0..((size / 2) - 1));
            let sy = rand::thread_rng().gen_range(0..(size - 1));

            // generates a random end point on the right half of the grid
            let tx = rand::thread_rng().gen_range((size / 2)..(size - 1));
            let ty = rand::thread_rng().gen_range(0..(size - 1));

            let from = sy * size + sx;
            let to = ty * size + tx;

            let path_result = pathfinder.build_positional_path(from, to, &mut path);

            let is_result_expected = path_result.as_ref().is_ok_and(|k| k.eq(&PathKind::Partial))
                || path_result.is_err_and(|e| matches!(e, PathError::HitMaxSearchIterations(_)));

            assert!(is_result_expected);
            assert!(!path.is_empty());

            if path.len() > 1 {
                // partial path should be along the divide
                assert_eq!(path.first().unwrap().x as i32, ((size / 2) - 1) as i32);
                // start point should be start point
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

        println!("paths found in: {:?}", setup_complete_time.elapsed());
        println!("Total time: {:?}\n", start_time.elapsed());
    }

    #[ignore = "takes multiple seconds to run"]
    #[test]
    /// Tests A*'s speed when when finding indirect paths (major obstacle in the way)
    fn astar_backwards_travel_benchmark() {
        let start_time = Instant::now();

        let size = 100;
        let mut path = Vec::new();
        let mut pathfinder = Graph::new();

        // Create vertices.
        let mut vertices = Vec::new();
        for y in 0..size {
            for x in 0..size {
                vertices.push(GraphVertex::new(Vector3::new(x as f32, y as f32, 0.0)));
            }
        }
        pathfinder.set_vertices(vertices);

        // Link vertices as grid.
        // seperates grid diagonally down the xy plane leaving only one connection in the corner
        for y in 0..(size - 1) {
            for x in (0..(size - 1)).rev() {
                if y == 0 || x != y {
                    pathfinder.link_bidirect(y * size + x, y * size + x + 1);
                    pathfinder.link_bidirect(y * size + x, (y + 1) * size + x);
                }
            }
        }

        let setup_complete_time = Instant::now();

        println!(
            "\nsetup in: {:?}",
            setup_complete_time.duration_since(start_time)
        );

        for _ in 0..1000 {
            // a point on the center right edge
            let from = (size / 2) * size + (size - 1);
            // a point on the center top edge
            let to = (size - 1) * size + (size / 2);

            assert!(pathfinder
                .build_positional_path(from, to, &mut path)
                .is_ok());
            assert!(!path.is_empty());

            if path.len() > 1 {
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

        println!("paths found in: {:?}", setup_complete_time.elapsed());
        println!("Total time: {:?}\n", start_time.elapsed());
    }
}
