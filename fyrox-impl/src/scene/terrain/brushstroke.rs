use crate::core::{
    algebra::{Matrix2, Vector2},
    math::{OptionRect, Rect},
    reflect::prelude::*,
};
use crate::fxhash::FxHashMap;
use fyrox_core::uuid_provider;

/// A value that is stored in a terrain and can be edited by a brush.
pub trait BrushValue {
    /// Increase the value by the given amount, or decrease it if the amount is negative.
    fn raise(self, amount: f32) -> Self;
    /// Create a value based upon a float representation.
    fn from_f32(value: f32) -> Self;
    /// Create an f32 representation of this value.
    fn into_f32(self) -> f32;
}

impl BrushValue for f32 {
    #[inline]
    fn raise(self, amount: f32) -> Self {
        self + amount
    }
    #[inline]
    fn from_f32(value: f32) -> Self {
        value
    }
    #[inline]
    fn into_f32(self) -> f32 {
        self
    }
}

impl BrushValue for u8 {
    #[inline]
    fn raise(self, amount: f32) -> Self {
        (self as f32 + amount * 255.0).clamp(0.0, 255.0) as Self
    }
    #[inline]
    fn from_f32(value: f32) -> Self {
        (value * 255.0).clamp(0.0, 255.0) as Self
    }
    #[inline]
    fn into_f32(self) -> f32 {
        self as f32 / 255.0
    }
}

/// Trait for any of the various data properties that may be edited
/// by a brush. V is the type of the elements of the data,
/// such as f32 for the height data and u8 for the mask data.
///
/// This trait encapsulates and hides the concept of chunks.
/// It pretends that terrain data is a continuous infinite array, and
/// accessing any data in that array requires only a Vector2<i32> index.
/// This simplifies brush algorithms.
pub trait BrushableTerrainData<V> {
    /// Returns the value at the given coordinates as it was *before* the current brushstroke.
    /// Previous calls to [BrushableTerrainData::update] will not affect the value returned until the current
    /// stroke ends and the changes to the terrain are completed.
    fn get_value(&self, position: Vector2<i32>) -> V;
    /// Updates the value of the terrain according to the given function
    /// if the given strength is greater than the current brush strength at
    /// the given coordinates. func is not called otherwise.
    ///
    /// If the value is updated, then the brush strength is increased to match
    /// the given strength at those coordinates so that the same position will
    /// not be updated again unless by a brush of greater strength than this one.
    ///
    /// If strength is 0.0 or less, nothing is done, since 0.0 is the minimum valid brush strength.
    ///
    /// The value passed to func is the same value that would be returned by [BrushableTerrainData::get_value],
    /// which is the value at that position *before* the current stroke began.
    /// Even if update is called multiple times on a single position, the value passed to func will be
    /// the same each time until the current stroke is completed.
    fn update<F>(&mut self, position: Vector2<i32>, strength: f32, func: F)
    where
        F: FnOnce(&Self, V) -> V;
    /// Calculate a value for a kernel of the given radius around the given position
    /// by summing the values of the neighborhood surrounding the position.
    fn sum_kernel(&self, position: Vector2<i32>, kernel_radius: u32) -> f32;
}

#[derive(Debug, Default)]
/// Data for an in-progress terrain painting operation
pub struct BrushStroke {
    /// The height pixels that have been drawn to
    pub height_pixels: StrokeData<f32>,
    /// The mask pixels that have been drawn to
    pub mask_pixels: StrokeData<u8>,
    /// A value that may change over the course of a stroke.
    pub value: f32,
}

/// The pixels for a stroke, generalized over the type of data being edited.
#[derive(Debug, Default)]
pub struct StrokeData<V>(FxHashMap<Vector2<i32>, StrokeElement<V>>);

