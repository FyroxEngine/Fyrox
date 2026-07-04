/// params.rs — KernelParams and all supporting data types.
///
/// Pure data. No logic. Every type here is Serialize + Deserialize.
/// Collider assembles these. Field consumes them. Output records them.

use glam::Vec3;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// World scope — passed in from the genesis seed
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldScope {
    /// Unique identifier for this world.
    pub world_id: String,
    /// Resonance frequency derived from the genesis seed (Hz).
    /// Used to set the default iso_threshold via resonance_mod.
    pub resonance_hz: f64,
    /// Voxel resolution per axis. Cube assumed: [n, n, n].
    pub resolution: u32,
}

// ---------------------------------------------------------------------------
// ATOM manifest — 16 slots across 4 phases
// ---------------------------------------------------------------------------

/// The exact ATOM IDs loaded into each Collider slot.
/// Same manifest + same seed = same world. Fully deterministic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomManifest {
    /// Slots 1–4. Controls: power, formula_slot[0..1]
    pub void_phase: [String; 4],
    /// Slots 5–8. Controls: max_iterations, bailout, formula_slot[2]
    pub spark_phase: [String; 4],
    /// Slots 9–12. Controls: julia_offset x/y/z, formula_slot[3]
    pub law_phase: [String; 4],
    /// Slots 13–16. Controls: mandelbox params, material_seed, resonance_mod, formula_slot[4..5]
    pub bloom_phase: [String; 4],
}

// ---------------------------------------------------------------------------
// ATOM department prefix — determines which parameter the ATOM influences
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtomDepartment {
    /// SYS_ — influences `power`
    Sys,
    /// SEC_ — influences `bailout`
    Sec,
    /// NAV_ — influences `max_iterations`
    Nav,
    /// COG_ — influences `julia_offset`
    Cog,
    /// NAR_ — selects `formula_slot`
    Nar,
    /// UI_  — sets `material_seed`
    Ui,
    /// MED_ — sets `resonance_mod`
    Med,
    /// Unrecognised prefix — no effect on kernel params
    Unknown,
}

impl AtomDepartment {
    pub fn from_id(atom_id: &str) -> Self {
        let id = atom_id.to_uppercase();
        if id.starts_with("SYS_") {
            Self::Sys
        } else if id.starts_with("SEC_") {
            Self::Sec
        } else if id.starts_with("NAV_") {
            Self::Nav
        } else if id.starts_with("COG_") {
            Self::Cog
        } else if id.starts_with("NAR_") {
            Self::Nar
        } else if id.starts_with("UI_") {
            Self::Ui
        } else if id.starts_with("MED_") {
            Self::Med
        } else {
            Self::Unknown
        }
    }
}

// ---------------------------------------------------------------------------
// ATOM status — determines intensity multiplier
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AtomStatus {
    Validated,
    Stable,
    Active,
    InDevelopment,
    GhostLogic,
    Conceptual,
    CriticalPatch,
}

impl AtomStatus {
    pub fn intensity(&self) -> f32 {
        match self {
            Self::Validated | Self::Stable => 1.0,
            Self::Active                   => 1.2,
            Self::InDevelopment            => 0.8,
            Self::GhostLogic               => 0.6,
            Self::Conceptual               => 0.3,
            Self::CriticalPatch            => 1.5,
        }
    }
}

// ---------------------------------------------------------------------------
// Formula types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormulaType {
    /// Bulbous, organic, spherical. Default. Most stable.
    MandelbulbIntegerPower,
    /// Boxlike, architectural, interior spaces. City-like, cave systems.
    MandelboxAmazingBox,
    /// Fixed attractor, uniform geometry. Crystalline, symmetrical.
    JuliaSet,
    /// Multiple formulas in sequence. Unpredictable, emergent.
    HybridChain,
    /// Layered planes, non-Euclidean. Impossible geometries.
    PseudoKleinian,
    /// Self-similar at all scales. Recursive terrain, fractal forests.
    IteratedFunctionSystem,
    /// Caller-defined formula by name — reserved for future extension.
    Custom(String),
}

impl Default for FormulaType {
    fn default() -> Self {
        Self::MandelbulbIntegerPower
    }
}

/// One slot in the formula chain. Up to 6 slots run in sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaSlot {
    pub formula:         FormulaType,
    /// How many iterations this slot runs before passing to the next.
    pub iteration_count: u32,
    /// Blend weight when mixing outputs in a HybridChain. 0.0–1.0.
    pub weight:          f32,
}

impl Default for FormulaSlot {
    fn default() -> Self {
        Self {
            formula:         FormulaType::default(),
            iteration_count: 8,
            weight:          1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Mandelbulb parameters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MandelbulbParams {
    /// Mandelbulb exponent. Low = smooth organic. High = angular spikes.
    /// Range: 1.0–16.0. Classic = 8.0.
    pub power:          f32,
    /// Iteration depth. More = finer detail, higher computation.
    /// Range: 4–30.
    pub max_iterations: u32,
    /// Escape threshold. High = denser surface before escape.
    /// Range: 1.0–100.0.
    pub bailout:        f32,
    /// Attractor offset. Shifts world shape without changing formula.
    /// Vec3::ZERO = standard Mandelbulb. Non-zero = Julia-shifted.
    pub julia_offset:   Vec3,
}

impl Default for MandelbulbParams {
    fn default() -> Self {
        Self {
            power:          8.0,
            max_iterations: 8,
            bailout:        2.0,
            julia_offset:   Vec3::ZERO,
        }
    }
}

// ---------------------------------------------------------------------------
// Mandelbox parameters (active when formula = MandelboxAmazingBox)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MandelboxParams {
    /// Primary parameter. Negative scale inverts structure.
    /// Range: -3.0–3.0.
    pub scale:        f32,
    /// Box fold clamp. Low = tight angular. High = organic.
    /// Range: 0.5–2.0.
    pub fold_limit:   f32,
    /// Sphere fold inner radius. Creates interior voids.
    /// Range: 0.0–1.0.
    pub min_radius:   f32,
    /// Sphere fold boundary. Creates interior complexity.
    /// Range: 0.5–2.0.
    pub fixed_radius: f32,
    /// Iteration depth shared with the Mandelbox loop.
    pub max_iterations: u32,
}

impl Default for MandelboxParams {
    fn default() -> Self {
        Self {
            scale:          -2.0,
            fold_limit:     1.0,
            min_radius:     0.5,
            fixed_radius:   1.0,
            max_iterations: 8,
        }
    }
}

// ---------------------------------------------------------------------------
// The assembled kernel params — produced by collider.rs, consumed by field.rs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelParams {
    /// Formula chain. Up to 6 slots evaluated in sequence.
    pub formula_chain:  Vec<FormulaSlot>,
    /// Assembled Mandelbulb params (always present — fallback if no box formula).
    pub mandelbulb:     MandelbulbParams,
    /// Mandelbox params — Some when any BLOOM NAR_ ATOM selects MandelboxAmazingBox.
    pub mandelbox:      Option<MandelboxParams>,
    /// Passed to Void Sculptor for surface material assignment. No geometry effect.
    pub material_seed:  u64,
    /// Adjusts iso-threshold offset derived from seed resonance_hz.
    pub resonance_mod:  f32,
    /// From genesis seed.
    pub resonance_hz:   f64,
    /// From genesis seed.
    pub scope:          WorldScope,
    /// Snapshot of which ATOMs produced these params. Reproducibility record.
    pub atom_manifest:  AtomManifest,
}
