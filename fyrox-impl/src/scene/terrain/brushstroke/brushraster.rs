//! The brushraster module converts floating-point brush positions into rasterized
//! pixels, with each pixel given a strength based on how close it is to the center of the
//! brush and the hardness of the brush. Soft brushes gradually decrease in strength
//! from the center to the edge. Hard brushes have full strength until very close to the edge.
//!
//! This rasterization tool is used by the UI to convert mouse events into pixel messages
//! that get sent to the brush painting thread.
use fyrox_core::math::segment::LineSegment2;

use crate::core::{
    algebra::{Matrix2, Vector2},
    math::{OptionRect, Rect},
};

/// Adjust the strength of a brush pixel based on the hardness of the brush.
fn apply_hardness(hardness: f32, strength: f32) -> f32 {
    if strength == 0.0 {
        return 0.0;
    }
    let h = 1.0 - hardness;
    if strength < h {
        strength / h
    } else {
        1.0
    }
}

/// Trait for objects that are capable of assigning strengths to pixels.
pub trait BrushRaster {
    /// Calculate the strength of a pixel at the given position,
    /// as measured relative to the center of the brush, so that point (0.0, 0.0)
    /// is the exact center of the brush.
    fn strength_at(&self, point: Vector2<f32>) -> f32;
    /// An AABB that contains all the pixels of the brush, with (0.0, 0.0) being
    /// at the center of the brush.
    fn bounds(&self) -> Rect<f32>;
    /// An AABB that contains all the pixels of the brush after it has been transformed
    /// and translated. First the brush is multiplied by `transform` and then it is
    /// translated to `center`, and then an AABB it calculated for the resulting brush.
    fn transformed_bounds(&self, center: Vector2<f32>, transform: &Matrix2<f32>) -> Rect<i32> {
        let mut bounds = OptionRect::<i32>::default();
        let rect = self.bounds();
        for p in [
            rect.left_top_corner(),
            rect.left_bottom_corner(),
            rect.right_top_corner(),
            rect.right_bottom_corner(),
        ] {
            let p1 = transform * p + center;
            let ceil = p1.map(|x| x.ceil() as i32);
            let floor = p1.map(|x| x.floor() as i32);
            let rect = Rect::new(floor.x, floor.y, ceil.x - floor.x, ceil.y - floor.y);
            bounds.extend_to_contain(rect);
        }
        bounds.unwrap()
    }
}

/// Rasterize a round brush with the given radius.
#[derive(Debug, Clone)]
pub struct CircleRaster(pub f32);

impl BrushRaster for CircleRaster {
    fn strength_at(&self, point: Vector2<f32>) -> f32 {
        let radius = self.0;
        let dist_sqr = point.magnitude_squared();
        if dist_sqr >= radius * radius {
            0.0
        } else {
            (1.0 - dist_sqr.sqrt() / radius).max(0.0)
        }
    }
    fn bounds(&self) -> Rect<f32> {
        let radius = self.0;
        Rect::from_points(Vector2::new(-radius, -radius), Vector2::new(radius, radius))
    }
}

/// Rasterize a rectangular brush with the given x-radius and y-radius.
#[derive(Debug, Clone)]
pub struct RectRaster(pub f32, pub f32);

impl BrushRaster for RectRaster {
    fn strength_at(&self, point: Vector2<f32>) -> f32 {
        let radius = Vector2::new(self.0, self.1);
        // Flip p so that it is on the positive side of both axes.
        let p = point.abs();
        let min = radius.min();
        let inner = Vector2::new(radius.x - min, radius.y - min);
        let outer = radius - inner;
        if outer.min() == 0.0 {
            return 0.0;
        }
        let p = (p - inner).sup(&Vector2::new(0.0, 0.0));
        if p.x > outer.x || p.y > outer.y {
            0.0
        } else {
            1.0 - (p.x / outer.x).max(p.y / outer.y)
        }
    }
    fn bounds(&self) -> Rect<f32> {
        let RectRaster(x, y) = self;
        Rect::from_points(Vector2::new(-x, -y), Vector2::new(*x, *y))
    }
}

