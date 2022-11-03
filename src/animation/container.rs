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

#[derive(Clone, Copy, Debug, Visit, Reflect, PartialEq, Eq)]
pub enum TrackValueKind {
    Vector3,
    UnitQuaternion,
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
    pub fn with_n_curves(kind: TrackValueKind, curve_count: usize) -> Self {
        Self {
            kind,
            curves: vec![Default::default(); curve_count],
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
            TrackValueKind::Vector3 => Some(TrackValue::Vector3(Vector3::new(
                self.curves.get(0)?.value_at(time),
                self.curves.get(1)?.value_at(time),
                self.curves.get(2)?.value_at(time),
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
