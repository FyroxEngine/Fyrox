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

use crate::buffer::VkGpuBuffer;
use crate::device::VkDevice;
use crate::ToVkType;
use ash::vk;
use fyrox_graphics::{
    buffer::GpuBuffer,
    core::math::TriangleDefinition,
    error::FrameworkError,
    geometry_buffer::{
        AttributeDefinition, AttributeKind, ElementsDescriptor, GpuGeometryBufferDescriptor,
        GpuGeometryBufferTrait,
    },
};
use std::rc::Rc;
use std::sync::Arc;

impl ToVkType<vk::Format> for AttributeKind {
    fn to_vk(self) -> vk::Format {
        match self {
            AttributeKind::Float => vk::Format::R32_SFLOAT,
            AttributeKind::UnsignedByte => vk::Format::R8_UINT,
            AttributeKind::UnsignedShort => vk::Format::R16_UINT,
            AttributeKind::UnsignedInt => vk::Format::R32_UINT,
        }
    }
}

/// Vulkan geometry buffer implementation.
pub struct VkGpuGeometryBuffer {
    /// Vertex buffers.
    vertex_buffers: Vec<GpuBuffer>,
    /// Element buffer (index buffer).
    element_buffer: Option<GpuBuffer>,
    /// Element count.
    element_count: usize,
    /// Vertex attribute descriptors.
    vertex_attributes: Vec<AttributeDefinition>,
    /// Device reference.
    #[allow(dead_code)]
    device: Arc<VkDevice>,
}

impl VkGpuGeometryBuffer {
    /// Creates a new Vulkan geometry buffer.
    pub fn new(
        device: Arc<VkDevice>,
        descriptor: GpuGeometryBufferDescriptor,
    ) -> Result<Self, FrameworkError> {
        // Calculate element count based on ElementsDescriptor
        let element_count = match descriptor.elements {
            ElementsDescriptor::Triangles(triangles) => triangles.len() * 3,
            ElementsDescriptor::Lines(lines) => lines.len() * 2,
            ElementsDescriptor::Points(points) => points.len(),
        };

        // For now, create empty buffers - proper implementation would upload data
        Ok(Self {
            vertex_buffers: vec![],
            element_buffer: None,
            element_count,
            vertex_attributes: vec![],
            device,
        })
    }

    /// Gets vertex buffer binding descriptions for pipeline creation.
    pub fn get_vertex_binding_descriptions(&self) -> Vec<vk::VertexInputBindingDescription> {
        self.vertex_buffers
            .iter()
            .enumerate()
            .map(|(binding, _buffer)| {
                vk::VertexInputBindingDescription::default()
                    .binding(binding as u32)
                    .stride(self.calculate_vertex_stride(binding)) // This would need proper calculation
                    .input_rate(vk::VertexInputRate::VERTEX)
            })
            .collect()
    }

    /// Gets vertex attribute descriptions for pipeline creation.
    pub fn get_vertex_attribute_descriptions(&self) -> Vec<vk::VertexInputAttributeDescription> {
        self.vertex_attributes
            .iter()
            .enumerate()
            .map(|(location, attr)| {
                vk::VertexInputAttributeDescription::default()
                    .binding(0) // Simplified - would need proper binding logic
                    .location(location as u32)
                    .format(attr.kind.to_vk())
                    .offset(0) // Simplified - would need proper offset calculation
            })
            .collect()
    }

    /// Calculates vertex stride for a given binding.
    fn calculate_vertex_stride(&self, binding: usize) -> u32 {
        // This is a simplified calculation - in practice, you'd want to
        // calculate the actual stride based on the attributes
        match binding {
            0 => 12, // Assuming position (3 floats)
            1 => 8,  // Assuming texture coordinates (2 floats)
            2 => 12, // Assuming normal (3 floats)
            _ => 4,  // Default
        }
    }

    /// Gets the Vulkan vertex buffers.
    pub fn vk_vertex_buffers(&self) -> Vec<vk::Buffer> {
        self.vertex_buffers
            .iter()
            .filter_map(|buffer| {
                buffer
                    .as_any()
                    .downcast_ref::<VkGpuBuffer>()
                    .map(|vk_buffer| vk_buffer.vk_buffer())
            })
            .collect()
    }

    /// Gets the Vulkan index buffer if present.
    pub fn vk_index_buffer(&self) -> Option<vk::Buffer> {
        self.element_buffer.as_ref().and_then(|buffer| {
            buffer
                .as_any()
                .downcast_ref::<VkGpuBuffer>()
                .map(|vk_buffer| vk_buffer.vk_buffer())
        })
    }
}

impl GpuGeometryBufferTrait for VkGpuGeometryBuffer {
    fn set_buffer_data(&self, buffer: usize, data: &[u8]) {
        // Write data to the specified vertex buffer
        if let Some(vertex_buffer) = self.vertex_buffers.get(buffer) {
            let _ = vertex_buffer.write_data(data);
        }
    }

    fn element_count(&self) -> usize {
        self.element_count
    }

    fn set_triangles(&self, triangles: &[TriangleDefinition]) {
        // Convert triangles to index buffer data and write to element buffer
        if let Some(element_buffer) = &self.element_buffer {
            let indices: Vec<u32> = triangles
                .iter()
                .flat_map(|tri| [tri[0], tri[1], tri[2]])
                .collect();
            let _ = element_buffer.write_data_of_type(&indices);
        }
    }

    fn set_lines(&self, lines: &[[u32; 2]]) {
        // Convert lines to index buffer data and write to element buffer
        if let Some(element_buffer) = &self.element_buffer {
            let indices: Vec<u32> = lines.iter().flat_map(|line| [line[0], line[1]]).collect();
            let _ = element_buffer.write_data_of_type(&indices);
        }
    }

    fn set_points(&self, points: &[u32]) {
        // Write points to element buffer
        if let Some(element_buffer) = &self.element_buffer {
            let _ = element_buffer.write_data_of_type(points);
        }
    }
}

/// Creates a Vulkan geometry buffer.
pub fn create_geometry_buffer(
    device: Arc<VkDevice>,
    descriptor: GpuGeometryBufferDescriptor,
) -> Result<Rc<dyn GpuGeometryBufferTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuGeometryBuffer::new(device, descriptor)?))
}
