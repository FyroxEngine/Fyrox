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

//! Orthogonal transformations are some combination of 90-degree turns and
//! flips across the x-axis and y-axis. These transformations are important to
//! tiles because they are constrained along the tile grid.
//!
//! [`OrthoTransformation`] represents some combination of flips and turns,
//! and [`OrthoTransform`] is a class for objects that can be transformed.

use std::{
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Neg},
};

use fxhash::FxHashMap;

use crate::core::{
    algebra::{Matrix2, Scalar, SimdPartialOrd, Vector2},
    math::{Number, Rect},
    reflect::prelude::*,
    visitor::prelude::*,
};

use super::OptionTileRect;

/// The amount of an orthogonal transformation.
/// The tranformation either includes an optional initial x-flip that turns (1,0) into (-1,0), or not.
/// Following the optional flip, the object being transformed is rotated counter-clockwise by some number
/// of right-angle turns: 0, 1, 2, or 3. Multiple transformations can be chained together for arbitrary
/// sequences of x-flips, y-flis, clockwise and counter-clockwise rotations.
///
/// These transformations are useful in situations where positions are
/// restricted to an orthogonal grid, as in a tile map.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Visit, Reflect)]
pub struct OrthoTransformation(i8);

/// A map from `Vector2<i32>` to values. It can be transformed to flip and rotate the positions of the values.
#[derive(Clone, Debug, Visit)]
pub struct OrthoTransformMap<V> {
    transform: OrthoTransformation,
    map: FxHashMap<Vector2<i32>, V>,
}

impl Default for OrthoTransformation {
    fn default() -> Self {
        Self::identity()
    }
}

const ROTATION_MATRICES: [Matrix2<f32>; 4] = [
    Matrix2::new(1.0, 0.0, 0.0, 1.0),
    Matrix2::new(0.0, -1.0, 1.0, 0.0),
    Matrix2::new(-1.0, 0.0, 0.0, -1.0),
    Matrix2::new(0.0, 1.0, -1.0, 0.0),
];

const X_FLIP_MATRIX: Matrix2<f32> = Matrix2::new(-1.0, 0.0, 0.0, 1.0);

impl OrthoTransformation {
    /// The transform that does nothing. It has no flip, and rotates by 0 right-angle turns.
    #[inline]
    pub const fn identity() -> Self {
        Self(1)
    }
    /// The transformation that first does an optional x-flip,
    /// then rotates counter-clockwise by the given amount.
    /// The rotation is measured in units of 90-degree rotations.
    /// Positive rotation is counter-clockwise. Negative rotation is clockwise.
    #[inline]
    pub const fn new(flipped: bool, rotation: i8) -> Self {
        let rotation = rotation.rem_euclid(4);
        Self(if flipped { -rotation - 1 } else { rotation + 1 })
    }
    /// An iterator over all 8 possible OrthoTransformations.
    pub fn all() -> impl Iterator<Item = OrthoTransformation> {
        [-4i8, -3, -2, -1, 1, 2, 3, 4]
            .into_iter()
            .map(OrthoTransformation)
    }
    /// True if this tranformation is the idenity transformation that leaves the transformed object unmodified.
    #[inline]
    pub const fn is_identity(&self) -> bool {
        self.0 == 1
    }
    /// Reverse this transformation, to it would return a transformed object
    /// back to where it started.
    /// In other words: `x.transformed(t).transformed(t.inverted()) == x`.
    #[inline]
    pub const fn inverted(self) -> Self {
        Self(match self.0 {
            1 => 1,
            2 => 4,
            3 => 3,
            4 => 2,
            -1 => -1,
            -2 => -2,
            -3 => -3,
            -4 => -4,
            _ => unreachable!(),
        })
    }
    /// True if this transform starts with an x-flip.
    #[inline]
    pub const fn is_flipped(&self) -> bool {
        self.0 < 0
    }
    /// The amount of rotation following the optional x-flip.
    /// The value is always 0, 1, 2, or 3, representing counter-clockwise rotations of
    /// 0, 90, 180, or 270 degrees.
    #[inline]
    pub const fn rotation(&self) -> i8 {
        self.0.abs() - 1
    }
    /// Matrix representation of this transformation.
    pub fn matrix(&self) -> Matrix2<f32> {
        let matrix = if self.is_flipped() {
            X_FLIP_MATRIX
        } else {
            Matrix2::identity()
        };
        ROTATION_MATRICES[self.rotation() as usize] * matrix
    }
}

