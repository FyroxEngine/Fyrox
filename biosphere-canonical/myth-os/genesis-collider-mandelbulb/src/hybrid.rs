/// hybrid.rs — Multi-formula slot chaining.
///
/// Evaluates a sequence of FormulaSlots in order.
/// Each slot runs its own iteration count, then passes the result
/// to the next slot. Final DE is a weighted blend of all slot outputs.
///
/// Used when KernelParams.formula_chain has more than one slot,
/// or when FormulaType::HybridChain is selected directly.

use crate::params::{FormulaSlot, FormulaType, KernelParams, MandelboxParams, MandelbulbParams};
use crate::{mandelbulb::mandelbulb_de, mandelbox::mandelbox_de};
use glam::Vec3;

/// Evaluate the full formula chain and return a combined distance estimate.
///
/// Single-slot chains: delegates directly to the appropriate DE function.
/// Multi-slot chains: blends slot outputs by their weight fields.
pub fn evaluate_chain(pos: Vec3, params: &KernelParams) -> f32 {
    let chain = &params.formula_chain;

    match chain.len() {
        0 => {
            // Empty chain — fall back to default Mandelbulb
            mandelbulb_de(pos, &params.mandelbulb)
        }
        1 => {
            // Single slot — dispatch directly, no blending overhead
            evaluate_slot(pos, &chain[0], params)
        }
        _ => {
            // Multi-slot — weighted blend
            let total_weight: f32 = chain.iter().map(|s| s.weight).sum();
            if total_weight < f32::EPSILON {
                return mandelbulb_de(pos, &params.mandelbulb);
            }

            let weighted_sum: f32 = chain
                .iter()
                .map(|slot| evaluate_slot(pos, slot, params) * slot.weight)
                .sum();

            weighted_sum / total_weight
        }
    }
}

/// Evaluate a single FormulaSlot at `pos`.
///
/// Uses the slot's formula type to select the right DE function.
/// `iteration_count` in the slot overrides params.mandelbulb.max_iterations
/// for that slot's evaluation — letting each slot run at its own depth.
fn evaluate_slot(pos: Vec3, slot: &FormulaSlot, params: &KernelParams) -> f32 {
    match &slot.formula {
        FormulaType::MandelbulbIntegerPower => {
            let overridden = MandelbulbParams {
                max_iterations: slot.iteration_count,
                ..params.mandelbulb.clone()
            };
            mandelbulb_de(pos, &overridden)
        }

        FormulaType::MandelboxAmazingBox => {
            // Use assembled mandelbox params if present, else fall back to defaults
            let box_params = params.mandelbox.clone().unwrap_or_else(|| {
                MandelboxParams {
                    max_iterations: slot.iteration_count,
                    ..MandelboxParams::default()
                }
            });
            let overridden = MandelboxParams {
                max_iterations: slot.iteration_count,
                ..box_params
            };
            mandelbox_de(pos, &overridden)
        }

        FormulaType::JuliaSet => {
            // Julia set: Mandelbulb formula with non-zero julia_offset already
            // baked into params.mandelbulb.julia_offset by collider.rs.
            // If offset is zero, this produces the same result as MandelbulbIntegerPower.
            let overridden = MandelbulbParams {
                max_iterations: slot.iteration_count,
                ..params.mandelbulb.clone()
            };
            mandelbulb_de(pos, &overridden)
        }

        FormulaType::HybridChain => {
            // HybridChain inside a slot: run both Mandelbulb and Mandelbox
            // at half iterations each, return the minimum distance.
            let half_iter = (slot.iteration_count / 2).max(1);

            let mb_params = MandelbulbParams {
                max_iterations: half_iter,
                ..params.mandelbulb.clone()
            };
            let box_params = params.mandelbox.clone().unwrap_or_default();
            let box_overridden = MandelboxParams {
                max_iterations: half_iter,
                ..box_params
            };

            let de_mb  = mandelbulb_de(pos, &mb_params);
            let de_box = mandelbox_de(pos, &box_overridden);

            de_mb.min(de_box)
        }

        FormulaType::PseudoKleinian => {
            // PseudoKleinian: Mandelbox with negative scale and extreme fold.
            // Creates layered plane / impossible geometry character.
            let kleinian = MandelboxParams {
                scale:          -1.5,
                fold_limit:     1.5,
                min_radius:     0.3,
                fixed_radius:   0.9,
                max_iterations: slot.iteration_count,
            };
            mandelbox_de(pos, &kleinian)
        }

        FormulaType::IteratedFunctionSystem => {
            // IFS: self-similar at all scales.
            // Implemented as a Mandelbulb with power=2 (Mandelbrot classic)
            // and reduced bailout to create recursive terrain character.
            let ifs = MandelbulbParams {
                power:          2.0,
                max_iterations: slot.iteration_count,
                bailout:        4.0,
                julia_offset:   params.mandelbulb.julia_offset,
            };
            mandelbulb_de(pos, &ifs)
        }

        FormulaType::Custom(_name) => {
            // Custom formula by name — not yet implemented.
            // Falls back to standard Mandelbulb until extension point is wired.
            let overridden = MandelbulbParams {
                max_iterations: slot.iteration_count,
                ..params.mandelbulb.clone()
            };
            mandelbulb_de(pos, &overridden)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{AtomManifest, FormulaSlot, FormulaType, WorldScope};

    fn test_params() -> KernelParams {
        KernelParams {
            formula_chain: vec![FormulaSlot::default()],
            mandelbulb: MandelbulbParams {
                power:          8.0,
                max_iterations: 8,
                bailout:        2.0,
                julia_offset:   Vec3::ZERO,
            },
            mandelbox:      None,
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
    fn single_slot_matches_direct_call() {
        let params = test_params();
        let pos    = Vec3::new(0.5, 0.3, 0.2);
        let chain  = evaluate_chain(pos, &params);
        let direct = mandelbulb_de(pos, &params.mandelbulb);
        assert!((chain - direct).abs() < 1e-5, "single slot chain should match direct DE");
    }

    #[test]
    fn empty_chain_falls_back_to_mandelbulb() {
        let mut params = test_params();
        params.formula_chain.clear();
        let pos = Vec3::new(1.0, 1.0, 1.0);
        let de  = evaluate_chain(pos, &params);
        assert!(!de.is_nan());
    }

    #[test]
    fn multi_slot_chain_does_not_panic() {
        let mut params = test_params();
        params.formula_chain = vec![
            FormulaSlot { formula: FormulaType::MandelbulbIntegerPower, iteration_count: 4, weight: 0.5 },
            FormulaSlot { formula: FormulaType::MandelboxAmazingBox,    iteration_count: 4, weight: 0.5 },
        ];
        let de = evaluate_chain(Vec3::new(0.5, 0.5, 0.5), &params);
        assert!(!de.is_nan());
    }

    #[test]
    fn all_formula_types_run_without_panic() {
        let params = test_params();
        let pos    = Vec3::new(0.3, 0.4, 0.5);
        let formulas = [
            FormulaType::MandelbulbIntegerPower,
            FormulaType::MandelboxAmazingBox,
            FormulaType::JuliaSet,
            FormulaType::HybridChain,
            FormulaType::PseudoKleinian,
            FormulaType::IteratedFunctionSystem,
            FormulaType::Custom("test_custom".into()),
        ];
        for formula in formulas {
            let slot = FormulaSlot { formula, iteration_count: 4, weight: 1.0 };
            let de   = evaluate_slot(pos, &slot, &params);
            assert!(!de.is_nan(), "formula produced NaN");
        }
    }
}