/// A pixel within a brush shape.
#[derive(Debug, Copy, Clone)]
pub struct BrushPixel {
    /// The position of the pixel
    pub position: Vector2<i32>,
    /// The strength of the brush at this pixel, with 0.0 indicating the pixel is outside the bounds of the brush,
    /// and 1.0 indicating the maximum strength of the brush.
    pub strength: f32,
}

/// An iterator that produces coordinates by scanning an integer Rect.
#[derive(Debug, Clone)]
pub struct RectIter {
    bounds: Rect<i32>,
    next_pos: Vector2<i32>,
}

impl RectIter {
    /// Create an iterator that returns coordinates within the given bounds.
    pub fn new(bounds: Rect<i32>) -> Self {
        Self {
            bounds,
            next_pos: Vector2::default(),
        }
    }
    /// The Rect that this iter is scanning.
    pub fn bounds(&self) -> Rect<i32> {
        self.bounds
    }
}

impl Iterator for RectIter {
    type Item = Vector2<i32>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next_pos.y > self.bounds.size.y {
            return None;
        }
        let result = self.next_pos + self.bounds.position;
        if self.next_pos.x < self.bounds.size.x {
            self.next_pos.x += 1;
        } else {
            self.next_pos.y += 1;
            self.next_pos.x = 0;
        }
        Some(result)
    }
}

/// An iterator over the pixels of a [BrushRaster] object.
/// For each pixel, it produces a [BrushPixel].
/// The pixels produced can include pixels with zero strength.
#[derive(Debug, Clone)]
pub struct StampPixels<R> {
    brush_raster: R,
    center: Vector2<f32>,
    hardness: f32,
    inv_transform: Matrix2<f32>,
    bounds_iter: RectIter,
}

impl<R> StampPixels<R>
where
    R: BrushRaster,
{
    /// An AABB containing all the pixels that this iterator produces.
    pub fn bounds(&self) -> Rect<i32> {
        self.bounds_iter.bounds()
    }
    /// Construct a new pixel iterator for a stamp at the given location.
    pub fn new(
        brush_raster: R,
        center: Vector2<f32>,
        hardness: f32,
        transform: Matrix2<f32>,
    ) -> Self {
        let (transform, inv_transform) = transform
            .try_inverse()
            .map(|m| (transform, m))
            .unwrap_or((Matrix2::identity(), Matrix2::identity()));
        let bounds = brush_raster.transformed_bounds(center, &transform);
        Self {
            brush_raster,
            center,
            hardness,
            inv_transform,
            bounds_iter: RectIter::new(bounds),
        }
    }
}

impl<R> Iterator for StampPixels<R>
where
    R: BrushRaster,
{
    type Item = BrushPixel;

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.bounds_iter.next()?;
        let fx = position.x as f32;
        let fy = position.y as f32;
        let p = Vector2::new(fx, fy) - self.center;
        let p = self.inv_transform * p;
        let strength = self.brush_raster.strength_at(p);
        let strength = apply_hardness(self.hardness, strength);
        Some(BrushPixel { position, strength })
    }
}

/// An iterator of the pixels that are painted when a brush
/// is smeared from a start point to an end point.
/// It works just like [StampPixels] but across a line segment instead of at a single point.
#[derive(Debug, Clone)]
pub struct SmearPixels<R> {
    brush_raster: R,
    segment: LineSegment2<f32>,
    aspect_segment: LineSegment2<f32>,
    hardness: f32,
    inv_transform: Matrix2<f32>,
    aspect_transform: Matrix2<f32>,
    bounds_iter: RectIter,
}