impl Debug for OrthoTransformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OrthoTransformation")
            .field(&self.0)
            .field(&self.is_flipped())
            .field(&self.rotation())
            .finish()
    }
}

impl Display for OrthoTransformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rotation = match self.rotation() {
            0 => 0,
            1 => 90,
            2 => 180,
            3 => 270,
            _ => unreachable!(),
        };
        if self.is_flipped() {
            write!(f, "rotate({})(flipped)", rotation)
        } else {
            write!(f, "rotate({})", rotation)
        }
    }
}

/// Trait for objects that can perform a 2D orthogonal transformation.
/// In other words, they can be flipped along the x and y axis,
/// and they can be rotated by multiples of 90 degrees.
pub trait OrthoTransform: Sized {
    /// Flip the object parallel to the x axis, so x becomes -x.
    fn x_flipped(self) -> Self;
    /// Flip the object parallel to the y axis, so y becomes -y.
    fn y_flipped(self) -> Self {
        self.x_flipped().rotated(2)
    }
    /// Rotate the object counter-clockwise by the given amount.
    fn rotated(self, amount: i8) -> Self;
    /// Transform the object by the given amount.
    fn transformed(self, amount: OrthoTransformation) -> Self {
        (if amount.is_flipped() {
            self.x_flipped()
        } else {
            self
        })
        .rotated(amount.rotation())
    }
}

impl OrthoTransform for OrthoTransformation {
    fn x_flipped(self) -> Self {
        Self(match self.0 {
            1 => -1,
            2 => -4,
            3 => -3,
            4 => -2,
            -1 => 1,
            -2 => 4,
            -3 => 3,
            -4 => 2,
            _ => unreachable!(),
        })
    }
    fn rotated(self, amount: i8) -> Self {
        let amount = amount.rem_euclid(4);
        if self.0 > 0 {
            Self((self.0 + amount - 1).rem_euclid(4) + 1)
        } else {
            Self(-(self.0.abs() + amount - 1).rem_euclid(4) - 1)
        }
    }
}

impl<V: Neg<Output = V> + Scalar + Clone> OrthoTransform for Vector2<V> {
    fn x_flipped(self) -> Self {
        Self::new(-self.x.clone(), self.y.clone())
    }

    fn rotated(self, amount: i8) -> Self {
        let amount = amount.rem_euclid(4);
        match amount {
            0 => self,
            1 => Self::new(-self.y.clone(), self.x.clone()),
            2 => Self::new(-self.x.clone(), -self.y.clone()),
            3 => Self::new(self.y.clone(), -self.x.clone()),
            _ => unreachable!(),
        }
    }
}

impl<V: Number + SimdPartialOrd + Add + AddAssign + Neg<Output = V> + Scalar> OrthoTransform
    for Rect<V>
{
    fn x_flipped(self) -> Self {
        Rect::from_points(
            self.position.x_flipped(),
            (self.position + self.size).x_flipped(),
        )
    }

    fn rotated(self, amount: i8) -> Self {
        Rect::from_points(
            self.position.rotated(amount),
            (self.position + self.size).rotated(amount),
        )
    }
}

impl<V> OrthoTransformMap<V> {
    /// Bounding rectangle the contains the keys.
    pub fn bounding_rect(&self) -> OptionTileRect {
        let mut result = OptionTileRect::default();
        for position in self.keys() {
            result.push(position);
        }
        result
    }

    /// Clear the elements of the map.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear()
    }
    /// True if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    /// True if the map contains an element at the given position.
    #[inline]
    pub fn contains_key(&self, position: Vector2<i32>) -> bool {
        self.map
            .contains_key(&position.transformed(self.transform.inverted()))
    }
    /// Remove and return the element at the given position, if one exists.
    #[inline]
    pub fn remove(&mut self, position: Vector2<i32>) -> Option<V> {
        self.map
            .remove(&position.transformed(self.transform.inverted()))
    }
    /// Return a reference to the element at the given position, if any.
    #[inline]
    pub fn get(&self, position: Vector2<i32>) -> Option<&V> {
        self.map
            .get(&position.transformed(self.transform.inverted()))
    }
    /// Return a reference to the element at the given position, if any.
    #[inline]
    pub fn get_mut(&mut self, position: Vector2<i32>) -> Option<&mut V> {
        self.map
            .get_mut(&position.transformed(self.transform.inverted()))
    }
    /// Put an element into the map at the given position, and return the element that was previously at that position.
    #[inline]
    pub fn insert(&mut self, position: Vector2<i32>, value: V) -> Option<V> {
        self.map
            .insert(position.transformed(self.transform.inverted()), value)
    }
    /// Iterate through the map.
    #[inline]
    pub fn iter(&self) -> Iter<V> {
        Iter(self.transform, self.map.iter())
    }
    /// Iterate through the keys.
    #[inline]
    pub fn keys(&self) -> Keys<V> {
        Keys(self.transform, self.map.keys())
    }
    /// Iterate through the values.
    #[inline]
    pub fn values(&self) -> std::collections::hash_map::Values<Vector2<i32>, V> {
        self.map.values()
    }
}