/// A single pixel data of a brush stroke
#[derive(Debug, Copy, Clone)]
pub struct StrokeElement<V> {
    /// The intensity of the brush stroke, with 0.0 indicating a pixel that brush has not touched
    /// and 1.0 indicates a pixel fully covered by the brush.
    pub strength: f32,
    /// The value of the pixel before the stroke began.
    pub original_value: V,
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

impl BrushStroke {
    /// Prepare this object for a new brushstroke.
    pub fn clear(&mut self) {
        self.height_pixels.clear();
        self.mask_pixels.clear();
    }
}

impl<V> StrokeData<V> {
    /// Reset the brush stroke so it is ready to begin a new stroke.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear()
    }
    /// Return the StrokeElement stored at the give position, if there is one.
    #[inline]
    pub fn get(&self, position: Vector2<i32>) -> Option<&StrokeElement<V>> {
        self.0.get(&position)
    }
    /// Stores or modifies the StrokeElement at the given position.
    /// If the element is updated, return the original pixel value of the element.
    /// - `position`: The position of the data to modify within the terrain.
    /// - `strength`: The strength of the brush at the position, from 0.0 to 1.0.
    /// The element is updated if the stored strength is less than the given strength.
    /// If there is no stored strength, that is treated as a strength of 0.0.
    /// - `pixel_value`: The current value of the data.
    /// This may be stored in the StrokeData if no pixel value is currently recorded for the given position.
    /// Otherwise, this value is ignored.
    #[inline]
    pub fn update_pixel(
        &mut self,
        position: Vector2<i32>,
        strength: f32,
        pixel_value: V,
    ) -> Option<V>
    where
        V: Clone,
    {
        if strength == 0.0 {
            None
        } else if let Some(element) = self.0.get_mut(&position) {
            if element.strength < strength {
                element.strength = strength;
                Some(element.original_value.clone())
            } else {
                None
            }
        } else {
            let element = StrokeElement {
                strength,
                original_value: pixel_value.clone(),
            };
            self.0.insert(position, element);
            Some(pixel_value)
        }
    }
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

/// An iterator of the pixels of a round brush.
#[derive(Debug, Clone)]
pub struct CircleBrushPixels {
    center: Vector2<f32>,
    radius: f32,
    hardness: f32,
    inv_transform: Matrix2<f32>,
    bounds_iter: RectIter,
}

impl CircleBrushPixels {
    /// The bounding rectangle of the pixels.
    pub fn bounds(&self) -> Rect<i32> {
        self.bounds_iter.bounds()
    }
}

impl CircleBrushPixels {
    /// Construct a new pixel iterator for a round brush at the given position, radius,
    /// and 2x2 transform matrix.
    pub fn new(center: Vector2<f32>, radius: f32, hardness: f32, transform: Matrix2<f32>) -> Self {
        let mut bounds: OptionRect<i32> = Default::default();
        let transform = if transform.is_invertible() {
            transform
        } else {
            Matrix2::identity()
        };
        let inv_transform = transform.try_inverse().unwrap();
        for p in [
            Vector2::new(radius, radius),
            Vector2::new(radius, -radius),
            Vector2::new(-radius, radius),
            Vector2::new(-radius, -radius),
        ] {
            let p1 = transform * p + center;
            let ceil = p1.map(|x| x.ceil() as i32);
            let floor = p1.map(|x| x.floor() as i32);
            let rect = Rect::new(floor.x, floor.y, ceil.x - floor.x, ceil.y - floor.y);
            bounds.extend_to_contain(rect);
        }
        Self {
            center,
            radius,
            hardness,
            inv_transform,
            bounds_iter: RectIter::new(bounds.unwrap()),
        }
    }
}

impl Iterator for CircleBrushPixels {
    type Item = BrushPixel;

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.bounds_iter.next()?;
        let fx = position.x as f32;
        let fy = position.y as f32;
        let p = Vector2::new(fx, fy) - self.center;
        let p1 = self.inv_transform * p;
        let dist_sqr = p1.magnitude_squared();
        let radius = self.radius;
        let strength = if dist_sqr >= radius * radius {
            0.0
        } else {
            (1.0 - dist_sqr.sqrt() / radius).max(0.0)
        };
        let strength = apply_hardness(self.hardness, strength);
        Some(BrushPixel { position, strength })
    }
}

impl RectBrushPixels {
    /// The bounds of the pixels.
    pub fn bounds(&self) -> Rect<i32> {
        self.bounds_iter.bounds()
    }
}

/// An iterator of the pixels of a rectangular brush.
#[derive(Debug, Clone)]
pub struct RectBrushPixels {
    center: Vector2<f32>,
    radius: Vector2<f32>,
    hardness: f32,
    inv_transform: Matrix2<f32>,
    bounds_iter: RectIter,
}

