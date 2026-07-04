/// void-sculptor standalone runner.
///
/// Fires the full sculptor pipeline from the command line.
/// Reads a .raw scalar field + SculptorInput JSON and writes a .obj mesh.
///
/// Usage:
///   cargo run -p void-sculptor -- \
///     --input  path/to/sculptor_input.json \
///     --output ./output/world_name/
///
/// Or using individual flags (bypasses the input JSON):
///   cargo run -p void-sculptor -- \
///     --raw    path/to/world.raw \
///     --params path/to/kernel_params.json \
///     --output ./output/world_name/
///
/// Produces:
///   <world_id>.obj                  triangle mesh
///   <world_id>_materials.json       material zone assignments
///   sculptor_report.json            stats and config snapshot

use void_sculptor::{params::SculptorInput, sculpt};
use std::path::PathBuf;

fn main() {
    let args = parse_args();

    // --- Load SculptorInput ---
    let input = load_input(&args);

    println!("[void-sculptor] world_id       = {}", input.world_id);
    println!("[void-sculptor] resolution     = {}³", input.field_dimensions[0]);
    println!("[void-sculptor] iso_threshold  = {:.6}", input.iso_threshold);
    println!("[void-sculptor] mode           = {:?}", input.config.mode);
    println!("[void-sculptor] material_seed  = {}", input.material_seed);

    // --- Create output directory ---
    std::fs::create_dir_all(&args.output)
        .unwrap_or_else(|e| fatal(&format!("Cannot create output dir '{}': {e}", args.output.display())));

    // --- Run the full pipeline ---
    println!("[void-sculptor] running pipeline …");
    let packet = sculpt(&input, &args.output)
        .unwrap_or_else(|e| fatal(&format!("Sculptor error: {e}")));

    // --- Report ---
    let report_path = args.output.join("sculptor_report.json");
    if report_path.exists() {
        let report = std::fs::read_to_string(&report_path).unwrap_or_default();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&report) {
            println!("[void-sculptor] vertices       = {}", v["vertex_count"]);
            println!("[void-sculptor] triangles      = {}", v["triangle_count"]);
            println!("[void-sculptor] material_zones = {}", v["material_zones"]);
        }
    }

    println!("[void-sculptor] wrote {}.obj", input.world_id);
    println!("[void-sculptor] WirePacket emitted  wire_type = {:?}", packet.wire_type);
    println!("[void-sculptor] done.");
}

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

struct Args {
    /// Path to a SculptorInput JSON (all-in-one input)
    input:  Option<PathBuf>,
    /// Path to a .raw scalar field (used with --params)
    raw:    Option<PathBuf>,
    /// Path to a kernel_params.json (used with --raw)
    params: Option<PathBuf>,
    /// Output directory
    output: PathBuf,
}

fn parse_args() -> Args {
    let mut input:  Option<PathBuf> = None;
    let mut raw:    Option<PathBuf> = None;
    let mut params: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;

    let raw_args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < raw_args.len() {
        match raw_args[i].as_str() {
            "--input"  => { i += 1; input  = Some(PathBuf::from(&raw_args[i])); }
            "--raw"    => { i += 1; raw    = Some(PathBuf::from(&raw_args[i])); }
            "--params" => { i += 1; params = Some(PathBuf::from(&raw_args[i])); }
            "--output" => { i += 1; output = Some(PathBuf::from(&raw_args[i])); }
            other      => fatal(&format!("Unknown argument: {other}")),
        }
        i += 1;
    }

    Args {
        input,
        raw,
        params,
        output: output.unwrap_or_else(|| fatal("--output is required")),
    }
}

// ---------------------------------------------------------------------------
// Input loader
// ---------------------------------------------------------------------------

fn load_input(args: &Args) -> SculptorInput {
    if let Some(ref input_path) = args.input {
        // All-in-one JSON
        let json = std::fs::read_to_string(input_path)
            .unwrap_or_else(|e| fatal(&format!("Cannot read input '{}': {e}", input_path.display())));
        serde_json::from_str(&json)
            .unwrap_or_else(|e| fatal(&format!("Invalid SculptorInput JSON: {e}")))
    } else {
        // Build from --raw + --params
        let raw_path = args.raw.clone()
            .unwrap_or_else(|| fatal("Either --input or both --raw and --params are required"));
        let params_path = args.params.clone()
            .unwrap_or_else(|| fatal("Either --input or both --raw and --params are required"));

        let params_json = std::fs::read_to_string(&params_path)
            .unwrap_or_else(|e| fatal(&format!("Cannot read params '{}': {e}", params_path.display())));
        let kernel_params: mandelbulb::params::KernelParams = serde_json::from_str(&params_json)
            .unwrap_or_else(|e| fatal(&format!("Invalid kernel_params JSON: {e}")));

        let iso = void_sculptor::field_reader::read_field(
            &raw_path,
            [
                kernel_params.scope.resolution,
                kernel_params.scope.resolution,
                kernel_params.scope.resolution,
            ],
            0.01, // temporary — will be re-derived from resonance
        ).map(|f| f.iso_threshold)
         .unwrap_or(0.01);

        SculptorInput::new(
            kernel_params.scope.world_id.clone(),
            raw_path,
            [
                kernel_params.scope.resolution,
                kernel_params.scope.resolution,
                kernel_params.scope.resolution,
            ],
            iso,
            kernel_params.clone(),
            kernel_params.atom_manifest.clone(),
            kernel_params.material_seed,
            kernel_params.resonance_mod,
        )
    }
}

// ---------------------------------------------------------------------------
// Fatal
// ---------------------------------------------------------------------------

fn fatal(msg: &str) -> ! {
    eprintln!("[void-sculptor] ERROR: {msg}");
    std::process::exit(1);
}
