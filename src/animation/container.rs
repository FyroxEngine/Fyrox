use crate::{
    animation::value::TrackValue,
    core::{
        algebra::Vector3,
        curve::Curve,
        math::{quat_from_euler, RotationOrder},
        reflect::prelude::*,
        visitor::prelude::*,
    },
};
use fyrox_core::algebra::{Vector2, Vector4};

#[derive(Clone, Copy, Debug, Visit, Reflect, PartialEq, Eq)]
pub enum TrackValueKind {
    /// A real number.
    Real,

    /// A 2-dimensional vector of real values.
    Vector2,

    /// A 3-dimensional vector of real values.
    Vector3,

    /// A 4-dimensional vector of real values.
    Vector4,

    /// A quaternion that represents some rotation.
    UnitQuaternion,
}

impl TrackValueKind {
    /// Returns count of elementary components of a value kind. For example: Vector3 consists of
    /// 3 components where each component has its own parametric curve.
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

#[derive(Visit, Reflect, Debug, Clone, Default)]
pub struct TrackFramesContainer {
    curves: Vec<Curve>,
    kind: TrackValueKind,
}

impl TrackFramesContainer {
    pub fn new(kind: TrackValueKind) -> Self {
        Self {
            kind,
            curves: vec![Default::default(); kind.components_count()],
        }
    }

    pub fn add_curve(&mut self, curve: Curve) {
        self.curves.push(curve)
    }

    pub fn curve(&self, index: usize) -> Option<&Curve> {
        self.curves.get(index)
    }

    pub fn curve_mut(&mut self, index: usize) -> Option<&mut Curve> {
        self.curves.get_mut(index)
    }

    pub fn curves_ref(&self) -> &[Curve] {
        &self.curves
    }

    pub fn curves_mut(&mut self) -> &mut [Curve] {
        &mut self.curves
    }

    pub fn set_value_kind(&mut self, kind: TrackValueKind) {
        self.kind = kind;
    }

    pub fn value_kind(&self) -> TrackValueKind {
        self.kind
    }

    pub fn fetch(&self, time: f32) -> Option<TrackValue> {
        match self.kind {
            TrackValueKind::Real => Some(TrackValue::Real(self.curves.get(0)?.value_at(time))),
            TrackValueKind::Vector2 => Some(TrackValue::Vector2(Vector2::new(
                self.curves.get(0)?.value_at(time),
                self.curves.get(1)?.value_at(time),
            ))),
            TrackValueKind::Vector3 => Some(TrackValue::Vector3(Vector3::new(
                self.curves.get(0)?.value_at(time),
                self.curves.get(1)?.value_at(time),
                self.curves.get(2)?.value_at(time),
            ))),
            TrackValueKind::Vector4 => Some(TrackValue::Vector4(Vector4::new(
                self.curves.get(0)?.value_at(time),
                self.curves.get(1)?.value_at(time),
                self.curves.get(2)?.value_at(time),
                self.curves.get(3)?.value_at(time),
            ))),
            TrackValueKind::UnitQuaternion => {
                // Convert Euler angles to quaternion
                let x = self.curves.get(0)?.value_at(time);
                let y = self.curves.get(1)?.value_at(time);
                let z = self.curves.get(2)?.value_at(time);

                Some(TrackValue::UnitQuaternion(quat_from_euler(
                    Vector3::new(x, y, z),
                    RotationOrder::XYZ,
                )))
            }
        }
    }

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
