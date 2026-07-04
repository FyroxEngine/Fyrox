# void-sculptor — Iso-Surface Extraction

Layer 2 module crate for the myth-os / BioSpark Quantum Ecosystem.

Receives the SPA WirePacket from the mandelbulb kernel, reads the `.raw`
scalar field, extracts a triangle mesh, assigns material zones, and emits a
new SPA WirePacket to the Theater.

**Zero renderer dependencies. Runs fully headless.**

---

## File Layout

```
modules/void-sculptor/
├── Cargo.toml
├── main.rs                         standalone CLI runner (~90 lines)
├── README.md
└── src/
    ├── lib.rs                      public API, pipeline runner
    ├── params.rs                   SculptorInput, SculptorConfig, ExtractionMode
    ├── field_reader.rs             read .raw → ScalarField
    ├── seed.rs                     find iso-crossing voxels (seed points)
    ├── propagating.rs              Propagating Contours — fast BFS extraction
    ├── marching_cubes.rs           256-entry table + brute force march
    ├── mesh.rs                     TriangleSoup → IndexedMesh, smooth, islands
    ├── material.rs                 material zone assignment from material_seed
    ├── output.rs                   .obj write + WirePacket assembly
    └── test_fixtures/
        └── 0x7E2-ALPHA_params.json real ceremony log — use for validation
```

`main.rs` lives next to `Cargo.toml`, not inside `src/`. Same pattern as
the mandelbulb crate — binary is wiring, library is logic.

---

## Prerequisites

- `myth-wire` at `../../crates/myth-wire/`
- `mandelbulb` at `../mandelbulb/` (imports `KernelParams` and `AtomManifest`)
- Both crates must be members of the workspace

### Add to workspace

In `myth-os/Cargo.toml`:

```toml
[workspace]
members = [
    "crates/myth-wire",
    "modules/mandelbulb",
    "modules/void-sculptor",   # ← add this
]
```

---

## Building

```bash
# From workspace root
cargo build -p void-sculptor

# Verify zero renderer dependencies (must print nothing)
cargo tree -p void-sculptor | grep bevy
cargo tree -p void-sculptor | grep egui

# Run all tests
cargo test -p void-sculptor
```

---

## Running Standalone

### Option A — All-in-one input JSON

```bash
cargo run -p void-sculptor -- \
  --input  src/test_fixtures/0x7E2-ALPHA_params.json \
  --output ./output/0x7E2-ALPHA/
```

The input JSON is a serialized `SculptorInput`. The test fixture
`0x7E2-ALPHA_params.json` is a real Genesis Ceremony log from May 29 2026
and works immediately once the corresponding `.raw` field exists.

### Option B — raw field + kernel_params separately

```bash
cargo run -p void-sculptor -- \
  --raw    ./output/0x7E2-ALPHA/0x7E2-ALPHA.raw \
  --params ./output/0x7E2-ALPHA/kernel_params.json \
  --output ./output/0x7E2-ALPHA/
```

The `kernel_params.json` is written by `mandelbulb` alongside the `.raw` file.
This is the most natural way to chain the two modules.

### Full two-module pipeline

```bash
# Step 1 — fire the Genesis Collider
cargo run -p mandelbulb -- \
  --seed   world.json \
  --atoms  atoms.json \
  --output ./output/my_world/

# Step 2 — sculpt the mesh
cargo run -p void-sculptor -- \
  --raw    ./output/my_world/my_world.raw \
  --params ./output/my_world/kernel_params.json \
  --output ./output/my_world/
```

---

## Output Files

```
output/my_world/
├── my_world.raw                   (from mandelbulb)
├── kernel_params.json             (from mandelbulb)
├── my_world.obj                   triangle mesh — open in Blender
├── my_world_materials.json        per-vertex zone IDs
└── sculptor_report.json           vertex/triangle counts, config snapshot
```

### Previewing the .obj in Python

```python
# Quick vertex count check
with open("my_world.obj") as f:
    lines = f.readlines()
verts = [l for l in lines if l.startswith("v ")]
faces = [l for l in lines if l.startswith("f ")]
print(f"{len(verts)} vertices, {len(faces)} triangles")
```

### Opening in Blender

File → Import → Wavefront (.obj) → select the `.obj` file.
The mesh imports with normals. Material zones are in `_materials.json`
and can be applied manually via a Python script in Blender's scripting tab.

---

## Extraction Algorithms

### Propagating Contours (default)

Seed-based BFS extraction. Finds voxels that cross the iso-threshold,
then propagates outward through connected crossing voxels. Only evaluates
voxels that contain the surface — typically 10–15% of the volume.

Speed advantage over brute-force: **30–50×** (matches Acropora benchmark).

### Marching Cubes (fallback)

Evaluates every voxel in the field. Guaranteed to find the surface
regardless of seed point quality. Used automatically when Propagating
Contours finds no valid seed points (`auto_fallback: true`).

### Configuration

```json
{
  "config": {
    "mode":               "PropagatingContours",
    "auto_fallback":      true,
    "min_triangle_count": 8,
    "smooth_passes":      1,
    "min_island_size":    4
  }
}
```

`smooth_passes` — Laplacian smoothing iterations. 0 = none, 1 = mild, 3+ = aggressive.
`min_island_size` — remove disconnected triangle components smaller than this. 0 = disabled.

---

## The 0x7E2-ALPHA Test World

The file `src/test_fixtures/0x7E2-ALPHA_params.json` is a real Genesis
Ceremony log. World parameters:

| Parameter | Value |
|---|---|
| Power | 9.6 |
| Max iterations | 18 |
| Bailout | 55.0 |
| Julia offset | (0.3, 0.6, 0.6) |
| Formula chain | Mandelbulb + Julia + Mandelbox + PseudoKleinian |
| Material seed | 77239104 |
| Resonance | 432 Hz |

To use it: first run the mandelbulb kernel with the matching ATOMs to
generate the `.raw` file, then run the sculptor with this fixture as input.

---

## Architecture Law

After any change:

```bash
cargo tree -p void-sculptor | grep bevy   # must be empty
cargo tree -p void-sculptor | grep egui   # must be empty
cargo tree -p void-sculptor | grep wgpu   # must be empty
```

This crate is Layer 2. No renderer. No window. No GPU. See `myth-os-architecture.md`.

---

## Engine Integration

When running inside the full engine, import the library directly:

```rust
use void_sculptor::{sculpt, params::SculptorInput};

// input assembled from the incoming SPA WirePacket
let packet = sculpt(&input, &output_path)?;
theater.emit(packet);
```

The `sculpt()` function runs the full pipeline in one call and returns
the outgoing WirePacket ready for the Theater.