impl<V> Default for OrthoTransformMap<V> {
    fn default() -> Self {
        Self {
            transform: Default::default(),
            map: Default::default(),
        }
    }
}

impl<V> OrthoTransform for OrthoTransformMap<V> {
    fn x_flipped(self) -> Self {
        Self {
            transform: self.transform.x_flipped(),
            map: self.map,
        }
    }

    fn rotated(self, amount: i8) -> Self {
        Self {
            transform: self.transform.rotated(amount),
            map: self.map,
        }
    }
}
/// Iterator for [`OrthoTransformMap`].
pub struct Iter<'a, V>(
    OrthoTransformation,
    std::collections::hash_map::Iter<'a, Vector2<i32>, V>,
);

/// Keys iterator for [`OrthoTransformMap`].
pub struct Keys<'a, V>(
    OrthoTransformation,
    std::collections::hash_map::Keys<'a, Vector2<i32>, V>,
);

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (Vector2<i32>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let (k, v) = self.1.next()?;
        Some((k.transformed(self.0), v))
    }
}
impl<V> Iterator for Keys<'_, V> {
    type Item = Vector2<i32>;

    fn next(&mut self) -> Option<Self::Item> {
        let k = self.1.next()?;
        Some(k.transformed(self.0))
    }
}

/// TransformSetCell represents a position within a transform set page and the corresponding transform.
/// This object fascilitates using a transform set page to transform tiles by moving between positions
/// in the page based on the desired transformation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransformSetCell(Vector2<i32>, OrthoTransformation);

fn transform_to_cell_position(rotation: i8) -> Vector2<i32> {
    match rotation {
        -1 => Vector2::new(3, 0),
        -2 => Vector2::new(3, 1),
        -3 => Vector2::new(2, 1),
        -4 => Vector2::new(2, 0),
        1 => Vector2::new(0, 0),
        2 => Vector2::new(1, 0),
        3 => Vector2::new(1, 1),
        4 => Vector2::new(0, 1),
        _ => panic!(),
    }
}

fn cell_position_to_transform(position: Vector2<i32>) -> i8 {
    match (position.x, position.y) {
        (3, 0) => -1,
        (3, 1) => -2,
        (2, 1) => -3,
        (2, 0) => -4,
        (0, 0) => 1,
        (1, 0) => 2,
        (1, 1) => 3,
        (0, 1) => 4,
        _ => panic!(),
    }
}

impl TransformSetCell {
    /// Convert this cell into the corresponding position within a TransformSet page.
    pub fn into_position(self) -> Vector2<i32> {
        self.0 + transform_to_cell_position(self.1 .0)
    }
    /// Convert a position within a TransformSet into a transform set cell that specifies the corresponding orthogonal transformation.
    pub fn from_position(position: Vector2<i32>) -> Self {
        let rem = Vector2::new(position.x.rem_euclid(4), position.y.rem_euclid(2));
        let pos = Vector2::new(position.x - rem.x, position.y - rem.y);
        TransformSetCell(pos, OrthoTransformation(cell_position_to_transform(rem)))
    }
    /// Replace the transformation of this cell with a different transformation.
    /// This is part of the process of rotating a tile:
    /// 1. Find the transform set [`TileDefinitionHandle`](super::TileDefinitionHandle) from the tile's data.
    /// 2. Convert the tile position from the `TileDefinitonHandle` into a `TransformSetCell` using [TransformSetCell::from_position].
    /// 3. Use `with_transformation` to replace the transformation from the `TransformSetCell` with the desired transformation.
    /// 4. Convert the updated `TransformSetCell` into a tile position using [TransformSetCell::into_position].
    /// 5. Get the `TileDefinitionHandle` from the transform set page at that position.
    pub fn with_transformation(self, trans: OrthoTransformation) -> Self {
        TransformSetCell(self.0, trans)
    }
}

