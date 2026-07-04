/// mandelbulb — Genesis Collider fractal world mesh generator.
///
/// Layer 2 module crate. Pure math. Zero renderer deps.
/// Runs headless. Communicates via WirePackets only.
///
/// # Usage
///
/// ```rust
/// use mandelbulb::{
///     collider::{assemble_params, AtomStatusMap},
///     field::generate,
///     output::write_and_emit,
///     params::{AtomManifest, WorldScope},
/// };
/// use std::path::Path;
///
/// let manifest = AtomManifest { /* ... */ };
/// let scope    = WorldScope { /* ... */ };
/// let statuses = AtomStatusMap::default();
///
/// let params = assemble_params(&manifest, scope, &statuses);
/// let field  = generate(&params);
/// let packet = write_and_emit(&field, &params, Path::new("./output/")).unwrap();
/// ```
///
/// # Compile-time invariants
///
/// After every change, verify:
/// ```bash
/// cargo build -p mandelbulb
/// cargo tree -p mandelbulb | grep bevy   # must be empty
/// cargo test -p mandelbulb
/// ```

// Module declarations
pub mod collider;
pub mod distance;
pub mod field;
pub mod hybrid;
pub mod mandelbulb;
pub mod mandelbox;
pub mod output;
pub mod params;

// Convenience re-exports — the surface API callers actually need.
pub use collider::{assemble_params, AtomStatusMap};
pub use distance::{distance_estimate, iso_threshold};
pub use field::{generate, ScalarField};
pub use output::{write_and_emit, OutputError};
pub use params::{
    AtomDepartment, AtomManifest, AtomStatus, FormulaSlot, FormulaType, KernelParams,
    MandelboxParams, MandelbulbParams, WorldScope,
};
