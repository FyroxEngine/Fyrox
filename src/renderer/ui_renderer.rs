use std::ffi::CString;

use crate::core::math::mat4::Mat4;
use crate::{
    renderer::{
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
    },
    gui::{
        draw::{DrawingContext, CommandKind},
        self,
    },
    gui::draw::CommandTexture,
};
use crate::renderer::RenderPassStatistics;
use std::sync::{Mutex};
use crate::resource::texture::Texture;
use std::any::Any;

struct UIShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
    is_font: UniformLocation,
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
}

pub struct UIRenderer {
    shader: UIShader,
    geometry_buffer: GeometryBuffer<gui::draw::Vertex>,
}

impl UIRenderer {
    pub(in crate::renderer) fn new() -> Result<Self, RendererError> {
        let geometry_buffer = GeometryBuffer::new(GeometryBufferKind::DynamicDraw);

        geometry_buffer.describe_attributes(vec![
            AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
            AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
            AttributeDefinition { kind: AttributeKind::UnsignedByte4, normalized: true },
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

            for cmd in drawing_context.get_commands() {
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
                        match cmd.get_texture() {
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