/// mandelbulb standalone runner.
///
/// Fires the Genesis Collider from the command line.
/// Thin wiring only — all logic lives in the library crate.
///
/// Usage:
///   cargo run -p mandelbulb -- \
///     --seed    path/to/world.qgenesis \
///     --atoms   path/to/atom_manifest.json \
///     --output  ./output/world_name/
///
/// Produces:
///   <world_id>.raw          32-bit float scalar field
///   kernel_params.json      assembled params for inspection

use mandelbulb::{assemble_params, generate, write_and_emit, AtomManifest, AtomStatusMap, WorldScope};
use std::path::PathBuf;

fn main() {
    let args = parse_args();

    // --- Load atom manifest ---
    let manifest_json = std::fs::read_to_string(&args.atoms)
        .unwrap_or_else(|e| fatal(&format!("Cannot read atom manifest '{}': {e}", args.atoms.display())));
    let manifest: AtomManifest = serde_json::from_str(&manifest_json)
        .unwrap_or_else(|e| fatal(&format!("Invalid atom manifest JSON: {e}")));

    // --- Load world scope from genesis seed ---
    let scope = load_scope(&args.seed);

    println!("[mandelbulb] world_id      = {}", scope.world_id);
    println!("[mandelbulb] resolution    = {}³", scope.resolution);
    println!("[mandelbulb] resonance_hz  = {:.2} Hz", scope.resonance_hz);

    // --- Assemble KernelParams ---
    // Status map is empty here — all ATOMs default to Stable (1.0).
    // When wired into the engine, statuses come from the ATOM registry.
    let status_map = AtomStatusMap::default();
    let params     = assemble_params(&manifest, scope, &status_map);

    println!("[mandelbulb] power         = {:.2}", params.mandelbulb.power);
    println!("[mandelbulb] max_iter      = {}", params.mandelbulb.max_iterations);
    println!("[mandelbulb] bailout       = {:.2}", params.mandelbulb.bailout);
    println!("[mandelbulb] formula slots = {}", params.formula_chain.len());

    // --- Create output directory ---
    std::fs::create_dir_all(&args.output)
        .unwrap_or_else(|e| fatal(&format!("Cannot create output dir '{}': {e}", args.output.display())));

    // --- Generate scalar field ---
    println!("[mandelbulb] generating scalar field …");
    let field = generate(&params);
    let total = field.resolution.pow(3);
    println!("[mandelbulb] field size    = {}³ = {} voxels", field.resolution, total);
    println!("[mandelbulb] iso_threshold = {:.6}", field.iso_threshold);

    // --- Write output and emit WirePacket ---
    let packet = write_and_emit(&field, &params, &args.output)
        .unwrap_or_else(|e| fatal(&format!("Output error: {e}")));

    println!("[mandelbulb] wrote {}.raw", params.scope.world_id);
    println!("[mandelbulb] wrote kernel_params.json");
    println!("[mandelbulb] WirePacket emitted  wire_type = {:?}", packet.wire_type);
    println!("[mandelbulb] done.");
}

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

struct Args {
    seed:   PathBuf,
    atoms:  PathBuf,
    output: PathBuf,
}

fn parse_args() -> Args {
    let mut seed:   Option<PathBuf> = None;
    let mut atoms:  Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;

    let raw: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--seed"   => { i += 1; seed   = Some(PathBuf::from(&raw[i])); }
            "--atoms"  => { i += 1; atoms  = Some(PathBuf::from(&raw[i])); }
            "--output" => { i += 1; output = Some(PathBuf::from(&raw[i])); }
            other      => fatal(&format!("Unknown argument: {other}")),
        }
        i += 1;
    }

    Args {
        seed:   seed  .unwrap_or_else(|| fatal("--seed is required")),
        atoms:  atoms .unwrap_or_else(|| fatal("--atoms is required")),
        output: output.unwrap_or_else(|| fatal("--output is required")),
    }
}

// ---------------------------------------------------------------------------
// Seed loader — reads a .qgenesis file or a plain JSON WorldScope
// ---------------------------------------------------------------------------

fn load_scope(seed_path: &PathBuf) -> WorldScope {
    let raw = std::fs::read_to_string(seed_path)
        .unwrap_or_else(|e| fatal(&format!("Cannot read seed '{}': {e}", seed_path.display())));

    // Try deserializing as a plain WorldScope JSON first.
    // When the full qgcp crate is wired in, this branch expands to parse .qgenesis.
    serde_json::from_str::<WorldScope>(&raw)
        .unwrap_or_else(|e| fatal(&format!("Cannot parse seed as WorldScope: {e}")))
}

// ---------------------------------------------------------------------------
// Fatal error — prints message and exits 1
// ---------------------------------------------------------------------------

fn fatal(msg: &str) -> ! {
    eprintln!("[mandelbulb] ERROR: {msg}");
    std::process::exit(1);
}
