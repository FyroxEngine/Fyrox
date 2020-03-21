#[allow(clippy::all)]
pub(in crate) mod gl;
pub mod surface;
pub mod gpu_program;
pub mod error;
pub mod debug_renderer;

macro_rules! check_gl_error {
    () => (crate::renderer::check_gl_error_internal(line!(), file!()))
}

mod geometry_buffer;
mod ui_renderer;
mod particle_system_renderer;
mod gbuffer;
mod deferred_light_renderer;
mod shadow_map_renderer;
mod flat_shader;
mod gpu_texture;
mod sprite_renderer;
mod framebuffer;
mod state;

use glutin::PossiblyCurrent;
use std::{
    rc::Rc,
    sync::{
        Arc,
        Mutex,
    },
    time,
    ffi::CStr,
    collections::HashMap,
    cell::RefCell,
};
use crate::{
    resource::texture::Texture,
    renderer::{
        ui_renderer::UiRenderer,
        surface::SurfaceSharedData,
        particle_system_renderer::ParticleSystemRenderer,
        gbuffer::GBuffer,
        deferred_light_renderer::{
            DeferredLightRenderer,
            DeferredRendererContext,
        },
        error::RendererError,
        gpu_texture::{
            GpuTexture,
            GpuTextureKind,
            PixelKind,
            MininificationFilter,
            MagnificationFilter,
        },
        flat_shader::FlatShader,
        sprite_renderer::SpriteRenderer,
        gl::types::{GLsizei, GLenum, GLuint, GLchar},
        debug_renderer::DebugRenderer,
        geometry_buffer::{
            GeometryBuffer,
            GeometryBufferKind,
            ElementKind,
            AttributeKind,
            AttributeDefinition,
        },
        framebuffer::{BackBuffer, FrameBufferTrait, DrawParameters, CullFace},
        gpu_program::UniformValue,
    },
    scene::{
        SceneContainer,
        node::Node,
    },
    core::{
        math::{
            vec3::Vec3,
            mat4::Mat4,
            vec2::Vec2,
            TriangleDefinition,
        },
        color::Color,
        math::Rect,
        pool::Handle,
    },
    utils::log::Log,
    gui::draw::DrawingContext,
    engine::resource_manager::TimedEntry,
};
use crate::renderer::state::State;

#[derive(Copy, Clone)]
pub struct Statistics {
    /// Geometry statistics.
    pub geometry: RenderPassStatistics,
    /// Real time consumed to render frame.
    pub pure_frame_time: f32,
    /// Total time renderer took to process single frame, usually includes
    /// time renderer spend to wait to buffers swap (can include vsync)
    pub capped_frame_time: f32,
    /// Total amount of frames been rendered in one second.
    pub frames_per_second: usize,
    frame_counter: usize,
    frame_start_time: time::Instant,
    last_fps_commit_time: time::Instant,
}

#[derive(Copy, Clone)]
pub struct RenderPassStatistics {
    pub draw_calls: usize,
    pub triangles_rendered: usize,
}

impl Default for RenderPassStatistics {
    fn default() -> Self {
        Self {
            draw_calls: 0,
            triangles_rendered: 0,
        }
    }
}

impl std::ops::AddAssign for RenderPassStatistics {
    fn add_assign(&mut self, rhs: Self) {
        self.draw_calls += rhs.draw_calls;
        self.triangles_rendered += rhs.triangles_rendered;
    }
}

impl RenderPassStatistics {
    pub fn add_draw_call(&mut self, triangles_rendered: usize) {
        self.triangles_rendered += triangles_rendered;
        self.draw_calls += 1;
    }
}

impl std::ops::AddAssign<RenderPassStatistics> for Statistics {
    fn add_assign(&mut self, rhs: RenderPassStatistics) {
        self.geometry += rhs;
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct QualitySettings {
    /// Point shadows
    /// Size of cube map face of shadow map texture in pixels.
    pub point_shadow_map_size: usize,
    /// Use or not percentage close filtering (smoothing) for point shadows.
    pub point_soft_shadows: bool,
    /// Point shadows enabled or not.
    pub point_shadows_enabled: bool,
    /// Maximum distance from camera to draw shadows.
    pub point_shadows_distance: f32,

    /// Spot shadows
    /// Size of square shadow map texture in pixels
    pub spot_shadow_map_size: usize,
    /// Use or not percentage close filtering (smoothing) for spot shadows.
    pub spot_soft_shadows: bool,
    /// Spot shadows enabled or not.
    pub spot_shadows_enabled: bool,
    /// Maximum distance from camera to draw shadows.
    pub spot_shadows_distance: f32,
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self {
            point_shadow_map_size: 1024,
            point_shadows_distance: 5.0,
            point_shadows_enabled: true,
            point_soft_shadows: true,

            spot_shadow_map_size: 1024,
            spot_shadows_distance: 5.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: true,
        }
    }
}

impl Statistics {
    /// Must be called before render anything.
    fn begin_frame(&mut self) {
        self.frame_start_time = time::Instant::now();
        self.geometry = Default::default();
    }

