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

use crate::{
    define_shared_wrapper,
    gpu_texture::{MagnificationFilter, MinificationFilter, WrapMode},
};
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

/// Sampler is a GPU entity that defines how texels will be fetched from a texture. See [`GpuSamplerDescriptor`]
/// docs for more info.
pub trait GpuSamplerTrait: GpuSamplerAsAny + Debug {}

define_shared_wrapper!(GpuSampler<dyn GpuSamplerTrait>);
