use crate::{
    math::{cubicf, lerpf},
    reflect::prelude::*,
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

#[derive(Visit, Reflect, Clone, Debug, PartialEq)]
#[reflect(hide_all)]
pub struct Curve {
    #[visit(optional)] // Backward compatibility
    id: Uuid,

    #[visit(optional)] // Backward compatibility
    name: String,

    keys: Vec<CurveKey>,
}

impl Default for Curve {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: Default::default(),
            keys: Default::default(),
        }
    }
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
        Self {
            id: Uuid::new_v4(),
            name: Default::default(),
            keys,
        }
    }
}

impl Curve {
    #[inline]
    pub fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    #[inline]
    pub fn id(&self) -> Uuid {
        self.id
    }

    #[inline]
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = name.as_ref().to_owned();
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

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
    pub fn keys_values(&mut self) -> impl Iterator<Item = &mut f32> {
        self.keys.iter_mut().map(|k| &mut k.value)
    }

    #[inline]
    pub fn add_key(&mut self, new_key: CurveKey) {
        let pos = self.keys.partition_point(|k| k.location < new_key.location);
        self.keys.insert(pos, new_key);
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
        if let (Some(first), Some(last)) = (self.keys.first(), self.keys.last()) {
            if location <= first.location {
                first.value
            } else if location >= last.location {
                last.value
            } else {
                // Use binary search for multiple spans.
                let pos = self.keys.partition_point(|k| k.location < location);
                let left = self.keys.get(pos.saturating_sub(1)).unwrap();
                let right = self.keys.get(pos).unwrap();
                left.interpolate(
                    right,
                    (location - left.location) / (right.location - left.location),
                )
            }
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod test {
    use uuid::Uuid;

    use crate::curve::{Curve, CurveKey, CurveKeyKind};

    #[test]
    fn test_curve_key_insertion_order() {
        let mut curve = Curve::default();

        // Insert keys in arbitrary order with arbitrary location.
        curve.add_key(CurveKey::new(0.0, 0.0, CurveKeyKind::Constant));
        curve.add_key(CurveKey::new(-1.0, 0.0, CurveKeyKind::Constant));
        curve.add_key(CurveKey::new(3.0, 0.0, CurveKeyKind::Constant));
        curve.add_key(CurveKey::new(2.0, 0.0, CurveKeyKind::Constant));
        curve.add_key(CurveKey::new(-5.0, 0.0, CurveKeyKind::Constant));

        // Ensure that keys are sorted by their location.
        assert_eq!(curve.keys[0].location, -5.0);
        assert_eq!(curve.keys[1].location, -1.0);
        assert_eq!(curve.keys[2].location, 0.0);
        assert_eq!(curve.keys[3].location, 2.0);
        assert_eq!(curve.keys[4].location, 3.0);
    }

    #[test]
    fn test_curve() {
        let mut curve = Curve::default();

        // Test fetching from empty curve.
        assert_eq!(curve.value_at(0.0), 0.0);

        curve.add_key(CurveKey::new(0.0, 1.0, CurveKeyKind::Linear));

        // One-key curves must always return its single key value.
        assert_eq!(curve.value_at(-1.0), 1.0);
        assert_eq!(curve.value_at(1.0), 1.0);
        assert_eq!(curve.value_at(0.0), 1.0);

        curve.add_key(CurveKey::new(1.0, 0.0, CurveKeyKind::Linear));

        // Two-key curves must always use interpolation.
        assert_eq!(curve.value_at(-1.0), 1.0);
        assert_eq!(curve.value_at(2.0), 0.0);
        assert_eq!(curve.value_at(0.5), 0.5);

        // Add one more key and do more checks.
        curve.add_key(CurveKey::new(2.0, 1.0, CurveKeyKind::Linear));

        // Check order of the keys.
        assert!(curve.keys[0].location <= curve.keys[1].location);
        assert!(curve.keys[1].location <= curve.keys[2].location);

        // Check generic out-of-bounds fetching.
        assert_eq!(curve.value_at(-1.0), 1.0); // Left side oob
        assert_eq!(curve.value_at(3.0), 1.0); // Right side oob.

        // Check edge cases.
        assert_eq!(curve.value_at(0.0), 1.0); // Left edge.
        assert_eq!(curve.value_at(2.0), 1.0); // Right edge.

        // Check interpolation.
        assert_eq!(curve.value_at(0.5), 0.5);

        // Check id.
        let id = Uuid::new_v4();
        curve.set_id(id);
        assert_eq!(curve.id(), id);

        // Check name.
        let name = "name";
        curve.set_name(name);
        assert_eq!(curve.name(), name);

        // Check keys capacity.
        assert!(!curve.is_empty());
        curve.clear();
        assert!(curve.is_empty());

        // Check keys.
        let key = CurveKey::new(0.0, 5.0, CurveKeyKind::Constant);
        let key2 = CurveKey::new(1.0, 10.0, CurveKeyKind::Linear);
        curve.add_key(key.clone());
        curve.add_key(key2.clone());
        assert_eq!(curve.keys(), vec![key.clone(), key2.clone()]);

        // Check keys values.
        let mut values = vec![5.0, 10.0];
        assert!(curve.keys_values().eq(values.iter_mut()));

        // Check max location.
        assert_eq!(curve.max_location(), 1.0);

        // Check key moving.
        let mut curve2 = curve.clone();
        let key3 = CurveKey::default();
        curve2.add_key(key3.clone());
        assert_eq!(curve2.keys(), vec![key3.clone(), key.clone(), key2.clone()]);
        curve2.move_key(key3.id.get_version_num(), 20.0);
        assert_eq!(
            curve2.keys(),
            vec![
                key.clone(),
                key2.clone(),
                CurveKey {
                    location: 20.0,
                    ..Default::default()
                }
            ]
        );
    }

    #[test]
    fn test_curve_key_kind() {
        assert_eq!(CurveKeyKind::default(), CurveKeyKind::Constant);
        assert_eq!(
            CurveKeyKind::new_cubic(0.0, 0.0),
            CurveKeyKind::Cubic {
                left_tangent: 0.0,
                right_tangent: 0.0
            }
        );
    }

    #[test]
    fn test_curve_key() {
        assert_eq!(
            CurveKey::default(),
            CurveKey {
                id: Uuid::default(),
                location: 0.0,
                value: 0.0,
                kind: CurveKeyKind::Constant,
            },
        );

        let key = CurveKey::new(0.0, 5.0, CurveKeyKind::Constant);
        let key2 = CurveKey::new(1.0, 10.0, CurveKeyKind::Linear);
        let key3 = CurveKey::new(2.0, 20.0, CurveKeyKind::new_cubic(0.0, 0.0));
        let key4 = CurveKey::new(3.0, 30.0, CurveKeyKind::new_cubic(0.0, 0.0));

        assert_eq!(key.location(), 0.0);

        // Constant-to-any
        assert_eq!(key.interpolate(&key2, 1.0), 10.0);
        assert_eq!(key.interpolate(&key2, 0.0), 5.0);
        assert_eq!(key.interpolate(&key3, 1.0), 20.0);
        assert_eq!(key.interpolate(&key3, 0.0), 5.0);

        // Linear-to-any
        assert_eq!(key2.interpolate(&key, 1.0), 5.0);
        assert_eq!(key2.interpolate(&key, 0.0), 10.0);
        assert_eq!(key2.interpolate(&key3, 1.0), 20.0);
        assert_eq!(key2.interpolate(&key3, 0.0), 10.0);

        // Cubic-to-constant or cubic-to-linear
        assert_eq!(key3.interpolate(&key, 1.0), 5.0);
        assert_eq!(key3.interpolate(&key, 0.0), 20.0);
        assert_eq!(key3.interpolate(&key2, 1.0), 10.0);
        assert_eq!(key3.interpolate(&key2, 0.0), 20.0);

        // Cubic-to-cubic
        assert_eq!(key3.interpolate(&key4, 1.0), 30.0);
        assert_eq!(key3.interpolate(&key4, 0.0), 20.0);
    }

    #[test]
    fn test_curve_from_vec() {
        let key = CurveKey::new(-1.0, -1.0, CurveKeyKind::Constant);
        let key2 = CurveKey::new(0.0, 0.0, CurveKeyKind::Constant);
        let key3 = CurveKey::new(1.0, 1.0, CurveKeyKind::Constant);
        let key4 = key2.clone();
        let curve = Curve::from(vec![key2.clone(), key3.clone(), key.clone(), key4.clone()]);
        assert_eq!(curve.name(), "");
        assert_eq!(curve.keys(), vec![key, key2, key4, key3,]);
    }
}
