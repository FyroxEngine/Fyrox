/// output.rs — Write the mesh to disk and assemble the outgoing WirePacket.
///
/// The only file in this crate that touches myth-wire.
///
/// Writes:
///   <output_dir>/<world_id>.obj           — Wavefront OBJ mesh
///   <output_dir>/<world_id>_materials.json — material zone map
///   <output_dir>/sculptor_report.json     — stats and params for inspection
///
/// Emits:
///   WirePacket { source: "void-sculptor", wire_type: SPA,
///                payload: VoidSculptorOutput { … } }
///
/// The .obj format is readable by Blender, MagicaVoxel converters,
/// and any standard 3D tool — making it the right format for the
/// Python preview workflow described in the mandelbulb README.

use crate::mesh::IndexedMesh;
use crate::params::SculptorInput;
use mandelbulb::params::AtomManifest;
use myth_wire::{MythId, WirePacket, WireType};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Payload type — bincode-encoded into WirePacket.payload
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidSculptorOutput {
    pub world_id:          String,
    pub obj_path:          PathBuf,
    pub material_map_path: PathBuf,
    pub vertex_count:      u32,
    pub triangle_count:    u32,
    pub material_seed:     u64,
    pub iso_threshold:     f32,
    pub atom_manifest:     AtomManifest,
}

