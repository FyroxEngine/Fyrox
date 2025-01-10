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
        server::GraphicsServer,
    },
    scene::mesh::{buffer::VertexAttributeDataType, surface::SurfaceData},
};
pub use fyrox_graphics::*;

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
            usage,
        };

        let geometry_buffer = server.create_geometry_buffer(geometry_buffer_desc)?;

        geometry_buffer.set_triangles(data.geometry_buffer.triangles_ref());

        Ok(geometry_buffer)
    }
}
