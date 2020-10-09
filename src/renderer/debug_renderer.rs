//! Debug renderer allows you to create debug geometry (wireframe) on the fly. As it said
//! in its name its purpose - output debug information. It can be used to render collision
//! shapes, contact information (normals, positions, etc.), paths build by navmesh and so
//! on. It contains implementations to draw most common shapes (line, box, oob, frustum, etc).

use crate::{
    core::{
        math::{vec3::Vec3, Rect},
        scope_profile,
    },
    renderer::{
        error::RendererError,
        framework::{
            framebuffer::{CullFace, DrawParameters, FrameBuffer, FrameBufferTrait},
            geometry_buffer::{
                AttributeDefinition, AttributeKind, ElementKind, GeometryBuffer, GeometryBufferKind,
            },
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            state::State,
        },
        RenderPassStatistics,
    },
    scene::{camera::Camera, SceneDrawingContext},
};

#[repr(C)]
struct Vertex {
    position: Vec3,
    color: u32,
}

/// See module docs.
pub struct DebugRenderer {
    geometry: GeometryBuffer<Vertex>,
    vertices: Vec<Vertex>,
    line_indices: Vec<[u32; 2]>,
    shader: DebugShader,
}

pub(in crate) struct DebugShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
}

impl DebugShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/debug_fs.glsl");
        let vertex_source = include_str!("shaders/debug_vs.glsl");
        let program = GpuProgram::from_source("DebugShader", &vertex_source, &fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            program,
        })
    }
}

impl DebugRenderer {
    pub(in crate) fn new(state: &mut State) -> Result<Self, RendererError> {
        let geometry = GeometryBuffer::new(GeometryBufferKind::DynamicDraw, ElementKind::Line);

        geometry.bind(state).describe_attributes(vec![
            AttributeDefinition {
                kind: AttributeKind::Float3,
                normalized: false,
            },
            AttributeDefinition {
                kind: AttributeKind::UnsignedByte4,
                normalized: true,
            },
        ])?;

        Ok(Self {
            geometry,
            shader: DebugShader::new()?,
            vertices: Default::default(),
            line_indices: Default::default(),
        })
    }

    pub(in crate) fn render(
        &mut self,
        state: &mut State,
        viewport: Rect<i32>,
        framebuffer: &mut FrameBuffer,
        drawing_context: &SceneDrawingContext,
        camera: &Camera,
    ) -> RenderPassStatistics {
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
        self.geometry
            .bind(state)
            .set_vertices(&self.vertices)
            .set_lines(&self.line_indices);

        statistics += framebuffer.draw(
            &self.geometry,
            state,
            viewport,
            &self.shader.program,
            DrawParameters {
                cull_face: CullFace::Back,
                culling: false,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: false,
                depth_test: true,
                blend: false,
            },
            &[(
                self.shader.wvp_matrix,
                UniformValue::Mat4(camera.view_projection_matrix()),
            )],
        );

        statistics.draw_calls += 1;

        statistics
    }
}