// ---------------------------------------------------------------------------
// Output error
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
// Sculptor report — written alongside the mesh for inspection
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct SculptorReport {
    pub world_id:          String,
    pub vertex_count:      usize,
    pub triangle_count:    usize,
    pub material_zones:    u8,
    pub smooth_passes:     u32,
    pub islands_removed:   bool,
    pub extraction_mode:   String,
    pub iso_threshold:     f32,
    pub field_resolution:  u32,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Write the mesh and report to disk, then return the assembled WirePacket.
pub fn write_and_emit(
    mesh:       &IndexedMesh,
    input:      &SculptorInput,
    output_dir: &Path,
) -> Result<WirePacket, OutputError> {
    // --- Write .obj ---
    let obj_path = write_obj(mesh, input, output_dir)?;

    // --- Write material zones ---
    let mat_path = write_material_zones(mesh, input, output_dir)?;

    // --- Write sculptor report ---
    write_report(mesh, input, output_dir)?;

    // --- Assemble WirePacket ---
    let packet = assemble_packet(mesh, input, obj_path, mat_path);

    Ok(packet)
}

// ---------------------------------------------------------------------------
// .obj writer
// ---------------------------------------------------------------------------

fn write_obj(
    mesh:       &IndexedMesh,
    input:      &SculptorInput,
    output_dir: &Path,
) -> Result<PathBuf, OutputError> {
    let file_name = format!("{}.obj", input.world_id);
    let obj_path  = output_dir.join(&file_name);

    let mut file = std::fs::File::create(&obj_path)
        .map_err(|e| OutputError::Io(e.to_string()))?;

    // Header
    writeln!(file, "# Void Sculptor output")
        .map_err(|e| OutputError::Io(e.to_string()))?;
    writeln!(file, "# world_id: {}", input.world_id)
        .map_err(|e| OutputError::Io(e.to_string()))?;
    writeln!(file, "# vertices: {}", mesh.vertex_count())
        .map_err(|e| OutputError::Io(e.to_string()))?;
    writeln!(file, "# triangles: {}", mesh.triangle_count())
        .map_err(|e| OutputError::Io(e.to_string()))?;
    writeln!(file, "o {}", input.world_id)
        .map_err(|e| OutputError::Io(e.to_string()))?;

    // Vertices
    for v in &mesh.vertices {
        writeln!(file, "v {:.6} {:.6} {:.6}", v.x, v.y, v.z)
            .map_err(|e| OutputError::Io(e.to_string()))?;
    }

    // Normals
    for n in &mesh.normals {
        writeln!(file, "vn {:.6} {:.6} {:.6}", n.x, n.y, n.z)
            .map_err(|e| OutputError::Io(e.to_string()))?;
    }

    // Faces — OBJ uses 1-based indices; format: f v//vn v//vn v//vn
    for tri_base in (0..mesh.indices.len()).step_by(3) {
        let a = mesh.indices[tri_base]     + 1;
        let b = mesh.indices[tri_base + 1] + 1;
        let c = mesh.indices[tri_base + 2] + 1;
        writeln!(file, "f {0}//{0} {1}//{1} {2}//{2}", a, b, c)
            .map_err(|e| OutputError::Io(e.to_string()))?;
    }

    file.flush().map_err(|e| OutputError::Io(e.to_string()))?;
    Ok(obj_path)
}

// ---------------------------------------------------------------------------
// Material zones writer
// ---------------------------------------------------------------------------

fn write_material_zones(
    mesh:       &IndexedMesh,
    input:      &SculptorInput,
    output_dir: &Path,
) -> Result<PathBuf, OutputError> {
    let file_name = format!("{}_materials.json", input.world_id);
    let mat_path  = output_dir.join(&file_name);

    // Simple JSON: { "material_seed": N, "zones": [0, 1, 0, 2, ...] }
    let json = serde_json::json!({
        "world_id":      input.world_id,
        "material_seed": input.material_seed,
        "vertex_count":  mesh.vertex_count(),
        "zones":         mesh.material_ids,
    });

    std::fs::write(&mat_path, serde_json::to_string_pretty(&json)
        .map_err(|e| OutputError::Serialization(e.to_string()))?)
        .map_err(|e| OutputError::Io(e.to_string()))?;

    Ok(mat_path)
}

// ---------------------------------------------------------------------------
// Report writer
// ---------------------------------------------------------------------------

fn write_report(
    mesh:       &IndexedMesh,
    input:      &SculptorInput,
    output_dir: &Path,
) -> Result<(), OutputError> {
    let report = SculptorReport {
        world_id:         input.world_id.clone(),
        vertex_count:     mesh.vertex_count(),
        triangle_count:   mesh.triangle_count(),
        material_zones:   *mesh.material_ids.iter().max().unwrap_or(&0) + 1,
        smooth_passes:    input.config.smooth_passes,
        islands_removed:  input.config.min_island_size > 0,
        extraction_mode:  format!("{:?}", input.config.mode),
        iso_threshold:    input.iso_threshold,
        field_resolution: input.field_dimensions[0],
    };

    let json_path = output_dir.join("sculptor_report.json");
    std::fs::write(
        &json_path,
        serde_json::to_string_pretty(&report)
            .map_err(|e| OutputError::Serialization(e.to_string()))?,
    ).map_err(|e| OutputError::Io(e.to_string()))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// WirePacket assembly
// ---------------------------------------------------------------------------

fn assemble_packet(
    mesh:     &IndexedMesh,
    input:    &SculptorInput,
    obj_path: PathBuf,
    mat_path: PathBuf,
) -> WirePacket {
    let output = VoidSculptorOutput {
        world_id:          input.world_id.clone(),
        obj_path,
        material_map_path: mat_path,
        vertex_count:      mesh.vertex_count() as u32,
        triangle_count:    mesh.triangle_count() as u32,
        material_seed:     input.material_seed,
        iso_threshold:     input.iso_threshold,
        atom_manifest:     input.atom_manifest.clone(),
    };
    WirePacket::encode(WireType::Spatial, MythId::new(), 0, &output)
        .expect("VoidSculptorOutput serialization failed")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{Triangle, TriangleSoup, build_mesh};
    use crate::params::{ExtractionMode, SculptorConfig, SculptorInput};
    use mandelbulb::params::{
        AtomManifest, FormulaSlot, FormulaType, KernelParams,
        MandelbulbParams, WorldScope,
    };
    use glam::Vec3;
    use std::path::PathBuf;

    fn test_mesh() -> IndexedMesh {
        let mut soup = TriangleSoup::new();
        soup.push(Triangle {
            vertices: [Vec3::new(0.0,0.0,0.0), Vec3::new(1.0,0.0,0.0), Vec3::new(0.0,1.0,0.0)],
            normal: Vec3::Z,
        });
        build_mesh(&soup)
    }

    fn test_input(output_dir: &Path) -> SculptorInput {
        let manifest = AtomManifest {
            void_phase:  ["SYS_01".into(), "NAR_56".into(), "SYS_04".into(), "UI_70".into()],
            spark_phase: ["NAV_39".into(), "SEC_79".into(), "NAR_12".into(), "NAV_40".into()],
            law_phase:   ["COG_41".into(), "SEC_03".into(), "NAR_50".into(), "COG_47".into()],
            bloom_phase: ["MED_82".into(), "UI_81".into(),  "NAR_90".into(), "COG_10".into()],
        };
        let scope = WorldScope {
            world_id:     "0x7E2-ALPHA".into(),
            resonance_hz: 432.0,
            resolution:   64,
        };
        let kernel_params = KernelParams {
            formula_chain: vec![FormulaSlot {
                formula:         FormulaType::MandelbulbIntegerPower,
                iteration_count: 8,
                weight:          1.0,
            }],
            mandelbulb:     MandelbulbParams::default(),
            mandelbox:      None,
            material_seed:  77239104,
            resonance_mod:  1.2,
            resonance_hz:   432.0,
            atom_manifest:  manifest.clone(),
            scope,
        };
        SculptorInput::new(
            "0x7E2-ALPHA",
            output_dir.join("0x7E2-ALPHA.raw"),
            [64, 64, 64],
            0.01,
            kernel_params,
            manifest,
            77239104,
            1.2,
        )
    }

    #[test]
    fn writes_obj_and_report() {
        let tmp   = tempfile::tempdir().unwrap();
        let mesh  = test_mesh();
        let input = test_input(tmp.path());

        write_and_emit(&mesh, &input, tmp.path()).unwrap();

        assert!(tmp.path().join("0x7E2-ALPHA.obj").exists(), ".obj not created");
        assert!(tmp.path().join("sculptor_report.json").exists(), "report not created");
        assert!(tmp.path().join("0x7E2-ALPHA_materials.json").exists(), "materials not created");
    }

    #[test]
    fn obj_has_correct_vertex_count() {
        let tmp   = tempfile::tempdir().unwrap();
        let mesh  = test_mesh();
        let input = test_input(tmp.path());
        write_and_emit(&mesh, &input, tmp.path()).unwrap();

        let obj   = std::fs::read_to_string(tmp.path().join("0x7E2-ALPHA.obj")).unwrap();
        let verts = obj.lines().filter(|l| l.starts_with("v ")).count();
        assert_eq!(verts, mesh.vertex_count(), "OBJ vertex count mismatch");
    }

    #[test]
    fn packet_wire_type_is_spatial() {
        let tmp   = tempfile::tempdir().unwrap();
        let mesh  = test_mesh();
        let input = test_input(tmp.path());
        let pkt   = write_and_emit(&mesh, &input, tmp.path()).unwrap();
        assert!(matches!(pkt.wire_type, WireType::Spatial));
    }

    #[test]
    fn packet_payload_decodes_to_void_sculptor_output() {
        let tmp   = tempfile::tempdir().unwrap();
        let mesh  = test_mesh();
        let input = test_input(tmp.path());
        let pkt   = write_and_emit(&mesh, &input, tmp.path()).unwrap();
        let decoded: VoidSculptorOutput = pkt.decode().unwrap();
        assert_eq!(decoded.world_id, "0x7E2-ALPHA");
        assert_eq!(decoded.vertex_count, mesh.vertex_count() as u32);
    }
}
