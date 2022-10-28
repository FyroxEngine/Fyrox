use crate::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        reflect::{Reflect, ResolvePath},
        visitor::prelude::*,
    },
    scene::node::Node,
};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub enum TrackValue {
    Vector3(Vector3<f32>),
    UnitQuaternion(UnitQuaternion<f32>),
}

impl TrackValue {
    pub fn weighted_clone(&self, weight: f32) -> Self {
        match self {
            TrackValue::Vector3(v) => TrackValue::Vector3(v.scale(weight)),
            TrackValue::UnitQuaternion(v) => TrackValue::UnitQuaternion(*v),
        }
    }

    pub fn blend_with(&mut self, other: &Self, weight: f32) {
        match (self, other) {
            (Self::Vector3(a), Self::Vector3(b)) => *a += b.scale(weight),
            (Self::UnitQuaternion(a), Self::UnitQuaternion(b)) => *a = a.nlerp(b, weight),
            _ => (),
        }
    }

    pub fn interpolate(&self, other: &Self, t: f32) -> Option<Self> {
        match (self, other) {
            (Self::Vector3(a), Self::Vector3(b)) => Some(Self::Vector3(a.lerp(b, t))),
            (Self::UnitQuaternion(a), Self::UnitQuaternion(b)) => {
                Some(Self::UnitQuaternion(a.nlerp(b, t)))
            }
            _ => None,
        }
    }

    pub fn boxed_value(&self) -> Box<dyn Reflect> {
        match self {
            TrackValue::Vector3(v) => Box::new(*v),
            TrackValue::UnitQuaternion(v) => Box::new(*v),
        }
    }
}

#[derive(Clone, Visit, Debug, PartialEq, Eq)]
pub enum ValueBinding {
    Position,
    Scale,
    Rotation,
    Property(String),
}

#[derive(Clone, Debug)]
pub struct BoundValue {
    pub binding: ValueBinding,
    pub value: TrackValue,
}

impl BoundValue {
    pub fn weighted_clone(&self, weight: f32) -> Self {
        Self {
            binding: self.binding.clone(),
            value: self.value.weighted_clone(weight),
        }
    }

    pub fn blend_with(&mut self, other: &Self, weight: f32) {
        self.value.blend_with(&other.value, weight);
    }

    pub fn interpolate(&self, other: &BoundValue, t: f32) -> Option<BoundValue> {
        self.value
            .interpolate(&other.value, t)
            .map(|value| BoundValue {
                binding: self.binding.clone(),
                value,
            })
    }

    pub fn boxed_value(&self) -> Box<dyn Reflect> {
        self.value.boxed_value()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BoundValueCollection {
    pub values: Vec<BoundValue>,
}

impl BoundValueCollection {
    pub fn weighted_clone(&self, weight: f32) -> Self {
        Self {
            values: self
                .values
                .iter()
                .map(|v| BoundValue {
                    binding: v.binding.clone(),
                    value: v.value.weighted_clone(weight),
                })
                .collect::<Vec<_>>(),
        }
    }

    pub fn blend_with(&mut self, other: &Self, weight: f32) {
        for (a, b) in self.values.iter_mut().zip(other.values.iter()) {
            a.blend_with(b, weight)
        }
    }

    pub fn interpolate(&self, other: &Self, t: f32) -> Self {
        Self {
            values: self
                .values
                .iter()
                .zip(&other.values)
                .filter_map(|(a, b)| a.interpolate(b, t))
                .collect(),
        }
    }

    pub fn apply(&self, node_ref: &mut Node) {
        for bound_value in self.values.iter() {
            match bound_value.binding {
                ValueBinding::Position => {
                    if let TrackValue::Vector3(v) = bound_value.value {
                        node_ref.local_transform_mut().set_position(v);
                    }
                }
                ValueBinding::Scale => {
                    if let TrackValue::Vector3(v) = bound_value.value {
                        node_ref.local_transform_mut().set_scale(v);
                    }
                }
                ValueBinding::Rotation => {
                    if let TrackValue::UnitQuaternion(v) = bound_value.value {
                        node_ref.local_transform_mut().set_rotation(v);
                    }
                }
                ValueBinding::Property(ref property_name) => {
                    if let Ok(property) = node_ref.as_reflect_mut().resolve_path_mut(property_name)
                    {
                        let _ = property.set(bound_value.boxed_value());
                    }
                }
            }
        }
    }
}
