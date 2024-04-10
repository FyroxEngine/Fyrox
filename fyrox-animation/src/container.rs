//! Track data container is a flexible source of data for numeric parameters, it is built using a set
//! of parametric curves. See [`TrackDataContainer`] docs for more info.

use crate::{
    core::{
        algebra::{Vector2, Vector3, Vector4},
        math::curve::Curve,
        math::{quat_from_euler, RotationOrder},
        reflect::prelude::*,
        visitor::prelude::*,
    },
    value::TrackValue,
};

/// The kind of track output value, the animation system works only with numeric properties and the number
/// of variants is small.
#[derive(Clone, Copy, Debug, Visit, Reflect, PartialEq, Eq)]
pub enum TrackValueKind {
    /// A real number. Requires only 1 parametric curve.
    Real,

    /// A 2-dimensional vector of real values. Requires 2 parametric curves, where `X = 0` and `Y = 1`.
    Vector2,

    /// A 3-dimensional vector of real values. Requires 3 parametric curves, where `X = 0`, `Y = 1`, `Z = 2`.
    Vector3,

    /// A 4-dimensional vector of real values. Requires 4 parametric curves, where `X = 0`, `Y = 1`, `Z = 2`, `W = 3`.
    Vector4,

    /// A quaternion that represents some rotation. Requires 3 parametric curves, where `XAngle = 0`, `YAngle = 1`,
    /// `ZAngle = 2`. The order of rotations is `XYZ`. This triple of curves forms Euler angles which are interpolated
    /// and then converted to a quaternion.
    UnitQuaternion,
}

impl TrackValueKind {
    /// Returns count of elementary components of a value kind. For example: Vector3 consists of 3 components where
    /// each component has its own parametric curve.
    pub fn components_count(self) -> usize {
        match self {
            TrackValueKind::Real => 1,
            TrackValueKind::Vector2 => 2,
            TrackValueKind::Vector3 => 3,
            TrackValueKind::Vector4 => 4,
            TrackValueKind::UnitQuaternion => {
                // Euler angles
                3
            }
        }
    }
}

impl Default for TrackValueKind {
    fn default() -> Self {
        Self::Vector3
    }
}

/// Interpolation mode for track data.
#[derive(Visit, Reflect, Debug, Clone, Default, PartialEq)]
pub enum InterpolationMode {
    /// Default interpolation mode.
    #[default]
    Default,
    /// This mode forces the engine to use short-path angle interpolation.
    ShortPath,
}

/// Container for a track data. Strictly speaking, it is just a set of parametric curves which can be
/// fetched at a given time position simultaneously, producing a value of desired type. Which type of
/// value is produced is defined by [`TrackValueKind`] enumeration. Usually a container contains up to
/// 4 curves (the largest type supported is [`Vector4`]).
///
/// Each component is bound to a specific curve. For example, in case of [`Vector3`] its components bound
/// to the following curve indices: `X = 0`, `Y = 1`, `Z = 2`. This order cannot be changed.
#[derive(Visit, Reflect, Debug, Clone, Default, PartialEq)]
pub struct TrackDataContainer {
    curves: Vec<Curve>,
    kind: TrackValueKind,
    /// Interpolation mode.
    #[visit(optional)] // Backward compatibility.
    pub mode: InterpolationMode,
}

impl TrackDataContainer {
    /// Creates a new container, that is able to produce values defined by [`TrackValueKind`] input parameter.
    /// The number of curves in the output container is defined by [`TrackValueKind::components_count`], for
    /// example [`Vector3`] has 3 components (X, Y, Z). An empty container can be created using [`Self::default`]
    /// method.
    pub fn new(kind: TrackValueKind) -> Self {
        Self {
            kind,
            // Do not use `vec![Default::default(); kind.components_count()]` here because
            // it clones a curve that was created in first macro argument which leads to
            // non-unique ids of the curves.
            curves: (0..kind.components_count())
                .map(|_| Curve::default())
                .collect(),
            mode: Default::default(),
        }
    }

    /// Adds a new curve to the container. Keep in mind, that the actual useful amount of curves has soft limit
    /// of four due to [`TrackValueKind`], any excessive curves will be ignored.
    pub fn add_curve(&mut self, curve: Curve) {
        self.curves.push(curve)
    }

    /// Tries to borrow a curve at a given index.
    pub fn curve(&self, index: usize) -> Option<&Curve> {
        self.curves.get(index)
    }

    /// Tries to borrow a curve at a given index.
    pub fn curve_mut(&mut self, index: usize) -> Option<&mut Curve> {
        self.curves.get_mut(index)
    }

    /// Returns a reference to curves container.
    pub fn curves_ref(&self) -> &[Curve] {
        &self.curves
    }

    /// Tries to borrow a curve at a given index.
    pub fn curves_mut(&mut self) -> &mut [Curve] {
        &mut self.curves
    }

    /// Sets new kind of output value. Keep in mind, that the curves will remain unchanged, if you need
    /// to re-use the container you might need to re-create/re-fill the curves too.
    pub fn set_value_kind(&mut self, kind: TrackValueKind) {
        self.kind = kind;
    }

    /// Returns the kind of output value produced by the container.
    pub fn value_kind(&self) -> TrackValueKind {
        self.kind
    }

    /// Tries to get a value at a given time. The method could fail if the internal set of curves is malformed
    /// and cannot produce a desired value (for example, [`Vector3`] can be fetched only if the amount of curves
    /// is 3).
    pub fn fetch(&self, time: f32) -> Option<TrackValue> {
        match self.kind {
            TrackValueKind::Real => Some(TrackValue::Real(self.curves.first()?.value_at(time))),
            TrackValueKind::Vector2 => Some(TrackValue::Vector2(Vector2::new(
                self.curves.first()?.value_at(time),
                self.curves.get(1)?.value_at(time),
            ))),
            TrackValueKind::Vector3 => Some(TrackValue::Vector3(Vector3::new(
                self.curves.first()?.value_at(time),
                self.curves.get(1)?.value_at(time),
                self.curves.get(2)?.value_at(time),
            ))),
            TrackValueKind::Vector4 => Some(TrackValue::Vector4(Vector4::new(
                self.curves.first()?.value_at(time),
                self.curves.get(1)?.value_at(time),
                self.curves.get(2)?.value_at(time),
                self.curves.get(3)?.value_at(time),
            ))),
            TrackValueKind::UnitQuaternion => {
                // Convert Euler angles to quaternion
                let (x, y, z) = match self.mode {
                    InterpolationMode::Default => (
                        self.curves.first()?.value_at(time),
                        self.curves.get(1)?.value_at(time),
                        self.curves.get(2)?.value_at(time),
                    ),
                    InterpolationMode::ShortPath => (
                        self.curves.first()?.angle_at(time),
                        self.curves.get(1)?.angle_at(time),
                        self.curves.get(2)?.angle_at(time),
                    ),
                };

                Some(TrackValue::UnitQuaternion(quat_from_euler(
                    Vector3::new(x, y, z),
                    RotationOrder::XYZ,
                )))
            }
        }
    }

    /// Find a right-most key on one of the curves in the container and returns its position. This position
    /// can be treated as a maximum "length" of the container.
    pub fn time_length(&self) -> f32 {
        let mut length = 0.0;
        for curve in self.curves.iter() {
            let max_location = curve.max_location();
            if max_location > length {
                length = max_location;
            }
        }
        length
    }
}