    /// Must be called before SwapBuffers but after all rendering is done.
    fn end_frame(&mut self) {
        let current_time = time::Instant::now();

        self.pure_frame_time = current_time.duration_since(self.frame_start_time).as_secs_f32();
        self.frame_counter += 1;

        if current_time.duration_since(self.last_fps_commit_time).as_secs_f32() >= 1.0 {
            self.last_fps_commit_time = current_time;
            self.frames_per_second = self.frame_counter;
            self.frame_counter = 0;
        }
    }

    /// Must be called after SwapBuffers to get capped frame time.
    fn finalize(&mut self) {
        self.capped_frame_time = time::Instant::now().duration_since(self.frame_start_time).as_secs_f32();
    }
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            geometry: RenderPassStatistics::default(),
            pure_frame_time: 0.0,
            capped_frame_time: 0.0,
            frames_per_second: 0,
            frame_counter: 0,
            frame_start_time: time::Instant::now(),
            last_fps_commit_time: time::Instant::now(),
        }
    }
}

pub struct Renderer {
    state: State,
    backbuffer: BackBuffer,
    deferred_light_renderer: DeferredLightRenderer,
    flat_shader: FlatShader,
    sprite_renderer: SpriteRenderer,
    particle_system_renderer: ParticleSystemRenderer,
    /// Dummy white one pixel texture which will be used as stub when rendering
    /// something without texture specified.
    white_dummy: Rc<RefCell<GpuTexture>>,
    /// Dummy one pixel texture with (0, 1, 0) vector is used as stub when rendering
    /// something without normal map.
    normal_dummy: Rc<RefCell<GpuTexture>>,
    ui_renderer: UiRenderer,
    statistics: Statistics,
    quad: SurfaceSharedData,
    frame_size: (u32, u32),
    ambient_color: Color,
    quality_settings: QualitySettings,
    pub debug_renderer: DebugRenderer,
    gbuffers: HashMap<Handle<Node>, GBuffer>,
    backbuffer_clear_color: Color,
    texture_cache: TextureCache,
    geometry_cache: GeometryCache,
}

#[derive(Default)]
pub struct GeometryCache {
    map: HashMap<usize, TimedEntry<GeometryBuffer<surface::Vertex>>>
}

impl GeometryCache {
    fn get(&mut self, data: &SurfaceSharedData) -> &mut GeometryBuffer<surface::Vertex> {
        let key = (data as *const _) as usize;

        let geometry_buffer = self.map.entry(key).or_insert_with(|| {
            let mut triangles = Vec::with_capacity(data.indices.len() / 3);
            for i in (0..data.indices.len()).step_by(3) {
                triangles.push(TriangleDefinition { indices: [data.indices[i], data.indices[i + 1], data.indices[i + 2]] });
            }

            let mut geometry_buffer = GeometryBuffer::new(GeometryBufferKind::StaticDraw, ElementKind::Triangle);

            geometry_buffer.bind()
                .describe_attributes(vec![
                    AttributeDefinition { kind: AttributeKind::Float3, normalized: false },
                    AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
                    AttributeDefinition { kind: AttributeKind::Float3, normalized: false },
                    AttributeDefinition { kind: AttributeKind::Float4, normalized: false },
                    AttributeDefinition { kind: AttributeKind::Float4, normalized: false },
                    AttributeDefinition { kind: AttributeKind::UnsignedByte4, normalized: false }])
                .unwrap()
                .set_vertices(data.vertices.as_slice())
                .set_triangles(&triangles);

            TimedEntry { value: geometry_buffer, time_to_live: 20.0 }
        });

        geometry_buffer.time_to_live = 20.0;
        geometry_buffer
    }

    fn update(&mut self, dt: f32) {
        for entry in self.map.values_mut() {
            entry.time_to_live -= dt;
        }
        self.map.retain(|_, v| {
            v.time_to_live > 0.0
        });
    }
}