impl OrthoTransform for TransformSetCell {
    fn x_flipped(self) -> Self {
        Self(self.0, self.1.x_flipped())
    }

    fn rotated(self, amount: i8) -> Self {
        Self(self.0, self.1.rotated(amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::algebra::Point2;
    use OrthoTransformation as Trans;
    use TransformSetCell as Cell;

    #[test]
    fn identity() {
        assert_eq!(Trans::identity(), Trans::new(false, 0));
        for v in [
            Vector2::new(2, 1),
            Vector2::new(1, 2),
            Vector2::new(-1, 5),
            Vector2::new(-2, -2),
        ] {
            assert_eq!(v.transformed(Trans::identity()), v);
        }
    }
    fn matrix_check(trans: OrthoTransformation) {
        let v = Vector2::new(1.0, 0.5);
        let m = trans.matrix().to_homogeneous();
        let p = m.transform_point(&Point2::from(v)).coords;
        assert_eq!(p, v.transformed(trans), "{}", trans);
    }
    #[test]
    fn matrix() {
        for i in 0..4 {
            matrix_check(Trans::new(false, i))
        }
        for i in 0..4 {
            matrix_check(Trans::new(true, i))
        }
    }
    #[test]
    fn rotate_4() {
        assert_eq!(Trans::identity(), Trans::new(false, 4))
    }
    #[test]
    fn invert() {
        for i in [-4, -3, -2, -1, 1, 2, 3, 4] {
            assert_eq!(
                Trans(i).transformed(Trans(i).inverted()),
                Trans::identity(),
                "{:?}: {:?} {:?}",
                i,
                Trans(i),
                Trans(i).inverted()
            );
        }
    }
    #[test]
    fn inverse_undoes_transform() {
        for i in [-4, -3, -2, -1, 1, 2, 3, 4] {
            assert_eq!(
                Vector2::new(2, 3)
                    .transformed(Trans(i))
                    .transformed(Trans(i).inverted()),
                Vector2::new(2, 3),
            );
        }
    }
    #[test]
    fn rotate_trans() {
        assert_eq!(Trans::new(false, 0).rotated(0), Trans::new(false, 0));
        assert_eq!(Trans::new(true, 0).rotated(0), Trans::new(true, 0));
        assert_eq!(Trans::new(true, 2).rotated(0), Trans::new(true, 2));
        assert_eq!(Trans::new(false, 0).rotated(1), Trans::new(false, 1));
        assert_eq!(Trans::new(true, 0).rotated(1), Trans::new(true, 1));
        assert_eq!(Trans::new(true, 2).rotated(1), Trans::new(true, 3));
        assert_eq!(Trans::new(false, 0).rotated(2), Trans::new(false, 2));
        assert_eq!(Trans::new(true, 0).rotated(2), Trans::new(true, 2));
        assert_eq!(Trans::new(true, 2).rotated(2), Trans::new(true, 0));
    }
    #[test]
    fn flipped_trans() {
        assert_eq!(Trans::new(false, 0).x_flipped(), Trans::new(true, 0));
        assert_eq!(Trans::new(true, 0).x_flipped(), Trans::new(false, 0));
        assert_eq!(Trans::new(true, 2).x_flipped(), Trans::new(false, 2));
        assert_eq!(Trans::new(true, 1).x_flipped(), Trans::new(false, 3));
        assert_eq!(Trans::new(false, 0).y_flipped(), Trans::new(true, 2));
        assert_eq!(Trans::new(true, 0).y_flipped(), Trans::new(false, 2));
        assert_eq!(Trans::new(true, 2).y_flipped(), Trans::new(false, 0));
        assert_eq!(Trans::new(true, 1).y_flipped(), Trans::new(false, 1));
    }
    #[test]
    fn rotate_vector() {
        assert_eq!(Vector2::new(1, 0).rotated(0), Vector2::new(1, 0));
        assert_eq!(Vector2::new(0, 1).rotated(0), Vector2::new(0, 1));
        assert_eq!(Vector2::new(2, 3).rotated(0), Vector2::new(2, 3));
        assert_eq!(Vector2::new(1, 0).rotated(1), Vector2::new(0, 1));
        assert_eq!(Vector2::new(0, 1).rotated(1), Vector2::new(-1, 0));
        assert_eq!(Vector2::new(2, 3).rotated(1), Vector2::new(-3, 2));
        assert_eq!(Vector2::new(1, 0).rotated(2), Vector2::new(-1, 0));
        assert_eq!(Vector2::new(0, 1).rotated(2), Vector2::new(0, -1));
        assert_eq!(Vector2::new(2, 3).rotated(2), Vector2::new(-2, -3));
        assert_eq!(Vector2::new(1, 0).rotated(3), Vector2::new(0, -1));
        assert_eq!(Vector2::new(0, 1).rotated(3), Vector2::new(1, 0));
        assert_eq!(Vector2::new(2, 3).rotated(3), Vector2::new(3, -2));
        assert_eq!(Vector2::new(1, 0).rotated(4), Vector2::new(1, 0));
        assert_eq!(Vector2::new(0, 1).rotated(4), Vector2::new(0, 1));
        assert_eq!(Vector2::new(2, 3).rotated(4), Vector2::new(2, 3));
    }
    #[test]
    fn flipped_vector() {
        assert_eq!(Vector2::new(1, 0).x_flipped(), Vector2::new(-1, 0));
        assert_eq!(Vector2::new(0, 1).x_flipped(), Vector2::new(0, 1));
        assert_eq!(Vector2::new(2, 3).x_flipped(), Vector2::new(-2, 3));
        assert_eq!(Vector2::new(1, 0).y_flipped(), Vector2::new(1, 0));
        assert_eq!(Vector2::new(0, 1).y_flipped(), Vector2::new(0, -1));
        assert_eq!(Vector2::new(2, 3).y_flipped(), Vector2::new(2, -3));
    }
    #[test]
    fn flipped() {
        assert!(!Trans::new(false, -3).is_flipped());
        assert!(!Trans::new(false, -2).is_flipped());
        assert!(!Trans::new(false, -1).is_flipped());
        assert!(!Trans::new(false, 0).is_flipped());
        assert!(!Trans::new(false, 1).is_flipped());
        assert!(!Trans::new(false, 2).is_flipped());
        assert!(!Trans::new(false, 3).is_flipped());
        assert!(!Trans::new(false, 4).is_flipped());
        assert!(!Trans::new(false, 5).is_flipped());
        assert!(Trans::new(true, -3).is_flipped());
        assert!(Trans::new(true, -2).is_flipped());
        assert!(Trans::new(true, -1).is_flipped());
        assert!(Trans::new(true, 0).is_flipped());
        assert!(Trans::new(true, 1).is_flipped());
        assert!(Trans::new(true, 2).is_flipped());
        assert!(Trans::new(true, 3).is_flipped());
        assert!(Trans::new(true, 4).is_flipped());
        assert!(Trans::new(true, 5).is_flipped());
    }
    #[test]
    fn rotate_amount() {
        assert_eq!(Trans::new(false, -3).rotation(), 1);
        assert_eq!(Trans::new(false, -2).rotation(), 2);
        assert_eq!(Trans::new(false, -1).rotation(), 3);
        assert_eq!(Trans::new(false, 0).rotation(), 0);
        assert_eq!(Trans::new(false, 1).rotation(), 1);
        assert_eq!(Trans::new(false, 2).rotation(), 2);
        assert_eq!(Trans::new(false, 3).rotation(), 3);
        assert_eq!(Trans::new(false, 4).rotation(), 0);
        assert_eq!(Trans::new(false, 5).rotation(), 1);
    }
    #[test]
    fn flipped_rotate_amount() {
        assert_eq!(Trans::new(true, -3).rotation(), 1);
        assert_eq!(Trans::new(true, -2).rotation(), 2);
        assert_eq!(Trans::new(true, -1).rotation(), 3);
        assert_eq!(Trans::new(true, 0).rotation(), 0);
        assert_eq!(Trans::new(true, 1).rotation(), 1);
        assert_eq!(Trans::new(true, 2).rotation(), 2);
        assert_eq!(Trans::new(true, 3).rotation(), 3);
        assert_eq!(Trans::new(true, 4).rotation(), 0);
        assert_eq!(Trans::new(true, 5).rotation(), 1);
    }
    #[test]
    fn double_x_flip() {
        assert_eq!(Trans::identity(), Trans::identity().x_flipped().x_flipped())
    }
    #[test]
    fn double_y_flip() {
        assert_eq!(Trans::identity(), Trans::identity().y_flipped().y_flipped())
    }
    #[test]
    fn cell_from_position_0() {
        assert_eq!(
            Cell::from_position(Vector2::new(0, 0)),
            Cell(Vector2::new(0, 0), Trans::identity())
        );
    }
    #[test]
    fn cell_from_position_1() {
        assert_eq!(
            Cell::from_position(Vector2::new(1, 0)),
            Cell(Vector2::new(0, 0), Trans::identity().rotated(1))
        );
    }
    #[test]
    fn cell_from_position_2() {
        assert_eq!(
            Cell::from_position(Vector2::new(2, 0)),
            Cell(Vector2::new(0, 0), Trans::identity().x_flipped().rotated(3))
        );
    }
    #[test]
    fn cell_from_position_negative_0() {
        assert_eq!(
            Cell::from_position(Vector2::new(0, -2)),
            Cell(Vector2::new(0, -2), Trans::identity())
        );
    }
    #[test]
    fn cell_into_position_0() {
        assert_eq!(
            Cell(Vector2::new(0, 0), Trans::identity().rotated(0)).into_position(),
            Vector2::new(0, 0),
        );
        assert_eq!(
            Cell(Vector2::new(0, 0), Trans::identity().rotated(1)).into_position(),
            Vector2::new(1, 0),
        );
        assert_eq!(
            Cell(Vector2::new(0, 0), Trans::identity().rotated(2)).into_position(),
            Vector2::new(1, 1),
        );
        assert_eq!(
            Cell(Vector2::new(0, 0), Trans::identity().rotated(3)).into_position(),
            Vector2::new(0, 1),
        );
    }
    #[test]
    fn cell_into_position_1() {
        assert_eq!(
            Cell(Vector2::new(0, 0), Trans::identity().x_flipped().rotated(0)).into_position(),
            Vector2::new(3, 0),
        );
        assert_eq!(
            Cell(Vector2::new(0, 0), Trans::identity().x_flipped().rotated(1)).into_position(),
            Vector2::new(3, 1),
        );
        assert_eq!(
            Cell(Vector2::new(0, 0), Trans::identity().x_flipped().rotated(2)).into_position(),
            Vector2::new(2, 1),
        );
        assert_eq!(
            Cell(Vector2::new(0, 0), Trans::identity().x_flipped().rotated(3)).into_position(),
            Vector2::new(2, 0),
        );
    }
    #[test]
    fn cell_into_position_negative_0() {
        assert_eq!(
            Cell(Vector2::new(0, -2), Trans::identity().rotated(0)).into_position(),
            Vector2::new(0, -2),
        );
        assert_eq!(
            Cell(Vector2::new(0, -2), Trans::identity().rotated(1)).into_position(),
            Vector2::new(1, -2),
        );
        assert_eq!(
            Cell(Vector2::new(0, -2), Trans::identity().rotated(2)).into_position(),
            Vector2::new(1, -1),
        );
        assert_eq!(
            Cell(Vector2::new(0, -2), Trans::identity().rotated(3)).into_position(),
            Vector2::new(0, -1),
        );
    }
    #[test]
    fn cell_uniqueness() {
        let mut set = fxhash::FxHashSet::<Vector2<i32>>::default();
        for t in Trans::all() {
            set.insert(Cell(Vector2::new(0, 0), t).into_position());
        }
        assert_eq!(set.len(), 8, "{set:?}");
    }
    #[test]
    fn cell_correctness() {
        for x in 0..4 {
            for y in 0..2 {
                assert_eq!(
                    Cell::from_position(Vector2::new(x, y)).0,
                    Vector2::new(0, 0),
                    "{x}, {y}"
                );
            }
        }
        for x in 4..8 {
            for y in -2..0 {
                assert_eq!(
                    Cell::from_position(Vector2::new(x, y)).0,
                    Vector2::new(4, -2),
                    "{x}, {y}"
                );
            }
        }
    }
    #[test]
    fn cell_from_position_and_back() {
        for x in -3..4 {
            for y in -2..4 {
                let p = Vector2::new(x, y);
                assert_eq!(
                    Cell::from_position(p).into_position(),
                    p,
                    "Cell: {:?}",
                    Cell::from_position(p)
                );
            }
        }
    }
}
