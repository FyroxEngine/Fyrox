/// collider.rs — The Genesis Collider.
///
/// Takes an AtomManifest + WorldScope and produces a KernelParams.
/// This is the only file that knows about ATOM department rules and
/// intensity multipliers. Everything downstream just consumes KernelParams.
///
/// Rules:
/// - Each ATOM is identified by its ID prefix (department).
/// - The phase it sits in determines which parameter family it feeds.
/// - Status multiplies the raw contribution.
/// - Multiple ATOMs from the same department in the same phase amplify each other.
/// - ATOMs loaded outside their natural phase affinity create cross-phase
///   interference — unusual combinations that produce unexpected geometry.
///   This is intentional.

use crate::params::{
    AtomDepartment, AtomManifest, AtomStatus, FormulaSlot, FormulaType, KernelParams,
    MandelboxParams, MandelbulbParams, WorldScope,
};
use glam::Vec3;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Assemble a KernelParams from an AtomManifest and WorldScope.
///
/// Caller is responsible for supplying ATOM statuses via the AtomStatusMap.
/// If a status is unknown, Stable (1.0) is assumed.
pub fn assemble_params(
    manifest:   &AtomManifest,
    scope:      WorldScope,
    status_map: &AtomStatusMap,
) -> KernelParams {
    let mut builder = ParamBuilder::new(scope.clone(), manifest.clone());

    // Phase 1 — VOID: power, formula_slot[0..1]
    for atom_id in &manifest.void_phase {
        let status = status_map.get(atom_id);
        builder.apply_void(atom_id, &status);
    }

    // Phase 2 — SPARK: max_iterations, bailout, formula_slot[2]
    for atom_id in &manifest.spark_phase {
        let status = status_map.get(atom_id);
        builder.apply_spark(atom_id, &status);
    }

    // Phase 3 — LAW: julia_offset x/y/z, formula_slot[3]
    for atom_id in &manifest.law_phase {
        let status = status_map.get(atom_id);
        builder.apply_law(atom_id, &status);
    }

    // Phase 4 — BLOOM: mandelbox params, material_seed, resonance_mod, formula_slot[4..5]
    for atom_id in &manifest.bloom_phase {
        let status = status_map.get(atom_id);
        builder.apply_bloom(atom_id, &status);
    }

    builder.build()
}

// ---------------------------------------------------------------------------
// AtomStatusMap — caller supplies statuses keyed by ATOM ID
// ---------------------------------------------------------------------------

/// Lookup table: ATOM ID → AtomStatus.
/// Missing entries default to AtomStatus::Stable (multiplier 1.0).
pub struct AtomStatusMap(std::collections::HashMap<String, AtomStatus>);

impl AtomStatusMap {
    pub fn new() -> Self {
        Self(std::collections::HashMap::new())
    }

    pub fn insert(&mut self, atom_id: impl Into<String>, status: AtomStatus) {
        self.0.insert(atom_id.into(), status);
    }

    pub fn get(&self, atom_id: &str) -> AtomStatus {
        self.0
            .get(atom_id)
            .cloned()
            .unwrap_or(AtomStatus::Stable)
    }
}

impl Default for AtomStatusMap {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ParamBuilder — accumulates contributions across all 16 slots
// ---------------------------------------------------------------------------

struct ParamBuilder {
    scope:    WorldScope,
    manifest: AtomManifest,

    // Mandelbulb accumulators
    power_acc:      f32,
    power_count:    u32,
    iter_acc:       f32,
    iter_count:     u32,
    bailout_acc:    f32,
    bailout_count:  u32,

    // Julia offset accumulators (one per axis, filled in order by COG_ ATOMs)
    julia_x:        f32,
    julia_y:        f32,
    julia_z:        f32,
    cog_count:      u32, // tracks which axis the next COG_ ATOM fills

    // Formula chain (up to 6 slots)
    formula_slots:  Vec<FormulaSlot>,

    // Mandelbox accumulators (only populated when a NAR_ ATOM selects MandelboxAmazingBox)
    box_scale_acc:      f32,
    box_fold_acc:       f32,
    box_minr_acc:       f32,
    box_fixr_acc:       f32,
    box_iter_acc:       f32,
    box_count:          u32,

