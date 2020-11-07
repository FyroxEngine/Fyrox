use crate::core::algebra::{Matrix4, Vector2, Vector4};
use crate::resource::texture::{TextureData, TextureKind, TextureState};
use crate::{
    core::{color::Color, math::Rect, scope_profile},
    gui::{
        self,
        brush::Brush,
        draw::{CommandKind, CommandTexture, DrawingContext, SharedTexture},
    },
    renderer::{
        error::RendererError,
        framework::{
            framebuffer::{
                BackBuffer, CullFace, DrawParameters, DrawPartContext, FrameBufferTrait,
            },
            geometry_buffer::{
                AttributeDefinition, AttributeKind, ElementKind, GeometryBuffer, GeometryBufferKind,
            },
            gl,
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::GpuTexture,
            state::{ColorMask, State, StencilFunc, StencilOp},
        },
        RenderPassStatistics, TextureCache,
    },
    resource::texture::{Texture, TexturePixelKind},
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

struct UiShader {
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

impl UiShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/ui_fs.glsl");
        let vertex_source = include_str!("shaders/ui_vs.glsl");
        let program = GpuProgram::from_source("UIShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            is_font: program.uniform_location("isFont")?,
            solid_color: program.uniform_location("solidColor")?,
            brush_type: program.uniform_location("brushType")?,
            gradient_point_count: program.uniform_location("gradientPointCount")?,
            gradient_colors: program.uniform_location("gradientColors")?,
            gradient_stops: program.uniform_location("gradientStops")?,
            gradient_origin: program.uniform_location("gradientOrigin")?,
            gradient_end: program.uniform_location("gradientEnd")?,
            bounds_min: program.uniform_location("boundsMin")?,
            bounds_max: program.uniform_location("boundsMax")?,
            resolution: program.uniform_location("resolution")?,
            program,
        })
    }
}

pub struct UiRenderer {
    shader: UiShader,
    geometry_buffer: GeometryBuffer<gui::draw::Vertex>,
}

pub(in crate) struct UiRenderContext<'a, 'b, 'c> {
    pub state: &'a mut State,
    pub viewport: Rect<i32>,
    pub backbuffer: &'b mut BackBuffer,
    pub frame_width: f32,
    pub frame_height: f32,
    pub drawing_context: &'c DrawingContext,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub texture_cache: &'a mut TextureCache,
}

impl UiRenderer {
    pub(in crate::renderer) fn new(state: &mut State) -> Result<Self, RendererError> {
        let geometry_buffer =
            GeometryBuffer::new(GeometryBufferKind::DynamicDraw, ElementKind::Triangle);

        geometry_buffer.bind(state).describe_attributes(vec![
            AttributeDefinition {
                kind: AttributeKind::Float2,
                normalized: false,
            },
            AttributeDefinition {
                kind: AttributeKind::Float2,
                normalized: false,
            },
            AttributeDefinition {
                kind: AttributeKind::UnsignedByte4,
                normalized: true, // Make sure [0; 255] -> [0; 1]
            },
        ])?;

        Ok(Self {
            geometry_buffer,
            shader: UiShader::new()?,
        })
    }