impl<R> SmearPixels<R> {
    /// The bounding rectangle of the pixels.
    pub fn bounds(&self) -> Rect<i32> {
        self.bounds_iter.bounds()
    }
    /// Construct a new pixel iterator for a smear with the given start and end points.
    pub fn new(
        brush_raster: R,
        start: Vector2<f32>,
        end: Vector2<f32>,
        hardness: f32,
        transform: Matrix2<f32>,
    ) -> Self
    where
        R: BrushRaster,
    {
        let mut bounds: OptionRect<i32> = Default::default();
        let (transform, inv_transform) = transform
            .try_inverse()
            .map(|m| (transform, m))
            .unwrap_or((Matrix2::identity(), Matrix2::identity()));
        bounds.extend_to_contain(brush_raster.transformed_bounds(start, &transform));
        bounds.extend_to_contain(brush_raster.transformed_bounds(end, &transform));
        let segment = LineSegment2::new(&start, &end);
        let aspect_bounds = brush_raster.bounds();
        let aspect_transform =
            Matrix2::new(1.0 / aspect_bounds.w(), 0.0, 0.0, 1.0 / aspect_bounds.h());
        let aspect_transform = aspect_transform * inv_transform;
        let aspect_segment = LineSegment2 {
            start: aspect_transform * segment.start,
            end: aspect_transform * segment.end,
        };
        Self {
            brush_raster,
            segment,
            aspect_segment,
            hardness,
            inv_transform,
            aspect_transform,
            bounds_iter: RectIter::new(bounds.unwrap()),
        }
    }
}

