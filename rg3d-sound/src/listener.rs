//! Listener module.
//!
//! # Overview
//!
//! Engine has only one listener which can be positioned and oriented in space. Listener defined as coordinate
//! system which is used to compute spatial properties of sound sources.

use crate::math::mat3::Mat3;
use crate::math::vec3::Vec3;
use rg3d_core::visitor::{Visit, VisitResult, Visitor};

/// See module docs.
pub struct Listener {
    basis: Mat3,
    position: Vec3,
}

impl Listener {
    pub(in crate) fn new() -> Self {
        Self {
            basis: Default::default(),
            position: Default::default(),
        }
    }

    /// Sets new basis from given vectors in left-handed coordinate system.
    /// See `set_basis` for more info.
    pub fn set_orientation_lh(&mut self, look: Vec3, up: Vec3) {
        self.basis = Mat3::from_vectors(look.cross(&up), up, look)
    }

    /// Sets new basis from given vectors in right-handed coordinate system.
    /// See `set_basis` for more info.
    pub fn set_orientation_rh(&mut self, look: Vec3, up: Vec3) {
        self.basis = Mat3::from_vectors(up.cross(&look), up, look)
    }

    /// Sets arbitrary basis. Basis defines orientation of the listener in space.
    /// In your application you can take basis of camera in world coordinates and
    /// pass it to this method. If you using HRTF, make sure your basis is in
    /// right-handed coordinate system! You can make fake right-handed basis from
    /// left handed, by inverting Z axis. It is fake because it will work only for
    /// positions (engine interested in positions only), but not for rotation, shear
    /// etc.
    ///
    /// # Notes
    ///
    /// Basis must have mutually perpendicular axes.
    ///
    /// ```
    /// use rg3d_sound::listener::Listener;
    /// use rg3d_sound::math::mat3::Mat3;
    /// use rg3d_sound::math::vec3::Vec3;
    /// use rg3d_sound::math::quat::Quat;
    ///
    /// fn orient_listener(listener: &mut Listener) {
    ///     let basis = Mat3::from_quat(Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 45.0f32.to_radians()));
    ///     listener.set_basis(basis);
    /// }
    /// ```
    pub fn set_basis(&mut self, matrix: Mat3) {
        self.basis = matrix;
    }

    /// Returns shared reference to current basis.
    pub fn basis(&self) -> &Mat3 {
        &self.basis
    }

    /// Sets current position in world space.
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    /// Returns position of listener.
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Returns up axis from basis.
    pub fn up_axis(&self) -> Vec3 {
        self.basis.up()
    }

    /// Returns look axis from basis.
    pub fn look_axis(&self) -> Vec3 {
        self.basis.look()
    }

    /// Returns ear axis from basis.
    pub fn ear_axis(&self) -> Vec3 {
        self.basis.side()
    }
}

impl Visit for Listener {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.basis.visit("Basis", visitor)?;
        self.position.visit("Position", visitor)?;

        visitor.leave_region()
    }
}
