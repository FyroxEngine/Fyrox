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
    core::{algebra::Vector3, math::Rect, sstorage::ImmutableString},
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::FrameBuffer,
            geometry_buffer::{
                AttributeDefinition, AttributeKind, BufferBuilder, GeometryBuffer,
                GeometryBufferBuilder,
            },
            gpu_program::{GpuProgram, UniformLocation},
            state::GlGraphicsServer,
            DrawParameters, ElementKind, ElementRange,
        },
        RenderPassStatistics,
    },
    scene::debug::Line,
};
use bytemuck::{Pod, Zeroable};
use fyrox_core::color::Color;
use fyrox_graphics::buffer::BufferUsage;
use fyrox_graphics::CompareFunc;
use rapier2d::na::Matrix4;

#[repr(C)]
#[derive(Copy, Pod, Zeroable, Clone)]
struct Vertex {
    position: Vector3<f32>,
    color: u32,
}

/// See module docs.
pub struct DebugRenderer {
    geometry: GeometryBuffer,
    vertices: Vec<Vertex>,
    line_indices: Vec<[u32; 2]>,
    shader: DebugShader,
}

pub(crate) struct DebugShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
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
    fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/debug_fs.glsl");
        let vertex_source = include_str!("shaders/debug_vs.glsl");
        let program =
            GpuProgram::from_source(server, "DebugShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program
                .uniform_location(server, &ImmutableString::new("worldViewProjection"))?,
            program,
        })
    }
}

impl DebugRenderer {
    pub(crate) fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let geometry = GeometryBufferBuilder::new(ElementKind::Line)
            .with_buffer_builder(
                BufferBuilder::new::<Vertex>(BufferUsage::DynamicDraw, None)
                    .with_attribute(AttributeDefinition {
                        location: 0,
                        divisor: 0,
                        kind: AttributeKind::Float3,
                        normalized: false,
                    })
                    .with_attribute(AttributeDefinition {
                        location: 1,
                        kind: AttributeKind::UnsignedByte4,
                        normalized: true,
                        divisor: 0,
                    }),
            )
            .build(server)?;

        Ok(Self {
            geometry,
            shader: DebugShader::new(server)?,
            vertices: Default::default(),
            line_indices: Default::default(),
        })
    }

    /// Uploads the new set of lines to GPU.
    pub fn set_lines(&mut self, server: &GlGraphicsServer, lines: &[Line]) {
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
        self.geometry.set_buffer_data(0, &self.vertices);
        self.geometry.bind(server).set_lines(&self.line_indices);
    }

    pub(crate) fn render(
        &mut self,
        server: &GlGraphicsServer,
        viewport: Rect<i32>,
        framebuffer: &mut FrameBuffer,
        view_projection: Matrix4<f32>,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

        statistics += framebuffer.draw(
            &self.geometry,
            server,
            viewport,
            &self.shader.program,
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
            ElementRange::Full,
            |mut program_binding| {
                program_binding.set_matrix4(&self.shader.wvp_matrix, &view_projection);
            },
        )?;

        statistics.draw_calls += 1;

        Ok(statistics)
    }
}
