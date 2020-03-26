use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};
use crate::core::math::TriangleDefinition;

#[derive(Copy, Clone)]
struct IndexedStorage<T> {
    index: u32,
    vertex: T,
}

impl<T> PartialEq for IndexedStorage<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.vertex == other.vertex
    }
}

impl<T> Eq for IndexedStorage<T> where T: PartialEq {}

impl<T> Hash for IndexedStorage<T> where T: Hash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.vertex.hash(state)
    }
}

impl<T> Default for RawMeshBuilder<T> where T: Hash + PartialEq {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            indices: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct RawMeshBuilder<T> where T: Hash + PartialEq {
    vertices: HashSet<IndexedStorage<T>>,
    indices: Vec<u32>,
}

pub struct RawMesh<T> {
    pub vertices: Vec<T>,
    pub triangles: Vec<TriangleDefinition>,
}

impl<T> RawMeshBuilder<T> where T: Hash + PartialEq {
    /// Creates new builder with given start values of capacity for internal
    /// buffers. These values doesn't need to be precise.
    pub fn new(vertices: usize, indices: usize) -> Self {
        Self {
            vertices: HashSet::with_capacity(vertices),
            indices: Vec::with_capacity(indices),
        }
    }

    /// Inserts new vertex in mesh. Index buffer is populated automatically -
    /// when duplicate vertex is found, it not added into vertices array, but its
    /// index gets added into indices array.
    pub fn insert(&mut self, vertex: T) -> bool {
        let mut wrapper = IndexedStorage::<T> { index: 0, vertex };
        if let Some(existing) = self.vertices.get(&wrapper) {
            self.indices.push(existing.index);
            false
        } else {
            wrapper.index = self.vertices.len() as u32;
            self.indices.push(wrapper.index);
            self.vertices.insert(wrapper);
            true
        }
    }

    pub fn build(self) -> RawMesh<T> {
        let mut vertices = self.vertices.into_iter().collect::<Vec<_>>();
        vertices.sort_unstable_by_key(|w| w.index);
        RawMesh {
            vertices: vertices
                .into_iter()
                .map(|w| w.vertex)
                .collect(),
            triangles: self
                .indices
                .chunks_exact(3)
                .map(|i| TriangleDefinition { indices: [i[0], i[1], i[2]] })
                .collect(),
        }
    }
}
