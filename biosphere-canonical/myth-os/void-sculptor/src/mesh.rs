/// mesh.rs — Triangle soup → indexed mesh, smoothing, island removal.
///
/// TriangleSoup is the raw output of both extraction algorithms.
/// IndexedMesh is the final form — deduplicated vertices, index buffer,
/// normals per vertex, and optional material zone IDs.
///
/// Post-processing matches Acropora CORE_ATOM_94 (Mesh Topology Optimizer):
///   - Vertex deduplication
///   - Laplacian smoothing
///   - Island removal (disconnected component filtering)

use glam::Vec3;
use hashbrown::HashMap;
use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// Triangle — raw output unit from extraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
    pub normal:   Vec3,
}

// ---------------------------------------------------------------------------
// TriangleSoup — unindexed, unordered collection of triangles
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct TriangleSoup {
    pub triangles: Vec<Triangle>,
}

impl TriangleSoup {
    pub fn new() -> Self {
        Self { triangles: Vec::new() }
    }

    pub fn push(&mut self, tri: Triangle) {
        self.triangles.push(tri);
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    pub fn len(&self) -> usize {
        self.triangles.len()
    }
}

// ---------------------------------------------------------------------------
// IndexedMesh — the final deliverable
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct IndexedMesh {
    /// Deduplicated vertex positions.
    pub vertices: Vec<Vec3>,

    /// Per-vertex averaged normals.
    pub normals: Vec<Vec3>,

    /// Triangle index buffer. Every 3 indices = one triangle.
    pub indices: Vec<u32>,

    /// Per-vertex material zone ID. 0 = default zone.
    /// Populated by material::assign_zones() after mesh is built.
    pub material_ids: Vec<u8>,
}

impl IndexedMesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Vertex key for deduplication — quantized to avoid float equality issues
// ---------------------------------------------------------------------------

/// Quantization factor for vertex positions.
/// 1e4 = 0.0001 world units precision — adequate for fractal meshes.
const QUANTIZE: f32 = 1e4;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VertexKey(i64, i64, i64);

impl VertexKey {
    fn from_vec3(v: Vec3) -> Self {
        Self(
            (v.x * QUANTIZE).round() as i64,
            (v.y * QUANTIZE).round() as i64,
            (v.z * QUANTIZE).round() as i64,
        )
    }
}

// ---------------------------------------------------------------------------
// Build an IndexedMesh from a TriangleSoup
// ---------------------------------------------------------------------------

/// Convert a TriangleSoup into a deduplicated IndexedMesh.
///
/// Vertices within QUANTIZE precision are merged.
/// Per-vertex normals are averaged across all contributing triangles.
pub fn build_mesh(soup: &TriangleSoup) -> IndexedMesh {
    if soup.is_empty() {
        return IndexedMesh {
            vertices:     Vec::new(),
            normals:      Vec::new(),
            indices:      Vec::new(),
            material_ids: Vec::new(),
        };
    }

    let mut vertex_map: HashMap<VertexKey, u32> = HashMap::new();
    let mut vertices:   Vec<Vec3>               = Vec::new();
    let mut normal_acc: Vec<Vec3>               = Vec::new(); // accumulated normals
    let mut indices:    Vec<u32>                = Vec::new();

    for tri in &soup.triangles {
        for &v in &tri.vertices {
            let key = VertexKey::from_vec3(v);
            let idx = *vertex_map.entry(key).or_insert_with(|| {
                let i = vertices.len() as u32;
                vertices.push(v);
                normal_acc.push(Vec3::ZERO);
                i
            });
            indices.push(idx);
            // Accumulate face normal into all 3 vertices
            normal_acc[idx as usize] += tri.normal;
        }
    }

    // Normalize accumulated normals
    let normals: Vec<Vec3> = normal_acc
        .iter()
        .map(|n| {
            let len = n.length();
            if len < f32::EPSILON { Vec3::Y } else { *n / len }
        })
        .collect();

    let n = vertices.len();
    IndexedMesh {
        vertices,
        normals,
        indices,
        material_ids: vec![0u8; n],
    }
}

// ---------------------------------------------------------------------------
// Laplacian smoothing
// ---------------------------------------------------------------------------

/// Apply Laplacian smoothing passes to the mesh.
///
/// Each pass moves each vertex toward the average of its neighbors.
/// `factor` controls the blend: 0.0 = no movement, 1.0 = full move to centroid.
/// Typical value: 0.5.
///
/// Does NOT change vertex count or index buffer.
pub fn smooth(mesh: &mut IndexedMesh, passes: u32, factor: f32) {
    if passes == 0 || mesh.vertices.is_empty() { return; }

    // Build adjacency: vertex index → set of neighbor vertex indices
    let n = mesh.vertices.len();
    let mut neighbors: Vec<Vec<u32>> = vec![Vec::new(); n];

    for tri_base in (0..mesh.indices.len()).step_by(3) {
        let a = mesh.indices[tri_base]     as usize;
        let b = mesh.indices[tri_base + 1] as usize;
        let c = mesh.indices[tri_base + 2] as usize;
        // Each vertex neighbors the other two in the triangle
        if !neighbors[a].contains(&(b as u32)) { neighbors[a].push(b as u32); }
        if !neighbors[a].contains(&(c as u32)) { neighbors[a].push(c as u32); }
        if !neighbors[b].contains(&(a as u32)) { neighbors[b].push(a as u32); }
        if !neighbors[b].contains(&(c as u32)) { neighbors[b].push(c as u32); }
        if !neighbors[c].contains(&(a as u32)) { neighbors[c].push(a as u32); }
        if !neighbors[c].contains(&(b as u32)) { neighbors[c].push(b as u32); }
    }

    for _ in 0..passes {
        let old = mesh.vertices.clone();
        for i in 0..n {
            if neighbors[i].is_empty() { continue; }
            let centroid = neighbors[i]
                .iter()
                .fold(Vec3::ZERO, |acc, &j| acc + old[j as usize])
                / neighbors[i].len() as f32;
            mesh.vertices[i] = old[i].lerp(centroid, factor);
        }
    }

    // Re-compute normals after smoothing
    recompute_normals(mesh);
}

/// Recompute per-vertex normals from the current vertex positions.
pub fn recompute_normals(mesh: &mut IndexedMesh) {
    let n = mesh.vertices.len();
    let mut normal_acc = vec![Vec3::ZERO; n];

    for tri_base in (0..mesh.indices.len()).step_by(3) {
        let ia = mesh.indices[tri_base]     as usize;
        let ib = mesh.indices[tri_base + 1] as usize;
        let ic = mesh.indices[tri_base + 2] as usize;
        let va = mesh.vertices[ia];
        let vb = mesh.vertices[ib];
        let vc = mesh.vertices[ic];
        let normal = (vb - va).cross(vc - va);
        normal_acc[ia] += normal;
        normal_acc[ib] += normal;
        normal_acc[ic] += normal;
    }

    mesh.normals = normal_acc.iter().map(|n| {
        let len = n.length();
        if len < f32::EPSILON { Vec3::Y } else { *n / len }
    }).collect();
}

// ---------------------------------------------------------------------------
// Island removal — remove disconnected components smaller than min_size
// ---------------------------------------------------------------------------

/// Remove disconnected triangle islands smaller than `min_faces` triangles.
///
/// Uses BFS over triangle connectivity (shared edges) to find components.
/// Returns a new IndexedMesh with small islands removed and vertices re-indexed.
pub fn remove_islands(mesh: &IndexedMesh, min_faces: usize) -> IndexedMesh {
    if mesh.is_empty() || min_faces == 0 {
        return mesh.clone();
    }

    let tri_count = mesh.triangle_count();

    // Build triangle adjacency via shared vertices
    // Map vertex index → list of triangle indices
    let n_verts = mesh.vertices.len();
    let mut vert_to_tris: Vec<Vec<usize>> = vec![Vec::new(); n_verts];
    for t in 0..tri_count {
        for k in 0..3 {
            let vi = mesh.indices[t * 3 + k] as usize;
            vert_to_tris[vi].push(t);
        }
    }

    // BFS to find connected components of triangles
    let mut visited  = vec![false; tri_count];
    let mut components: Vec<Vec<usize>> = Vec::new();

    for start in 0..tri_count {
        if visited[start] { continue; }
        let mut component = Vec::new();
        let mut queue     = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;

        while let Some(t) = queue.pop_front() {
            component.push(t);
            // Find neighbor triangles via shared vertices
            for k in 0..3 {
                let vi = mesh.indices[t * 3 + k] as usize;
                for &neighbor_t in &vert_to_tris[vi] {
                    if !visited[neighbor_t] {
                        visited[neighbor_t] = true;
                        queue.push_back(neighbor_t);
                    }
                }
            }
        }
        components.push(component);
    }

    // Keep only components with at least min_faces triangles
    let kept_tris: Vec<usize> = components
        .iter()
        .filter(|c| c.len() >= min_faces)
        .flatten()
        .copied()
        .collect();

    if kept_tris.is_empty() {
        // All components were too small — return empty mesh
        return IndexedMesh {
            vertices:     Vec::new(),
            normals:      Vec::new(),
            indices:      Vec::new(),
            material_ids: Vec::new(),
        };
    }

    // Rebuild mesh from kept triangles, re-indexing vertices
    let mut new_vertices:     Vec<Vec3> = Vec::new();
    let mut new_normals:      Vec<Vec3> = Vec::new();
    let mut new_material_ids: Vec<u8>   = Vec::new();
    let mut new_indices:      Vec<u32>  = Vec::new();
    let mut old_to_new:       HashMap<u32, u32> = HashMap::new();

    for t in &kept_tris {
        for k in 0..3 {
            let old_vi = mesh.indices[t * 3 + k];
            let new_vi = *old_to_new.entry(old_vi).or_insert_with(|| {
                let ni = new_vertices.len() as u32;
                new_vertices.push(mesh.vertices[old_vi as usize]);
                new_normals.push(mesh.normals[old_vi as usize]);
                new_material_ids.push(
                    mesh.material_ids.get(old_vi as usize).copied().unwrap_or(0)
                );
                ni
            });
            new_indices.push(new_vi);
        }
    }

    IndexedMesh {
        vertices:     new_vertices,
        normals:      new_normals,
        indices:      new_indices,
        material_ids: new_material_ids,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn two_triangles() -> TriangleSoup {
        let mut soup = TriangleSoup::new();
        soup.push(Triangle {
            vertices: [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            normal: Vec3::Z,
        });
        soup.push(Triangle {
            vertices: [
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            normal: Vec3::Z,
        });
        soup
    }

    #[test]
    fn build_mesh_deduplicates_vertices() {
        let soup = two_triangles();
        let mesh = build_mesh(&soup);
        // Two triangles sharing an edge → 4 unique vertices, not 6
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
    }

    #[test]
    fn normals_are_unit_length() {
        let mesh = build_mesh(&two_triangles());
        for n in &mesh.normals {
            assert!((n.length() - 1.0).abs() < 1e-5, "normal not unit length: {n}");
        }
    }

    #[test]
    fn empty_soup_gives_empty_mesh() {
        let soup = TriangleSoup::new();
        let mesh = build_mesh(&soup);
        assert!(mesh.is_empty());
    }

    #[test]
    fn smooth_does_not_change_vertex_count() {
        let mut mesh = build_mesh(&two_triangles());
        let before   = mesh.vertex_count();
        smooth(&mut mesh, 2, 0.5);
        assert_eq!(mesh.vertex_count(), before);
    }

    #[test]
    fn remove_islands_keeps_large_component() {
        // Two separate triangles — one "island" of 1 tri each
        // min_faces = 2 → both get removed; min_faces = 1 → both kept
        let mut soup = TriangleSoup::new();
        // Triangle 1
        soup.push(Triangle {
            vertices: [Vec3::new(0.0,0.0,0.0), Vec3::new(1.0,0.0,0.0), Vec3::new(0.0,1.0,0.0)],
            normal: Vec3::Z,
        });
        // Triangle 2 — completely separate vertices
        soup.push(Triangle {
            vertices: [Vec3::new(5.0,5.0,0.0), Vec3::new(6.0,5.0,0.0), Vec3::new(5.0,6.0,0.0)],
            normal: Vec3::Z,
        });
        let mesh    = build_mesh(&soup);
        let cleaned = remove_islands(&mesh, 1);
        assert_eq!(cleaned.triangle_count(), 2, "min_faces=1 should keep all triangles");

        let cleaned2 = remove_islands(&mesh, 2);
        assert_eq!(cleaned2.triangle_count(), 0, "min_faces=2 removes 1-tri islands");
    }
}
