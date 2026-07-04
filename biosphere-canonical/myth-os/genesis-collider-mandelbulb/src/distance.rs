/// distance.rs — Distance estimation dispatch.
///
/// The single call site that field.rs uses for every voxel.
/// Selects the right evaluation path based on KernelParams:
///   - Single Mandelbulb slot   → direct mandelbulb_de()
///   - Single Mandelbox slot    → direct mandelbox_de()
///   - Everything else          → hybrid chain evaluation
///
/// Keeping the hot path flat avoids one level of indirection in the
/// rayon parallel loop.

use crate::hybrid::evaluate_chain;
use crate::mandelbulb::mandelbulb_de;
use crate::mandelbox::mandelbox_de;
use crate::params::{FormulaType, KernelParams};
use glam::Vec3;

/// Compute the signed distance estimate from `pos` to the world surface.
///
/// Positive  → outside the set (safe to march toward).
/// Near zero → on the surface.
/// Negative  → inside the set (typical for bulb interiors).
#[inline]
pub fn distance_estimate(pos: Vec3, params: &KernelParams) -> f32 {
    // Fast path: single-slot formula — skip chain overhead entirely.
    if params.formula_chain.len() == 1 {
        match &params.formula_chain[0].formula {
            FormulaType::MandelbulbIntegerPower | FormulaType::JuliaSet => {
                return mandelbulb_de(pos, &params.mandelbulb);
            }
            FormulaType::MandelboxAmazingBox => {
                if let Some(ref box_params) = params.mandelbox {
                    return mandelbox_de(pos, box_params);
                }
                // Mandelbox selected but params not assembled — fall through
            }
            _ => {} // HybridChain, PseudoKleinian, IFS, Custom → full chain
        }
    }

    // General path: multi-slot or non-trivial formula type
    evaluate_chain(pos, params)
}

/// Compute the iso-threshold from the genesis seed.
///
/// The surface boundary in the scalar field. Points with a field value
/// below this threshold are considered inside the surface.
///
/// Derived from resonance_hz with resonance_mod offset applied.
pub fn iso_threshold(params: &KernelParams) -> f32 {
    // Base: map resonance_hz to a small float in [0.001, 0.05]
    // 440 Hz (concert A) → ~0.01, which is a reasonable default threshold.
    let base = (params.resonance_hz as f32 / 44_000.0).clamp(0.001, 0.05);
    (base + params.resonance_mod).clamp(0.0001, 0.1)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{AtomManifest, FormulaSlot, FormulaType, MandelbulbParams, WorldScope};

    fn minimal_params() -> KernelParams {
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
                resolution:   32,
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
    fn fast_path_mandelbulb_matches_direct() {
        let params = minimal_params();
        let pos    = Vec3::new(0.5, 0.3, 0.2);
        let fast   = distance_estimate(pos, &params);
        let direct = mandelbulb_de(pos, &params.mandelbulb);
        assert!((fast - direct).abs() < 1e-6, "fast path mismatch: {fast} vs {direct}");
    }

    #[test]
    fn iso_threshold_is_positive() {
        let params = minimal_params();
        assert!(iso_threshold(&params) > 0.0);
    }

    #[test]
    fn iso_threshold_resonance_mod_shifts_value() {
        let base_params = minimal_params();
        let mut shifted = minimal_params();
        shifted.resonance_mod = 0.005;
        assert!(iso_threshold(&shifted) > iso_threshold(&base_params));
    }
}