impl<R> Iterator for SmearPixels<R>
where
    R: BrushRaster,
{
    type Item = BrushPixel;

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.bounds_iter.next()?;
        let fx = position.x as f32;
        let fy = position.y as f32;
        let p = Vector2::new(fx, fy);
        let aspect_p = self.aspect_transform * p;
        let t = self.aspect_segment.nearest_t(&aspect_p);
        let center = self.segment.interpolate_clamped(t);
        let p = p - center;
        let p = self.inv_transform * p;
        let s = self.brush_raster.strength_at(p);
        let strength = apply_hardness(self.hardness, s);
        Some(BrushPixel { position, strength })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rect_extend_to_contain_f64() {
        let mut rect = Rect::new(0.0, 0.0, 1.0, 1.0);
        rect.extend_to_contain(Rect::new(-1.0, -1.0, 1.0, 1.0));
        assert_eq!(rect, Rect::new(-1.0, -1.0, 2.0, 2.0));
    }
    #[test]
    fn rect_extend_to_contain_i32() {
        let mut rect: Rect<i32> = Rect::new(0, 0, 1, 1);

        rect.extend_to_contain(Rect::new(1, 1, 1, 1));
        assert_eq!(rect, Rect::<i32>::new(0, 0, 2, 2));

        rect.extend_to_contain(Rect::new(-1, -1, 1, 1));
        assert_eq!(rect, Rect::<i32>::new(-1, -1, 3, 3));
    }
    #[test]
    fn bounds_iter() {
        let result: Vec<Vector2<i32>> = RectIter::new(Rect::new(-1, -2, 3, 2)).collect();
        let expected: Vec<Vector2<i32>> = vec![
            (-1, -2),
            (0, -2),
            (1, -2),
            (2, -2),
            (-1, -1),
            (0, -1),
            (1, -1),
            (2, -1),
            (-1, 0),
            (0, 0),
            (1, 0),
            (2, 0),
        ]
        .into_iter()
        .map(|(x, y)| Vector2::new(x, y))
        .collect();
        assert_eq!(result, expected);
    }
    #[test]
    fn finite_pixels_circle() {
        let mut iter = StampPixels::new(
            CircleRaster(2.0),
            Vector2::default(),
            1.0,
            Matrix2::identity(),
        );
        for _ in 0..100 {
            if iter.next().is_none() {
                return;
            }
        }
        panic!("Iter went over 100.");
    }
    #[test]
    fn finite_pixels_rect() {
        let mut iter = StampPixels::new(
            RectRaster(2.0, 2.0),
            Vector2::default(),
            1.0,
            Matrix2::identity(),
        );
        for _ in 0..100 {
            if iter.next().is_none() {
                return;
            }
        }
        panic!("Iter went over 100.");
    }
    #[test]
    fn pixels_range_circle() {
        let mut iter = StampPixels::new(
            CircleRaster(2.0),
            Vector2::default(),
            1.0,
            Matrix2::identity(),
        );
        let mut rect = Rect::new(0, 0, 0, 0);
        let mut points: Vec<Vector2<i32>> = Vec::default();
        for _ in 0..100 {
            if let Some(BrushPixel { position, .. }) = iter.next() {
                points.push(Vector2::new(position.x, position.y));
                rect.extend_to_contain(Rect::new(position.x, position.y, 0, 0));
            } else {
                break;
            }
        }
        assert_eq!(
            rect,
            Rect::new(-2, -2, 4, 4),
            "{:?} {:?}",
            iter.bounds(),
            points
        );
    }
    #[test]
    fn pixels_range_rect() {
        let mut iter = StampPixels::new(
            RectRaster(2.0, 2.0),
            Vector2::default(),
            1.0,
            Matrix2::identity(),
        );
        let mut rect = Rect::new(0, 0, 0, 0);
        let mut points: Vec<Vector2<i32>> = Vec::default();
        for _ in 0..100 {
            if let Some(BrushPixel { position, .. }) = iter.next() {
                points.push(Vector2::new(position.x, position.y));
                rect.extend_to_contain(Rect::new(position.x, position.y, 0, 0));
            } else {
                break;
            }
        }
        assert_eq!(
            rect,
            Rect::new(-2, -2, 4, 4),
            "{:?} {:?}",
            iter.bounds(),
            points
        );
    }
    fn find_strength_at(iter: &StampPixels<RectRaster>, (x, y): (i32, i32)) -> f32 {
        let p = Vector2::new(x, y);
        let pix = iter.clone().find(|x| x.position == p);
        pix.map(|p| p.strength).unwrap_or(0.0)
    }
    #[test]
    fn simple_rect() {
        let iter = StampPixels::new(
            RectRaster(1.1, 1.1),
            Vector2::new(1.0, 1.0),
            1.0,
            Matrix2::identity(),
        );
        assert_eq!(iter.bounds().w(), 4, "w != 4: {:?}", iter.bounds());
        assert_eq!(iter.bounds().h(), 4, "h != 4: {:?}", iter.bounds());
        assert_eq!(find_strength_at(&iter, (1, 1)), 1.0);
        assert_eq!(find_strength_at(&iter, (0, 1)), 1.0);
        assert_eq!(find_strength_at(&iter, (0, 2)), 1.0);
        assert_eq!(find_strength_at(&iter, (0, 0)), 1.0);
        assert_eq!(find_strength_at(&iter, (-1, 1)), 0.0);
    }
    #[test]
    fn distant_rect() {
        let iter = StampPixels::new(
            RectRaster(1.1, 1.1),
            Vector2::new(1001.0, 2501.0),
            1.0,
            Matrix2::identity(),
        );
        assert_eq!(iter.bounds().w(), 4, "w != 4: {:?}", iter.bounds());
        assert_eq!(iter.bounds().h(), 4, "h != 4: {:?}", iter.bounds());
        assert_eq!(find_strength_at(&iter, (1001, 2501)), 1.0);
        assert_eq!(find_strength_at(&iter, (1000, 2501)), 1.0);
        assert_eq!(find_strength_at(&iter, (1000, 2502)), 1.0);
        assert_eq!(find_strength_at(&iter, (1000, 2500)), 1.0);
        assert_eq!(find_strength_at(&iter, (999, 2501)), 0.0);
    }
}
