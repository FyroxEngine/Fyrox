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
    buffer::BufferUsage,
    core::{array_as_u8_slice, math::TriangleDefinition, Downcast},
    ElementKind,
};
use bytemuck::Pod;
use std::mem::size_of;

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub enum AttributeKind {
    Float,
    UnsignedByte,
    UnsignedShort,
    UnsignedInt,
}

pub struct AttributeDefinition {
    pub location: u32,
    pub kind: AttributeKind,
    pub component_count: usize,
    pub normalized: bool,
    pub divisor: u32,
}

impl AttributeKind {
    pub fn size(self) -> usize {
        match self {
            AttributeKind::Float => size_of::<f32>(),
            AttributeKind::UnsignedByte => size_of::<u8>(),
            AttributeKind::UnsignedShort => size_of::<u16>(),
            AttributeKind::UnsignedInt => size_of::<u32>(),
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct DrawCallStatistics {
    pub triangles: usize,
}

pub struct VertexBufferData<'a> {
    pub element_size: usize,
    pub bytes: Option<&'a [u8]>,
}

impl<'a> VertexBufferData<'a> {
    pub fn new<T: Pod>(vertices: Option<&'a [T]>) -> Self {
        Self {
            element_size: size_of::<T>(),
            bytes: vertices.map(|v| array_as_u8_slice(v)),
        }
    }
}

pub struct VertexBufferDescriptor<'a> {
    pub usage: BufferUsage,
    pub attributes: &'a [AttributeDefinition],
    pub data: VertexBufferData<'a>,
}

pub struct GeometryBufferDescriptor<'a> {
    pub element_kind: ElementKind,
    pub buffers: &'a [VertexBufferDescriptor<'a>],
    pub usage: BufferUsage,
}

pub trait GeometryBuffer: Downcast {
    fn set_buffer_data(&self, buffer: usize, data: &[u8]);
    fn element_count(&self) -> usize;
    fn set_triangles(&self, triangles: &[TriangleDefinition]);
    fn set_lines(&self, lines: &[[u32; 2]]);
}

impl dyn GeometryBuffer {
    pub fn set_buffer_data_of_type<T: Pod>(&mut self, buffer: usize, data: &[T]) {
        self.set_buffer_data(buffer, array_as_u8_slice(data))
    }
}
