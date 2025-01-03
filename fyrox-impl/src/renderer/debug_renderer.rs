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

//! Debug renderer allows you to create debug geometry (wireframe) on the fly. As it said
//! in its name its purpose - output debug information. It can be used to render collision
//! shapes, contact information (normals, positions, etc.), paths build by navmesh and so
//! on. It contains implementations to draw most common shapes (line, box, oob, frustum, etc).

use crate::{
    core::color::Color,
    core::{
        algebra::{Matrix4, Vector3},
        math::Rect,
        sstorage::ImmutableString,
    },
    renderer::{
        cache::uniform::UniformBufferCache,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{FrameBuffer, ResourceBindGroup, ResourceBinding},
            geometry_buffer::{
                AttributeDefinition, AttributeKind, GeometryBuffer, GeometryBufferDescriptor,
                VertexBufferData, VertexBufferDescriptor,
            },
            gpu_program::GpuProgram,
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            CompareFunc, DrawParameters, ElementKind, ElementRange,
        },
        RenderPassStatistics,
    },
    scene::debug::Line,
};
use bytemuck::{Pod, Zeroable};
use fyrox_graphics::framebuffer::BufferLocation;

#[repr(C)]
#[derive(Copy, Pod, Zeroable, Clone)]
struct Vertex {
    position: Vector3<f32>,
    color: u32,
}

/// See module docs.
pub struct DebugRenderer {
    geometry: Box<dyn GeometryBuffer>,
    vertices: Vec<Vertex>,
    line_indices: Vec<[u32; 2]>,
    shader: DebugShader,
}

pub(crate) struct DebugShader {
    program: Box<dyn GpuProgram>,
    pub uniform_buffer_binding: usize,
}

/// "Draws" a rectangle into a list of lines.
pub fn draw_rect(rect: &Rect<f32>, lines: &mut Vec<Line>, color: Color) {
    for (a, b) in [
        (rect.left_top_corner(), rect.right_top_corner()),
        (rect.right_top_corner(), rect.right_bottom_corner()),
        (rect.right_bottom_corner(), rect.left_bottom_corner()),
        (rect.left_bottom_corner(), rect.left_top_corner()),
    ] {
        lines.push(Line {
            begin: a.to_homogeneous(),
            end: b.to_homogeneous(),
            color,
        });
    }
}

impl DebugShader {
    fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/debug_fs.glsl");
        let vertex_source = include_str!("shaders/debug_vs.glsl");
        let program = server.create_program("DebugShader", vertex_source, fragment_source)?;
        Ok(Self {
            uniform_buffer_binding: program
                .uniform_block_index(&ImmutableString::new("Uniforms"))?,
            program,
        })
    }
}

impl DebugRenderer {
    pub(crate) fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let desc = GeometryBufferDescriptor {
            element_kind: ElementKind::Line,
            buffers: &[VertexBufferDescriptor {
                usage: BufferUsage::DynamicDraw,
                attributes: &[
                    AttributeDefinition {
                        location: 0,
                        divisor: 0,
                        kind: AttributeKind::Float,
                        component_count: 3,
                        normalized: false,
                    },
                    AttributeDefinition {
                        location: 1,
                        kind: AttributeKind::UnsignedByte,
                        component_count: 4,
                        normalized: true,
                        divisor: 0,
                    },
                ],
                data: VertexBufferData::new::<Vertex>(None),
            }],
            usage: BufferUsage::DynamicDraw,
        };

        Ok(Self {
            geometry: server.create_geometry_buffer(desc)?,
            shader: DebugShader::new(server)?,
            vertices: Default::default(),
            line_indices: Default::default(),
        })
    }

    /// Uploads the new set of lines to GPU.
    pub fn set_lines(&mut self, lines: &[Line]) {
        self.vertices.clear();
        self.line_indices.clear();

        let mut i = 0;
        for line in lines.iter() {
            let color = line.color.into();
            self.vertices.push(Vertex {
                position: line.begin,
                color,
            });
            self.vertices.push(Vertex {
                position: line.end,
                color,
            });
            self.line_indices.push([i, i + 1]);
            i += 2;
        }
        self.geometry.set_buffer_data_of_type(0, &self.vertices);
        self.geometry.set_lines(&self.line_indices);
    }

    pub(crate) fn render(
        &mut self,
        uniform_buffer_cache: &mut UniformBufferCache,
        viewport: Rect<i32>,
        framebuffer: &mut dyn FrameBuffer,
        view_projection: Matrix4<f32>,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

        let uniform_buffer =
            uniform_buffer_cache.write(StaticUniformBuffer::<256>::new().with(&view_projection))?;

        statistics += framebuffer.draw(
            &*self.geometry,
            viewport,
            &*self.shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: Some(CompareFunc::Less),
                blend: None,
                stencil_op: Default::default(),
                scissor_box: None,
            },
            &[ResourceBindGroup {
                bindings: &[ResourceBinding::Buffer {
                    buffer: uniform_buffer,
                    binding: BufferLocation::Auto {
                        shader_location: self.shader.uniform_buffer_binding,
                    },
                    data_usage: Default::default(),
                }],
            }],
            ElementRange::Full,
        )?;

        statistics.draw_calls += 1;

        Ok(statistics)
    }
}
