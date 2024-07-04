//! Debug renderer allows you to create debug geometry (wireframe) on the fly. As it said
//! in its name its purpose - output debug information. It can be used to render collision
//! shapes, contact information (normals, positions, etc.), paths build by navmesh and so
//! on. It contains implementations to draw most common shapes (line, box, oob, frustum, etc).

use crate::core::sstorage::ImmutableString;
use crate::renderer::framework::geometry_buffer::ElementRange;
use crate::{
    core::{algebra::Vector3, math::Rect, scope_profile},
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{DrawParameters, FrameBuffer},
        geometry_buffer::{
            AttributeDefinition, AttributeKind, BufferBuilder, ElementKind, GeometryBuffer,
            GeometryBufferBuilder, GeometryBufferKind,
        },
        gpu_program::{GpuProgram, UniformLocation},
        state::PipelineState,
    },
    renderer::RenderPassStatistics,
    scene::{camera::Camera, debug::SceneDrawingContext},
};
use bytemuck::{Pod, Zeroable};

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

impl DebugShader {
    fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/debug_fs.glsl");
        let vertex_source = include_str!("shaders/debug_vs.glsl");
        let program =
            GpuProgram::from_source(state, "DebugShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            program,
        })
    }
}

impl DebugRenderer {
    pub(crate) fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let geometry = GeometryBufferBuilder::new(ElementKind::Line)
            .with_buffer_builder(
                BufferBuilder::new::<Vertex>(GeometryBufferKind::DynamicDraw, None)
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
            .build(state)?;

        Ok(Self {
            geometry,
            shader: DebugShader::new(state)?,
            vertices: Default::default(),
            line_indices: Default::default(),
        })
    }

    pub(crate) fn render(
        &mut self,
        state: &PipelineState,
        viewport: Rect<i32>,
        framebuffer: &mut FrameBuffer,
        drawing_context: &SceneDrawingContext,
        camera: &Camera,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        self.vertices.clear();
        self.line_indices.clear();

        let mut i = 0;
        for line in drawing_context.lines.iter() {
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
        self.geometry.set_buffer_data(state, 0, &self.vertices);
        self.geometry.bind(state).set_lines(&self.line_indices);

        statistics += framebuffer.draw(
            &self.geometry,
            state,
            viewport,
            &self.shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: true,
                blend: None,
                stencil_op: Default::default(),
            },
            ElementRange::Full,
            |mut program_binding| {
                program_binding
                    .set_matrix4(&self.shader.wvp_matrix, &camera.view_projection_matrix());
            },
        )?;

        statistics.draw_calls += 1;

        Ok(statistics)
    }
}
