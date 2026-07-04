# mandelbulb — Genesis Collider

Layer 2 module crate for the myth-os / BioSpark Quantum Ecosystem.

Reads 16 ATOMs across 4 phases, assembles `KernelParams`, and runs a
fractal distance estimator over a 3D voxel grid. Writes a `.raw` scalar
field to disk and emits a `WirePacket` to the Void Sculptor.

**Zero renderer dependencies. Runs fully headless.**

---

## File Layout

```
modules/mandelbulb/
├── Cargo.toml        crate manifest — deps, binary declaration
├── main.rs           standalone CLI runner (~90 lines of wiring)
└── src/
    ├── lib.rs        public API, re-exports
    ├── params.rs     all data types (KernelParams, FormulaSlot, etc.)
    ├── collider.rs   ATOM slot loading → KernelParams assembly
    ├── mandelbulb.rs Mandelbulb DE formula
    ├── mandelbox.rs  Mandelbox box fold + sphere fold formula
    ├── hybrid.rs     multi-formula slot chain dispatch
    ├── distance.rs   top-level DE dispatch (used by field.rs)
    ├── field.rs      3D scalar field — rayon parallel voxel loop
    └── output.rs     .raw file write + WirePacket assembly
```

`main.rs` lives next to `Cargo.toml`, not inside `src/`. This is
intentional — the `Cargo.toml` declares `path = "main.rs"` explicitly.
The `src/` folder is the library. The binary is just wiring.

---

## Prerequisites

- Rust toolchain 1.75+ (`rustup update stable`)
- The workspace root `myth-os/Cargo.toml` must include this crate
- `myth-wire` must be present at `../../crates/myth-wire/`

### Add to workspace

In `myth-os/Cargo.toml`:

```toml
[workspace]
members = [
    "crates/myth-wire",
    "crates/mythos",
    # ... other crates
    "modules/mandelbulb",   # ← add this line
]
```

---

## Building

From the workspace root:

```bash
# Build only this crate
cargo build -p mandelbulb

# Verify zero renderer dependencies (must print nothing)
cargo tree -p mandelbulb | grep bevy

# Run all tests
cargo test -p mandelbulb
```

From inside the crate directory (`modules/mandelbulb/`):

```bash
cargo build
cargo test
```

---

## Running Standalone

The binary requires two input files and an output directory.

```bash
cargo run -p mandelbulb -- \
  --seed   path/to/world.json \
  --atoms  path/to/atom_manifest.json \
  --output ./output/my_world/
```

### `--seed` format (WorldScope JSON)

```json
{
  "world_id":     "xyrona-prime",
  "resonance_hz": 440.0,
  "resolution":   64
}
```

`resolution` is voxels per axis. The field is always a cube.
Start with 32 or 64 for fast iteration. 128+ for production quality.

### `--atoms` format (AtomManifest JSON)

```json
{
  "void_phase":  ["SYS_04", "NAR_02", "SYS_07", "UI_01"],
  "spark_phase": ["NAV_04", "SEC_02", "NAV_01", "NAR_06"],
  "law_phase":   ["COG_01", "COG_06", "NAR_03", "SEC_01"],
  "bloom_phase": ["MED_001", "NAR_07", "COG_04", "UI_05"]
}
```

Each phase must have exactly 4 ATOM IDs. The IDs are matched by their
prefix to determine which parameter they influence:

| Prefix | Influences |
|--------|-----------|
| `SYS_` | `power` (1.0–16.0) — organic vs. spiked geometry |
| `NAV_` | `max_iterations` — surface detail depth |
| `SEC_` | `bailout` — surface density |
| `COG_` | `julia_offset` x/y/z — attractor shape shift |
| `NAR_` | `formula_slot` selection |
| `MED_` | `resonance_mod` — iso-threshold offset |
| `UI_`  | `material_seed` — passed downstream to Void Sculptor |

### Output

```
output/my_world/
├── xyrona-prime.raw       32-bit float scalar field, little-endian
└── kernel_params.json     assembled params — keep for reproducibility
```

The `.raw` file is a flat binary array of `f32` values in `[z][y][x]`
order. Read it back in Python with:

```python
import numpy as np
res = 64
field = np.fromfile("xyrona-prime.raw", dtype=np.float32).reshape(res, res, res)
```

---

## ATOM Phase Rules

Each phase has a primary affinity. ATOMs loaded outside their natural
phase create cross-phase interference — unusual geometry combinations.
This is intentional design space, not a bug.

| Phase | Primary Affinity | Slots |
|-------|-----------------|-------|
| VOID  | `power`, formula slots 0–1 | 1–4 |
| SPARK | `max_iterations`, `bailout`, formula slot 2 | 5–8 |
| LAW   | `julia_offset` x/y/z, formula slot 3 | 9–12 |
| BLOOM | Mandelbox params, `material_seed`, `resonance_mod`, formula slots 4–5 | 13–16 |

---

## Determinism

Same `atom_manifest.json` + same `--seed` file = identical `.raw` output
every time. The kernel has no randomness — all variation comes from ATOM
selection and the genesis seed's `resonance_hz`.

To reproduce a previous world exactly, keep `kernel_params.json`. It
records the full assembled parameters alongside the original manifest.

---

## Architecture Law

This crate must never import a renderer. After any change:

```bash
cargo tree -p mandelbulb | grep bevy   # must be empty
cargo tree -p mandelbulb | grep egui   # must be empty
cargo tree -p mandelbulb | grep wgpu   # must be empty
```

If any renderer appears in the tree — revert immediately. Logic goes in
the library. Renderers go in `adapters/`. See `myth-os-architecture.md`.

---

## Connecting to the Engine

When running inside the full myth-os engine (not standalone), the binary
is not used. Instead, the library crate is imported directly:

```rust
use mandelbulb::{assemble_params, generate, write_and_emit, AtomStatusMap};

let params = assemble_params(&manifest, scope, &status_map);
let field  = generate(&params);
let packet = write_and_emit(&field, &params, &output_path)?;

// Send packet to the Void Sculptor via Theater
theater.emit(packet);
```

ATOM statuses come from the ATOM registry in `mythos`. The standalone
runner defaults all statuses to `Stable` (multiplier 1.0).
