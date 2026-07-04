/// mandelbulb.rs — Mandelbulb distance estimator.
///
/// Pure math. Takes a position and params, returns a distance estimate.
/// No side effects. No state. Callable from rayon parallel iterators.

use crate::params::MandelbulbParams;
use glam::Vec3;

/// Mandelbulb distance estimator (DE).
///
/// Returns an approximate distance from `pos` to the Mandelbulb surface.
/// Positive = outside the set. Negative = inside (rare with standard params).
///
/// `julia_offset` is Vec3::ZERO for a standard Mandelbulb.
/// Non-zero shifts into Julia-set territory.
#[inline]
pub fn mandelbulb_de(pos: Vec3, params: &MandelbulbParams) -> f32 {
    let mut z  = pos;
    let mut dr = 1.0_f32;
    let mut r  = 0.0_f32;

    for _ in 0..params.max_iterations {
        r = z.length();
        if r > params.bailout {
            break;
        }

        // Convert to polar coordinates
        let theta = (z.z / r).clamp(-1.0, 1.0).acos();
        let phi   = z.y.atan2(z.x);

        // Scale the derivative
        dr = r.powf(params.power - 1.0) * params.power * dr + 1.0;

        // Scale and rotate the point
        let zr    = r.powf(params.power);
        let theta = theta * params.power;
        let phi   = phi   * params.power;

        let (sin_theta, cos_theta) = theta.sin_cos();
        let (sin_phi,   cos_phi)   = phi.sin_cos();

        z = zr * Vec3::new(
            sin_theta * cos_phi,
            sin_phi   * sin_theta,
            cos_theta,
        );

        // julia_offset = Vec3::ZERO for standard Mandelbulb.
        // Non-zero produces a Julia-shifted variant.
        z += params.julia_offset;
    }

    // Distance estimate. Guard against dr = 0.
    if dr.abs() < f32::EPSILON {
        return 0.0;
    }

    0.5 * r.ln() * r / dr
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params() -> MandelbulbParams {
        MandelbulbParams {
            power:          8.0,
            max_iterations: 8,
            bailout:        2.0,
            julia_offset:   Vec3::ZERO,
        }
    }

    #[test]
    fn origin_is_inside_or_zero() {
        // The origin is inside the Mandelbulb for power=8; DE ≤ 0.
        let de = mandelbulb_de(Vec3::ZERO, &default_params());
        assert!(de <= 0.0, "origin should be inside or on the surface, got {de}");
    }

    #[test]
    fn far_point_is_outside() {
        let de = mandelbulb_de(Vec3::new(10.0, 10.0, 10.0), &default_params());
        assert!(de > 0.0, "far point should be outside the set, got {de}");
    }

    #[test]
    fn does_not_panic_on_zero_radius() {
        // Exercises the dr guard
        let params = MandelbulbParams {
            power:          1.0,
            max_iterations: 1,
            bailout:        100.0,
            julia_offset:   Vec3::ZERO,
        };
        let _ = mandelbulb_de(Vec3::ZERO, &params);
    }

    #[test]
    fn julia_offset_changes_result() {
        let base = default_params();
        let julia = MandelbulbParams {
            julia_offset: Vec3::new(0.5, 0.3, 0.1),
            ..default_params()
        };
        let pos = Vec3::new(0.5, 0.5, 0.5);
        let de_base  = mandelbulb_de(pos, &base);
        let de_julia = mandelbulb_de(pos, &julia);
        assert_ne!(de_base, de_julia, "julia offset should change the DE result");
    }

    #[test]
    fn higher_power_changes_result() {
        let p8  = default_params();
        let p2  = MandelbulbParams { power: 2.0, ..default_params() };
        let pos = Vec3::new(0.5, 0.3, 0.2);
        assert_ne!(
            mandelbulb_de(pos, &p8),
            mandelbulb_de(pos, &p2),
            "different power values should produce different DE results"
        );
    }
}
