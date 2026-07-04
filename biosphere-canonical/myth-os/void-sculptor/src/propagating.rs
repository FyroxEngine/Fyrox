/// propagating.rs — Propagating Contours iso-surface extraction.
///
/// The primary extraction algorithm. Matches Acropora's "Fast Analysis" mode.
///
/// From the Acropora manual (§1.2):
///   "Once a seed point is found, its immediately connected neighbors are
///    evaluated. Neighbors that produce triangles then become the next seed
///    point, and so on. Only the voxels containing a surface are evaluated,
///    reducing processing times dramatically."
///
/// This implementation uses a BFS queue. Each visited crossing voxel
/// contributes its local triangle geometry to the soup. Visited voxels
/// are tracked in a flat bitset to avoid re-processing.
///
/// Triangle interpolation: linear interpolation along each edge where
/// the iso-value crosses. Same math as Marching Cubes — only the traversal
/// order differs.

use crate::field_reader::ScalarField;
use crate::marching_cubes::{cube_case, triangulate_cube};
use crate::mesh::TriangleSoup;
use crate::seed::SeedSet;
use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// Propagation
// ---------------------------------------------------------------------------

/// Extract a triangle mesh by propagating from seed points.
///
/// Returns a TriangleSoup — raw triangles, not yet indexed or deduplicated.
/// Pass the result to mesh::build_mesh() to get an IndexedMesh.
pub fn propagate(field: &ScalarField, seeds: &SeedSet) -> TriangleSoup {
    let res     = field.resolution as usize;
    let total   = res * res * res;

    // Flat bitset — one bit per voxel. True = already visited.
    let mut visited: Vec<bool> = vec![false; total];
    let mut queue:   VecDeque<[usize; 3]> = VecDeque::new();
    let mut soup     = TriangleSoup::new();

    // Seed the queue
    for &[iz, iy, ix] in &seeds.seeds {
        let idx = flat_index(iz, iy, ix, res);
        if !visited[idx] {
            visited[idx] = true;
            queue.push_back([iz, iy, ix]);
        }
    }

    // BFS propagation
    while let Some([iz, iy, ix]) = queue.pop_front() {
        // Triangulate this voxel using Marching Cubes local case
        let case = cube_case(field, iz, iy, ix);
        triangulate_cube(field, iz, iy, ix, case, &mut soup);

        // Expand to face neighbors
        let neighbors: [(isize, isize, isize); 6] = [
            (iz as isize - 1, iy as isize,     ix as isize    ),
            (iz as isize + 1, iy as isize,     ix as isize    ),
            (iz as isize,     iy as isize - 1, ix as isize    ),
            (iz as isize,     iy as isize + 1, ix as isize    ),
            (iz as isize,     iy as isize,     ix as isize - 1),
            (iz as isize,     iy as isize,     ix as isize + 1),
        ];

        for (nz, ny, nx) in neighbors {
            // Bounds check
            if nz < 0 || ny < 0 || nx < 0 { continue; }
            let nz = nz as usize;
            let ny = ny as usize;
            let nx = nx as usize;
            if nz >= res || ny >= res || nx >= res { continue; }

            let idx = flat_index(nz, ny, nx, res);
            if visited[idx] { continue; }

            // Only enqueue if this neighbor is a crossing voxel
            // (has at least one neighbor on the other side of the threshold)
            if is_surface_cube(field, nz, ny, nx) {
                visited[idx] = true;
                queue.push_back([nz, ny, nx]);
            }
        }
    }

    soup
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Flat array index for [iz, iy, ix] in a res×res×res cube.
#[inline]
fn flat_index(iz: usize, iy: usize, ix: usize, res: usize) -> usize {
    iz * res * res + iy * res + ix
}

/// Returns true if this voxel's 8 corners span both sides of the iso-threshold.
/// A cube that spans both sides will produce at least one triangle.
#[inline]
fn is_surface_cube(field: &ScalarField, iz: usize, iy: usize, ix: usize) -> bool {
    let case = cube_case(field, iz, iy, ix);
    // Case 0 (all outside) and Case 255 (all inside) produce no triangles
    case != 0 && case != 255
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_reader::field_from_array;
    use crate::seed::find_seeds;
    use ndarray::Array3;

    fn sphere_field(res: usize, radius_fraction: f32) -> ScalarField {
        let center = res as f32 / 2.0;
        let mut data = Array3::from_elem((res, res, res), 1.0_f32);
        for iz in 0..res {
            for iy in 0..res {
                for ix in 0..res {
                    let r = ((iz as f32 - center).powi(2)
                           + (iy as f32 - center).powi(2)
                           + (ix as f32 - center).powi(2))
                           .sqrt();
                    if r < center * radius_fraction {
                        data[[iz, iy, ix]] = 0.0;
                    }
                }
            }
        }
        field_from_array(data, 0.5)
    }

    #[test]
    fn extracts_triangles_from_sphere() {
        let field  = sphere_field(16, 0.6);
        let seeds  = find_seeds(&field, 1);
        assert!(!seeds.is_empty(), "need seeds to propagate");
        let soup   = propagate(&field, &seeds);
        assert!(!soup.triangles.is_empty(), "expected triangles from sphere field");
    }

    #[test]
    fn uniform_field_produces_no_triangles() {
        let data   = Array3::from_elem((8, 8, 8), 0.0_f32); // all inside
        let field  = field_from_array(data, 0.5);
        let seeds  = find_seeds(&field, 1);
        assert!(seeds.is_empty());
        let soup   = propagate(&field, &seeds);
        assert!(soup.triangles.is_empty());
    }

    #[test]
    fn no_nan_in_triangle_positions() {
        let field = sphere_field(12, 0.5);
        let seeds = find_seeds(&field, 1);
        let soup  = propagate(&field, &seeds);
        for tri in &soup.triangles {
            for v in &tri.vertices {
                assert!(!v.x.is_nan(), "NaN in triangle vertex x");
                assert!(!v.y.is_nan(), "NaN in triangle vertex y");
                assert!(!v.z.is_nan(), "NaN in triangle vertex z");
            }
        }
    }

    #[test]
    fn propagation_visits_full_surface() {
        // A box-shaped interior — should produce faces on all 6 sides
        let res  = 8;
        let mut data = Array3::from_elem((res, res, res), 1.0_f32);
        for iz in 2..6 { for iy in 2..6 { for ix in 2..6 {
            data[[iz, iy, ix]] = 0.0;
        }}}
        let field = field_from_array(data, 0.5);
        let seeds = find_seeds(&field, 1);
        let soup  = propagate(&field, &seeds);
        // A box should produce many triangles — at least 12 (2 per face × 6 faces)
        assert!(soup.triangles.len() >= 12,
            "expected at least 12 triangles for a box, got {}", soup.triangles.len());
    }
}
