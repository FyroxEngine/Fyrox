//! Raw mesh is a procedural mesh builder, all you can do with it is to insert vertices
//! one-by-one and it will automatically build faces by skipping duplicated vertices.
//! Main usage of it - optimize "triangle soup" into mesh so adjacent faces will have
//! shared edges. Raw mesh itself does not have any methods, it is just a final result
//! of RawMeshBuilder.

use crate::core::math::TriangleDefinition;
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

#[derive(Copy, Clone)]
struct IndexedStorage<T> {
    index: u32,
    vertex: T,
}

impl<T> PartialEq for IndexedStorage<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.vertex == other.vertex
    }
}

impl<T> Eq for IndexedStorage<T> where T: PartialEq {}

impl<T> Hash for IndexedStorage<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.vertex.hash(state)
    }
}

impl<T> Default for RawMeshBuilder<T>
where
    T: Hash + PartialEq,
{
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            indices: Default::default(),
        }
    }
}

/// See module docs.
#[derive(Clone)]
pub struct RawMeshBuilder<T>
where
    T: Hash + PartialEq,
{
    vertices: HashSet<IndexedStorage<T>>,
    indices: Vec<u32>,
}

/// See module docs.
pub struct RawMesh<T> {
    /// Vertices of mesh.
    pub vertices: Vec<T>,
    /// Triangles of mesh. Each triangle contains indices of vertices.
    pub triangles: Vec<TriangleDefinition>,
}

impl<T> RawMeshBuilder<T>
where
    T: Hash + PartialEq,
{
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

    /// Creates new raw mesh from internal set of vertices and indices. If last "triangle" has
    /// insufficient vertex count (less than 3), it will be discarded.
    pub fn build(self) -> RawMesh<T> {
        let mut vertices = self.vertices.into_iter().collect::<Vec<_>>();
        vertices.sort_unstable_by_key(|w| w.index);
        RawMesh {
            vertices: vertices.into_iter().map(|w| w.vertex).collect(),
            triangles: self
                .indices
                .chunks_exact(3)
                .map(|i| TriangleDefinition([i[0], i[1], i[2]]))
                .collect(),
        }
    }
}