    pub(in crate::renderer) fn render(
        &mut self,
        args: UiRenderContext,
    ) -> Result<RenderPassStatistics, RendererError> {
        scope_profile!();

        let UiRenderContext {
            state,
            viewport,
            backbuffer,
            frame_width,
            frame_height,
            drawing_context,
            white_dummy,
            texture_cache,
        } = args;

        let mut statistics = RenderPassStatistics::default();

        state.set_blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        let geometry_buffer = self.geometry_buffer.bind(state);

        geometry_buffer
            .set_triangles(drawing_context.get_triangles())
            .set_vertices(drawing_context.get_vertices());

        let ortho = Matrix4::new_orthographic(0.0, frame_width, frame_height, 0.0, -1.0, 1.0);

        for cmd in drawing_context.get_commands() {
            let mut diffuse_texture = white_dummy.clone();
            let mut is_font_texture = false;
            let mut color_write = true;

            match cmd.kind {
                CommandKind::Clip => {
                    if cmd.nesting == 1 {
                        backbuffer.clear(state, viewport, None, None, Some(0));
                    }
                    state.set_stencil_op(StencilOp {
                        zpass: gl::INCR,
                        ..Default::default()
                    });
                    // Make sure that clipping rect will be drawn at previous nesting level only (clip to parent)
                    state.set_stencil_func(StencilFunc {
                        func: gl::EQUAL,
                        ref_value: i32::from(cmd.nesting - 1),
                        ..Default::default()
                    });
                    // Draw clipping geometry to stencil buffers
                    state.set_stencil_mask(0xFF);
                    color_write = false;
                }
                CommandKind::Geometry => {
                    // Make sure to draw geometry only on clipping geometry with current nesting level
                    state.set_stencil_func(StencilFunc {
                        func: gl::EQUAL,
                        ref_value: i32::from(cmd.nesting),
                        ..Default::default()
                    });

                    match &cmd.texture {
                        CommandTexture::Font(font_arc) => {
                            let mut font = font_arc.0.lock().unwrap();
                            if font.texture.is_none() {
                                let size = font.atlas_size() as u32;
                                if let Ok(details) = TextureData::from_bytes(
                                    TextureKind::Rectangle {
                                        width: size,
                                        height: size,
                                    },
                                    TexturePixelKind::R8,
                                    font.atlas_pixels().to_vec(),
                                ) {
                                    font.texture = Some(SharedTexture(Arc::new(Mutex::new(
                                        TextureState::Ok(details),
                                    ))));
                                }
                            }
                            let tex = font
                                .texture
                                .clone()
                                .unwrap()
                                .0
                                .downcast::<Mutex<TextureState>>()
                                .unwrap();
                            if let Some(texture) = texture_cache.get(state, Texture::from(tex)) {
                                diffuse_texture = texture;
                            }
                            is_font_texture = true;
                        }
                        CommandTexture::Texture(texture) => {
                            if let Ok(texture) = texture.clone().0.downcast::<Mutex<TextureState>>()
                            {
                                if let Some(texture) =
                                    texture_cache.get(state, Texture::from(texture))
                                {
                                    diffuse_texture = texture;
                                }
                            }
                        }
                        _ => (),
                    }

                    // Do not draw geometry to stencil buffer
                    state.set_stencil_mask(0);
                }
            }

            let mut raw_stops = [0.0; 16];
            let mut raw_colors = [Vector4::default(); 16];

            let uniforms = [
                (
                    self.shader.diffuse_texture,
                    UniformValue::Sampler {
                        index: 0,
                        texture: diffuse_texture,
                    },
                ),
                (self.shader.wvp_matrix, UniformValue::Matrix4(ortho)),
                (
                    self.shader.resolution,
                    UniformValue::Vector2(Vector2::new(frame_width, frame_height)),
                ),
                (
                    self.shader.bounds_min,
                    UniformValue::Vector2(cmd.bounds.min),
                ),
                (
                    self.shader.bounds_max,
                    UniformValue::Vector2(cmd.bounds.max),
                ),
                (self.shader.is_font, UniformValue::Bool(is_font_texture)),
                (
                    self.shader.brush_type,
                    UniformValue::Integer({
                        match cmd.brush {
                            Brush::Solid(_) => 0,
                            Brush::LinearGradient { .. } => 1,
                            Brush::RadialGradient { .. } => 2,
                        }
                    }),
                ),
                (
                    self.shader.solid_color,
                    UniformValue::Color({
                        match cmd.brush {
                            Brush::Solid(color) => color,
                            _ => Color::WHITE,
                        }
                    }),
                ),
                (
                    self.shader.gradient_origin,
                    UniformValue::Vector2({
                        match cmd.brush {
                            Brush::Solid(_) => Vector2::default(),
                            Brush::LinearGradient { from, .. } => from,
                            Brush::RadialGradient { center, .. } => center,
                        }
                    }),
                ),
                (
                    self.shader.gradient_end,
                    UniformValue::Vector2({
                        match cmd.brush {
                            Brush::Solid(_) => Vector2::default(),
                            Brush::LinearGradient { to, .. } => to,
                            Brush::RadialGradient { .. } => Vector2::default(),
                        }
                    }),
                ),
                (
                    self.shader.gradient_point_count,
                    UniformValue::Integer({
                        match &cmd.brush {
                            Brush::Solid(_) => 0,
                            Brush::LinearGradient { stops, .. }
                            | Brush::RadialGradient { stops, .. } => stops.len() as i32,
                        }
                    }),
                ),
                (
                    self.shader.gradient_stops,
                    UniformValue::FloatArray({
                        match &cmd.brush {
                            Brush::Solid(_) => &[],
                            Brush::LinearGradient { stops, .. }
                            | Brush::RadialGradient { stops, .. } => {
                                for (i, point) in stops.iter().enumerate() {
                                    raw_stops[i] = point.stop;
                                }
                                &raw_stops
                            }
                        }
                    }),
                ),
                (
                    self.shader.gradient_colors,
                    UniformValue::Vec4Array({
                        match &cmd.brush {
                            Brush::Solid(_) => &[],
                            Brush::LinearGradient { stops, .. }
                            | Brush::RadialGradient { stops, .. } => {
                                for (i, point) in stops.iter().enumerate() {
                                    raw_colors[i] = point.color.as_frgba();
                                }
                                &raw_colors
                            }
                        }
                    }),
                ),
            ];

            let params = DrawParameters {
                cull_face: CullFace::Back,
                culling: false,
                color_write: ColorMask::all(color_write),
                depth_write: false,
                stencil_test: cmd.nesting != 0,
                depth_test: false,
                blend: true,
            };

            statistics += backbuffer.draw_part(DrawPartContext {
                state,
                viewport,
                geometry: &mut self.geometry_buffer,
                program: &mut self.shader.program,
                params,
                uniforms: &uniforms,
                offset: cmd.triangles.start,
                count: cmd.triangles.end - cmd.triangles.start,
            })?;
        }
        Ok(statistics)
    }
}
