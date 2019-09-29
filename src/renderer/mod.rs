#[allow(clippy::all)]
mod gl;
pub mod surface;
pub mod gpu_program;
pub mod error;

mod geometry_buffer;
mod ui_renderer;
mod particle_system_renderer;
mod gbuffer;
mod deferred_light_renderer;
mod shadow_map_renderer;
mod flat_shader;
pub mod gpu_texture;

use crate::{
    engine::state::State,
    gui::draw::DrawingContext,
    scene::node::NodeKind,
    renderer::{
        ui_renderer::UIRenderer,
        surface::SurfaceSharedData,
        particle_system_renderer::ParticleSystemRenderer,
        gbuffer::GBuffer,
        deferred_light_renderer::DeferredLightRenderer,
        error::RendererError,
        gpu_texture::{GpuTexture, GpuTextureKind, PixelKind},
        flat_shader::FlatShader,
    },
};
use std::time;
use glutin::{PossiblyCurrent, GlProfile, GlRequest, Api};
use rg3d_core::math::{
    vec3::Vec3,
    mat4::Mat4,
    vec2::Vec2,
};

#[repr(C)]
pub struct TriangleDefinition {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

fn check_gl_error_internal(line: u32, file: &str) {
    unsafe {
        let error_code = gl::GetError();
        if error_code != gl::NO_ERROR {
            match error_code {
                gl::INVALID_ENUM => print!("GL_INVALID_ENUM"),
                gl::INVALID_VALUE => print!("GL_INVALID_VALUE"),
                gl::INVALID_OPERATION => print!("GL_INVALID_OPERATION"),
                gl::STACK_OVERFLOW => print!("GL_STACK_OVERFLOW"),
                gl::STACK_UNDERFLOW => print!("GL_STACK_UNDERFLOW"),
                gl::OUT_OF_MEMORY => print!("GL_OUT_OF_MEMORY"),
                _ => (),
            };

            println!(" error has occurred! At line {} in file {}, stability is not guaranteed!", line, file);
        }
    }
}

macro_rules! check_gl_error {
    () => (check_gl_error_internal(line!(), file!()))
}

#[derive(Copy, Clone)]
pub struct Statistics {
    /// Real time consumed to render frame.
    pub pure_frame_time: f32,
    /// Total time renderer took to process single frame, usually includes
    /// time renderer spend to wait to buffers swap (can include vsync)
    pub capped_frame_time: f32,
    /// Total amount of frames been rendered in one second.
    pub frames_per_second: usize,
    /// Max amount of frames per second renderer could give if it was not capped.
    pub potential_frame_per_second: usize,
    frame_counter: usize,
    frame_start_time: time::Instant,
    last_fps_commit_time: time::Instant,
}

impl Statistics {
    /// Must be called before render anything.
    fn begin_frame(&mut self) {
        self.frame_start_time = time::Instant::now();
    }

    /// Must be called before SwapBuffers but after all rendering is done.
    fn end_frame(&mut self) {
        let current_time = time::Instant::now();

        self.pure_frame_time = current_time.duration_since(self.frame_start_time).as_secs_f32();
        self.frame_counter += 1;
        if self.pure_frame_time > 0.0 {
            self.potential_frame_per_second = (1.0 / self.pure_frame_time) as usize;
        }

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
            pure_frame_time: 0.0,
            capped_frame_time: 0.0,
            frames_per_second: 0,
            potential_frame_per_second: 0,
            frame_counter: 0,
            frame_start_time: time::Instant::now(),
            last_fps_commit_time: time::Instant::now(),
        }
    }
}

pub struct Renderer {
    // Must be on top!
    pub(crate) context: glutin::WindowedContext<PossiblyCurrent>,
    pub(crate) events_loop: glutin::EventsLoop,
    deferred_light_renderer: DeferredLightRenderer,
    gbuffer: GBuffer,
    flat_shader: FlatShader,
    particle_system_renderer: ParticleSystemRenderer,
    /// Dummy white one pixel texture which will be used as stub when rendering
    /// something without texture specified.
    white_dummy: GpuTexture,
    normal_dummy: GpuTexture,
    ui_renderer: UIRenderer,
    statistics: Statistics,
    quad: SurfaceSharedData,
    last_render_time: time::Instant,
}

