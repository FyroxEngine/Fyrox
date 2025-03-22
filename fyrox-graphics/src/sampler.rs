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

use crate::define_shared_wrapper;
use fyrox_core::define_as_any_trait;
use std::fmt::Debug;

define_as_any_trait!(GpuSamplerAsAny => GpuSamplerTrait);

/// A set of potential options that can be used to configure a GPU sampler.
pub struct GpuSamplerDescriptor {
    /// Minification filter of the texture. See [`MinificationFilter`] docs for more info.
    pub min_filter: MinificationFilter,
    /// Magnification filter of the texture. See [`MagnificationFilter`] docs for more info.
    pub mag_filter: MagnificationFilter,
    /// S coordinate wrap mode. See [`WrapMode`] docs for more info.
    pub s_wrap_mode: WrapMode,
    /// T coordinate wrap mode. See [`WrapMode`] docs for more info.
    pub t_wrap_mode: WrapMode,
    /// R coordinate wrap mode. See [`WrapMode`] docs for more info.
    pub r_wrap_mode: WrapMode,
    /// Anisotropy level of the texture. Default is 1.0. Max number is usually depends on the
    /// GPU, but the cap is 16.0 on pretty much any platform. This number should be a power of two.
    pub anisotropy: f32,
    /// Sets the minimum level-of-detail parameter. This floating-point value limits the selection
    /// of highest resolution mipmap (lowest mipmap level). The initial value is -1000.0.
    pub min_lod: f32,
    /// Sets the maximum level-of-detail parameter. This floating-point value limits the selection
    /// of the lowest resolution mipmap (highest mipmap level). The initial value is 1000.0.
    pub max_lod: f32,
    /// Specifies a fixed bias value that is to be added to the level-of-detail parameter for the
    /// texture before texture sampling. The specified value is added to the shader-supplied bias
    /// value (if any) and subsequently clamped into the implementation-defined range
    /// `−bias_max..bias_max`, where `bias_max` is the value that can be fetched from the current
    /// graphics server. The initial value is 0.0.
    pub lod_bias: f32,
}

impl Default for GpuSamplerDescriptor {
    fn default() -> Self {
        Self {
            min_filter: Default::default(),
            mag_filter: Default::default(),
            s_wrap_mode: Default::default(),
            t_wrap_mode: Default::default(),
            r_wrap_mode: Default::default(),
            anisotropy: 1.0,
            min_lod: -1000.0,
            max_lod: 1000.0,
            lod_bias: 0.0,
        }
    }
}

impl GpuSamplerDescriptor {
    pub fn new_rt_sampler() -> Self {
        Self {
            min_filter: MinificationFilter::Nearest,
            mag_filter: MagnificationFilter::Nearest,
            s_wrap_mode: WrapMode::ClampToEdge,
            t_wrap_mode: WrapMode::ClampToEdge,
            r_wrap_mode: WrapMode::ClampToEdge,
            ..Default::default()
        }
    }
}

/// Sampler is a GPU entity that defines how texels will be fetched from a texture. See [`GpuSamplerDescriptor`]
/// docs for more info.
pub trait GpuSamplerTrait: GpuSamplerAsAny + Debug {}

define_shared_wrapper!(GpuSampler<dyn GpuSamplerTrait>);

/// The texture magnification function is used when the pixel being textured maps to an area
/// less than or equal to one texture element.
#[derive(Default, Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Debug)]
#[repr(u32)]
pub enum MagnificationFilter {
    /// Returns the value of the texture element that is nearest to the center of the pixel
    /// being textured.
    Nearest,
    /// Returns the weighted average of the four texture elements that are closest to the
    /// center of the pixel being textured.
    #[default]
    Linear,
}

/// The texture minifying function is used whenever the pixel being textured maps to an area
/// greater than one texture element.
#[derive(Default, Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Debug)]
pub enum MinificationFilter {
    /// Returns the value of the texture element that is nearest to the center of the pixel
    /// being textured.
    Nearest,
    /// Chooses the mipmap that most closely matches the size of the pixel being textured and
    /// uses the Nearest criterion (the texture element nearest to the center of the pixel)
    /// to produce a texture value.
    NearestMipMapNearest,
    /// Chooses the two mipmaps that most closely match the size of the pixel being textured
    /// and uses the Nearest criterion (the texture element nearest to the center of the pixel)
    /// to produce a texture value from each mipmap. The final texture value is a weighted average
    /// of those two values.
    NearestMipMapLinear,
    /// Returns the weighted average of the four texture elements that are closest to the
    /// center of the pixel being textured.
    #[default]
    Linear,
    /// Chooses the mipmap that most closely matches the size of the pixel being textured and
    /// uses the Linear criterion (a weighted average of the four texture elements that are
    /// closest to the center of the pixel) to produce a texture value.
    LinearMipMapNearest,
    /// Chooses the two mipmaps that most closely match the size of the pixel being textured
    /// and uses the Linear criterion (a weighted average of the four texture elements that
    /// are closest to the center of the pixel) to produce a texture value from each mipmap.
    /// The final texture value is a weighted average of those two values.
    LinearMipMapLinear,
}

/// Defines a law of texture coordinate modification.
#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
pub enum WrapMode {
    /// Causes the integer part of a coordinate to be ignored; GPU uses only the fractional part,
    /// thereby creating a repeating pattern.
    #[default]
    Repeat,
    /// Causes a coordinates to be clamped to the range, where N is the size of the texture
    /// in the direction of clamping
    ClampToEdge,
    /// Evaluates a coordinates in a similar manner to ClampToEdge. However, in cases where clamping
    /// would have occurred in ClampToEdge mode, the fetched texel data is substituted with the values
    /// specified by border color.
    ClampToBorder,
    /// Causes the coordinate to be set to the fractional part of the texture coordinate if the integer
    /// part of coordinate is even; if the integer part of coordinate is odd, then the coordinate texture
    /// coordinate is set to 1-frac, where frac represents the fractional part of coordinate.
    MirroredRepeat,
    /// Causes a coordinate to be repeated as for MirroredRepeat for one repetition of the texture, at
    /// which point the coordinate to be clamped as in ClampToEdge.
    MirrorClampToEdge,
}

/// Texture coordinate.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Coordinate {
    /// S coordinate, similar to X axis.
    S,
    /// T coordinate, similar to Y axis.
    T,
    /// R coordinate, similar to Z axis.
    R,
}