#[derive(Default)]
pub struct TextureCache {
    map: HashMap<usize, TimedEntry<Rc<RefCell<GpuTexture>>>>
}

impl TextureCache {
    fn get(&mut self, texture: Arc<Mutex<Texture>>) -> Option<Rc<RefCell<GpuTexture>>> {
        if texture.lock().unwrap().loaded {
            let key = (&*texture as *const _) as usize;
            let gpu_texture = self.map.entry(key).or_insert_with(move || {
                let texture = texture.lock().unwrap();
                let kind = GpuTextureKind::Rectangle {
                    width: texture.width as usize,
                    height: texture.height as usize,
                };
                let mut gpu_texture = GpuTexture::new(
                    kind,
                    PixelKind::from(texture.kind),
                    Some(texture.bytes.as_slice()))
                    .unwrap();
                gpu_texture.bind_mut(0)
                    .generate_mip_maps()
                    .set_minification_filter(MininificationFilter::LinearMip)
                    .set_magnification_filter(MagnificationFilter::Linear)
                    .set_max_anisotropy();
                TimedEntry {
                    value: Rc::new(RefCell::new(gpu_texture)),
                    time_to_live: 20.0,
                }
            });
            // Texture won't be destroyed while it used.
            gpu_texture.time_to_live = 20.0;
            Some(gpu_texture.value.clone())
        } else {
            None
        }
    }

    fn update(&mut self, dt: f32) {
        for entry in self.map.values_mut() {
            entry.time_to_live -= dt;
        }
        self.map.retain(|_, v| {
            v.time_to_live > 0.0
        });
    }
}

impl Renderer {
    pub(in crate) fn new(frame_size: (u32, u32)) -> Result<Self, RendererError> {
        let settings = QualitySettings::default();
        let mut state = State::new();

        Ok(Self {
            backbuffer: BackBuffer {},
            frame_size,
            deferred_light_renderer: DeferredLightRenderer::new(&mut state, &settings)?,
            flat_shader: FlatShader::new()?,
            statistics: Statistics::default(),
            sprite_renderer: SpriteRenderer::new()?,
            white_dummy: Rc::new(RefCell::new(GpuTexture::new(GpuTextureKind::Rectangle { width: 1, height: 1 },
                                                              PixelKind::RGBA8, Some(&[255, 255, 255, 255]))?)),
            normal_dummy: Rc::new(RefCell::new(GpuTexture::new(GpuTextureKind::Rectangle { width: 1, height: 1 },
                                                               PixelKind::RGBA8, Some(&[128, 128, 255, 255]))?)),
            quad: SurfaceSharedData::make_unit_xy_quad(),
            ui_renderer: UiRenderer::new()?,
            particle_system_renderer: ParticleSystemRenderer::new()?,
            ambient_color: Color::opaque(100, 100, 100),
            quality_settings: settings,
            debug_renderer: DebugRenderer::new()?,
            gbuffers: Default::default(),
            backbuffer_clear_color: Color::from_rgba(0, 0, 0, 0),
            texture_cache: Default::default(),
            geometry_cache: Default::default(),
            state
        })
    }

    pub fn set_ambient_color(&mut self, color: Color) {
        self.ambient_color = color;
    }

    pub fn get_ambient_color(&self) -> Color {
        self.ambient_color
    }

    pub fn get_statistics(&self) -> Statistics {
        self.statistics
    }

    pub fn set_backbuffer_clear_color(&mut self, color: Color) {
        self.backbuffer_clear_color = color;
    }

    /// Sets new frame size, should be called when received a Resize event.
    ///
    /// # Notes
    ///
    /// Input values will be set to 1 pixel if new size is 0. Rendering cannot
    /// be performed into 0x0 texture.
    pub fn set_frame_size(&mut self, new_size: (u32, u32)) {
        self.frame_size.0 = new_size.0.max(1);
        self.frame_size.1 = new_size.1.max(1);
        // Invalidate all g-buffers.
        self.gbuffers.clear();
    }

    pub fn get_frame_size(&self) -> (u32, u32) {
        self.frame_size
    }

    pub fn set_quality_settings(&mut self, settings: &QualitySettings) -> Result<(), RendererError> {
        self.quality_settings = *settings;
        self.deferred_light_renderer.set_quality_settings(&mut self.state, settings)
    }

    pub fn get_quality_settings(&self) -> QualitySettings {
        self.quality_settings
    }