    // Downstream pass-through
    material_seed:  u64,
    resonance_mod:  f32,
}

impl ParamBuilder {
    fn new(scope: WorldScope, manifest: AtomManifest) -> Self {
        Self {
            scope,
            manifest,
            power_acc:          0.0,
            power_count:        0,
            iter_acc:           0.0,
            iter_count:         0,
            bailout_acc:        0.0,
            bailout_count:      0,
            julia_x:            0.0,
            julia_y:            0.0,
            julia_z:            0.0,
            cog_count:          0,
            formula_slots:      Vec::with_capacity(6),
            box_scale_acc:      0.0,
            box_fold_acc:       0.0,
            box_minr_acc:       0.0,
            box_fixr_acc:       0.0,
            box_iter_acc:       0.0,
            box_count:          0,
            material_seed:      0,
            resonance_mod:      0.0,
        }
    }

    // -----------------------------------------------------------------------
    // Phase application methods
    // -----------------------------------------------------------------------

    /// VOID phase: primary affinity = power, formula_slot[0..1]
    fn apply_void(&mut self, atom_id: &str, status: &AtomStatus) {
        let m = status.intensity();
        match AtomDepartment::from_id(atom_id) {
            AtomDepartment::Sys => {
                // SYS_ in VOID — primary home. Contributes to power.
                self.power_acc   += 8.0 * m; // base 8.0 scaled by intensity
                self.power_count += 1;
            }
            AtomDepartment::Nar => {
                // NAR_ in VOID — formula slot selection
                self.push_formula_slot(FormulaType::MandelbulbIntegerPower, m);
            }
            AtomDepartment::Nav => {
                // Cross-phase: NAV_ in VOID — mild iteration contribution
                self.iter_acc   += 6.0 * m;
                self.iter_count += 1;
            }
            AtomDepartment::Sec => {
                // Cross-phase: SEC_ in VOID — mild bailout contribution
                self.bailout_acc   += 2.0 * m;
                self.bailout_count += 1;
            }
            _ => {} // UI_, MED_, COG_, Unknown — no VOID affinity
        }
    }

    /// SPARK phase: primary affinity = max_iterations, bailout, formula_slot[2]
    fn apply_spark(&mut self, atom_id: &str, status: &AtomStatus) {
        let m = status.intensity();
        match AtomDepartment::from_id(atom_id) {
            AtomDepartment::Nav => {
                // NAV_ in SPARK — primary home. max_iterations.
                self.iter_acc   += 8.0 * m;
                self.iter_count += 1;
            }
            AtomDepartment::Sec => {
                // SEC_ in SPARK — primary home. bailout.
                self.bailout_acc   += 10.0 * m;
                self.bailout_count += 1;
            }
            AtomDepartment::Nar => {
                // NAR_ in SPARK — formula slot [2]
                self.push_formula_slot(FormulaType::MandelbulbIntegerPower, m);
            }
            AtomDepartment::Sys => {
                // Cross-phase: SYS_ in SPARK — power cross-contribution
                self.power_acc   += 4.0 * m;
                self.power_count += 1;
            }
            _ => {}
        }
    }

    /// LAW phase: primary affinity = julia_offset, formula_slot[3]
    fn apply_law(&mut self, atom_id: &str, status: &AtomStatus) {
        let m = status.intensity();
        match AtomDepartment::from_id(atom_id) {
            AtomDepartment::Cog => {
                // COG_ in LAW — primary home. Fills julia_offset axes in order.
                let offset = 0.5 * m; // base offset scaled by intensity
                match self.cog_count {
                    0 => self.julia_x += offset,
                    1 => self.julia_y += offset,
                    2 => self.julia_z += offset,
                    _ => {
                        // 4+ COG_ ATOMs: amplify all axes
                        self.julia_x += offset * 0.5;
                        self.julia_y += offset * 0.5;
                        self.julia_z += offset * 0.5;
                    }
                }
                self.cog_count += 1;
            }
            AtomDepartment::Nar => {
                // NAR_ in LAW — formula slot [3], can trigger Mandelbox
                self.push_formula_slot(FormulaType::MandelboxAmazingBox, m);
                // A NAR_ ATOM in LAW that selects Mandelbox seeds box params
                self.accumulate_box_defaults(m);
            }
            AtomDepartment::Nav => {
                // Cross-phase: NAV_ in LAW — iteration bleed
                self.iter_acc   += 4.0 * m;
                self.iter_count += 1;
            }
            _ => {}
        }
    }

