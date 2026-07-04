/// field.rs — 3D scalar field generation.
///
/// Allocates an ndarray of shape [resolution, resolution, resolution].
/// Iterates the voxel grid in parallel via rayon.
/// Each voxel stores a 32-bit float distance estimate.
///
/// The field is defined over the unit cube [-2.0, 2.0]³ by default —
/// the standard viewing volume for Mandelbulb/Mandelbox.
///
/// Output: ndarray::Array3<f32> + the iso_threshold used to define the surface.

use crate::distance::{distance_estimate, iso_threshold};
use crate::params::KernelParams;
use glam::Vec3;
use ndarray::Array3;
use rayon::prelude::*;

/// The spatial extent of the voxel field.
/// All worlds are sampled over this cube.
pub const FIELD_MIN: f32 = -2.0;
pub const FIELD_MAX: f32 =  2.0;

/// Result of a field generation pass.
pub struct ScalarField {
    /// Raw distance values. Shape: [res, res, res].
    pub data:          Array3<f32>,
    /// Resolution per axis (cube assumed).
    pub resolution:    u32,
    /// Surface boundary. Voxels below this value are inside the surface.
    pub iso_threshold: f32,
}

/// Generate the full scalar field for the given KernelParams.
///
/// Runs in parallel across the Z axis slices via rayon.
/// Returns a ScalarField ready to be written to disk by output.rs.
pub fn generate(params: &KernelParams) -> ScalarField {
    let res  = params.scope.resolution as usize;
    let iso  = iso_threshold(params);
    let step = (FIELD_MAX - FIELD_MIN) / res as f32;

    // Allocate flat buffer: res³ values
    // rayon fills slices in parallel, then we reassemble into Array3.
    let total = res * res * res;
    let mut flat: Vec<f32> = vec![0.0; total];

    // Parallel fill over Z slices.
    // Each Z slice is a contiguous block of res*res values.
    flat.par_chunks_mut(res * res)
        .enumerate()
        .for_each(|(iz, slice)| {
            let z = FIELD_MIN + (iz as f32 + 0.5) * step;
            for iy in 0..res {
                let y = FIELD_MIN + (iy as f32 + 0.5) * step;
                for ix in 0..res {
                    let x   = FIELD_MIN + (ix as f32 + 0.5) * step;
                    let pos = Vec3::new(x, y, z);
                    slice[iy * res + ix] = distance_estimate(pos, params);
                }
            }
        });

    // Reshape into Array3<f32> with shape [res, res, res].
    // ndarray uses row-major order; our layout is [z][y][x].
    let data = Array3::from_shape_vec((res, res, res), flat)
        .expect("flat buffer length must equal res³");

    ScalarField {
        data,
        resolution: res as u32,
        iso_threshold: iso,
    }
}

/// Count voxels that are on or inside the surface (DE ≤ iso_threshold).
/// Useful for quick sanity checks and progress reporting.
pub fn surface_voxel_count(field: &ScalarField) -> usize {
    field
        .data
        .iter()
        .filter(|&&v| v <= field.iso_threshold)
        .count()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{
        AtomManifest, FormulaSlot, FormulaType, MandelbulbParams, WorldScope,
    };

    fn minimal_params(resolution: u32) -> KernelParams {
        KernelParams {
            formula_chain: vec![FormulaSlot {
                formula:         FormulaType::MandelbulbIntegerPower,
                iteration_count: 8,
                weight:          1.0,
            }],
            mandelbulb: MandelbulbParams::default(),
            mandelbox:  None,
            material_seed:  0,
            resonance_mod:  0.0,
            resonance_hz:   440.0,
            scope: WorldScope {
                world_id:     "test".into(),
                resonance_hz: 440.0,
                resolution,
            },
            atom_manifest: AtomManifest {
                void_phase:  ["".into(), "".into(), "".into(), "".into()],
                spark_phase: ["".into(), "".into(), "".into(), "".into()],
                law_phase:   ["".into(), "".into(), "".into(), "".into()],
                bloom_phase: ["".into(), "".into(), "".into(), "".into()],
            },
        }
    }

    #[test]
    fn field_shape_matches_resolution() {
        let params = minimal_params(8);
        let field  = generate(&params);
        assert_eq!(field.data.shape(), &[8, 8, 8]);
    }

    #[test]
    fn no_nan_in_field() {
        let params = minimal_params(8);
        let field  = generate(&params);
        for &v in field.data.iter() {
            assert!(!v.is_nan(), "NaN found in scalar field");
        }
    }

    #[test]
    fn classic_mandelbulb_has_interior_voxels() {
        // With power=8 and resolution=16 we should find some voxels inside the set.
        let params = minimal_params(16);
        let field  = generate(&params);
        let count  = surface_voxel_count(&field);
        assert!(count > 0, "expected some interior voxels, got 0");
    }

    #[test]
    fn iso_threshold_is_positive() {
        let params = minimal_params(4);
        let field  = generate(&params);
        assert!(field.iso_threshold > 0.0);
    }
}
