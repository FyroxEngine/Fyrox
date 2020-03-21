use std::{
    rc::Rc,
    sync::{
        Mutex,
        Arc,
    },
    cell::RefCell,
};
use crate::{
    renderer::{
        framebuffer::{
            BackBuffer,
            FrameBufferTrait,
            DrawParameters,
            CullFace,
        },
        state::State,
        gpu_program::{
            UniformValue,
            GpuProgram,
            UniformLocation,
        },
        RenderPassStatistics,
        gl,
        error::RendererError,
        geometry_buffer::{
            GeometryBuffer,
            AttributeDefinition,
            AttributeKind,
            GeometryBufferKind,
        },
        gpu_texture::GpuTexture,
        geometry_buffer::ElementKind,
        TextureCache,
    },
    gui::{
        brush::Brush,
        draw::{
            DrawingContext,
            CommandKind,
            CommandTexture,
        },
        self,
    },
    resource::texture::{
        Texture,
        TextureKind,
    },
    core::{
        math::{
            Rect,
            mat4::Mat4,
            vec4::Vec4,
            vec2::Vec2,
        },
        color::Color,
    },
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
        let mut program = GpuProgram::from_source("UIShader", vertex_source, fragment_source)?;
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
}

pub struct UiRenderer {
    shader: UiShader,
    geometry_buffer: GeometryBuffer<gui::draw::Vertex>,
}

impl UiRenderer {
    pub(in crate::renderer) fn new() -> Result<Self, RendererError> {
        let mut geometry_buffer = GeometryBuffer::new(GeometryBufferKind::DynamicDraw, ElementKind::Triangle);

        geometry_buffer.bind()
            .describe_attributes(vec![
                AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
                AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
            ])?;

        Ok(Self {
            geometry_buffer,
            shader: UiShader::new()?,
        })
    }

