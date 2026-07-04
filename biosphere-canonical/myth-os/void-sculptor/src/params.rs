/// params.rs — Void Sculptor input and configuration types.
///
/// SculptorInput is assembled from the incoming SPA WirePacket.
/// SculptorConfig controls how extraction runs.
/// These are the only types the sculptor needs from the outside world.

use mandelbulb::params::{AtomManifest, KernelParams};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Extraction mode
// ---------------------------------------------------------------------------

/// Which iso-surface extraction algorithm to use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtractionMode {
    /// Propagating Contours — fast, seed-based, 30–50× faster than brute force.
    /// Primary mode. Matches Acropora's default "Fast Analysis" setting.
    PropagatingContours,

    /// Marching Cubes — brute force, evaluates every voxel.
    /// Fallback when no valid seed points can be resolved.
    /// Also used for validation and ground-truth comparison.
    MarchingCubes,
}

impl Default for ExtractionMode {
    fn default() -> Self {
        Self::PropagatingContours
    }
}

// ---------------------------------------------------------------------------
// Sculptor configuration
// ---------------------------------------------------------------------------

/// Controls how the Void Sculptor runs.
/// These are the knobs — independent of what world is being sculpted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SculptorConfig {
    /// Which extraction algorithm to use.
    pub mode: ExtractionMode,

    /// If PropagatingContours finds zero seeds, automatically fall back
    /// to MarchingCubes rather than producing an empty mesh.
    pub auto_fallback: bool,

    /// Minimum triangle count. If the extracted mesh has fewer triangles
    /// than this, the sculptor retries with MarchingCubes.
    /// Set to 0 to disable.
    pub min_triangle_count: usize,

    /// Apply Laplacian smoothing passes after extraction.
    /// 0 = no smoothing. 1–3 = mild. 4+ = aggressive.
    pub smooth_passes: u32,

    /// Remove disconnected triangle islands smaller than this face count.
    /// Set to 0 to disable island removal.
    pub min_island_size: usize,
}

impl Default for SculptorConfig {
    fn default() -> Self {
        Self {
            mode:               ExtractionMode::PropagatingContours,
            auto_fallback:      true,
            min_triangle_count: 8,
            smooth_passes:      1,
            min_island_size:    4,
        }
    }
}

// ---------------------------------------------------------------------------
// Sculptor input — assembled from the incoming WirePacket
// ---------------------------------------------------------------------------

/// Everything the Void Sculptor needs to do its job.
/// Populated by the caller from the SPA WirePacket emitted by mandelbulb.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SculptorInput {
    /// World identifier — carried through to the output packet.
    pub world_id: String,

    /// Path to the .raw scalar field on disk.
    /// Written by mandelbulb::output::write_and_emit().
    pub raw_field_path: PathBuf,

    /// Voxel resolution per axis. Cube assumed: [n, n, n].
    pub field_dimensions: [u32; 3],

    /// The iso-threshold value used to define the surface boundary.
    /// Points with field value ≤ iso_threshold are inside the surface.
    pub iso_threshold: f32,

    /// Assembled kernel params from the mandelbulb kernel.
    /// Used for material zone assignment and downstream metadata.
    pub kernel_params: Box<KernelParams>,

    /// ATOM manifest that produced the kernel params.
    /// Carried through for reproducibility records.
    pub atom_manifest: AtomManifest,

    /// Passed from mandelbulb — drives material zone assignment.
    pub material_seed: u64,

    /// Resonance modifier — carried through for downstream modules.
    pub resonance_mod: f32,

    /// Sculptor configuration. Defaults are good for most worlds.
    pub config: SculptorConfig,
}

impl SculptorInput {
    /// Construct a SculptorInput with default config from the minimum required fields.
    pub fn new(
        world_id:         impl Into<String>,
        raw_field_path:   PathBuf,
        field_dimensions: [u32; 3],
        iso_threshold:    f32,
        kernel_params:    KernelParams,
        atom_manifest:    AtomManifest,
        material_seed:    u64,
        resonance_mod:    f32,
    ) -> Self {
        Self {
            world_id:        world_id.into(),
            raw_field_path,
            field_dimensions,
            iso_threshold,
            kernel_params:   Box::new(kernel_params),
            atom_manifest,
            material_seed,
            resonance_mod,
            config:          SculptorConfig::default(),
        }
    }

    /// Override the extraction config.
    pub fn with_config(mut self, config: SculptorConfig) -> Self {
        self.config = config;
        self
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_propagating() {
        assert_eq!(ExtractionMode::default(), ExtractionMode::PropagatingContours);
    }

    #[test]
    fn default_config_has_fallback_enabled() {
        assert!(SculptorConfig::default().auto_fallback);
    }

    #[test]
    fn config_serializes_round_trip() {
        let cfg  = SculptorConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: SculptorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.smooth_passes, cfg.smooth_passes);
        assert_eq!(back.min_island_size, cfg.min_island_size);
    }
}
