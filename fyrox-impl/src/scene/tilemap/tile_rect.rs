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

//! A structure to represent rectanglular regions within a tile set page or
//! a tile map. See [`TileRect`] for more information.
//! [`OptionTileRect`] is for regions that may be empty.

use std::ops::{Deref, DerefMut};

use crate::core::algebra::Vector2;

/// This is a variation of `Rect` that is specifically designed for use with
/// TileMap. While the y-axis of `Rect` points downward to match the y-axis
/// of UI, the y-axis of `TileRect` points upward to match the y-axis of
/// TileMap, TileSet, and TileMapBrush.
///
/// Unlike `Rect`, `TileRect` is designed to contain a WxH region of tiles,
/// which means that `position + size` is *not* considered to be a point
/// contained in the rect. A point is only contained in the rect if its
/// entire 1x1 area is contained in the rect, so the left-top corner
/// of the rect is `position + size - (1,1)`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TileRect {
    /// The left-bottom corner of the rect.
    pub position: Vector2<i32>,
    /// The width and height of the rect.
    pub size: Vector2<i32>,
}

/// An option version of `TileRect` that contains nothing.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct OptionTileRect(Option<TileRect>);

#[derive(Debug, Clone)]
/// Iterator for the cells contained in a TileRect.
pub struct RectIter(Vector2<i32>, TileRect);
/// Iterator for the cells contained in an OptionTileRect.
#[derive(Debug, Clone)]
pub struct OptionRectIter(Option<RectIter>);

impl Iterator for RectIter {
    type Item = Vector2<i32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.y >= self.1.h() {
            return None;
        }
        let result = self.0 + self.1.position;
        self.0.x += 1;
        if self.0.x >= self.1.w() {
            self.0.x = 0;
            self.0.y += 1;
        }
        Some(result)
    }
}

impl Iterator for OptionRectIter {
    type Item = Vector2<i32>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.as_mut()?.next()
    }
}

impl From<RectIter> for OptionRectIter {
    fn from(value: RectIter) -> Self {
        Self(Some(value))
    }
}