    pub(in crate::renderer) fn render(&mut self,
                                      state: &mut State,
                                      viewport: Rect<i32>,
                                      backbuffer: &mut BackBuffer,
                                      frame_width: f32,
                                      frame_height: f32,
                                      drawing_context: &DrawingContext,
                                      white_dummy: Rc<RefCell<GpuTexture>>,
                                      texture_cache: &mut TextureCache,
    ) -> Result<RenderPassStatistics, RendererError> {
        let mut statistics = RenderPassStatistics::default();


        state.set_blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        let mut geometry_buffer = self.geometry_buffer.bind();

        geometry_buffer
            .set_triangles(drawing_context.get_triangles())
            .set_vertices(drawing_context.get_vertices());

        let ortho = Mat4::ortho(0.0, frame_width, frame_height,
                                0.0, -1.0, 1.0);

        for cmd in drawing_context.get_commands() {
            let mut diffuse_texture = white_dummy.clone();
            let mut is_font_texture = false;
            let mut color_write = true;

            match cmd.get_kind() {
                CommandKind::Clip => {
                    if cmd.get_nesting() == 1 {
                        backbuffer.clear(state, viewport, None, None, Some(0));
                    }
                    unsafe {
                        gl::StencilOp(gl::KEEP, gl::KEEP, gl::INCR);
                        // Make sure that clipping rect will be drawn at previous nesting level only (clip to parent)
                        gl::StencilFunc(gl::EQUAL, i32::from(cmd.get_nesting() - 1), 0xFF);
                    }
                    // Draw clipping geometry to stencil buffer
                    state.set_stencil_mask(0xFF);
                    color_write = false;
                }
                CommandKind::Geometry => {
                    unsafe {
                        // Make sure to draw geometry only on clipping geometry with current nesting level
                        gl::StencilFunc(gl::EQUAL, i32::from(cmd.get_nesting()), 0xFF);
                    }

                    match cmd.texture() {
                        CommandTexture::Font(font_arc) => {
                            let mut font = font_arc.lock().unwrap();
                            if font.texture.is_none() {
                                let tex = Texture::from_bytes(
                                    font.get_atlas_size() as u32,
                                    font.get_atlas_size() as u32,
                                    TextureKind::R8,
                                    font.get_atlas_pixels().to_vec(),
                                );
                                font.texture = Some(Arc::new(Mutex::new(tex)));
                            }
                            if let Some(texture) = texture_cache.get(font.texture.clone().unwrap().downcast::<Mutex<Texture>>().unwrap()) {
                                diffuse_texture = texture;
                            }
                            is_font_texture = true;
                        }
                        CommandTexture::Texture(texture) => {
                            if let Ok(texture) = texture.clone().downcast::<Mutex<Texture>>() {
                                if let Some(texture) = texture_cache.get(texture) {
                                    diffuse_texture = texture;
                                }
                            }
                        }
                        _ => ()
                    }

                    // Do not draw geometry to stencil buffer
                    state.set_stencil_mask(0);
                }
            }

            let mut raw_stops = [0.0; 16];
            let mut raw_colors = [Vec4::default(); 16];

            let uniforms = [
                (self.shader.diffuse_texture, UniformValue::Sampler { index: 0, texture: diffuse_texture }),
                (self.shader.wvp_matrix, UniformValue::Mat4(ortho)),
                (self.shader.resolution, UniformValue::Vec2(Vec2::new(frame_width, frame_height))),
                (self.shader.bounds_min, UniformValue::Vec2(cmd.min())),
                (self.shader.bounds_max, UniformValue::Vec2(cmd.max())),
                (self.shader.is_font, UniformValue::Bool(is_font_texture)),
                (self.shader.brush_type, UniformValue::Integer({
                    match cmd.brush() {
                        Brush::Solid(_) => 0,
                        Brush::LinearGradient { .. } => 1,
                        Brush::RadialGradient { .. } => 2,
                    }
                })),
                (self.shader.solid_color, UniformValue::Color({
                    match cmd.brush() {
                        Brush::Solid(color) => *color,
                        _ => Color::WHITE,
                    }
                })),
                (self.shader.gradient_origin, UniformValue::Vec2({
                    match cmd.brush() {
                        Brush::Solid(_) => Vec2::ZERO,
                        Brush::LinearGradient { from, .. } => *from,
                        Brush::RadialGradient { center, .. } => *center,
                    }
                })),
                (self.shader.gradient_end, UniformValue::Vec2({
                    match cmd.brush() {
                        Brush::Solid(_) => Vec2::ZERO,
                        Brush::LinearGradient { to, .. } => *to,
                        Brush::RadialGradient { .. } => Vec2::ZERO,
                    }
                })),
                (self.shader.gradient_point_count, UniformValue::Integer({
                    match cmd.brush() {
                        Brush::Solid(_) => 0,
                        Brush::LinearGradient { stops, .. } | Brush::RadialGradient { stops, .. } => stops.len() as i32,
                    }
                })),
                (self.shader.gradient_stops, UniformValue::FloatArray({
                    match cmd.brush() {
                        Brush::Solid(_) => &[],
                        Brush::LinearGradient { stops, .. } | Brush::RadialGradient { stops, .. } => {
                            for (i, point) in stops.iter().enumerate() {
                                raw_stops[i] = point.stop;
                            }
                            &raw_stops
                        }
                    }
                })),
                (self.shader.gradient_colors, UniformValue::Vec4Array({
                    match cmd.brush() {
                        Brush::Solid(_) => &[],
                        Brush::LinearGradient { stops, .. } | Brush::RadialGradient { stops, .. } => {
                            for (i, point) in stops.iter().enumerate() {
                                raw_colors[i] = point.color.as_frgba();
                            }
                            &raw_colors
                        }
                    }
                }))
            ];

            let params = DrawParameters {
                cull_face: CullFace::Back,
                culling: false,
                color_write: (color_write, color_write, color_write, color_write),
                depth_write: false,
                stencil_test: cmd.get_nesting() != 0,
                depth_test: false,
                blend: true,
            };

            statistics.add_draw_call(
                backbuffer.draw_part(
                    state,
                    viewport,
                    &mut self.geometry_buffer,
                    &mut self.shader.program,
                    params,
                    &uniforms,
                    cmd.get_start_triangle(),
                    cmd.get_triangle_count(),
                )?
            );
        }
        Ok(statistics)
    }
}