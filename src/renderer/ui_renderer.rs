use crate::{
    renderer::{
        RenderPassStatistics,
        gpu_program::{GpuProgram, UniformLocation},
        gl,
        error::RendererError,
        geometry_buffer::{
            GeometryBuffer,
            AttributeDefinition,
            AttributeKind,
            GeometryBufferKind,
        },
        gpu_texture::{GpuTexture, GpuTextureKind, PixelKind},
        geometry_buffer::ElementKind
    },
    gui::{
        brush::Brush,
        draw::{DrawingContext, CommandKind},
        self,
        draw::CommandTexture
    },
    core::math::mat4::Mat4,
    resource::texture::Texture,
};
use std::{
    sync::{Mutex},
    ffi::CString,
    any::Any
};
use rg3d_core::math::{
    vec4::Vec4,
    vec2::Vec2
};
use rg3d_ui::brush::GradientPoint;

struct UIShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
    is_font: UniformLocation,
    solid_color: UniformLocation,
    brush_type: UniformLocation,
    gradient_point_count: UniformLocation,
    gradient_colors: UniformLocation,
    gradient_stops: UniformLocation,
    gradient_origin: UniformLocation,
    gradient_end: UniformLocation,
    resolution: UniformLocation,
    bounds_min: UniformLocation,
    bounds_max: UniformLocation,
}

impl UIShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = CString::new(include_str!("shaders/ui_fs.glsl"))?;
        let vertex_source = CString::new(include_str!("shaders/ui_vs.glsl"))?;
        let mut program = GpuProgram::from_source("UIShader", &vertex_source, &fragment_source)?;
        Ok(Self {
            wvp_matrix: program.get_uniform_location("worldViewProjection")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            is_font: program.get_uniform_location("isFont")?,
            solid_color: program.get_uniform_location("solidColor")?,
            brush_type: program.get_uniform_location("brushType")?,
            gradient_point_count: program.get_uniform_location("gradientPointCount")?,
            gradient_colors: program.get_uniform_location("gradientColors")?,
            gradient_stops: program.get_uniform_location("gradientStops")?,
            gradient_origin: program.get_uniform_location("gradientOrigin")?,
            gradient_end: program.get_uniform_location("gradientEnd")?,
            bounds_min: program.get_uniform_location("boundsMin")?,
            bounds_max: program.get_uniform_location("boundsMax")?,
            resolution: program.get_uniform_location("resolution")?,
            program,
        })
    }

    pub fn bind(&self) {
        self.program.bind()
    }

    pub fn set_wvp_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.wvp_matrix, mat)
    }

    pub fn set_is_font(&self, value: bool) {
        self.program.set_bool(self.is_font, value)
    }

    pub fn set_diffuse_texture_sampler_id(&self, id: i32) {
        self.program.set_int(self.diffuse_texture, id)
    }

    pub fn set_bounds_min(&self, v: Vec2) {
        self.program.set_vec2(self.bounds_min, v);
    }

    pub fn set_bounds_max(&self, v: Vec2) {
        self.program.set_vec2(self.bounds_max, v);
    }

    pub fn set_resolution(&self, v: Vec2) {
        self.program.set_vec2(self.resolution, v);
    }

    fn set_gradient_stops(&self, stops: &[GradientPoint]) {
        let mut raw_stops = [0.0; 16];
        let mut raw_colors = [Vec4::default(); 16];
        for (i, point) in stops.iter().enumerate() {
            raw_stops[i] = point.stop;
            raw_colors[i] = point.color.as_frgba();
        }

        self.program.set_int(self.gradient_point_count, stops.len() as i32);
        self.program.set_float_array(self.gradient_stops, &raw_stops);
        self.program.set_vec4_array(self.gradient_colors, &raw_colors);
    }

    pub fn set_brush(&self, brush: &Brush) {
        match brush {
            Brush::Solid(color) => {
                self.program.set_int(self.brush_type, 0);
                self.program.set_vec4(self.solid_color, &color.as_frgba());
            },
            Brush::LinearGradient { from, to, stops } => {
                self.program.set_int(self.brush_type, 1);
                self.program.set_vec2(self.gradient_origin, *from);
                self.program.set_vec2(self.gradient_end, *to);
                self.set_gradient_stops(stops);
            },
            Brush::RadialGradient { center, stops } => {
                self.program.set_int(self.brush_type, 2);
                self.program.set_vec2(self.gradient_origin, *center);
                self.set_gradient_stops(stops);
            },
        }
    }
}

