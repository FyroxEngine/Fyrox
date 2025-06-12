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

//! A set of textures of certain kinds. See [`RendererResources`] docs for more info.

use crate::renderer::framework::GeometryBufferExt;
use crate::scene::mesh::surface::SurfaceData;
use fyrox_core::{algebra::Matrix4, array_as_u8_slice};
use fyrox_graphics::geometry_buffer::GpuGeometryBuffer;
use fyrox_graphics::{
    buffer::{BufferKind, BufferUsage, GpuBuffer},
    error::FrameworkError,
    gpu_program::SamplerFallback,
    gpu_texture::{GpuTexture, GpuTextureDescriptor, GpuTextureKind, PixelKind},
    sampler::{
        GpuSampler, GpuSamplerDescriptor, MagnificationFilter, MinificationFilter, WrapMode,
    },
    server::GraphicsServer,
};
use fyrox_material::shader::ShaderDefinition;

/// A set of textures of certain kinds that could be used as a stub in cases when you don't have
/// your own texture of this kind.
pub struct RendererResources {
    /// White, one pixel, texture which will be used as stub when rendering something without
    /// a texture specified.
    pub white_dummy: GpuTexture,
    /// Black, one pixel, texture.
    pub black_dummy: GpuTexture,
    /// A cube map with 6 textures of 1x1 black pixel in size.
    pub environment_dummy: GpuTexture,
    /// One pixel texture with (0, 1, 0) vector is used as stub when rendering something without a
    /// normal map.
    pub normal_dummy: GpuTexture,
    /// One pixel texture used as stub when rendering something without a  metallic texture. Default
    /// metalness is 0.0
    pub metallic_dummy: GpuTexture,
    /// One pixel volume texture.
    pub volume_dummy: GpuTexture,
    /// A stub uniform buffer for situation when there's no actual bone matrices.
    pub bone_matrices_stub_uniform_buffer: GpuBuffer,
    /// A sampler with the linear filtration that clamps incoming UVs to `[0;1]` range.
    pub linear_clamp_sampler: GpuSampler,
    /// A sampler with the linear filtration and mipmapping that clamps incoming UVs to `[0;1]` range.
    pub linear_mipmap_linear_clamp_sampler: GpuSampler,
    /// A sampler with the linear filtration.
    pub linear_wrap_sampler: GpuSampler,
    /// A sampler with the nearest filtration that clamps incoming UVs to `[0;1]` range.
    pub nearest_clamp_sampler: GpuSampler,
    /// A sampler with the nearest filtration.
    pub nearest_wrap_sampler: GpuSampler,
    /// Unit oXY-oriented quad.
    pub quad: GpuGeometryBuffer,
}

impl RendererResources {
    /// Creates a new set of fallback resources.
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            white_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[255u8, 255u8, 255u8, 255u8]),
                ..Default::default()
            })?,
            black_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[0u8, 0u8, 0u8, 255u8]),
                ..Default::default()
            })?,
            environment_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Cube { size: 1 },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[
                    0u8, 0u8, 0u8, 255u8, // pos-x
                    0u8, 0u8, 0u8, 255u8, // neg-x
                    0u8, 0u8, 0u8, 255u8, // pos-y
                    0u8, 0u8, 0u8, 255u8, // neg-y
                    0u8, 0u8, 0u8, 255u8, // pos-z
                    0u8, 0u8, 0u8, 255u8, // neg-z
                ]),
                ..Default::default()
            })?,
            normal_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[128u8, 128u8, 255u8, 255u8]),
                ..Default::default()
            })?,
            metallic_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[0u8, 0u8, 0u8, 0u8]),
                ..Default::default()
            })?,
            volume_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Volume {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[0u8, 0u8, 0u8, 0u8]),
                ..Default::default()
            })?,
            bone_matrices_stub_uniform_buffer: {
                let buffer = server.create_buffer(
                    ShaderDefinition::MAX_BONE_MATRICES * size_of::<Matrix4<f32>>(),
                    BufferKind::Uniform,
                    BufferUsage::StaticDraw,
                )?;
                const SIZE: usize = ShaderDefinition::MAX_BONE_MATRICES * size_of::<Matrix4<f32>>();
                let zeros = [0.0; SIZE];
                buffer.write_data(array_as_u8_slice(&zeros))?;
                buffer
            },
            linear_clamp_sampler: server.create_sampler(GpuSamplerDescriptor {
                min_filter: MinificationFilter::Linear,
                mag_filter: MagnificationFilter::Linear,
                s_wrap_mode: WrapMode::ClampToEdge,
                t_wrap_mode: WrapMode::ClampToEdge,
                r_wrap_mode: WrapMode::ClampToEdge,
                ..Default::default()
            })?,
            linear_mipmap_linear_clamp_sampler: server.create_sampler(GpuSamplerDescriptor {
                min_filter: MinificationFilter::LinearMipMapLinear,
                mag_filter: MagnificationFilter::Linear,
                s_wrap_mode: WrapMode::ClampToEdge,
                t_wrap_mode: WrapMode::ClampToEdge,
                r_wrap_mode: WrapMode::ClampToEdge,
                ..Default::default()
            })?,
            linear_wrap_sampler: server.create_sampler(GpuSamplerDescriptor {
                min_filter: MinificationFilter::Linear,
                mag_filter: MagnificationFilter::Linear,
                ..Default::default()
            })?,
            nearest_clamp_sampler: server.create_sampler(GpuSamplerDescriptor {
                min_filter: MinificationFilter::Nearest,
                mag_filter: MagnificationFilter::Nearest,
                s_wrap_mode: WrapMode::ClampToEdge,
                t_wrap_mode: WrapMode::ClampToEdge,
                r_wrap_mode: WrapMode::ClampToEdge,
                ..Default::default()
            })?,
            nearest_wrap_sampler: server.create_sampler(GpuSamplerDescriptor {
                min_filter: MinificationFilter::Nearest,
                mag_filter: MagnificationFilter::Nearest,
                ..Default::default()
            })?,
            quad: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
        })
    }

    /// Picks a texture that corresponds to the actual value of the given sampler fallback.
    pub fn sampler_fallback(&self, sampler_fallback: SamplerFallback) -> &GpuTexture {
        match sampler_fallback {
            SamplerFallback::White => &self.white_dummy,
            SamplerFallback::Normal => &self.normal_dummy,
            SamplerFallback::Black => &self.black_dummy,
            SamplerFallback::Volume => &self.volume_dummy,
        }
    }
}