    pub(in crate) fn render(&mut self,
                            scenes: &SceneContainer,
                            drawing_context: &DrawingContext,
                            context: &glutin::WindowedContext<PossiblyCurrent>,
                            dt: f32,
    ) -> Result<(), RendererError> {
        self.geometry_cache.update(dt);
        self.texture_cache.update(dt);

        self.statistics.begin_frame();

        let window_viewport = Rect::new(0, 0, self.frame_size.0 as i32, self.frame_size.1 as i32);
        self.backbuffer.clear(&mut self.state, window_viewport, Some(self.backbuffer_clear_color), Some(1.0), Some(0));

        let frame_width = self.frame_size.0 as f32;
        let frame_height = self.frame_size.1 as f32;

        for scene in scenes.iter() {
            let graph = &scene.graph;

            for (camera_handle, camera) in graph.pair_iter().filter_map(|(handle, node)| {
                if let Node::Camera(camera) = node { Some((handle, camera)) } else { None }
            }) {
                if !camera.is_enabled() {
                    continue;
                }

                let viewport = camera.viewport_pixels(Vec2::new(frame_width, frame_height));

                let state = &mut self.state;
                let gbuffer = self.gbuffers
                    .entry(camera_handle)
                    .and_modify(|buf| {
                        if buf.width != viewport.w || buf.height != viewport.h {
                            *buf = GBuffer::new(state,viewport.w as usize, viewport.h as usize).unwrap();
                        }
                    })
                    .or_insert_with(|| GBuffer::new(state,viewport.w as usize, viewport.h as usize).unwrap());

                self.statistics += gbuffer.fill(
                    state,
                    graph,
                    camera,
                    self.white_dummy.clone(),
                    self.normal_dummy.clone(),
                    &mut self.texture_cache,
                    &mut self.geometry_cache,
                );

                self.statistics += self.deferred_light_renderer.render(DeferredRendererContext {
                    state,
                    scene,
                    camera,
                    gbuffer,
                    white_dummy: self.white_dummy.clone(),
                    ambient_color: self.ambient_color,
                    settings: &self.quality_settings,
                    textures: &mut self.texture_cache,
                    geometry_cache: &mut self.geometry_cache,
                });

                let depth = gbuffer.depth();

                self.statistics += self.particle_system_renderer.render(
                    state,
                    &mut gbuffer.opt_framebuffer,
                    graph,
                    camera,
                    self.white_dummy.clone(),
                    depth,
                    frame_width,
                    frame_height,
                    viewport,
                    &mut self.texture_cache,
                );

                self.statistics += self.sprite_renderer.render(
                    state,
                    &mut gbuffer.opt_framebuffer,
                    graph,
                    camera,
                    self.white_dummy.clone(),
                    viewport,
                    &mut self.texture_cache,
                    &mut self.geometry_cache,
                );

                self.statistics += self.debug_renderer.render(state, viewport, &mut gbuffer.opt_framebuffer, camera);

                // Finally render everything into back buffer.
                self.statistics.geometry.add_draw_call(
                    self.backbuffer.draw(
                        state,
                        viewport,
                        self.geometry_cache.get(&self.quad),
                        &mut self.flat_shader.program,
                        DrawParameters {
                            cull_face: CullFace::Back,
                            culling: false,
                            color_write: (true, true, true, true),
                            depth_write: true,
                            stencil_test: false,
                            depth_test: false,
                            blend: false,
                        },
                        &[
                            (self.flat_shader.wvp_matrix, UniformValue::Mat4({
                                Mat4::ortho(0.0, viewport.w as f32, viewport.h as f32, 0.0, -1.0, 1.0) *
                                    Mat4::scale(Vec3::new(viewport.w as f32, viewport.h as f32, 0.0))
                            })),
                            (self.flat_shader.diffuse_texture, UniformValue::Sampler {
                                index: 0,
                                texture: gbuffer.frame_texture(),
                            })
                        ],
                    ));
            }
        }

        // Render UI on top of everything.
        self.statistics += self.ui_renderer.render(
            &mut self.state,
            window_viewport,
            &mut self.backbuffer,
            frame_width,
            frame_height,
            drawing_context,
            self.white_dummy.clone(),
            &mut self.texture_cache,
        )?;

        self.statistics.end_frame();

        if context.swap_buffers().is_err() {
            Log::writeln("Failed to swap buffers!".to_owned());
        }

        check_gl_error!();

        self.statistics.finalize();

        Ok(())
    }
}

