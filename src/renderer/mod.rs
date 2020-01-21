#[allow(clippy::all)]
pub(in crate) mod gl;
pub mod surface;
pub mod gpu_program;
pub mod error;

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
pub mod gpu_texture;
mod sprite_renderer;

use std::{
    time,
    ffi::CStr,
};
use glutin::PossiblyCurrent;
use crate::{
    engine::resource_manager::ResourceManager,
    gui::draw::DrawingContext,
    renderer::{
        ui_renderer::UIRenderer,
        surface::SurfaceSharedData,
        particle_system_renderer::ParticleSystemRenderer,
        gbuffer::GBuffer,
        deferred_light_renderer::{
            DeferredLightRenderer,
            DeferredRendererContext
        },
        error::RendererError,
        gpu_texture::{GpuTexture, GpuTextureKind, PixelKind},
        flat_shader::FlatShader,
        sprite_renderer::SpriteRenderer,
        gl::types::{GLsizei, GLenum, GLuint, GLchar},
    },
    scene::{
        SceneContainer,
        node::Node,
    },
    core::{
        math::{vec3::Vec3, mat4::Mat4},
        color::Color,
        math::vec2::Vec2,
    },
    utils::log::Log,
    core::math::TriangleDefinition
};

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
            triangles_rendered: 0
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
    deferred_light_renderer: DeferredLightRenderer,
    gbuffer: GBuffer,
    flat_shader: FlatShader,
    sprite_renderer: SpriteRenderer,
    particle_system_renderer: ParticleSystemRenderer,
    /// Dummy white one pixel texture which will be used as stub when rendering
    /// something without texture specified.
    white_dummy: GpuTexture,
    /// Dummy one pixel texture with (0, 1, 0) vector is used as stub when rendering
    /// something without normal map.
    normal_dummy: GpuTexture,
    ui_renderer: UIRenderer,
    statistics: Statistics,
    quad: SurfaceSharedData,
    frame_size: (u32, u32),
    ambient_color: Color,
    quality_settings: QualitySettings,
}

impl Renderer {
    pub(in crate) fn new(frame_size: (u32, u32)) -> Result<Self, RendererError> {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
        }

        let settings = QualitySettings::default();

        Ok(Self {
            frame_size,
            deferred_light_renderer: DeferredLightRenderer::new(&settings)?,
            flat_shader: FlatShader::new()?,
            gbuffer: GBuffer::new(frame_size)?,
            statistics: Statistics::default(),
            sprite_renderer: SpriteRenderer::new()?,
            white_dummy: GpuTexture::new(GpuTextureKind::Rectangle { width: 1, height: 1 },
                                         PixelKind::RGBA8, &[255, 255, 255, 255],
                                         false)?,
            normal_dummy: GpuTexture::new(GpuTextureKind::Rectangle { width: 1, height: 1 },
                                          PixelKind::RGBA8, &[128, 128, 255, 255],
                                          false)?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
            ui_renderer: UIRenderer::new()?,
            particle_system_renderer: ParticleSystemRenderer::new()?,
            ambient_color: Color::opaque(100, 100, 100),
            quality_settings: settings,
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

    pub fn upload_resources(&mut self, resource_manager: &mut ResourceManager) {
        for texture_rc in resource_manager.get_textures() {
            let mut texture = texture_rc.lock().unwrap();
            if texture.loaded && texture.gpu_tex.is_none() {
                let gpu_texture = GpuTexture::new(
                    GpuTextureKind::Rectangle { width: texture.width as usize, height: texture.height as usize },
                    PixelKind::from(texture.kind), texture.bytes.as_slice(), true).unwrap();
                gpu_texture.set_max_anisotropy();
                texture.gpu_tex = Some(gpu_texture);
            }
        }
    }

    /// Sets new frame size, should be called when received a Resize event.
    pub fn set_frame_size(&mut self, new_size: (u32, u32)) -> Result<(), RendererError> {
        self.frame_size = new_size;
        self.gbuffer = GBuffer::new(new_size)?;
        Ok(())
    }

    pub fn get_frame_size(&self) -> (u32, u32) {
        self.frame_size
    }

    pub fn set_quality_settings(&mut self, settings: &QualitySettings) -> Result<(), RendererError> {
        self.quality_settings = *settings;
        self.deferred_light_renderer.set_quality_settings(settings)
    }

    pub fn get_quality_settings(&self) -> QualitySettings {
        self.quality_settings
    }

    pub(in crate) fn render(&mut self,
                            scenes: &SceneContainer,
                            drawing_context: &DrawingContext,
                            context: &glutin::WindowedContext<PossiblyCurrent>,
    ) -> Result<(), RendererError> {
        self.statistics.begin_frame();

        let frame_width = self.frame_size.0 as f32;
        let frame_height = self.frame_size.1 as f32;
        let frame_matrix =
            Mat4::ortho(0.0, frame_width, frame_height, 0.0, -1.0, 1.0) *
                Mat4::scale(Vec3::new(frame_width, frame_height, 0.0));

        // Render scenes into g-buffer.
        for scene in scenes.iter() {
            let graph = scene.interface().graph;

            // Prepare for render - fill lists of nodes participating in rendering.
            let camera = match graph.linear_iter().find(|node| node.is_camera()) {
                Some(camera) => camera,
                None => continue
            };

            let camera = match camera {
                Node::Camera(camera) => camera,
                _ => continue
            };

            self.statistics += self.gbuffer.fill(
                frame_width,
                frame_height,
                graph,
                camera,
                &self.white_dummy,
                &self.normal_dummy,
            );

            self.statistics += self.deferred_light_renderer.render(DeferredRendererContext {
                frame_size: Vec2::new(frame_width, frame_height),
                scene,
                camera,
                gbuffer: &self.gbuffer,
                white_dummy: &self.white_dummy,
                ambient_color: self.ambient_color,
                settings: &self.quality_settings,
            });
        }

        self.statistics += self.particle_system_renderer.render(
            scenes,
            &self.white_dummy,
            frame_width,
            frame_height,
            &self.gbuffer,
        );

        self.statistics += self.sprite_renderer.render(scenes, &self.white_dummy);

        unsafe {
            // Finally render everything into back buffer.
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, frame_width as i32, frame_height as i32);
            gl::StencilMask(0xFF);
            gl::DepthMask(gl::TRUE);
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        }
        self.flat_shader.bind();
        self.flat_shader.set_wvp_matrix(&frame_matrix);
        self.flat_shader.set_diffuse_texture(0);

        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.gbuffer.frame_texture);
        }
        self.statistics.geometry.add_draw_call(self.quad.draw());

        self.statistics += self.ui_renderer.render(
            frame_width,
            frame_height,
            drawing_context,
            &self.white_dummy,
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