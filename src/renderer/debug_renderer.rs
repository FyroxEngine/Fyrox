//! Debug renderer allows you to create debug geometry (wireframe) on the fly. As it said
//! in its name its purpose - output debug information. It can be used to render collision
//! shapes, contact information (normals, positions, etc.), paths build by navmesh and so
//! on. It contains implementations to draw most common shapes (line, box, oob, frustum, etc).

use crate::{
    core::{
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, mat4::Mat4, vec3::Vec3, Rect},
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
    scene::camera::Camera,
};

#[repr(C)]
struct Vertex {
    position: Vec3,
    color: u32,
}

/// See module docs.
pub struct DebugRenderer {
    geometry: GeometryBuffer<Vertex>,
    lines: Vec<Line>,
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

/// Colored line between two points.
pub struct Line {
    /// Beginning of the line.
    pub begin: Vec3,
    /// End of the line.    
    pub end: Vec3,
    /// Color of the line.
    pub color: Color,
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
            lines: Default::default(),
            vertices: Default::default(),
            line_indices: Default::default(),
        })
    }

    /// Adds single line into internal buffer.
    pub fn add_line(&mut self, line: Line) {
        self.lines.push(line);
    }

    /// Removes all lines from internal buffer.
    pub fn clear_lines(&mut self) {
        self.lines.clear()
    }

    /// Draws frustum with given color. Drawing is not immediate, it only pushes
    /// lines for frustum into internal buffer. It will be drawn later on in separate
    /// render pass.
    pub fn draw_frustum(&mut self, frustum: &Frustum, color: Color) {
        let left_top_front = frustum.left_top_front_corner();
        let left_bottom_front = frustum.left_bottom_front_corner();
        let right_bottom_front = frustum.right_bottom_front_corner();
        let right_top_front = frustum.right_top_front_corner();

        let left_top_back = frustum.left_top_back_corner();
        let left_bottom_back = frustum.left_bottom_back_corner();
        let right_bottom_back = frustum.right_bottom_back_corner();
        let right_top_back = frustum.right_top_back_corner();

        // Front face
        self.add_line(Line {
            begin: left_top_front,
            end: right_top_front,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: left_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_top_front,
            color,
        });

        // Back face
        self.add_line(Line {
            begin: left_top_back,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_back,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_back,
            end: left_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_back,
            end: left_top_back,
            color,
        });

        // Edges
        self.add_line(Line {
            begin: left_top_front,
            end: left_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_bottom_back,
            color,
        });
    }

    /// Draws axis-aligned bounding box with given color. Drawing is not immediate,
    /// it only pushes lines for bounding box into internal buffer. It will be drawn
    /// later on in separate render pass.
    pub fn draw_aabb(&mut self, aabb: &AxisAlignedBoundingBox, color: Color) {
        let left_bottom_front = Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z);
        let left_top_front = Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z);
        let right_top_front = Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z);
        let right_bottom_front = Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z);

        let left_bottom_back = Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z);
        let left_top_back = Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z);
        let right_top_back = Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z);
        let right_bottom_back = Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z);

        // Front face
        self.add_line(Line {
            begin: left_top_front,
            end: right_top_front,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: left_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_top_front,
            color,
        });

        // Back face
        self.add_line(Line {
            begin: left_top_back,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_back,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_back,
            end: left_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_back,
            end: left_top_back,
            color,
        });

        // Edges
        self.add_line(Line {
            begin: left_top_front,
            end: left_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_bottom_back,
            color,
        });
    }

    /// Draws object-oriented bounding box with given color. Drawing is not immediate,
    /// it only pushes lines for object-oriented bounding box into internal buffer. It
    /// will be drawn later on in separate render pass.
    pub fn draw_oob(&mut self, aabb: &AxisAlignedBoundingBox, transform: Mat4, color: Color) {
        let left_bottom_front =
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z));
        let left_top_front =
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z));
        let right_top_front =
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z));
        let right_bottom_front =
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z));

        let left_bottom_back =
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z));
        let left_top_back =
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z));
        let right_top_back =
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z));
        let right_bottom_back =
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z));

        // Front face
        self.add_line(Line {
            begin: left_top_front,
            end: right_top_front,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: left_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_top_front,
            color,
        });

        // Back face
        self.add_line(Line {
            begin: left_top_back,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_back,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_back,
            end: left_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_back,
            end: left_top_back,
            color,
        });

        // Edges
        self.add_line(Line {
            begin: left_top_front,
            end: left_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_bottom_back,
            color,
        });
    }

    pub(in crate) fn render(
        &mut self,
        state: &mut State,
        viewport: Rect<i32>,
        framebuffer: &mut FrameBuffer,
        camera: &Camera,
    ) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        self.vertices.clear();
        self.line_indices.clear();

        let mut i = 0;
        for line in self.lines.iter() {
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
