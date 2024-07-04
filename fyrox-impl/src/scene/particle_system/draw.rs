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
