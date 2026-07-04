/// void-sculptor — Iso-surface extraction and mesh generation.
///
/// Layer 2 module crate. Zero renderer deps. Runs fully headless.
/// Receives a SPA WirePacket from the mandelbulb kernel.
/// Extracts a triangle mesh via Propagating Contours or Marching Cubes.
/// Emits a SPA WirePacket with .obj mesh path to the Theater.
///
/// # Extraction pipeline
///
/// ```text
/// SPA WirePacket (from mandelbulb)
///       │
///       ▼
/// field_reader::read_field()       — load .raw → ScalarField
///       │
///       ▼
/// seed::find_seeds_auto()          — find iso-crossing voxels
///       │
///       ├─ seeds found ──▶ propagating::propagate()   — fast BFS extraction
///       │
///       └─ no seeds ─────▶ marching_cubes::march()    — brute force fallback
///                                │
///                                ▼
///                         mesh::build_mesh()           — dedup, index buffer
///                                │
///                                ▼
///                         mesh::smooth()               — Laplacian smoothing
///                                │
///                                ▼
///                         mesh::remove_islands()       — cleanup small islands
///                                │
///                                ▼
///                         material::assign_zones()     — zone IDs from seed
///                                │
///                                ▼
///                         output::write_and_emit()     — .obj + WirePacket
/// ```
///
/// # Compile-time invariants
///
/// After every change:
/// ```bash
/// cargo build -p void-sculptor
/// cargo tree  -p void-sculptor | grep bevy   # must be empty
/// cargo test  -p void-sculptor
/// ```

pub mod field_reader;
pub mod marching_cubes;
pub mod material;
pub mod mesh;
pub mod output;
pub mod params;
pub mod propagating;
pub mod seed;

// Convenience re-exports
pub use field_reader::{field_from_array, read_field, ScalarField};
pub use marching_cubes::march;
pub use material::{assign_zones, assign_zones_from_seed, MaterialMap};
pub use mesh::{build_mesh, remove_islands, smooth, IndexedMesh, Triangle, TriangleSoup};
pub use output::{write_and_emit, OutputError, SculptorReport};
pub use params::{ExtractionMode, SculptorConfig, SculptorInput};
pub use propagating::propagate;
pub use seed::{find_seeds, find_seeds_auto, SeedSet};

// ---------------------------------------------------------------------------
// Top-level pipeline runner
// ---------------------------------------------------------------------------

/// Run the full Void Sculptor pipeline from a SculptorInput.
///
/// This is the single call site for the engine integration.
/// Returns the assembled WirePacket on success.
pub fn sculpt(
    input:      &SculptorInput,
    output_dir: &std::path::Path,
) -> Result<myth_wire::WirePacket, output::OutputError> {
    use params::ExtractionMode;

    // 1. Load the scalar field
    let field = read_field(
        &input.raw_field_path,
        input.field_dimensions,
        input.iso_threshold,
    ).map_err(|e| output::OutputError::Io(e.to_string()))?;

    // 2. Extract triangle soup
    let soup = match input.config.mode {
        ExtractionMode::PropagatingContours => {
            let seeds = find_seeds_auto(&field);

            if seeds.is_empty() && input.config.auto_fallback {
                // No seeds found — fall back to brute force
                march(&field)
            } else {
                let s = propagate(&field, &seeds);
                // Check min triangle count threshold
                if s.len() < input.config.min_triangle_count && input.config.auto_fallback {
                    march(&field)
                } else {
                    s
                }
            }
        }
        ExtractionMode::MarchingCubes => march(&field),
    };

    // 3. Build indexed mesh
    let mut mesh = build_mesh(&soup);

    // 4. Smooth
    if input.config.smooth_passes > 0 {
        smooth(&mut mesh, input.config.smooth_passes, 0.5);
    }

    // 5. Remove small islands
    if input.config.min_island_size > 0 {
        mesh = remove_islands(&mesh, input.config.min_island_size);
    }

    // 6. Assign material zones
    assign_zones_from_seed(&mut mesh, input.material_seed);

    // 7. Write output and emit WirePacket
    std::fs::create_dir_all(output_dir)
        .map_err(|e| output::OutputError::Io(e.to_string()))?;

    write_and_emit(&mesh, input, output_dir)
}