pub struct UIRenderer {
    shader: UIShader,
    geometry_buffer: GeometryBuffer<gui::draw::Vertex>,
}

impl UIRenderer {
    pub(in crate::renderer) fn new() -> Result<Self, RendererError> {
        let geometry_buffer = GeometryBuffer::new(GeometryBufferKind::DynamicDraw, ElementKind::Triangle);

        geometry_buffer.describe_attributes(vec![
            AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
            AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
        ])?;

        Ok(Self {
            geometry_buffer,
            shader: UIShader::new()?,
        })
    }

    pub(in crate::renderer) fn render(&mut self,
                                      frame_width: f32,
                                      frame_height: f32,
                                      drawing_context: &DrawingContext,
                                      white_dummy: &GpuTexture) -> Result<RenderPassStatistics, RendererError> {
        let mut statistics = RenderPassStatistics::default();

        unsafe {
            // Render UI on top of everything
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Disable(gl::CULL_FACE);

            self.shader.bind();
            gl::ActiveTexture(gl::TEXTURE0);

            self.geometry_buffer.set_triangles(drawing_context.get_triangles());
            self.geometry_buffer.set_vertices(drawing_context.get_vertices());

            let ortho = Mat4::ortho(0.0, frame_width, frame_height,
                                    0.0, -1.0, 1.0);
            self.shader.set_wvp_matrix(&ortho);
            self.shader.set_resolution(Vec2::new(frame_width, frame_height));

            for cmd in drawing_context.get_commands() {
                self.shader.set_bounds_min(cmd.min());
                self.shader.set_bounds_max(cmd.max());

                if cmd.get_nesting() != 0 {
                    gl::Enable(gl::STENCIL_TEST);
                } else {
                    gl::Disable(gl::STENCIL_TEST);
                }

                match cmd.get_kind() {
                    CommandKind::Clip => {
                        if cmd.get_nesting() == 1 {
                            gl::Clear(gl::STENCIL_BUFFER_BIT);
                        }
                        gl::StencilOp(gl::KEEP, gl::KEEP, gl::INCR);
                        // Make sure that clipping rect will be drawn at previous nesting level only (clip to parent)
                        gl::StencilFunc(gl::EQUAL, i32::from(cmd.get_nesting() - 1), 0xFF);
                        // Draw clipping geometry to stencil buffer
                        gl::StencilMask(0xFF);
                        gl::ColorMask(gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
                    }
                    CommandKind::Geometry => {
                        // Make sure to draw geometry only on clipping geometry with current nesting level
                        gl::StencilFunc(gl::EQUAL, i32::from(cmd.get_nesting()), 0xFF);

                        self.shader.set_diffuse_texture_sampler_id(0);

                        match cmd.texture() {
                            CommandTexture::None => white_dummy.bind(0),
                            CommandTexture::Font(font) => {
                                let mut font = font.lock().unwrap();
                                if font.texture.is_none() {
                                    font.texture = Some(Box::new(GpuTexture::new(
                                        GpuTextureKind::Rectangle {
                                            width: font.get_atlas_size() as usize,
                                            height: font.get_atlas_size() as usize,
                                        }, PixelKind::R8, font.get_atlas_pixels(),
                                        false).unwrap()
                                    ));
                                }
                                (font.texture.as_ref().unwrap().as_ref() as &dyn Any)
                                    .downcast_ref::<GpuTexture>().unwrap().bind(0);
                                self.shader.set_is_font(true);
                            }
                            CommandTexture::Texture(texture) => {
                                let texture = texture.clone().downcast::<Mutex<Texture>>();
                                if let Ok(texture) = texture {
                                    let texture = texture.lock().unwrap();
                                    if let Some(texture) = &texture.gpu_tex {
                                        texture.bind(0)
                                    }
                                }

                                self.shader.set_is_font(false);
                            }
                        }

                        self.shader.set_brush(cmd.brush());

                        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
                        // Do not draw geometry to stencil buffer
                        gl::StencilMask(0x00);
                    }
                }

                statistics.triangles_rendered += self.geometry_buffer.draw_part(cmd.get_start_triangle(), cmd.get_triangle_count())?;
                statistics.draw_calls += 1;
            }
        }
        Ok(statistics)
    }
}