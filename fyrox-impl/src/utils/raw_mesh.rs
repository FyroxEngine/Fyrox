//! Raw mesh is a procedural mesh builder, all you can do with it is to insert vertices
//! one-by-one and it will automatically build faces by skipping duplicated vertices.
//! Main usage of it - optimize "triangle soup" into mesh so adjacent faces will have
//! shared edges. Raw mesh itself does not have any methods, it is just a final result
//! of RawMeshBuilder.

use crate::{
    core::hash_as_bytes,
    core::{algebra::Vector3, math::TriangleDefinition},
};
use bytemuck::{Pod, Zeroable};
use fxhash::{FxBuildHasher, FxHashSet};
use std::hash::{Hash, Hasher};
#[derive(Copy, Clone)]
struct IndexedStorage<T> {
    index: u32,
    vertex: T,
}

/// Raw vertex is just a point in 3d space that supports hashing.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RawVertex {
    /// An X component.
    pub x: f32,
    /// An Y component.
    pub y: f32,
    /// An Z component.
    pub z: f32,
}

impl PartialEq for RawVertex {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl From<Vector3<f32>> for RawVertex {
    fn from(v: Vector3<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl RawVertex {
    fn validate(&self) {
        debug_assert!(!self.x.is_nan());
        debug_assert!(!self.y.is_nan());
        debug_assert!(!self.z.is_nan());
    }
}

impl Hash for RawVertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.validate();
        hash_as_bytes(self, state);
    }
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
    vertices: FxHashSet<IndexedStorage<T>>,
    indices: Vec<u32>,
}

/// See module docs.
#[derive(Default, Debug, Clone)]
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
            // We can't use plain `with_capacity` with FxHashSet,
            // we need to specify the hahser manually too
            // (https://internals.rust-lang.org/t/hashmap-set-new-with-capacity-and-buildhasher/15622).
            vertices: FxHashSet::with_capacity_and_hasher(vertices, FxBuildHasher::default()),
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

    /// Returns total amount of vertices in the mesh builder so far.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
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
