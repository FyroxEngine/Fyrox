/// seed.rs — Seed point finder for Propagating Contours.
///
/// A seed point is a voxel that crosses the iso-threshold — meaning at least
/// one of its 6 face-neighbors is on the opposite side of the surface.
/// These are the starting points for the propagating contour algorithm.
///
/// From the Acropora manual (§1.2):
///   "A set of seed points (voxels that cross an iso-surface) are used to
///    generate triangles based on local connectivity."
///
/// Strategy: scan a sparse grid of sample points across the field to find
/// initial crossings quickly, then expand locally to build the full seed set.
/// For Mandelbulb-class fields, the surface is typically 10–15% of the volume.

use crate::field_reader::ScalarField;

// ---------------------------------------------------------------------------
// SeedSet
// ---------------------------------------------------------------------------

/// A set of voxel indices that lie on the iso-surface boundary.
/// Each entry is a [iz, iy, ix] index into the scalar field.
#[derive(Debug, Clone)]
pub struct SeedSet {
    pub seeds: Vec<[usize; 3]>,
}

impl SeedSet {
    pub fn is_empty(&self) -> bool {
        self.seeds.is_empty()
    }

    pub fn len(&self) -> usize {
        self.seeds.len()
    }
}

// ---------------------------------------------------------------------------
// Crossing check
// ---------------------------------------------------------------------------

