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

//! Shared texture format helpers and binding offset constants.
//!
//! This module centralizes format-related utilities used across the crate to avoid
//! duplication and keep the binding convention in one place.

/// Offset added to a texture binding slot to obtain the corresponding sampler slot.
///
/// For a texture bound at slot `N`, its sampler is bound at `N + SAMPLER_BINDING_OFFSET`.
pub const SAMPLER_BINDING_OFFSET: usize = 100;

/// Offset added to a resource binding slot to obtain the uniform buffer slot.
///
/// For a property group at binding `N`, its uniform buffer is bound at
/// `N + UNIFORM_BINDING_OFFSET`.
pub const UNIFORM_BINDING_OFFSET: usize = 200;

/// Returns `true` if the given texture format supports filtering samplers.
///
/// Non-filterable formats (32-bit float, uint, sint) require
/// [`wgpu::SamplerBindingType::NonFiltering`] when creating bind group layouts.
///
/// # Examples
///
/// ```ignore
/// use fyrox_graphics_wgpu::format_helpers::is_filterable_format;
///
/// assert!(is_filterable_format(wgpu::TextureFormat::Rgba8Unorm));
/// assert!(!is_filterable_format(wgpu::TextureFormat::R32Float));
/// ```
pub fn is_filterable_format(fmt: wgpu::TextureFormat) -> bool {
    !matches!(
        fmt,
        wgpu::TextureFormat::R32Float
            | wgpu::TextureFormat::Rg32Float
            | wgpu::TextureFormat::Rgba32Float
            | wgpu::TextureFormat::R8Uint
            | wgpu::TextureFormat::R16Uint
            | wgpu::TextureFormat::R32Uint
            | wgpu::TextureFormat::R8Sint
            | wgpu::TextureFormat::R16Sint
            | wgpu::TextureFormat::R32Sint
    )
}

/// Returns `true` if the given texture format is an integer format.
///
/// Hardware blending (e.g., alpha blending) relies on floating-point arithmetic.
/// Integer formats (such as `R8Uint`, `R16Sint`, etc.) cannot be blended by the GPU.
/// When configuring a [`wgpu::ColorTargetState`] for these formats, the `blend`
/// field MUST be explicitly set to `None` to prevent `wgpu` validation errors.
///
/// # Examples
///
/// ```ignore
/// use fyrox_graphics_wgpu::format_helpers::is_integer_format;
///
/// assert!(is_integer_format(wgpu::TextureFormat::R8Uint));
/// assert!(!is_integer_format(wgpu::TextureFormat::Rgba8Unorm));
/// ```
pub fn is_integer_format(fmt: wgpu::TextureFormat) -> bool {
    use wgpu::TextureFormat as F;
    matches!(
        fmt,
        F::R8Uint | F::R8Sint | F::R16Uint | F::R16Sint | F::R32Uint | F::R32Sint |
        F::Rg8Uint | F::Rg8Sint | F::Rg16Uint | F::Rg16Sint | F::Rg32Uint | F::Rg32Sint |
        F::Rgba8Uint | F::Rgba8Sint | F::Rgba16Uint | F::Rgba16Sint | F::Rgba32Uint | F::Rgba32Sint
    )
}

/// Returns the appropriate [`wgpu::TextureSampleType`] for the given texture format.
///
/// Used when creating bind group layouts from actual texture formats rather than
/// from [`SamplerKind`](fyrox_graphics::gpu_program::SamplerKind) inference.
/// Handles depth, uint, sint, and non-filterable float formats correctly.
pub fn sample_type_for_format(fmt: wgpu::TextureFormat) -> wgpu::TextureSampleType {
    use wgpu::TextureFormat as F;
    match fmt {
        F::Depth16Unorm
        | F::Depth24Plus
        | F::Depth24PlusStencil8
        | F::Depth32Float
        | F::Depth32FloatStencil8 => wgpu::TextureSampleType::Depth,
        F::R8Uint
        | F::R16Uint
        | F::R32Uint
        | F::Rg8Uint
        | F::Rg16Uint
        | F::Rg32Uint
        | F::Rgba8Uint
        | F::Rgba16Uint
        | F::Rgba32Uint => wgpu::TextureSampleType::Uint,
        F::R8Sint
        | F::R16Sint
        | F::R32Sint
        | F::Rg8Sint
        | F::Rg16Sint
        | F::Rg32Sint
        | F::Rgba8Sint
        | F::Rgba16Sint
        | F::Rgba32Sint => wgpu::TextureSampleType::Sint,
        F::R32Float | F::Rg32Float | F::Rgba32Float => {
            wgpu::TextureSampleType::Float { filterable: false }
        }
        _ => wgpu::TextureSampleType::Float { filterable: true },
    }
}