impl From<TileRect> for OptionTileRect {
    fn from(source: TileRect) -> Self {
        Self(Some(source))
    }
}
impl From<Option<TileRect>> for OptionTileRect {
    fn from(source: Option<TileRect>) -> Self {
        Self(source)
    }
}
impl Deref for OptionTileRect {
    type Target = Option<TileRect>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OptionTileRect {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for OptionTileRect {
    type Item = Vector2<i32>;

    type IntoIter = OptionRectIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl OptionTileRect {
    /// Create a new rectangle from two diagonally opposite corner points.
    /// In other words, create the smallest rectangle containing both given points.
    pub fn from_points(p0: Vector2<i32>, p1: Vector2<i32>) -> Self {
        TileRect::from_points(p0, p1).into()
    }

    /// Iterate over the cells contained within this rect.
    pub fn iter(&self) -> OptionRectIter {
        OptionRectIter(self.0.map(|x| x.iter()))
    }

    /// Deflates the rectangle by the given amounts. It offsets the rectangle by `(dw, dh)` and
    /// decreases its size by `(2 * dw, 2 * dh)`.
    #[inline]
    #[must_use = "this method creates new instance of OptionTileRect"]
    pub fn deflate(&self, dw: i32, dh: i32) -> Self {
        if let Some(rect) = &self.0 {
            rect.deflate(dw, dh)
        } else {
            Self(None)
        }
    }

    /// Clip the rectangle to the given bounds.
    #[inline]
    pub fn clip(&mut self, bounds: TileRect) {
        if let Some(rect) = &self.0 {
            *self = rect.clip_by(bounds);
        }
    }
    /// Extends the rectangle so it will contain the given point.
    #[inline]
    pub fn push(&mut self, p: Vector2<i32>) {
        if let Some(rect) = &mut self.0 {
            rect.push(p);
        } else {
            self.0 = Some(TileRect::new(p.x, p.y, 1, 1));
        }
    }
    /// Checks if the given point lies within the bounds of the rectangle.
    #[inline]
    pub fn contains(&self, p: Vector2<i32>) -> bool {
        if let Some(rect) = &self.0 {
            rect.contains(p)
        } else {
            false
        }
    }
    /// Checks if the rectangle intersects with some other rectangle.
    #[inline]
    pub fn intersects(&self, other: TileRect) -> bool {
        if let Some(rect) = &self.0 {
            rect.intersects(other)
        } else {
            false
        }
    }

    /// Extends the rectangle so it will contain the other rectangle.
    #[inline]
    pub fn extend_to_contain(&mut self, other: TileRect) {
        if let Some(rect) = &mut self.0 {
            rect.extend_to_contain(other);
        } else {
            self.0 = Some(other);
        }
    }
    /// Returns width of the rectangle.
    #[inline(always)]
    pub fn w(&self) -> i32 {
        if let Some(rect) = &self.0 {
            rect.size.x
        } else {
            0
        }
    }

    /// Returns height of the rectangle.
    #[inline(always)]
    pub fn h(&self) -> i32 {
        if let Some(rect) = &self.0 {
            rect.size.y
        } else {
            0
        }
    }
}

impl IntoIterator for TileRect {
    type Item = Vector2<i32>;

    type IntoIter = RectIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl TileRect {
    /// Creates a new rectangle from X, Y, width, height.
    #[inline]
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            position: Vector2::new(x, y),
            size: Vector2::new(w, h),
        }
    }

    /// Iterate over the cells contained within this rect.
    #[inline]
    pub fn iter(&self) -> RectIter {
        RectIter(Vector2::new(0, 0), *self)
    }

    /// Create a new rectangle from two diagonally opposite corner points.
    /// In other words, create the smallest rectangle containing both given points.
    pub fn from_points(p0: Vector2<i32>, p1: Vector2<i32>) -> Self {
        let inf = p0.inf(&p1);
        let sup = p0.sup(&p1);
        Self {
            position: inf,
            size: sup - inf + Vector2::new(1, 1),
        }
    }

    /// Sets the new position of the rectangle.
    #[inline]
    pub fn with_position(mut self, position: Vector2<i32>) -> Self {
        self.position = position;
        self
    }

    /// Sets the new size of the rectangle.
    #[inline]
    pub fn with_size(mut self, size: Vector2<i32>) -> Self {
        self.size = size;
        self
    }

    /// Inflates the rectangle by the given amounts. It offsets the rectangle by `(-dw, -dh)` and
    /// increases its size by `(2 * dw, 2 * dh)`.
    #[inline]
    #[must_use = "this method creates new instance of TileRect"]
    pub fn inflate(&self, dw: i32, dh: i32) -> Self {
        Self {
            position: Vector2::new(self.position.x - dw, self.position.y - dh),
            size: Vector2::new(self.size.x + dw + dw, self.size.y + dh + dh),
        }
    }

    /// Deflates the rectangle by the given amounts. It offsets the rectangle by `(dw, dh)` and
    /// decreases its size by `(2 * dw, 2 * dh)`.
    #[inline]
    #[must_use = "this method creates new instance of OptionTileRect"]
    pub fn deflate(&self, dw: i32, dh: i32) -> OptionTileRect {
        if self.size.x > dw + dw && self.size.y > dh + dh {
            OptionTileRect(Some(TileRect {
                position: Vector2::new(self.position.x + dw, self.position.y + dh),
                size: Vector2::new(self.size.x - (dw + dw), self.size.y - (dh + dh)),
            }))
        } else {
            OptionTileRect(None)
        }
    }

    /// Checks if the given point lies within the bounds of the rectangle.
    #[inline]
    pub fn contains(&self, pt: Vector2<i32>) -> bool {
        pt.x >= self.position.x
            && pt.x < self.position.x + self.size.x
            && pt.y >= self.position.y
            && pt.y < self.position.y + self.size.y
    }

    /// Returns center point of the rectangle.
    #[inline]
    pub fn center(&self) -> Vector2<i32> {
        self.position + Vector2::new(self.size.x / 2, self.size.y / 2)
    }

    /// Extends the rectangle to contain the given point.
    #[inline]
    pub fn push(&mut self, p: Vector2<i32>) {
        let p0 = self.left_bottom_corner();
        let p1 = self.right_top_corner();
        *self = Self::from_points(p.inf(&p0), p.sup(&p1));
    }

    /// Clips the rectangle by some other rectangle and returns a new rectangle that corresponds to
    /// the intersection of both rectangles. If the rectangles does not intersects, the method
    /// returns none.
    #[inline]
    #[must_use = "this method creates new instance of OptionTileRect"]
    pub fn clip_by(&self, other: TileRect) -> OptionTileRect {
        let mut clipped = *self;

        if other.x() + other.w() <= self.x()
            || other.x() >= self.x() + self.w()
            || other.y() + other.h() <= self.y()
            || other.y() >= self.y() + self.h()
        {
            return OptionTileRect::default();
        }

        if clipped.position.x < other.position.x {
            clipped.size.x -= other.position.x - clipped.position.x;
            clipped.position.x = other.position.x;
        }

        if clipped.position.y < other.position.y {
            clipped.size.y -= other.position.y - clipped.position.y;
            clipped.position.y = other.position.y;
        }

        let clipped_right_top = clipped.right_top_corner();
        let other_right_top = other.right_top_corner();

        if clipped_right_top.x > other_right_top.x {
            clipped.size.x -= clipped_right_top.x - other_right_top.x;
        }
        if clipped_right_top.y > other_right_top.y {
            clipped.size.y -= clipped_right_top.y - other_right_top.y;
        }

        clipped.into()
    }

    /// Checks if the rectangle intersects with some other rectangle.
    #[inline]
    pub fn intersects(&self, other: TileRect) -> bool {
        if other.position.x < self.position.x + self.size.x
            && self.position.x < other.position.x + other.size.x
            && other.position.y < self.position.y + self.size.y
        {
            self.position.y < other.position.y + other.size.y
        } else {
            false
        }
    }

    /// Offsets the given rectangle and returns a new rectangle.
    #[inline]
    #[must_use = "this method creates new instance of TileRect"]
    pub fn translate(&self, translation: Vector2<i32>) -> Self {
        Self {
            position: Vector2::new(
                self.position.x + translation.x,
                self.position.y + translation.y,
            ),
            size: self.size,
        }
    }

    /// Extends the rectangle so it will contain the other rectangle.
    #[inline]
    pub fn extend_to_contain(&mut self, other: TileRect) {
        let p0 = self.left_bottom_corner();
        let p1 = self.right_top_corner();
        let o0 = other.left_bottom_corner();
        let o1 = other.right_top_corner();
        *self = Self::from_points(p0.inf(&o0), p1.sup(&o1));
    }

    /// Returns the top left corner of the rectangle.
    #[inline(always)]
    pub fn left_top_corner(&self) -> Vector2<i32> {
        Vector2::new(self.position.x, self.position.y + self.size.y - 1)
    }

    /// Returns the top right corner of the rectangle.
    #[inline(always)]
    pub fn right_top_corner(&self) -> Vector2<i32> {
        Vector2::new(
            self.position.x + self.size.x - 1,
            self.position.y + self.size.y - 1,
        )
    }

    /// Returns the bottom right corner of the rectangle.
    #[inline(always)]
    pub fn right_bottom_corner(&self) -> Vector2<i32> {
        Vector2::new(self.position.x + self.size.x - 1, self.position.y)
    }

    /// Returns the bottom left corner of the rectangle.
    #[inline(always)]
    pub fn left_bottom_corner(&self) -> Vector2<i32> {
        self.position
    }

    /// Returns width of the rectangle.
    #[inline(always)]
    pub fn w(&self) -> i32 {
        self.size.x
    }

    /// Returns height of the rectangle.
    #[inline(always)]
    pub fn h(&self) -> i32 {
        self.size.y
    }

    /// Returns horizontal position of the rectangle.
    #[inline(always)]
    pub fn x(&self) -> i32 {
        self.position.x
    }

    /// Returns vertical position of the rectangle.
    #[inline(always)]
    pub fn y(&self) -> i32 {
        self.position.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn iter() {
        let mut iter = TileRect::new(2, 3, 3, 2).iter();
        assert_eq!(iter.next(), Some(Vector2::new(2, 3)));
        assert_eq!(iter.next(), Some(Vector2::new(3, 3)));
        assert_eq!(iter.next(), Some(Vector2::new(4, 3)));
        assert_eq!(iter.next(), Some(Vector2::new(2, 4)));
        assert_eq!(iter.next(), Some(Vector2::new(3, 4)));
        assert_eq!(iter.next(), Some(Vector2::new(4, 4)));
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn intersects1() {
        let rect1 = TileRect::new(-1, -2, 4, 6);
        let rect2 = TileRect::new(2, 3, 2, 2);
        assert!(rect1.intersects(rect2));
    }
    #[test]
    fn intersects2() {
        let rect1 = TileRect::new(0, 0, 4, 6);
        let rect2 = TileRect::new(-1, -2, 2, 3);
        assert!(rect1.intersects(rect2));
    }
    #[test]
    fn not_intersects1() {
        let rect1 = TileRect::new(-1, -2, 3, 4);
        let rect2 = TileRect::new(3, 3, 2, 2);
        assert!(!rect1.intersects(rect2));
    }
    #[test]
    fn not_intersects2() {
        let rect1 = TileRect::new(-1, -2, 3, 4);
        let rect2 = TileRect::new(2, 1, 2, 2);
        assert!(!rect1.intersects(rect2));
    }
    #[test]
    fn from_points1() {
        let rect = TileRect::from_points(Vector2::new(-1, -2), Vector2::new(2, 1));
        assert_eq!(rect, TileRect::new(-1, -2, 4, 4));
    }
    #[test]
    fn from_points2() {
        let rect = TileRect::from_points(Vector2::new(-1, 1), Vector2::new(2, -2));
        assert_eq!(rect, TileRect::new(-1, -2, 4, 4));
    }
    #[test]
    fn rect_extend_to_contain() {
        let mut rect = TileRect::new(0, 0, 1, 1);

        rect.extend_to_contain(TileRect::new(1, 1, 1, 1));
        assert_eq!(rect, TileRect::new(0, 0, 2, 2));

        rect.extend_to_contain(TileRect::new(-1, -1, 1, 1));
        assert_eq!(rect, TileRect::new(-1, -1, 3, 3));

        rect.extend_to_contain(TileRect::new(10, -1, 1, 15));
        assert_eq!(rect, TileRect::new(-1, -1, 12, 15));
    }
    #[test]
    fn rect_push2() {
        let mut rect = TileRect::new(0, 0, 1, 1);

        rect.push(Vector2::new(1, 1));
        assert_eq!(rect, TileRect::new(0, 0, 2, 2));

        rect.push(Vector2::new(-1, -1));
        assert_eq!(rect, TileRect::new(-1, -1, 3, 3));

        rect.push(Vector2::new(10, -1));
        assert_eq!(rect, TileRect::new(-1, -1, 12, 3));
    }
    #[test]
    fn option_rect_extend_to_contain() {
        let mut rect = OptionTileRect::default();

        rect.extend_to_contain(TileRect::new(1, 1, 1, 1));
        assert_eq!(rect.unwrap(), TileRect::new(1, 1, 1, 1));

        rect.extend_to_contain(TileRect::new(-1, -1, 1, 1));
        assert_eq!(rect.unwrap(), TileRect::new(-1, -1, 3, 3));

        rect.extend_to_contain(TileRect::new(10, -1, 1, 15));
        assert_eq!(rect.unwrap(), TileRect::new(-1, -1, 12, 15));
    }
    #[test]
    fn option_rect_push() {
        let mut rect = OptionTileRect::default();

        rect.push(Vector2::new(1, 1));
        assert_eq!(rect.unwrap(), TileRect::new(1, 1, 1, 1));

        rect.push(Vector2::new(-1, -1));
        assert_eq!(rect.unwrap(), TileRect::new(-1, -1, 3, 3));

        rect.push(Vector2::new(10, -1));
        assert_eq!(rect.unwrap(), TileRect::new(-1, -1, 12, 3));
    }
    #[test]
    fn option_rect_clip() {
        let rect = OptionTileRect::from(TileRect::new(0, 0, 10, 10));

        let mut r = rect;
        r.clip(TileRect::new(2, 2, 1, 1));
        assert_eq!(r.unwrap(), TileRect::new(2, 2, 1, 1));

        let mut r = rect;
        r.clip(TileRect::new(0, 0, 15, 15));
        assert_eq!(r.unwrap(), TileRect::new(0, 0, 10, 10));

        // When there is no intersection.
        let mut r = OptionTileRect::default();
        r.clip(TileRect::new(0, 0, 10, 10));
        assert!(r.is_none());
        let mut r = rect;
        r.clip(TileRect::new(-2, 1, 1, 1));
        assert!(r.is_none());
        let mut r = rect;
        r.clip(TileRect::new(11, 1, 1, 1));
        assert!(r.is_none());
        let mut r = rect;
        r.clip(TileRect::new(1, -2, 1, 1));
        assert!(r.is_none());
        let mut r = rect;
        r.clip(TileRect::new(1, 11, 1, 1));
        assert!(r.is_none());
    }

    #[test]
    fn rect_with_position() {
        let rect = TileRect::new(0, 0, 1, 1);

        assert_eq!(
            rect.with_position(Vector2::new(1, 1)),
            TileRect::new(1, 1, 1, 1)
        );
    }

    #[test]
    fn rect_with_size() {
        let rect = TileRect::new(0, 0, 1, 1);

        assert_eq!(
            rect.with_size(Vector2::new(10, 10)),
            TileRect::new(0, 0, 10, 10)
        );
    }

    #[test]
    fn rect_inflate() {
        let rect = TileRect::new(0, 0, 1, 1);

        assert_eq!(rect.inflate(5, 5), TileRect::new(-5, -5, 11, 11));
    }

    #[test]
    fn rect_deflate() {
        let rect = TileRect::new(-5, -5, 11, 11);

        assert_eq!(rect.deflate(5, 5), TileRect::new(0, 0, 1, 1).into());
        assert_eq!(rect.deflate(6, 5), OptionTileRect::default());
    }

    #[test]
    fn rect_contains() {
        let rect = TileRect::new(0, 0, 10, 10);

        assert!(rect.contains(Vector2::new(0, 0)));
        assert!(rect.contains(Vector2::new(0, 9)));
        assert!(rect.contains(Vector2::new(9, 0)));
        assert!(rect.contains(Vector2::new(9, 9)));
        assert!(rect.contains(Vector2::new(5, 5)));

        assert!(!rect.contains(Vector2::new(0, 10)));
    }

    #[test]
    fn rect_center() {
        let rect = TileRect::new(0, 0, 10, 10);

        assert_eq!(rect.center(), Vector2::new(5, 5));
    }

    #[test]
    fn rect_push() {
        let mut rect = TileRect::new(10, 10, 10, 10);

        rect.push(Vector2::new(0, 0));
        assert_eq!(rect, TileRect::new(0, 0, 20, 20));

        rect.push(Vector2::new(0, 20));
        assert_eq!(rect, TileRect::new(0, 0, 20, 21));

        rect.push(Vector2::new(20, 20));
        assert_eq!(rect, TileRect::new(0, 0, 21, 21));

        rect.push(Vector2::new(30, 30));
        assert_eq!(rect, TileRect::new(0, 0, 31, 31));
    }

    #[test]
    fn rect_getters() {
        let rect = TileRect::new(0, 0, 2, 2);

        assert_eq!(rect.left_bottom_corner(), Vector2::new(0, 0));
        assert_eq!(rect.left_top_corner(), Vector2::new(0, 1));
        assert_eq!(rect.right_bottom_corner(), Vector2::new(1, 0));
        assert_eq!(rect.right_top_corner(), Vector2::new(1, 1));

        assert_eq!(rect.x(), 0);
        assert_eq!(rect.y(), 0);
        assert_eq!(rect.w(), 2);
        assert_eq!(rect.h(), 2);
    }

    #[test]
    fn rect_clip_by() {
        let rect = TileRect::new(0, 0, 10, 10);

        assert_eq!(
            rect.clip_by(TileRect::new(2, 2, 1, 1)).unwrap(),
            TileRect::new(2, 2, 1, 1)
        );
        assert_eq!(
            rect.clip_by(TileRect::new(0, 0, 15, 15)).unwrap(),
            TileRect::new(0, 0, 10, 10)
        );

        // When there is no intersection.
        assert!(rect.clip_by(TileRect::new(-2, 1, 1, 1)).is_none());
        assert!(rect.clip_by(TileRect::new(11, 1, 1, 1)).is_none());
        assert!(rect.clip_by(TileRect::new(1, -2, 1, 1)).is_none());
        assert!(rect.clip_by(TileRect::new(1, 11, 1, 1)).is_none());
    }

    #[test]
    fn rect_translate() {
        let rect = TileRect::new(0, 0, 10, 10);

        assert_eq!(
            rect.translate(Vector2::new(5, 5)),
            TileRect::new(5, 5, 10, 10)
        );
    }
}
