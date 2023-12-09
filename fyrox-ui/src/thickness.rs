//! Describes the thickness of a frame around a rectangle (for all four sides).

use crate::core::{algebra::Vector2, reflect::prelude::*, visitor::prelude::*};

/// Describes the thickness of a frame around a rectangle (for all four sides). It is primarily used to
/// define margins and to define stroke thickness for various widgets.
#[derive(Copy, Clone, PartialEq, Debug, Reflect, Visit)]
pub struct Thickness {
    /// Thickness of the left side of a rectangle.
    pub left: f32,
    /// Thickness of the top side of a rectangle.
    pub top: f32,
    /// Thickness of the right side of a rectangle.
    pub right: f32,
    /// Thickness of the bottom side of a rectangle.
    pub bottom: f32,
}

impl Default for Thickness {
    fn default() -> Self {
        Self::uniform(0.0)
    }
}

impl Thickness {
    /// Degenerate thickness that has no effect.
    pub fn zero() -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }

    /// Uniform thickness for all four sides of a rectangle.
    pub fn uniform(v: f32) -> Self {
        Self {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }

    /// Thickness for the bottom side of a rectangle.
    pub fn bottom(v: f32) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: v,
        }
    }

    /// Thickness for the top side of a rectangle.
    pub fn top(v: f32) -> Self {
        Self {
            left: 0.0,
            top: v,
            right: 0.0,
            bottom: 0.0,
        }
    }

    /// Thickness for the left side of a rectangle.
    pub fn left(v: f32) -> Self {
        Self {
            left: v,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }

    /// Thickness for the rigth side of a rectangle.
    pub fn right(v: f32) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: v,
            bottom: 0.0,
        }
    }

    /// Thickness for the top and right sides of a rectangle.
    pub fn top_right(v: f32) -> Self {
        Self {
            left: 0.0,
            top: v,
            right: v,
            bottom: 0.0,
        }
    }

    /// Thickness for the top and left sides of a rectangle.
    pub fn top_left(v: f32) -> Self {
        Self {
            left: v,
            top: v,
            right: 0.0,
            bottom: 0.0,
        }
    }

    /// Thickness for the bottom and right sides of a rectangle.
    pub fn bottom_right(v: f32) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: v,
            bottom: v,
        }
    }

    /// Thickness for the bottom and left sides of a rectangle.
    pub fn bottom_left(v: f32) -> Self {
        Self {
            left: v,
            top: 0.0,
            right: 0.0,
            bottom: v,
        }
    }

    /// Thickness for the top and bottom sides of a rectangle.
    pub fn top_bottom(v: f32) -> Self {
        Self {
            left: 0.0,
            top: v,
            right: 0.0,
            bottom: v,
        }
    }

    /// Thickness for the left and right sides of a rectangle.
    pub fn left_right(v: f32) -> Self {
        Self {
            left: v,
            top: 0.0,
            right: v,
            bottom: 0.0,
        }
    }

    /// Returns an offset defined by this thickness. It is just a vector `(left, top)`.
    pub fn offset(&self) -> Vector2<f32> {
        Vector2::new(self.left, self.top)
    }

    /// Returns a margin for each axis (horizontal and vertical).
    pub fn axes_margin(&self) -> Vector2<f32> {
        Vector2::new(self.left + self.right, self.top + self.bottom)
    }
}