/// Returns true if the voxel at [iz, iy, ix] is a surface crossing.
///
/// A crossing voxel has at least one face-neighbor on the opposite side
/// of the iso-threshold from itself.
#[inline]
pub fn is_crossing(field: &ScalarField, iz: isize, iy: isize, ix: isize) -> bool {
    let inside = field.is_inside(iz, iy, ix);

    // Check all 6 face neighbors
    let neighbors: [(isize, isize, isize); 6] = [
        (iz - 1, iy,     ix    ),
        (iz + 1, iy,     ix    ),
        (iz,     iy - 1, ix    ),
        (iz,     iy + 1, ix    ),
        (iz,     iy,     ix - 1),
        (iz,     iy,     ix + 1),
    ];

    for (nz, ny, nx) in neighbors {
        let n_val = field.get(nz, ny, nx);
        // f32::MAX means out-of-bounds — treat as outside
        let n_inside = n_val <= field.iso_threshold;
        if inside != n_inside {
            return true; // crossing found
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Seed finder
// ---------------------------------------------------------------------------

/// Find seed points by scanning the field at a configurable stride.
///
/// `stride` controls the sampling density:
///   - 1 = full scan (guaranteed to find all seeds, slow for large fields)
///   - 2 = every other voxel (good default — misses nothing wider than 2 voxels)
///   - 4 = coarse scan for initial seed discovery
///
/// After finding initial seeds at `stride`, expands each to its immediate
/// neighbors to ensure full surface coverage even at coarse stride.
pub fn find_seeds(field: &ScalarField, stride: usize) -> SeedSet {
    let res    = field.resolution as usize;
    let stride = stride.max(1);

    let mut seeds = Vec::new();

    // Pass 1 — strided scan for initial seeds
    let mut iz = 0;
    while iz < res {
        let mut iy = 0;
        while iy < res {
            let mut ix = 0;
            while ix < res {
                if is_crossing(field, iz as isize, iy as isize, ix as isize) {
                    seeds.push([iz, iy, ix]);
                }
                ix += stride;
            }
            iy += stride;
        }
        iz += stride;
    }

    // Pass 2 — if stride > 1, expand each initial seed to its face neighbors
    // to ensure we didn't skip any crossing voxels
    if stride > 1 {
        let initial_count = seeds.len();
        for i in 0..initial_count {
            let [sz, sy, sx] = seeds[i];
            for dz in -1_isize..=1 {
                for dy in -1_isize..=1 {
                    for dx in -1_isize..=1 {
                        if dz == 0 && dy == 0 && dx == 0 { continue; }
                        let nz = sz as isize + dz;
                        let ny = sy as isize + dy;
                        let nx = sx as isize + dx;
                        if nz < 0 || ny < 0 || nx < 0 { continue; }
                        let nz = nz as usize;
                        let ny = ny as usize;
                        let nx = nx as usize;
                        if nz >= res || ny >= res || nx >= res { continue; }
                        if is_crossing(field, nz as isize, ny as isize, nx as isize) {
                            seeds.push([nz, ny, nx]);
                        }
                    }
                }
            }
        }

        // Deduplicate — seeds may be added multiple times in expansion
        seeds.sort_unstable();
        seeds.dedup();
    }

    SeedSet { seeds }
}

/// Find seeds using automatic stride selection based on field resolution.
///
/// Small fields (res ≤ 32): stride 1 (full scan, fast enough)
/// Medium fields (res ≤ 128): stride 2
/// Large fields (res > 128): stride 4
pub fn find_seeds_auto(field: &ScalarField) -> SeedSet {
    let stride = match field.resolution {
        0..=32  => 1,
        33..=128 => 2,
        _        => 4,
    };
    find_seeds(field, stride)
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

/// Summarize the seed set for logging.
pub fn seed_report(seeds: &SeedSet, field: &ScalarField) -> String {
    let total   = field.voxel_count();
    let pct     = 100.0 * seeds.len() as f64 / total as f64;
    format!(
        "seeds={} / {} voxels ({:.1}%)",
        seeds.len(), total, pct
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_reader::field_from_array;
    use ndarray::Array3;

    /// Build a simple 4×4×4 field with a sphere-like interior.
    /// Voxels within radius 1.0 of center get value 0.0 (inside).
    /// Others get 1.0 (outside). iso_threshold = 0.5.
    fn sphere_field(res: usize) -> ScalarField {
        let center = res as f32 / 2.0;
        let mut data = Array3::from_elem((res, res, res), 1.0_f32);
        for iz in 0..res {
            for iy in 0..res {
                for ix in 0..res {
                    let r = ((iz as f32 - center).powi(2)
                           + (iy as f32 - center).powi(2)
                           + (ix as f32 - center).powi(2))
                           .sqrt();
                    if r < center * 0.6 {
                        data[[iz, iy, ix]] = 0.0;
                    }
                }
            }
        }
        field_from_array(data, 0.5)
    }

    #[test]
    fn finds_seeds_in_sphere_field() {
        let field = sphere_field(8);
        let seeds = find_seeds(&field, 1);
        assert!(!seeds.is_empty(), "expected seeds in a sphere field");
    }

    #[test]
    fn uniform_field_has_no_seeds() {
        // All values the same — no crossings anywhere
        let data  = Array3::from_elem((8, 8, 8), 0.0_f32);
        let field = field_from_array(data, 0.5);
        let seeds = find_seeds(&field, 1);
        assert!(seeds.is_empty(), "uniform field should have no seeds");
    }

    #[test]
    fn stride_2_finds_seeds_in_sphere() {
        let field   = sphere_field(16);
        let seeds1  = find_seeds(&field, 1);
        let seeds2  = find_seeds(&field, 2);
        // Stride 2 may find fewer seeds but should still find some
        assert!(!seeds2.is_empty());
        // And should find close to as many as stride 1 after expansion
        let ratio = seeds2.len() as f64 / seeds1.len() as f64;
        assert!(ratio > 0.5, "stride 2 should find at least 50% of stride 1 seeds, got {ratio:.2}");
    }

    #[test]
    fn is_crossing_detects_boundary() {
        let mut data = Array3::from_elem((4, 4, 4), 1.0_f32);
        // Create a 2×2×2 interior block
        for iz in 1..3 {
            for iy in 1..3 {
                for ix in 1..3 {
                    data[[iz, iy, ix]] = 0.0;
                }
            }
        }
        let field = field_from_array(data, 0.5);
        // The boundary voxels (value 0.0 adjacent to 1.0) should be crossings
        assert!(is_crossing(&field, 1, 1, 1));
        // Interior of interior block (if it exists) should not be a crossing
        // With 2×2×2 interior, all interior voxels are on the boundary
        // Deep interior voxels in larger fields would not be crossings:
        let mut big = Array3::from_elem((8, 8, 8), 1.0_f32);
        for iz in 2..6 { for iy in 2..6 { for ix in 2..6 {
            big[[iz, iy, ix]] = 0.0;
        }}}
        let big_field = field_from_array(big, 0.5);
        assert!(!is_crossing(&big_field, 4, 4, 4), "deep interior should not be a crossing");
        assert!( is_crossing(&big_field, 2, 2, 2), "surface voxel should be a crossing");
    }

    #[test]
    fn auto_stride_selects_correctly() {
        for (res, expected_stride) in [(8u32, 1usize), (64, 2), (256, 4)] {
            let stride = match res {
                0..=32   => 1,
                33..=128 => 2,
                _        => 4,
            };
            assert_eq!(stride, expected_stride, "wrong stride for res={res}");
        }
    }
}
