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
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
    },
    scene::mesh::buffer::{
        VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage, VertexTrait,
    },
};
use bytemuck::{Pod, Zeroable};

/// OpenGL expects this structure packed as in C.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub tex_coord: Vector2<f32>,
    pub size: f32,
    pub rotation: f32,
    pub color: Color,
}

impl VertexTrait for Vertex {
    fn layout() -> &'static [VertexAttributeDescriptor] {
        &[
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Position,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 0,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::TexCoord0,
                data_type: VertexAttributeDataType::F32,
                size: 2,
                divisor: 0,
                shader_location: 1,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Custom0,
                data_type: VertexAttributeDataType::F32,
                size: 1,
                divisor: 0,
                shader_location: 2,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Custom1,
                data_type: VertexAttributeDataType::F32,
                size: 1,
                divisor: 0,
                shader_location: 3,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Color,
                data_type: VertexAttributeDataType::U8,
                size: 4,
                divisor: 0,
                shader_location: 4,
                normalized: true,
            },
        ]
    }
}
