/// mandelbox.rs — Mandelbox (Amazing Box) distance estimator.
///
/// Pure math. Box fold + sphere fold iterated.
/// Produces architectural, cave-like, interior-space geometry.
/// Active when formula = MandelboxAmazingBox.

use crate::params::MandelboxParams;
use glam::Vec3;

/// Mandelbox distance estimator (DE).
///
/// Returns an approximate distance from `pos` to the Mandelbox surface.
/// Scale controls the overall structure. Negative scale inverts it.
///
/// `fold_limit` — box fold clamp. Lower = tighter angular shapes.
/// `min_radius` / `fixed_radius` — sphere fold inner/outer boundary.
#[inline]
pub fn mandelbox_de(pos: Vec3, params: &MandelboxParams) -> f32 {
    let mut z  = pos;
    let mut dr = 1.0_f32;

    for _ in 0..params.max_iterations {
        // --- Box fold ---
        // Reflect each component at ±fold_limit
        z = box_fold(z, params.fold_limit);

        // --- Sphere fold ---
        let r2 = z.length_squared();
        let (z_folded, dr_factor) = sphere_fold(z, r2, params);
        z  = z_folded;
        dr *= dr_factor;

        // --- Scale and translate ---
        z  = params.scale * z + pos;
        dr = dr * params.scale.abs() + 1.0;
    }

    // Guard against zero
    let dr_abs = dr.abs();
    if dr_abs < f32::EPSILON {
        return 0.0;
    }

    // Surface offset: the Mandelbox "bulb" has radius (|scale| - 1)
    let bulb_radius = (params.scale.abs() - 1.0).max(0.0);
    (z.length() - bulb_radius) / dr_abs
}

/// Box fold: reflect each axis at ±limit.
#[inline]
fn box_fold(z: Vec3, limit: f32) -> Vec3 {
    // clamp(z, -limit, limit) * 2 - z  is equivalent to reflect-at-boundary
    z.clamp(Vec3::splat(-limit), Vec3::splat(limit)) * 2.0 - z
}

/// Sphere fold: scale z based on its distance from the origin.
/// Returns (folded_z, dr_multiplier).
#[inline]
fn sphere_fold(z: Vec3, r2: f32, params: &MandelboxParams) -> (Vec3, f32) {
    let min_r2   = params.min_radius   * params.min_radius;
    let fixed_r2 = params.fixed_radius * params.fixed_radius;

    if r2 < min_r2 {
        // Inside inner sphere — scale up uniformly
        let f = fixed_r2 / min_r2;
        (z * f, f)
    } else if r2 < fixed_r2 {
        // Between inner and outer sphere — scale by 1/r
        let f = fixed_r2 / r2;
        (z * f, f)
    } else {
        // Outside outer sphere — no fold
        (z, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params() -> MandelboxParams {
        MandelboxParams {
            scale:          -2.0,
            fold_limit:     1.0,
            min_radius:     0.5,
            fixed_radius:   1.0,
            max_iterations: 8,
        }
    }

    #[test]
    fn does_not_nan_at_origin() {
        let de = mandelbox_de(Vec3::ZERO, &default_params());
        assert!(!de.is_nan(), "DE at origin must not be NaN, got {de}");
    }

    #[test]
    fn far_point_is_positive() {
        let de = mandelbox_de(Vec3::new(20.0, 20.0, 20.0), &default_params());
        assert!(de > 0.0, "far point should be outside, got {de}");
    }

    #[test]
    fn scale_changes_result() {
        let p1 = default_params();
        let p2 = MandelboxParams { scale: 2.0, ..default_params() };
        let pos = Vec3::new(1.0, 0.5, 0.3);
        assert_ne!(
            mandelbox_de(pos, &p1),
            mandelbox_de(pos, &p2),
            "different scale should produce different DE"
        );
    }

    #[test]
    fn box_fold_reflects_correctly() {
        // z = 1.5, limit = 1.0 → reflected to -0.5 (clamp(1.5,−1,1)*2 − 1.5 = 2−1.5 = 0.5)
        // Wait: clamp(1.5, -1, 1) = 1.0; 1.0*2 - 1.5 = 0.5
        let z      = Vec3::new(1.5, -1.5, 0.5);
        let folded = box_fold(z, 1.0);
        assert!((folded.x - 0.5).abs()  < 1e-5, "x fold incorrect: {}", folded.x);
        assert!((folded.y - -0.5).abs() < 1e-5, "y fold incorrect: {}", folded.y);
        assert!((folded.z - 0.5).abs()  < 1e-5, "z fold incorrect: {}", folded.z);
    }

    #[test]
    fn sphere_fold_inside_min_scales_up() {
        let params = default_params(); // min_radius=0.5, fixed_radius=1.0
        let z      = Vec3::new(0.1, 0.0, 0.0); // length = 0.1 < 0.5
        let r2     = z.length_squared();
        let (zf, dr) = sphere_fold(z, r2, &params);
        // f = fixed_r2 / min_r2 = 1.0 / 0.25 = 4.0
        assert!((dr - 4.0).abs() < 1e-5, "dr should be 4.0, got {dr}");
        assert!((zf.x - 0.4).abs() < 1e-5, "zf.x should be 0.4, got {}", zf.x);
    }
}
