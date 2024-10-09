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

//! Rendering framework.

use crate::{
    renderer::framework::{
        buffer::BufferUsage,
        error::FrameworkError,
        geometry_buffer::{
            AttributeDefinition, AttributeKind, GeometryBuffer, GeometryBufferDescriptor,
            VertexBufferData, VertexBufferDescriptor,
        },
        gpu_texture::{
            GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind, WrapMode,
        },
        server::GraphicsServer,
    },
    resource::texture::{
        TextureKind, TextureMagnificationFilter, TextureMinificationFilter, TexturePixelKind,
        TextureWrapMode,
    },
    scene::mesh::{buffer::VertexAttributeDataType, surface::SurfaceData},
};
pub use fyrox_graphics::*;

impl From<TextureKind> for GpuTextureKind {
    fn from(v: TextureKind) -> Self {
        match v {
            TextureKind::Line { length } => GpuTextureKind::Line {
                length: length as usize,
            },
            TextureKind::Rectangle { width, height } => GpuTextureKind::Rectangle {
                width: width as usize,
                height: height as usize,
            },
            TextureKind::Cube { width, height } => GpuTextureKind::Cube {
                width: width as usize,
                height: height as usize,
            },
            TextureKind::Volume {
                width,
                height,
                depth,
            } => GpuTextureKind::Volume {
                width: width as usize,
                height: height as usize,
                depth: depth as usize,
            },
        }
    }
}

impl From<TexturePixelKind> for PixelKind {
    fn from(texture_kind: TexturePixelKind) -> Self {
        match texture_kind {
            TexturePixelKind::R8 => Self::R8,
            TexturePixelKind::RGB8 => Self::RGB8,
            TexturePixelKind::RGBA8 => Self::RGBA8,
            TexturePixelKind::RG8 => Self::RG8,
            TexturePixelKind::R16 => Self::R16,
            TexturePixelKind::RG16 => Self::RG16,
            TexturePixelKind::BGR8 => Self::BGR8,
            TexturePixelKind::BGRA8 => Self::BGRA8,
            TexturePixelKind::RGB16 => Self::RGB16,
            TexturePixelKind::RGBA16 => Self::RGBA16,
            TexturePixelKind::RGB16F => Self::RGB16F,
            TexturePixelKind::DXT1RGB => Self::DXT1RGB,
            TexturePixelKind::DXT1RGBA => Self::DXT1RGBA,
            TexturePixelKind::DXT3RGBA => Self::DXT3RGBA,
            TexturePixelKind::DXT5RGBA => Self::DXT5RGBA,
            TexturePixelKind::R8RGTC => Self::R8RGTC,
            TexturePixelKind::RG8RGTC => Self::RG8RGTC,
            TexturePixelKind::RGB32F => Self::RGB32F,
            TexturePixelKind::RGBA32F => Self::RGBA32F,
            TexturePixelKind::Luminance8 => Self::L8,
            TexturePixelKind::LuminanceAlpha8 => Self::LA8,
            TexturePixelKind::Luminance16 => Self::L16,
            TexturePixelKind::LuminanceAlpha16 => Self::LA16,
            TexturePixelKind::R32F => Self::R32F,
            TexturePixelKind::R16F => Self::R16F,
        }
    }
}

impl From<TextureMagnificationFilter> for MagnificationFilter {
    fn from(v: TextureMagnificationFilter) -> Self {
        match v {
            TextureMagnificationFilter::Nearest => Self::Nearest,
            TextureMagnificationFilter::Linear => Self::Linear,
        }
    }
}

impl From<TextureMinificationFilter> for MinificationFilter {
    fn from(v: TextureMinificationFilter) -> Self {
        match v {
            TextureMinificationFilter::Nearest => Self::Nearest,
            TextureMinificationFilter::NearestMipMapNearest => Self::NearestMipMapNearest,
            TextureMinificationFilter::NearestMipMapLinear => Self::NearestMipMapLinear,
            TextureMinificationFilter::Linear => Self::Linear,
            TextureMinificationFilter::LinearMipMapNearest => Self::LinearMipMapNearest,
            TextureMinificationFilter::LinearMipMapLinear => Self::LinearMipMapLinear,
        }
    }
}

impl From<TextureWrapMode> for WrapMode {
    fn from(v: TextureWrapMode) -> Self {
        match v {
            TextureWrapMode::Repeat => WrapMode::Repeat,
            TextureWrapMode::ClampToEdge => WrapMode::ClampToEdge,
            TextureWrapMode::ClampToBorder => WrapMode::ClampToBorder,
            TextureWrapMode::MirroredRepeat => WrapMode::MirroredRepeat,
            TextureWrapMode::MirrorClampToEdge => WrapMode::MirrorClampToEdge,
        }
    }
}

/// Extension trait for [`GeometryBuffer`].
pub trait GeometryBufferExt {
    /// Creates [`GeometryBuffer`] from [`SurfaceData`].
    fn from_surface_data(
        data: &SurfaceData,
        usage: BufferUsage,
        server: &dyn GraphicsServer,
    ) -> Result<Box<dyn GeometryBuffer>, FrameworkError>;
}

impl GeometryBufferExt for dyn GeometryBuffer {
    fn from_surface_data(
        data: &SurfaceData,
        usage: BufferUsage,
        server: &dyn GraphicsServer,
    ) -> Result<Box<dyn GeometryBuffer>, FrameworkError> {
        let attributes = data
            .vertex_buffer
            .layout()
            .iter()
            .map(|a| AttributeDefinition {
                location: a.shader_location as u32,
                kind: match a.data_type {
                    VertexAttributeDataType::F32 => AttributeKind::Float,
                    VertexAttributeDataType::U32 => AttributeKind::UnsignedInt,
                    VertexAttributeDataType::U16 => AttributeKind::UnsignedShort,
                    VertexAttributeDataType::U8 => AttributeKind::UnsignedByte,
                },
                component_count: a.size as usize,
                normalized: a.normalized,
                divisor: a.divisor as u32,
            })
            .collect::<Vec<_>>();

        let geometry_buffer_desc = GeometryBufferDescriptor {
            element_kind: ElementKind::Triangle,
            buffers: &[VertexBufferDescriptor {
                usage,
                attributes: &attributes,
                data: VertexBufferData {
                    element_size: data.vertex_buffer.vertex_size() as usize,
                    bytes: Some(data.vertex_buffer.raw_data()),
                },
            }],
        };

        let geometry_buffer = server.create_geometry_buffer(geometry_buffer_desc)?;

        geometry_buffer.set_triangles(data.geometry_buffer.triangles_ref());

        Ok(geometry_buffer)
    }
}
