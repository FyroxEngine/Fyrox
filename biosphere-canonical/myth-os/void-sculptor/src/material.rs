/// material.rs — Material zone assignment.
///
/// Assigns material zone IDs to mesh vertices based on their world-space
/// position and the material_seed from the Genesis Ceremony.
///
/// Material zones control how the Theater adapter textures the mesh.
/// The sculptor doesn't pick textures — it tags vertices with zone IDs (0–15).
/// The Theater adapter maps zone IDs to actual materials.
///
/// From the Ceremony log:
///   material_seed: 77239104
///   "CORE_ATOM_81 (Particle System) was loaded to define the material seeds,
///    ensuring the resulting textures would shimmer with simulated data-dust."

use crate::mesh::IndexedMesh;
use glam::Vec3;

// ---------------------------------------------------------------------------
// MaterialMap
// ---------------------------------------------------------------------------

/// Zone assignment rules derived from a material_seed.
/// Deterministic — same seed + same vertex positions = same zone IDs.
#[derive(Debug, Clone)]
pub struct MaterialMap {
    /// The seed used to generate zone boundaries.
    pub seed: u64,
    /// Number of distinct zones. Always 1–16 (matches the 16-based hierarchy).
    pub zone_count: u8,
    /// Zone boundary thresholds along each axis. Derived from seed.
    zone_thresholds: [f32; 4],
}

impl MaterialMap {
    /// Create a MaterialMap from a material_seed.
    pub fn from_seed(seed: u64) -> Self {
        // Derive zone thresholds from the seed using FNV-based mixing.
        // Thresholds are in world space [-2.0, 2.0].
        let t0 = seed_to_threshold(seed,  0);
        let t1 = seed_to_threshold(seed,  1);
        let t2 = seed_to_threshold(seed,  2);
        let t3 = seed_to_threshold(seed,  3);

        // Derive zone count from seed: 4–12 zones, biased toward middle values.
        let zone_count = ((seed >> 8) % 9 + 4) as u8; // 4–12

        Self {
            seed,
            zone_count,
            zone_thresholds: [t0, t1, t2, t3],
        }
    }

    /// Assign a material zone ID (0–zone_count-1) to a world-space position.
    ///
    /// Zone assignment is based on a combination of:
    ///   - Height (Y axis) — major zone divisions
    ///   - Radial distance from origin — secondary variation
    ///   - Seed-derived noise — tertiary variation
    pub fn zone_for_position(&self, pos: Vec3) -> u8 {
        let r      = (pos.x * pos.x + pos.z * pos.z).sqrt(); // radial in XZ plane
        let height = pos.y;

        // Combine height + radius into a single value in [-2, 2]
        let combined = (height * 0.7 + r * 0.3).clamp(-2.0, 2.0);

        // Map combined value through thresholds to get zone index
        let raw_zone = if combined < self.zone_thresholds[0] {
            0
        } else if combined < self.zone_thresholds[1] {
            1
        } else if combined < self.zone_thresholds[2] {
            2
        } else if combined < self.zone_thresholds[3] {
            3
        } else {
            // Upper zone — mix based on seed
            let upper_mix = ((self.seed >> 16) % 4) as u8;
            4 + upper_mix
        };

        // Clamp to actual zone count
        (raw_zone as u8).min(self.zone_count - 1)
    }
}

/// Derive a world-space threshold from a seed and a mix index.
fn seed_to_threshold(seed: u64, mix: u64) -> f32 {
    // FNV-style mix
    let h = seed.wrapping_mul(0x517cc1b727220a95).wrapping_add(mix * 0x1234567890abcdef);
    // Map to [-1.5, 1.5] — leaves headroom at the extremes
    let normalized = ((h >> 32) as f32) / u32::MAX as f32; // 0.0–1.0
    normalized * 3.0 - 1.5
}

// ---------------------------------------------------------------------------
// Zone assignment
// ---------------------------------------------------------------------------

/// Assign material zone IDs to all vertices in the mesh.
///
/// Modifies `mesh.material_ids` in place.
/// Deterministic: same MaterialMap + same vertex positions = same IDs.
pub fn assign_zones(mesh: &mut IndexedMesh, map: &MaterialMap) {
    for (i, &pos) in mesh.vertices.iter().enumerate() {
        mesh.material_ids[i] = map.zone_for_position(pos);
    }
}

/// Convenience function: create a map from seed and assign zones in one call.
pub fn assign_zones_from_seed(mesh: &mut IndexedMesh, material_seed: u64) {
    let map = MaterialMap::from_seed(material_seed);
    assign_zones(mesh, &map);
}

/// Generate a zone summary for logging.
pub fn zone_report(mesh: &IndexedMesh, map: &MaterialMap) -> String {
    let mut counts = vec![0usize; map.zone_count as usize];
    for &id in &mesh.material_ids {
        let idx = id as usize;
        if idx < counts.len() {
            counts[idx] += 1;
        }
    }
    let parts: Vec<String> = counts
        .iter()
        .enumerate()
        .map(|(i, &c)| format!("zone{}={}", i, c))
        .collect();
    format!("material zones: {}", parts.join(", "))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_count_in_range() {
        for seed in [0u64, 1, 42, 77239104, u64::MAX] {
            let map = MaterialMap::from_seed(seed);
            assert!(map.zone_count >= 4, "zone_count should be >= 4, got {}", map.zone_count);
            assert!(map.zone_count <= 12, "zone_count should be <= 12, got {}", map.zone_count);
        }
    }

    #[test]
    fn zone_id_within_count() {
        let map = MaterialMap::from_seed(77239104); // from ceremony log
        let positions = [
            Vec3::new(0.0,  0.0, 0.0),
            Vec3::new(1.5,  1.5, 1.5),
            Vec3::new(-1.5,-1.5,-1.5),
            Vec3::new(0.5, -1.0, 0.3),
        ];
        for pos in positions {
            let zone = map.zone_for_position(pos);
            assert!(zone < map.zone_count, "zone {zone} out of range for count {}", map.zone_count);
        }
    }

    #[test]
    fn same_seed_same_result() {
        let map1 = MaterialMap::from_seed(12345);
        let map2 = MaterialMap::from_seed(12345);
        let pos  = Vec3::new(0.3, -0.5, 0.7);
        assert_eq!(map1.zone_for_position(pos), map2.zone_for_position(pos));
    }

    #[test]
    fn different_seeds_likely_differ() {
        let map1 = MaterialMap::from_seed(1);
        let map2 = MaterialMap::from_seed(77239104);
        // Test several positions — at least one should differ
        let positions = [
            Vec3::new(0.0,  1.0, 0.0),
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(-1.0, 0.0, 1.0),
        ];
        let any_differ = positions.iter().any(|&p| {
            map1.zone_for_position(p) != map2.zone_for_position(p)
        });
        assert!(any_differ, "different seeds should produce different zone assignments");
    }

    #[test]
    fn assign_zones_fills_all_vertices() {
        use crate::mesh::{Triangle, TriangleSoup, build_mesh};
        let mut soup = TriangleSoup::new();
        soup.push(Triangle {
            vertices: [Vec3::new(0.0,0.0,0.0), Vec3::new(1.0,0.0,0.0), Vec3::new(0.0,1.0,0.0)],
            normal: Vec3::Z,
        });
        let mut mesh = build_mesh(&soup);
        assign_zones_from_seed(&mut mesh, 77239104);
        assert_eq!(mesh.material_ids.len(), mesh.vertex_count());
        for &id in &mesh.material_ids {
            let map = MaterialMap::from_seed(77239104);
            assert!(id < map.zone_count);
        }
    }
}
