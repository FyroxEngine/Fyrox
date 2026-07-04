/// output.rs — Write scalar field to disk and assemble the WirePacket.
///
/// The only file in this crate that touches myth-wire.
/// Everything upstream is pure math; this is where the results leave.
///
/// Writes:
///   <output_dir>/<world_id>.raw          — 32-bit float scalar field, little-endian
///   <output_dir>/kernel_params.json      — assembled params for inspection/reproducibility
///
/// Emits:
///   WirePacket { source: "mandelbulb", wire_type: SPA, payload: MandelbulbOutput { … } }

use crate::field::ScalarField;
use crate::params::{AtomManifest, KernelParams};
use myth_wire::{MythId, WirePacket, WireType};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Payload type — bincode-encoded into WirePacket.payload
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MandelbulbOutput {
    pub world_id:         String,
    pub raw_field_path:   PathBuf,
    pub field_dimensions: [u32; 3],
    pub iso_threshold:    f32,
    pub kernel_params:    Box<KernelParams>,
    pub atom_manifest:    AtomManifest,
    pub material_seed:    u64,
    pub resonance_mod:    f32,
}

/// Write the scalar field and params to disk, then return the assembled WirePacket.
///
/// `output_dir` must exist before calling this function.
/// Caller is responsible for creating it.
pub fn write_and_emit(
    field:      &ScalarField,
    params:     &KernelParams,
    output_dir: &Path,
) -> Result<WirePacket, OutputError> {
    // --- Write .raw field ---
    let raw_path = write_raw(field, params, output_dir)?;

    // --- Write kernel_params.json ---
    write_params_json(params, output_dir)?;

    // --- Assemble WirePacket ---
    let packet = assemble_packet(field, params, raw_path);

    Ok(packet)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn write_raw(
    field:      &ScalarField,
    params:     &KernelParams,
    output_dir: &Path,
) -> Result<PathBuf, OutputError> {
    let file_name = format!("{}.raw", params.scope.world_id);
    let raw_path  = output_dir.join(&file_name);

    let mut file = std::fs::File::create(&raw_path)
        .map_err(|e| OutputError::Io(e.to_string()))?;

    // Write each f32 as 4 bytes little-endian.
    // ndarray stores data in row-major order; we iterate in that order.
    for &value in field.data.iter() {
        file.write_all(&value.to_le_bytes())
            .map_err(|e| OutputError::Io(e.to_string()))?;
    }

    file.flush().map_err(|e| OutputError::Io(e.to_string()))?;

    Ok(raw_path)
}

fn write_params_json(params: &KernelParams, output_dir: &Path) -> Result<(), OutputError> {
    let json_path = output_dir.join("kernel_params.json");
    let json = serde_json::to_string_pretty(params)
        .map_err(|e| OutputError::Serialization(e.to_string()))?;
    std::fs::write(&json_path, json)
        .map_err(|e| OutputError::Io(e.to_string()))?;
    Ok(())
}

fn assemble_packet(
    field:    &ScalarField,
    params:   &KernelParams,
    raw_path: PathBuf,
) -> WirePacket {
    let res = field.resolution;
    let output = MandelbulbOutput {
        world_id:         params.scope.world_id.clone(),
        raw_field_path:   raw_path,
        field_dimensions: [res, res, res],
        iso_threshold:    field.iso_threshold,
        kernel_params:    Box::new(params.clone()),
        atom_manifest:    params.atom_manifest.clone(),
        material_seed:    params.material_seed,
        resonance_mod:    params.resonance_mod,
    };
    WirePacket::encode(WireType::Spatial, MythId::new(), 0, &output)
        .expect("MandelbulbOutput serialization failed")
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum OutputError {
    Io(String),
    Serialization(String),
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg)            => write!(f, "IO error: {msg}"),
            Self::Serialization(msg) => write!(f, "Serialization error: {msg}"),
        }
    }
}

impl std::error::Error for OutputError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{
        AtomManifest, FormulaSlot, FormulaType, MandelbulbParams, WorldScope,
    };
    use ndarray::Array3;

    fn minimal_field() -> ScalarField {
        ScalarField {
            data:          Array3::from_elem((4, 4, 4), 1.0_f32),
            resolution:    4,
            iso_threshold: 0.01,
        }
    }

    fn minimal_params() -> KernelParams {
        KernelParams {
            formula_chain: vec![FormulaSlot {
                formula:         FormulaType::MandelbulbIntegerPower,
                iteration_count: 8,
                weight:          1.0,
            }],
            mandelbulb: MandelbulbParams::default(),
            mandelbox:  None,
            material_seed:  42,
            resonance_mod:  0.0,
            resonance_hz:   440.0,
            scope: WorldScope {
                world_id:     "test-world".into(),
                resonance_hz: 440.0,
                resolution:   4,
            },
            atom_manifest: AtomManifest {
                void_phase:  ["A".into(), "B".into(), "C".into(), "D".into()],
                spark_phase: ["E".into(), "F".into(), "G".into(), "H".into()],
                law_phase:   ["I".into(), "J".into(), "K".into(), "L".into()],
                bloom_phase: ["M".into(), "N".into(), "O".into(), "P".into()],
            },
        }
    }

    #[test]
    fn writes_raw_and_json() {
        let tmp    = tempfile::tempdir().unwrap();
        let field  = minimal_field();
        let params = minimal_params();

        write_and_emit(&field, &params, tmp.path()).unwrap();

        assert!(tmp.path().join("test-world.raw").exists(), ".raw not created");
        assert!(tmp.path().join("kernel_params.json").exists(), "kernel_params.json not created");
    }

    #[test]
    fn raw_file_correct_size() {
        let tmp    = tempfile::tempdir().unwrap();
        let field  = minimal_field();
        let params = minimal_params();

        write_and_emit(&field, &params, tmp.path()).unwrap();

        let meta = std::fs::metadata(tmp.path().join("test-world.raw")).unwrap();
        // 4×4×4 voxels × 4 bytes per f32 = 256 bytes
        assert_eq!(meta.len(), 256, "unexpected .raw file size");
    }

    #[test]
    fn packet_wire_type_is_spatial() {
        let tmp    = tempfile::tempdir().unwrap();
        let field  = minimal_field();
        let params = minimal_params();

        let packet = write_and_emit(&field, &params, tmp.path()).unwrap();
        assert!(matches!(packet.wire_type, WireType::Spatial));
    }

    #[test]
    fn packet_payload_decodes_to_mandelbulb_output() {
        let tmp    = tempfile::tempdir().unwrap();
        let field  = minimal_field();
        let params = minimal_params();

        let packet  = write_and_emit(&field, &params, tmp.path()).unwrap();
        let decoded: MandelbulbOutput = packet.decode().unwrap();
        assert_eq!(decoded.world_id, "test-world");
        assert_eq!(decoded.field_dimensions, [4, 4, 4]);
    }
}