impl RectBrushPixels {
    /// Construct a new pixel iterator for a rectangle brush at the given position,
    /// x-radius, y-radius, and 2x2 transform matrix for rotation.
    pub fn new(
        center: Vector2<f32>,
        radius: Vector2<f32>,
        hardness: f32,
        transform: Matrix2<f32>,
    ) -> Self {
        let mut bounds: Option<Rect<i32>> = None;
        let transform = if transform.is_invertible() {
            transform
        } else {
            Matrix2::identity()
        };
        let inv_transform = transform.try_inverse().unwrap();
        for p in [
            center + radius,
            center + Vector2::new(radius.x, -radius.y),
            center + Vector2::new(-radius.x, radius.y),
            center - radius,
        ] {
            let p = transform * p;
            let ceil = p.map(|x| x.ceil() as i32);
            let floor = p.map(|x| x.floor() as i32);
            let rect = Rect::new(floor.x, floor.y, ceil.x - floor.x, ceil.y - floor.y);
            if let Some(bounds) = &mut bounds {
                bounds.extend_to_contain(rect);
            } else {
                bounds = Some(rect);
            }
        }
        Self {
            center,
            radius,
            hardness,
            inv_transform,
            bounds_iter: RectIter::new(bounds.unwrap()),
        }
    }
}

impl Iterator for RectBrushPixels {
    type Item = BrushPixel;

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.bounds_iter.next()?;
        let fx = position.x as f32;
        let fy = position.y as f32;
        let p = Vector2::new(fx, fy) - self.center;
        let p = self.inv_transform * p;
        let radius = self.radius;
        let p = p.abs();
        let min = radius.min();
        let radius = Vector2::new(radius.x - min, radius.y - min);
        let p = Vector2::new(p.x - min, p.y - min).sup(&Vector2::new(0.0, 0.0));
        let strength = if p.x > radius.x || p.y > radius.y {
            0.0
        } else {
            1.0 - (p.x / radius.x).max(p.y / radius.y)
        };
        let strength = apply_hardness(self.hardness, strength);
        Some(BrushPixel { position, strength })
    }
}

/// Shape of a brush.
#[derive(Copy, Clone, Reflect, Debug)]
pub enum BrushShape {
    /// Circle with given radius.
    Circle {
        /// Radius of the circle.
        radius: f32,
    },
    /// Rectangle with given width and height.
    Rectangle {
        /// Width of the rectangle.
        width: f32,
        /// Length of the rectangle.
        length: f32,
    },
}

uuid_provider!(BrushShape = "a4dbfba0-077c-4658-9972-38384a8432f9");

impl BrushShape {
    /// Return true if the given point is within the shape when positioned at the given center point.
    pub fn contains(&self, brush_center: Vector2<f32>, pixel_position: Vector2<f32>) -> bool {
        match *self {
            BrushShape::Circle { radius } => (brush_center - pixel_position).norm() < radius,
            BrushShape::Rectangle { width, length } => Rect::new(
                brush_center.x - width * 0.5,
                brush_center.y - length * 0.5,
                width,
                length,
            )
            .contains(pixel_position),
        }
    }
}

/// Paint mode of a brush. It defines operation that will be performed on the terrain.
#[derive(Clone, PartialEq, PartialOrd, Reflect, Debug)]
pub enum BrushMode {
    /// Raise or lower the value
    Raise {
        /// An offset to change the value by
        amount: f32,
    },
    /// Flattens value of the terrain data
    Flatten,
    /// Assigns a particular value to anywhere the brush touches.
    Assign {
        /// Fixed value to paint into the data
        value: f32,
    },
    /// Reduce sharp changes in the data.
    Smooth {
        /// Determines the size of each pixel's neighborhood in terms of
        /// distance from the pixel.
        /// 0 means no smoothing at all.
        /// 1 means taking the mean of the 3x3 square of pixels surrounding each smoothed pixel.
        /// 2 means using a 5x5 square of pixels. And so on.
        kernel_radius: u32,
    },
}

uuid_provider!(BrushMode = "48ad4cac-05f3-485a-b2a3-66812713841f");

