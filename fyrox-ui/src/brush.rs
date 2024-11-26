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

//! Brush defines a way to fill an arbitrary surface. See [`Brush`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::core::{algebra::Vector2, color::Color, reflect::prelude::*, visitor::prelude::*};
use fyrox_core::uuid_provider;
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Gradient point defines a point on a surface with a color.
#[derive(Clone, Debug, PartialEq, Reflect, Visit, Default)]
pub struct GradientPoint {
    /// A distance from an origin of the gradient.
    pub stop: f32,
    /// Color of the point.
    pub color: Color,
}

uuid_provider!(GradientPoint = "e8503ec6-a1d0-4a9b-ab91-0d3f126254dd");

/// Brush defines a way to fill an arbitrary surface.
#[derive(Clone, Debug, PartialEq, Reflect, Visit, AsRefStr, EnumString, VariantNames)]
pub enum Brush {
    /// A brush, that fills a surface with a solid color.
    Solid(Color),
    /// A brush, that fills a surface with a linear gradient, which is defined by two points in local coordinates
    /// and a set of stop points. See [`GradientPoint`] for more info.
    LinearGradient {
        /// Beginning of the gradient in local coordinates.
        from: Vector2<f32>,
        /// End of the gradient in local coordinates.
        to: Vector2<f32>,
        /// Stops of the gradient.
        stops: Vec<GradientPoint>,
    },
    /// A brush, that fills a surface with a radial gradient, which is defined by a center point in local coordinates
    /// and a set of stop points. See [`GradientPoint`] for more info.
    RadialGradient {
        /// Center of the gradient in local coordinates.
        center: Vector2<f32>,
        /// Stops of the gradient.
        stops: Vec<GradientPoint>,
    },
}

impl From<Color> for Brush {
    fn from(color: Color) -> Self {
        Brush::Solid(color)
    }
}

uuid_provider!(Brush = "eceb3805-73b6-47e0-8582-38a01f7b70e1");

impl Default for Brush {
    fn default() -> Self {
        Self::Solid(Color::WHITE)
    }
}
