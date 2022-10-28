use crate::{
    math::{cubicf, lerpf},
    visitor::prelude::*,
};
use std::cmp::Ordering;
use uuid::Uuid;

fn stepf(p0: f32, p1: f32, t: f32) -> f32 {
    if t.eq(&1.0) {
        p1
    } else {
        p0
    }
}

#[derive(Visit, Clone, Debug, PartialEq)]
pub enum CurveKeyKind {
    Constant,
    Linear,
    Cubic {
        /// A `tan(angle)` of left tangent.
        left_tangent: f32,
        /// A `tan(angle)` of right tangent.
        right_tangent: f32,
    },
}

impl CurveKeyKind {
    #[inline]
    pub fn new_cubic(left_angle_radians: f32, right_angle_radians: f32) -> Self {
        Self::Cubic {
            left_tangent: left_angle_radians.tan(),
            right_tangent: right_angle_radians.tan(),
        }
    }
}

impl Default for CurveKeyKind {
    #[inline]
    fn default() -> Self {
        Self::Constant
    }
}

#[derive(Visit, Clone, Default, Debug, PartialEq)]
pub struct CurveKey {
    pub id: Uuid,
    location: f32,
    pub value: f32,
    pub kind: CurveKeyKind,
}

impl CurveKey {
    #[inline]
    pub fn new(location: f32, value: f32, kind: CurveKeyKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            location,
            value,
            kind,
        }
    }
}

impl CurveKey {
    #[inline]
    pub fn location(&self) -> f32 {
        self.location
    }

    #[inline]
    pub fn interpolate(&self, other: &Self, t: f32) -> f32 {
        match (&self.kind, &other.kind) {
            // Constant-to-any
            (CurveKeyKind::Constant, CurveKeyKind::Constant)
            | (CurveKeyKind::Constant, CurveKeyKind::Linear)
            | (CurveKeyKind::Constant, CurveKeyKind::Cubic { .. }) => {
                stepf(self.value, other.value, t)
            }

            // Linear-to-any
            (CurveKeyKind::Linear, CurveKeyKind::Constant)
            | (CurveKeyKind::Linear, CurveKeyKind::Linear)
            | (CurveKeyKind::Linear, CurveKeyKind::Cubic { .. }) => {
                lerpf(self.value, other.value, t)
            }

            // Cubic-to-constant or cubic-to-linear
            (
                CurveKeyKind::Cubic {
                    right_tangent: left_tangent,
                    ..
                },
                CurveKeyKind::Constant,
            )
            | (
                CurveKeyKind::Cubic {
                    right_tangent: left_tangent,
                    ..
                },
                CurveKeyKind::Linear,
            ) => cubicf(self.value, other.value, t, *left_tangent, 0.0),

            // Cubic-to-cubic
            (
                CurveKeyKind::Cubic {
                    right_tangent: left_tangent,
                    ..
                },
                CurveKeyKind::Cubic {
                    left_tangent: right_tangent,
                    ..
                },
            ) => cubicf(self.value, other.value, t, *left_tangent, *right_tangent),
        }
    }
}

#[derive(Visit, Default, Clone, Debug, PartialEq)]
pub struct Curve {
    keys: Vec<CurveKey>,
}

fn sort_keys(keys: &mut [CurveKey]) {
    keys.sort_by(|a, b| {
        if a.location > b.location {
            Ordering::Greater
        } else if a.location < b.location {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    });
}

impl From<Vec<CurveKey>> for Curve {
    fn from(mut keys: Vec<CurveKey>) -> Self {
        sort_keys(&mut keys);
        Self { keys }
    }
}

impl Curve {
    #[inline]
    pub fn clear(&mut self) {
        self.keys.clear()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    #[inline]
    pub fn keys(&self) -> &[CurveKey] {
        &self.keys
    }

    #[inline]
    pub fn add_key(&mut self, new_key: CurveKey) {
        self.keys.push(new_key);
        sort_keys(&mut self.keys);
    }

    #[inline]
    pub fn move_key(&mut self, key_id: usize, location: f32) {
        if let Some(key) = self.keys.get_mut(key_id) {
            key.location = location;
            sort_keys(&mut self.keys);
        }
    }

    #[inline]
    pub fn max_location(&self) -> f32 {
        self.keys.last().map(|k| k.location).unwrap_or_default()
    }

    #[inline]
    pub fn value_at(&self, location: f32) -> f32 {
        if self.keys.is_empty() {
            // Stub - zero
            return Default::default();
        } else if self.keys.len() == 1 {
            // Single key - just return its value
            return self.keys.first().unwrap().value;
        } else if self.keys.len() == 2 {
            // Special case for two keys (much faster than generic)
            let pt_a = self.keys.get(0).unwrap();
            let pt_b = self.keys.get(1).unwrap();
            if location >= pt_a.location && location <= pt_b.location {
                let span = pt_b.location - pt_a.location;
                let t = (location - pt_a.location) / span;
                return pt_a.interpolate(pt_b, t);
            } else if location < pt_a.location {
                return pt_a.value;
            } else {
                return pt_b.value;
            }
        }

        // Generic case - check for out-of-bounds
        let first = self.keys.first().unwrap();
        let last = self.keys.last().unwrap();
        if location <= first.location {
            first.value
        } else if location >= last.location {
            last.value
        } else {
            // Find span first
            let mut pt_a_index = 0;
            for (i, pt) in self.keys.iter().enumerate() {
                if location >= pt.location {
                    pt_a_index = i;
                }
            }
            let pt_b_index = pt_a_index + 1;

            let pt_a = self.keys.get(pt_a_index).unwrap();
            let pt_b = self.keys.get(pt_b_index).unwrap();

            let span = pt_b.location - pt_a.location;
            let t = (location - pt_a.location) / span;
            pt_a.interpolate(pt_b, t)
        }
    }
}