fn check_gl_error_internal(line: u32, file: &str) {
    unsafe {
        let error_code = gl::GetError();
        if error_code != gl::NO_ERROR {
            let code = match error_code {
                gl::INVALID_ENUM => "GL_INVALID_ENUM",
                gl::INVALID_VALUE => "GL_INVALID_VALUE",
                gl::INVALID_OPERATION => "GL_INVALID_OPERATION",
                gl::STACK_OVERFLOW => "GL_STACK_OVERFLOW",
                gl::STACK_UNDERFLOW => "GL_STACK_UNDERFLOW",
                gl::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
                _ => "Unknown",
            };

            Log::writeln(format!("{} error has occurred! At line {} in file {}, stability is not guaranteed!", code, line, file));

            if gl::GetDebugMessageLog::is_loaded() {
                let mut max_message_length = 0;
                gl::GetIntegerv(gl::MAX_DEBUG_MESSAGE_LENGTH, &mut max_message_length);

                let mut max_logged_messages = 0;
                gl::GetIntegerv(gl::MAX_DEBUG_LOGGED_MESSAGES, &mut max_logged_messages);

                let buffer_size = max_message_length * max_logged_messages;

                let mut message_buffer: Vec<GLchar> = Vec::with_capacity(buffer_size as usize);
                message_buffer.set_len(buffer_size as usize);

                let mut sources: Vec<GLenum> = Vec::with_capacity(max_logged_messages as usize);
                sources.set_len(max_logged_messages as usize);

                let mut types: Vec<GLenum> = Vec::with_capacity(max_logged_messages as usize);
                types.set_len(max_logged_messages as usize);

                let mut ids: Vec<GLuint> = Vec::with_capacity(max_logged_messages as usize);
                ids.set_len(max_logged_messages as usize);

                let mut severities: Vec<GLenum> = Vec::with_capacity(max_logged_messages as usize);
                severities.set_len(max_logged_messages as usize);

                let mut lengths: Vec<GLsizei> = Vec::with_capacity(max_logged_messages as usize);
                lengths.set_len(max_logged_messages as usize);

                let message_count = gl::GetDebugMessageLog(
                    max_logged_messages as u32,
                    buffer_size,
                    sources.as_mut_ptr(),
                    types.as_mut_ptr(),
                    ids.as_mut_ptr(),
                    severities.as_mut_ptr(),
                    lengths.as_mut_ptr(),
                    message_buffer.as_mut_ptr(),
                );

                if message_count == 0 {
                    Log::writeln("Debug info is not available - run with OpenGL debug flag!".to_owned());
                }

                let mut message = message_buffer.as_ptr();

                for i in 0..message_count as usize {
                    let source = sources[i];
                    let ty = types[i];
                    let severity = severities[i];
                    let id = ids[i];
                    let len = lengths[i] as usize;

                    let source_str =
                        match source {
                            gl::DEBUG_SOURCE_API => "API",
                            gl::DEBUG_SOURCE_SHADER_COMPILER => "Shader Compiler",
                            gl::DEBUG_SOURCE_WINDOW_SYSTEM => "Window System",
                            gl::DEBUG_SOURCE_THIRD_PARTY => "Third Party",
                            gl::DEBUG_SOURCE_APPLICATION => "Application",
                            gl::DEBUG_SOURCE_OTHER => "Other",
                            _ => "Unknown"
                        };

                    let type_str =
                        match ty {
                            gl::DEBUG_TYPE_ERROR => "Error",
                            gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "Deprecated Behavior",
                            gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "Undefined Behavior",
                            gl::DEBUG_TYPE_PERFORMANCE => "Performance",
                            gl::DEBUG_TYPE_PORTABILITY => "Portability",
                            gl::DEBUG_TYPE_OTHER => "Other",
                            _ => "Unknown",
                        };

                    let severity_str =
                        match severity {
                            gl::DEBUG_SEVERITY_HIGH => "High",
                            gl::DEBUG_SEVERITY_MEDIUM => "Medium",
                            gl::DEBUG_SEVERITY_LOW => "Low",
                            gl::DEBUG_SEVERITY_NOTIFICATION => "Notification",
                            _ => "Unknown"
                        };

                    let str_msg = CStr::from_ptr(message);

                    Log::writeln(format!("OpenGL message\nSource: {}\nType: {}\nId: {}\nSeverity: {}\nMessage: {:?}\n",
                                         source_str,
                                         type_str,
                                         id,
                                         severity_str,
                                         str_msg));

                    message = message.add(len);
                }
            } else {
                Log::writeln("Debug info is not available - glGetDebugMessageLog is not available!".to_owned());
            }
        }
    }
}