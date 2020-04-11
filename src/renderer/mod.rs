#![deny(unsafe_code)]

pub mod surface;
pub mod error;
pub mod debug_renderer;

// Framework wraps all OpenGL calls so it has to be unsafe. Rest of renderer
// code must be safe.
#[macro_use]
#[allow(unsafe_code)]
mod framework;

mod ui_renderer;
mod particle_system_renderer;
mod gbuffer;
mod deferred_light_renderer;
mod shadow_map_renderer;
mod flat_shader;
mod sprite_renderer;
mod ssao;
mod blur;
mod light_volume;

use glutin::PossiblyCurrent;
use std::{
    rc::Rc,
    sync::{
        Arc,
        Mutex,
    },
    time,
    collections::HashMap,
    cell::RefCell,
};
use crate::{
    resource::texture::Texture,
    renderer::{
        ui_renderer::{
            UiRenderer,
            UiRenderContext,
        },
        surface::SurfaceSharedData,
        particle_system_renderer::{
            ParticleSystemRenderer,
            ParticleSystemRenderContext,
        },
        gbuffer::{
            GBuffer,
            GBufferRenderContext,
        },
        deferred_light_renderer::{
            DeferredLightRenderer,
            DeferredRendererContext,
        },
        error::RendererError,
        framework::{
            gpu_texture::{
                GpuTexture,
                GpuTextureKind,
                PixelKind,
                MininificationFilter,
                MagnificationFilter,
            },
            geometry_buffer::{
                GeometryBuffer,
                GeometryBufferKind,
                ElementKind,
                AttributeKind,
                AttributeDefinition,
                DrawCallStatistics
            },
            framebuffer::{
                BackBuffer,
                FrameBufferTrait,
                DrawParameters,
                CullFace,
            },
            gpu_program::UniformValue,
            state::State,
            gl,
        },
        flat_shader::FlatShader,
        sprite_renderer::{
            SpriteRenderer,
            SpriteRenderContext,
        },
        debug_renderer::DebugRenderer,
    },
    scene::{
        SceneContainer,
        node::Node,
    },
    core::{
        scope_profile,
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
    gui::draw::DrawingContext,
    engine::resource_manager::TimedEntry,
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

impl std::ops::AddAssign<DrawCallStatistics> for RenderPassStatistics {
    fn add_assign(&mut self, rhs: DrawCallStatistics) {
        self.draw_calls += 1;
        self.triangles_rendered += rhs.triangles;
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

    /// Whether to use screen space ambient occlusion or not.
    pub use_ssao: bool,
    /// Radius of sampling hemisphere used in SSAO, it defines much ambient
    /// occlusion will be in your scene.
    pub ssao_radius: f32,

    /// Global switch to enable or disable light scattering. Each light can have
    /// its own scatter switch, but this one is able to globally disable scatter.
    pub light_scatter_enabled: bool,
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self {
            point_shadow_map_size: 1024,
            point_shadows_distance: 15.0,
            point_shadows_enabled: true,
            point_soft_shadows: true,

            spot_shadow_map_size: 1024,
            spot_shadows_distance: 15.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: true,

            use_ssao: true,
            ssao_radius: 0.5,

            light_scatter_enabled: true
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
    fn get(&mut self, state: &mut State, data: &SurfaceSharedData) -> &mut GeometryBuffer<surface::Vertex> {
        scope_profile!();

        let key = (data as *const _) as usize;

        let geometry_buffer = self.map.entry(key).or_insert_with(|| {
            let geometry_buffer = GeometryBuffer::new(GeometryBufferKind::StaticDraw, ElementKind::Triangle);

            geometry_buffer.bind(state)
                .describe_attributes(vec![
                    AttributeDefinition { kind: AttributeKind::Float3, normalized: false },
                    AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
                    AttributeDefinition { kind: AttributeKind::Float3, normalized: false },
                    AttributeDefinition { kind: AttributeKind::Float4, normalized: false },
                    AttributeDefinition { kind: AttributeKind::Float4, normalized: false },
                    AttributeDefinition { kind: AttributeKind::UnsignedByte4, normalized: false }])
                .unwrap()
                .set_vertices(data.vertices.as_slice())
                .set_triangles(data.triangles());

            TimedEntry { value: geometry_buffer, time_to_live: 20.0 }
        });

        geometry_buffer.time_to_live = 20.0;
        geometry_buffer
    }

    fn update(&mut self, dt: f32) {
        for entry in self.map.values_mut() {
            entry.time_to_live -= dt;
        }
        self.map.retain(|_, v| v.time_to_live > 0.0);
    }

    fn clear(&mut self) {
        self.map.clear();
    }
}

#[derive(Default)]
pub struct TextureCache {
    map: HashMap<usize, TimedEntry<Rc<RefCell<GpuTexture>>>>
}

impl TextureCache {
    fn get(&mut self, state: &mut State, texture: Arc<Mutex<Texture>>) -> Option<Rc<RefCell<GpuTexture>>> {
        scope_profile!();

        if texture.lock().unwrap().loaded {
            let key = (&*texture as *const _) as usize;
            let gpu_texture = self.map.entry(key).or_insert_with(move || {
                let texture = texture.lock().unwrap();
                let kind = GpuTextureKind::Rectangle {
                    width: texture.width as usize,
                    height: texture.height as usize,
                };
                let mut gpu_texture = GpuTexture::new(
                    state,
                    kind,
                    PixelKind::from(texture.kind),
                    Some(texture.bytes.as_slice()))
                    .unwrap();
                gpu_texture.bind_mut(state, 0)
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
        self.map.retain(|_, v| v.time_to_live > 0.0);
    }

    fn clear(&mut self) {
        self.map.clear();
    }
}

impl Renderer {
    pub(in crate) fn new(context: &mut glutin::WindowedContext<PossiblyCurrent>, frame_size: (u32, u32)) -> Result<Self, RendererError> {
        gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

        let settings = QualitySettings::default();
        let mut state = State::new();

        Ok(Self {
            backbuffer: BackBuffer,
            frame_size,
            deferred_light_renderer: DeferredLightRenderer::new(&mut state, frame_size, &settings)?,
            flat_shader: FlatShader::new()?,
            statistics: Statistics::default(),
            sprite_renderer: SpriteRenderer::new()?,
            white_dummy: Rc::new(RefCell::new(GpuTexture::new(&mut state, GpuTextureKind::Rectangle { width: 1, height: 1 },
                                                              PixelKind::RGBA8, Some(&[255, 255, 255, 255]))?)),
            normal_dummy: Rc::new(RefCell::new(GpuTexture::new(&mut state, GpuTextureKind::Rectangle { width: 1, height: 1 },
                                                               PixelKind::RGBA8, Some(&[128, 128, 255, 255]))?)),
            quad: SurfaceSharedData::make_unit_xy_quad(),
            ui_renderer: UiRenderer::new(&mut state)?,
            particle_system_renderer: ParticleSystemRenderer::new(&mut state)?,
            ambient_color: Color::opaque(100, 100, 100),
            quality_settings: settings,
            debug_renderer: DebugRenderer::new(&mut state)?,
            gbuffers: Default::default(),
            backbuffer_clear_color: Color::from_rgba(0, 0, 0, 0),
            texture_cache: Default::default(),
            geometry_cache: Default::default(),
            state,
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
        self.deferred_light_renderer.set_frame_size(&mut self.state, new_size).unwrap();
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

    pub(in crate) fn flush(&mut self) {
        self.texture_cache.clear();
        self.geometry_cache.clear();
    }

    fn render_frame(&mut self, scenes: &SceneContainer,
                    drawing_context: &DrawingContext,
                    dt: f32,
    ) -> Result<(), RendererError> {
        scope_profile!();

        // We have to invalidate resource bindings cache because some textures or programs,
        // or other GL resources can be destroyed and then on their "names" some new resource
        // are created, but cache still thinks that resource is correctly bound, but it is different
        // object have same name.
        self.state.invalidate_resource_bindings_cache();

        // Update caches - this will remove timed out resources.
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
                            *buf = GBuffer::new(state, viewport.w as usize, viewport.h as usize).unwrap();
                        }
                    })
                    .or_insert_with(|| GBuffer::new(state, viewport.w as usize, viewport.h as usize).unwrap());

                self.statistics += gbuffer.fill(
                    GBufferRenderContext {
                        state,
                        graph,
                        camera,
                        white_dummy: self.white_dummy.clone(),
                        normal_dummy: self.normal_dummy.clone(),
                        texture_cache: &mut self.texture_cache,
                        geom_cache: &mut self.geometry_cache,
                    });

                self.statistics += self.deferred_light_renderer.render(
                    DeferredRendererContext {
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
                    ParticleSystemRenderContext {
                        state,
                        framebuffer: &mut gbuffer.final_frame,
                        graph,
                        camera,
                        white_dummy: self.white_dummy.clone(),
                        depth,
                        frame_width,
                        frame_height,
                        viewport,
                        texture_cache: &mut self.texture_cache,
                    });

                self.statistics += self.sprite_renderer.render(
                    SpriteRenderContext {
                        state,
                        framebuffer: &mut gbuffer.final_frame,
                        graph,
                        camera,
                        white_dummy: self.white_dummy.clone(),
                        viewport,
                        textures: &mut self.texture_cache,
                        geom_map: &mut self.geometry_cache,
                    });

                self.statistics += self.debug_renderer.render(state, viewport, &mut gbuffer.final_frame, camera);

                // Finally render everything into back buffer.
                self.statistics.geometry += self.backbuffer.draw(
                    self.geometry_cache.get(state, &self.quad),
                    state,
                    viewport,
                    &self.flat_shader.program,
                    DrawParameters {
                        cull_face: CullFace::Back,
                        culling: false,
                        color_write: Default::default(),
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
                );
            }
        }

        // Render UI on top of everything.
        self.statistics += self.ui_renderer.render(
            UiRenderContext {
                state: &mut self.state,
                viewport: window_viewport,
                backbuffer: &mut self.backbuffer,
                frame_width,
                frame_height,
                drawing_context,
                white_dummy: self.white_dummy.clone(),
                texture_cache: &mut self.texture_cache,
            }
        )?;

        Ok(())
    }


    pub(in crate) fn render_and_swap_buffers(&mut self,
                                             scenes: &SceneContainer,
                                             drawing_context: &DrawingContext,
                                             context: &glutin::WindowedContext<PossiblyCurrent>,
                                             dt: f32,
    ) -> Result<(), RendererError> {
        scope_profile!();

        self.render_frame(scenes, drawing_context, dt)?;

        self.statistics.end_frame();
        context.swap_buffers()?;
        check_gl_error!();
        self.statistics.finalize();
        Ok(())
    }
}

