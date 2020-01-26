use crate::core::math::vec3::Vec3;
use crate::core::math;
use rg3d_core::math::PositionProvider;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PathVertexState {
    NonVisited,
    Open,
    Closed,
}

pub struct PathVertex {
    /// Position in world.
    position: Vec3,
    state: PathVertexState,
    g_score: f32,
    f_score: f32,
    parent: Option<usize>,
    neighbours: Vec<usize>,
}

impl PathVertex {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            parent: None,
            g_score: std::f32::MAX,
            f_score: std::f32::MAX,
            state: PathVertexState::NonVisited,
            neighbours: Default::default(),
        }
    }

    fn clear(&mut self) {
        self.g_score = std::f32::MAX;
        self.f_score = std::f32::MAX;
        self.state = PathVertexState::NonVisited;
        self.parent = None;
    }

    pub fn neighbours(&self) -> &[usize] {
        &self.neighbours
    }
}

pub struct PathFinder {
    vertices: Vec<PathVertex>
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PathKind {
    Full,
    Partial,
    Empty,
}

fn heuristic(a: Vec3, b: Vec3) -> f32 {
    a.sqr_distance(&b)
}

impl Default for PathFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionProvider for PathVertex {
    fn position(&self) -> Vec3 {
        self.position
    }
}

#[derive(Copy, Clone, Debug)]
pub enum PathError {
    InvalidIndex(usize),
    CyclicReferenceFound,
}

impl PathFinder {
    pub fn new() -> Self {
        Self {
            vertices: Default::default()
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
    pub fn get_closest_vertex_to(&self, point: Vec3) -> Option<usize> {
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
            vertex_a.neighbours.push(b);
        }
    }

    /// Returns shared reference to path vertex at given index.
    pub fn get_vertex(&self, index: usize) -> Option<&PathVertex> {
        self.vertices.get(index)
    }

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
    pub fn build(&mut self, from: usize, to: usize, path: &mut Vec<Vec3>) -> Result<PathKind, PathError> {
        if self.vertices.is_empty() {
            return Ok(PathKind::Empty);
        }

        path.clear();

        for vertex in self.vertices.iter_mut() {
            vertex.clear();
        }

        let end_pos = self.vertices.get(to)
            .ok_or(PathError::InvalidIndex(to))?.position;

        // Put start vertex in open set.
        let start = self.vertices.get_mut(from).ok_or(PathError::InvalidIndex(from))?;
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
            let unsafe_vertices: &mut Vec<PathVertex> = unsafe {
                &mut *(&mut self.vertices as *mut _)
            };

            let current_vertex = self.vertices
                .get_mut(current_index)
                .ok_or(PathError::InvalidIndex(current_index))?;

            current_vertex.state = PathVertexState::Closed;

            for neighbour_index in current_vertex.neighbours.iter() {
                // Make sure that borrowing rules are not violated.
                if *neighbour_index == current_index {
                    return Err(PathError::CyclicReferenceFound);
                }

                // Safely get mutable reference to neighbour
                let neighbour = unsafe_vertices
                    .get_mut(*neighbour_index)
                    .ok_or(PathError::InvalidIndex(*neighbour_index))?;

                let g_score = current_vertex.g_score + current_vertex.position.sqr_distance(&neighbour.position);
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
            let closest_vertex = self.vertices.get(closest_index)
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

    fn reconstruct_path(&self, mut current: usize, path: &mut Vec<Vec3>) {
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
    use crate::utils::astar::{PathFinder, PathVertex};
    use crate::core::math::vec3::Vec3;
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
                vertices.push(PathVertex::new(Vec3::new(x as f32, y as f32, 0.0)));
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
            let sx = rand::thread_rng().gen_range(0, size - 1);
            let sy = rand::thread_rng().gen_range(0, size - 1);

            let tx = rand::thread_rng().gen_range(0, size - 1);
            let ty = rand::thread_rng().gen_range(0, size - 1);

            let from = sy * size + sx;
            let to = ty * size + tx;

            assert!(pathfinder.build(from, to, &mut path).is_ok());
            assert!(!path.is_empty());

            if path.len() > 1 {
                paths_count += 1;

                assert_eq!(*path.first().unwrap(), pathfinder.get_vertex(to).unwrap().position);
                assert_eq!(*path.last().unwrap(), pathfinder.get_vertex(from).unwrap().position);
            } else {
                let point = *path.first().unwrap();
                assert_eq!(point, pathfinder.get_vertex(to).unwrap().position);
                assert_eq!(point, pathfinder.get_vertex(from).unwrap().position);
            }

            for pair in path.chunks(2) {
                if pair.len() == 2 {
                    let a = pair[0];
                    let b = pair[1];

                    assert!(a.distance(&b) <= 2.0f32.sqrt());
                }
            }
        }

        assert!(paths_count > 0);
    }
}