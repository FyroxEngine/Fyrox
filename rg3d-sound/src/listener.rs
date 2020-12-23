//! Listener module.
//!
//! # Overview
//!
//! Engine has only one listener which can be positioned and oriented in space. Listener defined as coordinate
//! system which is used to compute spatial properties of sound sources.

use rg3d_core::algebra::{Matrix3, Vector3};
use rg3d_core::math::Matrix3Ext;
use rg3d_core::visitor::{Visit, VisitResult, Visitor};

/// See module docs.
#[derive(Debug, Clone)]
pub struct Listener {
    basis: Matrix3<f32>,
    position: Vector3<f32>,
}

impl Default for Listener {
    fn default() -> Self {
        Self::new()
    }
}

impl Listener {
    pub(in crate) fn new() -> Self {
        Self {
            basis: Matrix3::identity(),
            position: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    /// Sets new basis from given vectors in left-handed coordinate system.
    /// See `set_basis` for more info.
    pub fn set_orientation_lh(&mut self, look: Vector3<f32>, up: Vector3<f32>) {
        self.basis = Matrix3::from_columns(&[look.cross(&up), up, look])
    }

    /// Sets new basis from given vectors in right-handed coordinate system.
    /// See `set_basis` for more info.
    pub fn set_orientation_rh(&mut self, look: Vector3<f32>, up: Vector3<f32>) {
        self.basis = Matrix3::from_columns(&[up.cross(&look), up, look])
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
    /// use rg3d_sound::math::mat3::Matrix3;
    /// use rg3d_sound::math::vec3::Vector3;
    /// use rg3d_sound::math::quat::UnitQuaternion;
    ///
    /// fn orient_listener(listener: &mut Listener) {
    ///     let basis = Matrix3::from_quat(UnitQuaternion::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), 45.0f32.to_radians()));
    ///     listener.set_basis(basis);
    /// }
    /// ```
    pub fn set_basis(&mut self, matrix: Matrix3<f32>) {
        self.basis = matrix;
    }

    /// Returns shared reference to current basis.
    pub fn basis(&self) -> &Matrix3<f32> {
        &self.basis
    }

    /// Sets current position in world space.
    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }

    /// Returns position of listener.
    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    /// Returns up axis from basis.
    pub fn up_axis(&self) -> Vector3<f32> {
        self.basis.up()
    }

    /// Returns look axis from basis.
    pub fn look_axis(&self) -> Vector3<f32> {
        self.basis.look()
    }

    /// Returns ear axis from basis.
    pub fn ear_axis(&self) -> Vector3<f32> {
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