    /// BLOOM phase: primary affinity = mandelbox params, material_seed, resonance_mod, formula_slot[4..5]
    fn apply_bloom(&mut self, atom_id: &str, status: &AtomStatus) {
        let m = status.intensity();
        match AtomDepartment::from_id(atom_id) {
            AtomDepartment::Nar => {
                // NAR_ in BLOOM — formula slots [4..5], Mandelbox params
                self.push_formula_slot(FormulaType::MandelboxAmazingBox, m);
                self.accumulate_box_defaults(m);
            }
            AtomDepartment::Ui => {
                // UI_ in BLOOM — material_seed (XOR successive seeds together)
                let seed_contribution = self.seed_from_id(atom_id);
                self.material_seed ^= seed_contribution;
            }
            AtomDepartment::Med => {
                // MED_ in BLOOM — resonance_mod offset
                self.resonance_mod += 0.1 * m;
            }
            AtomDepartment::Cog => {
                // Cross-phase: COG_ in BLOOM — additional julia offset
                self.julia_x += 0.25 * m;
                self.julia_y += 0.25 * m;
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Push a formula slot if the chain has room (max 6).
    fn push_formula_slot(&mut self, formula: FormulaType, intensity: f32) {
        if self.formula_slots.len() < 6 {
            self.formula_slots.push(FormulaSlot {
                formula,
                iteration_count: (8.0 * intensity).round() as u32,
                weight: intensity.clamp(0.0, 1.0),
            });
        }
    }

    /// Accumulate Mandelbox parameter defaults scaled by intensity.
    fn accumulate_box_defaults(&mut self, m: f32) {
        self.box_scale_acc += -2.0 * m;
        self.box_fold_acc  +=  1.0 * m;
        self.box_minr_acc  +=  0.5 * m;
        self.box_fixr_acc  +=  1.0 * m;
        self.box_iter_acc  +=  8.0 * m;
        self.box_count     += 1;
    }

    /// Derive a u64 seed from an ATOM ID string (deterministic).
    fn seed_from_id(&self, atom_id: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325; // FNV offset basis
        for byte in atom_id.bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(0x100000001b3); // FNV prime
        }
        h
    }

    // -----------------------------------------------------------------------
    // Final assembly
    // -----------------------------------------------------------------------

    fn build(self) -> KernelParams {
        // Derive power — average of contributions, clamped to [1.0, 16.0]
        let power = if self.power_count > 0 {
            (self.power_acc / self.power_count as f32).clamp(1.0, 16.0)
        } else {
            8.0 // default: classic Mandelbulb
        };

        // Derive max_iterations — average clamped to [4, 30]
        let max_iterations = if self.iter_count > 0 {
            ((self.iter_acc / self.iter_count as f32).round() as u32).clamp(4, 30)
        } else {
            8
        };

        // Derive bailout — average clamped to [1.0, 100.0]
        let bailout = if self.bailout_count > 0 {
            (self.bailout_acc / self.bailout_count as f32).clamp(1.0, 100.0)
        } else {
            2.0
        };

        // Julia offset — clamp each axis to [-2.0, 2.0]
        let julia_offset = Vec3::new(
            self.julia_x.clamp(-2.0, 2.0),
            self.julia_y.clamp(-2.0, 2.0),
            self.julia_z.clamp(-2.0, 2.0),
        );

        // Mandelbox — only Some if at least one box slot was pushed
        let mandelbox = if self.box_count > 0 {
            let n = self.box_count as f32;
            Some(MandelboxParams {
                scale:          (self.box_scale_acc / n).clamp(-3.0, 3.0),
                fold_limit:     (self.box_fold_acc  / n).clamp(0.5, 2.0),
                min_radius:     (self.box_minr_acc  / n).clamp(0.0, 1.0),
                fixed_radius:   (self.box_fixr_acc  / n).clamp(0.5, 2.0),
                max_iterations: ((self.box_iter_acc / n).round() as u32).clamp(4, 30),
            })
        } else {
            None
        };

        // Formula chain — default to a single MandelbulbIntegerPower slot if empty
        let formula_chain = if self.formula_slots.is_empty() {
            vec![FormulaSlot::default()]
        } else {
            self.formula_slots
        };

        KernelParams {
            formula_chain,
            mandelbulb: MandelbulbParams {
                power,
                max_iterations,
                bailout,
                julia_offset,
            },
            mandelbox,
            material_seed:  self.material_seed,
            resonance_mod:  self.resonance_mod,
            resonance_hz:   self.scope.resonance_hz,
            atom_manifest:  self.manifest,
            scope:          self.scope,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_scope() -> WorldScope {
        WorldScope {
            world_id:   "test-world".into(),
            resonance_hz: 440.0,
            resolution:   64,
        }
    }

    fn test_manifest() -> AtomManifest {
        AtomManifest {
            void_phase:  ["SYS_04".into(), "NAR_02".into(), "SYS_07".into(), "UI_01".into()],
            spark_phase: ["NAV_04".into(), "SEC_02".into(), "NAV_01".into(), "NAR_06".into()],
            law_phase:   ["COG_01".into(), "COG_06".into(), "NAR_03".into(), "SEC_01".into()],
            bloom_phase: ["MED_001".into(), "NAR_07".into(), "COG_04".into(), "UI_05".into()],
        }
    }

    #[test]
    fn assembles_without_panic() {
        let params = assemble_params(&test_manifest(), test_scope(), &AtomStatusMap::default());
        assert!(params.mandelbulb.power    >= 1.0);
        assert!(params.mandelbulb.power    <= 16.0);
        assert!(params.mandelbulb.bailout  >= 1.0);
        assert!(params.mandelbulb.bailout  <= 100.0);
        assert!(params.mandelbulb.max_iterations >= 4);
        assert!(params.mandelbulb.max_iterations <= 30);
    }

    #[test]
    fn formula_chain_not_empty() {
        let params = assemble_params(&test_manifest(), test_scope(), &AtomStatusMap::default());
        assert!(!params.formula_chain.is_empty());
    }

    #[test]
    fn default_power_is_eight_with_no_sys_atoms() {
        let manifest = AtomManifest {
            void_phase:  ["NAV_01".into(), "NAV_02".into(), "NAV_03".into(), "NAV_04".into()],
            spark_phase: ["NAV_05".into(), "NAV_06".into(), "NAV_07".into(), "NAV_08".into()],
            law_phase:   ["NAV_09".into(), "NAV_10".into(), "NAV_11".into(), "NAV_12".into()],
            bloom_phase: ["NAV_13".into(), "NAV_14".into(), "NAV_15".into(), "NAV_16".into()],
        };
        let params = assemble_params(&manifest, test_scope(), &AtomStatusMap::default());
        assert_eq!(params.mandelbulb.power, 8.0);
    }

    #[test]
    fn critical_patch_status_amplifies() {
        let manifest = AtomManifest {
            void_phase:  ["SYS_01".into(), "SYS_02".into(), "SYS_03".into(), "SYS_04".into()],
            spark_phase: ["NAV_01".into(), "NAV_02".into(), "NAV_03".into(), "NAV_04".into()],
            law_phase:   ["COG_01".into(), "COG_02".into(), "COG_03".into(), "COG_04".into()],
            bloom_phase: ["MED_01".into(), "MED_02".into(), "MED_03".into(), "MED_04".into()],
        };
        let mut stable_map  = AtomStatusMap::new();
        let mut patched_map = AtomStatusMap::new();
        stable_map.insert("SYS_01",  AtomStatus::Stable);
        patched_map.insert("SYS_01", AtomStatus::CriticalPatch);

        let stable  = assemble_params(&manifest, test_scope(), &stable_map);
        let patched = assemble_params(&manifest, test_scope(), &patched_map);

        // CriticalPatch (1.5) should produce a higher power than Stable (1.0)
        assert!(patched.mandelbulb.power >= stable.mandelbulb.power);
    }
}