impl BrushMode {
    /// Perform the operation represented by this BrushMode.
    /// - `pixels`: An iterator over the pixels that are covered by the shape of the brush and the position of the brush.
    /// - `data`: An abstraction of the terrain data that allows the brush mode to edit the terrain data without concern
    /// for chunks of what kind of data is being edited.
    /// - `value`: A value that is used to control some BrushModes, especially `Flatten` where it represents the level of flattened terrain.
    /// - `alpha`: A value between 0.0 and 1.0 that represents how much the brush's effect is weighted when combining it with the original
    /// value of the pixels, with 0.0 meaning the brush has no effect and 1.0 meaning that the original value of the pixels is completely covered.
    pub fn draw<I, D, V>(&self, pixels: I, data: &mut D, value: f32, alpha: f32)
    where
        I: Iterator<Item = BrushPixel>,
        D: BrushableTerrainData<V>,
        V: BrushValue,
    {
        match self {
            BrushMode::Raise { amount } => {
                for BrushPixel { position, strength } in pixels {
                    data.update(position, strength, |_, x| {
                        x.raise(amount * strength * alpha)
                    });
                }
            }
            BrushMode::Flatten => {
                for BrushPixel { position, strength } in pixels {
                    let alpha = strength * alpha;
                    data.update(position, strength, |_, x| {
                        V::from_f32(x.into_f32() * (1.0 - alpha) + value * alpha)
                    });
                }
            }
            BrushMode::Assign { value } => {
                for BrushPixel { position, strength } in pixels {
                    let alpha = strength * alpha;
                    data.update(position, strength, |_, x| {
                        V::from_f32(x.into_f32() * (1.0 - alpha) + value * alpha)
                    });
                }
            }
            BrushMode::Smooth { kernel_radius } => {
                if *kernel_radius == 0 || alpha == 0.0 {
                    return;
                }
                let size = kernel_radius * 2 + 1;
                let scale = 1.0 / (size * size) as f32;
                for BrushPixel { position, strength } in pixels {
                    let alpha = strength * alpha;
                    data.update(position, strength, |data, x| {
                        let value = data.sum_kernel(position, *kernel_radius) * scale;
                        V::from_f32(x.into_f32() * (1.0 - alpha) + value * alpha)
                    });
                }
            }
        }
    }
}

/// Paint target of a brush. It defines the data that the brush will operate on.
#[derive(Copy, Clone, Reflect, Debug, PartialEq, Eq)]
pub enum BrushTarget {
    /// Modifies the height map
    HeightMap,
    /// Draws on a given layer
    LayerMask {
        /// The number of the layer to modify
        layer: usize,
    },
}

uuid_provider!(BrushTarget = "461c1be7-189e-44ee-b8fd-00b8fdbc668f");

/// Brush is used to modify terrain. It supports multiple shapes and modes.
#[derive(Clone, Reflect, Debug)]
pub struct Brush {
    /// Shape of the brush.
    pub shape: BrushShape,
    /// Paint mode of the brush.
    pub mode: BrushMode,
    /// The data to modify with the brush
    pub target: BrushTarget,
    /// Transform that can modify the shape of the brush
    pub transform: Matrix2<f32>,
    /// The softness of the edges of the brush.
    /// 0.0 means that the brush fades very gradually from opaque to transparent.
    /// 1.0 means that the edges of the brush do not fade.
    pub hardness: f32,
    /// The transparency of the brush, allowing the values beneath the brushstroke to show throw.
    /// 0.0 means the brush is fully transparent and does not draw.
    /// 1.0 means the brush is fully opaque.
    pub alpha: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rect_extend_to_contain_f64() {
        let mut rect = Rect::new(0.0, 0.0, 1.0, 1.0);

        //rect.extend_to_contain(Rect::new(1.0, 1.0, 1.0, 1.0));
        //assert_eq!(rect, Rect::new(0.0, 0.0, 2.0, 2.0));

        rect.extend_to_contain(Rect::new(-1.0, -1.0, 1.0, 1.0));
        assert_eq!(rect, Rect::new(-1.0, -1.0, 2.0, 2.0));
    }
    #[test]
    fn rect_extend_to_contain_i32() {
        let mut rect: Rect<i32> = Rect::new(0, 0, 1, 1);

        rect.extend_to_contain(Rect::new(1, 1, 1, 1));
        assert_eq!(rect, Rect::<i32>::new(0, 0, 2, 2));

        rect.extend_to_contain(Rect::new(-1, -1, 1, 1));
        assert_eq!(rect, Rect::<i32>::new(-1, -1, 2, 2));
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
        let mut iter = CircleBrushPixels::new(Vector2::default(), 2.0, 1.0, Matrix2::identity());
        for _ in 0..100 {
            if iter.next().is_none() {
                return;
            }
        }
        panic!("Iter went over 100.");
    }
    #[test]
    fn finite_pixels_rect() {
        let mut iter = RectBrushPixels::new(
            Vector2::default(),
            Vector2::new(2.0, 2.0),
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
        let mut iter = CircleBrushPixels::new(Vector2::default(), 2.0, 1.0, Matrix2::identity());
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
        let mut iter = RectBrushPixels::new(
            Vector2::default(),
            Vector2::new(2.0, 2.0),
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
}