impl Renderer {
    pub fn new() -> Result<Self, RendererError> {
        let events_loop = glutin::EventsLoop::new();

        let primary_monitor = events_loop.get_primary_monitor();
        let mut monitor_dimensions = primary_monitor.get_dimensions();
        monitor_dimensions.height *= 0.7;
        monitor_dimensions.width *= 0.7;
        let window_size = monitor_dimensions.to_logical(primary_monitor.get_hidpi_factor());

        let window_builder = glutin::WindowBuilder::new()
            .with_title("RG3D")
            .with_dimensions(window_size)
            .with_resizable(true);

        let context_wrapper = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_gl_profile(GlProfile::Core)
            .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
            .build_windowed(window_builder, &events_loop)?;

        unsafe {
            let context = match context_wrapper.make_current() {
                Ok(context) => context,
                Err((_, error)) => return Err(RendererError::from(error)),
            };
            gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);
            gl::Enable(gl::DEPTH_TEST);

            Ok(Self {
                context,
                events_loop,
                deferred_light_renderer: DeferredLightRenderer::new()?,
                flat_shader: FlatShader::new()?,
                gbuffer: GBuffer::new(window_size.width as i32, window_size.height as i32)?,
                statistics: Statistics::default(),
                white_dummy: {
                    GpuTexture::new(GpuTextureKind::Rectangle { width: 1, height: 1 },
                                    PixelKind::RGBA8, &[255, 255, 255, 255],
                                    false)?
                },
                normal_dummy: {
                    GpuTexture::new(GpuTextureKind::Rectangle { width: 1, height: 1 },
                                    PixelKind::RGBA8, &[128, 128, 255, 255],
                                    false)?
                },
                quad: SurfaceSharedData::make_unit_xy_quad(),
                ui_renderer: UIRenderer::new()?,
                particle_system_renderer: ParticleSystemRenderer::new()?,
                last_render_time: time::Instant::now(), // TODO: Is this right?
            })
        }
    }

    pub fn get_statistics(&self) -> Statistics {
        self.statistics
    }

    pub fn upload_resources(&mut self, state: &mut State) {
        for texture_rc in state.get_resource_manager_mut().get_textures() {
            let mut texture = texture_rc.lock().unwrap();
            if texture.gpu_tex.is_none() {
                let gpu_texture = GpuTexture::new(
                    GpuTextureKind::Rectangle { width: texture.width as usize, height: texture.height as usize },
                    PixelKind::RGBA8, texture.bytes.as_slice(), true).unwrap();
                gpu_texture.set_max_anisotropy();
                texture.gpu_tex = Some(gpu_texture);
            }
        }
    }

    /// Sets new frame size, should be called when received a Resize event.
    pub fn set_frame_size(&mut self, new_size: Vec2) -> Result<(), RendererError> {
        self.gbuffer = GBuffer::new(new_size.x as i32, new_size.y as i32)?;
        Ok(())
    }

    pub fn get_frame_size(&self) -> Vec2 {
        let client_size = self.context.window().get_inner_size().unwrap();
        Vec2::make(client_size.width as f32, client_size.height as f32)
    }

    pub fn render(&mut self, state: &State, drawing_context: &DrawingContext) -> Result<(), RendererError> {
        self.statistics.begin_frame();
        let client_size = self.context.window().get_inner_size().unwrap();

        let frame_width = client_size.width as f32;
        let frame_height = client_size.height as f32;
        let frame_matrix =
            Mat4::ortho(0.0, frame_width, frame_height, 0.0, -1.0, 1.0) *
                Mat4::scale(Vec3::make(frame_width, frame_height, 0.0));

        unsafe {
            for scene in state.get_scenes().iter() {
                // Prepare for render - fill lists of nodes participating in rendering.
                let camera_node = match scene.get_active_camera() {
                    Some(camera_node) => camera_node,
                    None => continue
                };

                let camera =
                    if let NodeKind::Camera(camera) = camera_node.get_kind() {
                        camera
                    } else {
                        continue;
                    };

                self.gbuffer.fill(frame_width, frame_height, scene, camera, &self.white_dummy, &self.normal_dummy);

                self.deferred_light_renderer.render(frame_width, frame_height, scene, camera_node, camera, &self.gbuffer);
            }

            self.particle_system_renderer.render(state, &self.white_dummy, frame_width, frame_height, &self.gbuffer);

            // Finally render everything into back buffer.
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, frame_width as i32, frame_height as i32);
            gl::StencilMask(0xFF);
            gl::DepthMask(gl::TRUE);
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);

            self.flat_shader.bind();
            self.flat_shader.set_wvp_matrix(&frame_matrix);
            self.flat_shader.set_diffuse_texture(0);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.gbuffer.frame_texture);
            self.quad.draw();

            self.ui_renderer.render(frame_width, frame_height, drawing_context, &self.white_dummy)?;
        }

        self.statistics.end_frame();

        self.context.swap_buffers()?;

        check_gl_error!();

        self.statistics.finalize();

        Ok(())
    }
}