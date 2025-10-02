// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use nalgebra::{UnitQuaternion, Vector2, Vector3, Vector4};
use std::cmp::Ordering;
use std::fmt::Debug;
use uuid::Uuid;

pub trait Interpolatable: Clone + Debug + PartialEq + Default {
    fn interpolate(&self, other: &Self, t: f32) -> Self;
}

impl Interpolatable for f32 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        super::lerpf(*self, *other, t)
    }
}

impl Interpolatable for Vector2<f32> {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Interpolatable for Vector3<f32> {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Interpolatable for Vector4<f32> {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Interpolatable for UnitQuaternion<f32> {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.slerp(other, t)
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct InterpolationKey<T: Interpolatable> {
    pub id: Uuid,
    pub location: f32,
    pub value: T,
}

impl<T: Interpolatable> InterpolationKey<T> {
    #[inline]
    pub fn new(location: f32, value: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            location,
            value,
        }
    }
}

impl<T: Interpolatable> InterpolationKey<T> {
    #[inline]
    pub fn location(&self) -> f32 {
        self.location
    }

    #[inline]
    pub fn interpolate(&self, other: &Self, t: f32) -> T {
        self.value.interpolate(&other.value, t)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterpolationContainer<T: Interpolatable> {
    pub id: Uuid,
    pub name: String,
    pub keys: Vec<InterpolationKey<T>>,
}

impl<T: Interpolatable> Default for InterpolationContainer<T> {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: Default::default(),
            keys: Default::default(),
        }
    }
}

fn sort_keys<T: Interpolatable>(keys: &mut [InterpolationKey<T>]) {
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

impl<T: Interpolatable> From<Vec<InterpolationKey<T>>> for InterpolationContainer<T> {
    fn from(mut keys: Vec<InterpolationKey<T>>) -> Self {
        sort_keys(&mut keys);
        Self {
            id: Uuid::new_v4(),
            name: Default::default(),
            keys,
        }
    }
}

impl<T: Interpolatable> InterpolationContainer<T> {
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
        name.as_ref().clone_into(&mut self.name);
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
    pub fn keys(&self) -> &[InterpolationKey<T>] {
        &self.keys
    }

    #[inline]
    pub fn keys_values(&mut self) -> impl Iterator<Item = &mut T> {
        self.keys.iter_mut().map(|k| &mut k.value)
    }

    #[inline]
    pub fn add_key(&mut self, new_key: InterpolationKey<T>) {
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
    fn fetch_at<I>(&self, location: f32, interpolator: I) -> T
    where
        I: FnOnce(&InterpolationKey<T>, &InterpolationKey<T>, f32) -> T,
    {
        if let (Some(first), Some(last)) = (self.keys.first(), self.keys.last()) {
            if location <= first.location {
                first.value.clone()
            } else if location >= last.location {
                last.value.clone()
            } else {
                // Use binary search for multiple spans.
                let pos = self.keys.partition_point(|k| k.location < location);
                let left = self.keys.get(pos.saturating_sub(1)).unwrap();
                let right = self.keys.get(pos).unwrap();
                let t = (location - left.location) / (right.location - left.location);
                interpolator(left, right, t)
            }
        } else {
            T::default()
        }
    }

    #[inline]
    pub fn value_at(&self, location: f32) -> T {
        self.fetch_at(location, |a, b, t| a.interpolate(b, t))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_interpolation_key_insertion_order() {
        let mut curve = InterpolationContainer::default();

        // Insert keys in arbitrary order with arbitrary location.
        curve.add_key(InterpolationKey::new(0.0, 0.0));
        curve.add_key(InterpolationKey::new(-1.0, 0.0));
        curve.add_key(InterpolationKey::new(3.0, 0.0));
        curve.add_key(InterpolationKey::new(2.0, 0.0));
        curve.add_key(InterpolationKey::new(-5.0, 0.0));

        // Ensure that keys are sorted by their location.
        assert_eq!(curve.keys[0].location, -5.0);
        assert_eq!(curve.keys[1].location, -1.0);
        assert_eq!(curve.keys[2].location, 0.0);
        assert_eq!(curve.keys[3].location, 2.0);
        assert_eq!(curve.keys[4].location, 3.0);
    }

    #[test]
    fn test_interpolation() {
        let mut curve = InterpolationContainer::default();

        // Test fetching from empty curve.
        assert_eq!(curve.value_at(0.0), 0.0);

        curve.add_key(InterpolationKey::new(0.0, 1.0));

        // One-key curves must always return its single key value.
        assert_eq!(curve.value_at(-1.0), 1.0);
        assert_eq!(curve.value_at(1.0), 1.0);
        assert_eq!(curve.value_at(0.0), 1.0);

        curve.add_key(InterpolationKey::new(1.0, 0.0));

        // Two-key curves must always use interpolation.
        assert_eq!(curve.value_at(-1.0), 1.0);
        assert_eq!(curve.value_at(2.0), 0.0);
        assert_eq!(curve.value_at(0.5), 0.5);

        // Add one more key and do more checks.
        curve.add_key(InterpolationKey::new(2.0, 1.0));

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
        let key = InterpolationKey::new(0.0, 5.0);
        let key2 = InterpolationKey::new(1.0, 10.0);
        curve.add_key(key.clone());
        curve.add_key(key2.clone());
        assert_eq!(curve.keys(), vec![key.clone(), key2.clone()]);

        // Check keys values.
        let mut values = [5.0, 10.0];
        assert!(curve.keys_values().eq(values.iter_mut()));

        // Check max location.
        assert_eq!(curve.max_location(), 1.0);

        // Check key moving.
        let mut curve2 = curve.clone();
        let key3 = InterpolationKey::default();
        curve2.add_key(key3.clone());
        assert_eq!(curve2.keys(), vec![key3.clone(), key.clone(), key2.clone()]);
        curve2.move_key(key3.id.get_version_num(), 20.0);
        assert_eq!(
            curve2.keys(),
            vec![
                key.clone(),
                key2.clone(),
                InterpolationKey {
                    location: 20.0,
                    ..Default::default()
                }
            ]
        );
    }

    #[test]
    fn test_interpolation_from_vec() {
        let key = InterpolationKey::new(-1.0, -1.0);
        let key2 = InterpolationKey::new(0.0, 0.0);
        let key3 = InterpolationKey::new(1.0, 1.0);
        let key4 = key2.clone();
        let curve = InterpolationContainer::from(vec![
            key2.clone(),
            key3.clone(),
            key.clone(),
            key4.clone(),
        ]);
        assert_eq!(curve.name(), "");
        assert_eq!(curve.keys(), vec![key, key2, key4, key3,]);
    }
}
