//! Renderer is a "workhorse" of the engine, it draws scenes and user interface.
//! For now there is almost no possibility to change pipeline of renderer, you
//! can only modify quality settings. This will change in future to make renderer
//! more flexible.
//!
//! Renderer based on OpenGL 3.3+ Core.

#![warn(missing_docs)]
//#![deny(unsafe_code)]

pub mod debug_renderer;
pub mod framework;
pub mod renderer2d;

mod batch;
mod blur;
mod cache;
mod deferred_light_renderer;
mod flat_shader;
mod forward_renderer;
mod fxaa;
mod gbuffer;
mod light_volume;
mod particle_system_renderer;
mod shadow_map_renderer;
mod skybox_shader;
mod sprite_renderer;
mod ssao;
mod ui_renderer;

use crate::renderer::cache::CacheEntry;
use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
        instant,
        math::Rect,
        pool::Handle,
        scope_profile,
    },
    gui::{draw::DrawingContext, message::MessageData, Control, UserInterface},
    renderer::{
        batch::BatchStorage,
        cache::{GeometryCache, TextureCache},
        debug_renderer::DebugRenderer,
        deferred_light_renderer::{
            DeferredLightRenderer, DeferredRendererContext, LightingStatistics,
        },
        flat_shader::FlatShader,
        forward_renderer::{ForwardRenderContext, ForwardRenderer},
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer},
            geometry_buffer::DrawCallStatistics,
            gpu_texture::{
                GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
            },
            state::{PipelineState, PipelineStatistics},
        },
        fxaa::FxaaRenderer,
        gbuffer::{GBuffer, GBufferRenderContext},
        particle_system_renderer::{ParticleSystemRenderContext, ParticleSystemRenderer},
        renderer2d::Renderer2d,
        sprite_renderer::{SpriteRenderContext, SpriteRenderer},
        ui_renderer::{UiRenderContext, UiRenderer},
    },
    resource::texture::{Texture, TextureKind},
    scene::mesh::surface::SurfaceData,
    scene::{node::Node, Scene, SceneContainer},
    scene2d::Scene2dContainer,
};
#[cfg(feature = "serde_integration")]
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    fmt::{Display, Formatter},
    rc::Rc,
    sync::mpsc::{Receiver, Sender},
};

/// Renderer statistics for one frame, also includes current frames per second
/// amount.
#[derive(Copy, Clone)]
pub struct Statistics {
    /// Shows how many pipeline state changes was made per frame.
    pub pipeline: PipelineStatistics,
    /// Shows how many lights and shadow maps were rendered.
    pub lighting: LightingStatistics,
    /// Shows how many draw calls was made and how many triangles were rendered.
    pub geometry: RenderPassStatistics,
    /// Real time consumed to render frame. Time given in **seconds**.
    pub pure_frame_time: f32,
    /// Total time renderer took to process single frame, usually includes
    /// time renderer spend to wait to buffers swap (can include vsync).
    /// Time given in **seconds**.
    pub capped_frame_time: f32,
    /// Total amount of frames been rendered in one second.
    pub frames_per_second: usize,
    frame_counter: usize,
    frame_start_time: instant::Instant,
    last_fps_commit_time: instant::Instant,
}

impl Display for Statistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FPS: {}\n\
            Pure Frame Time: {:.2} ms\n\
            Capped Frame Time: {:.2} ms\n\
            {}\n\
            {}\n\
            {}\n",
            self.frames_per_second,
            self.pure_frame_time * 1000.0,
            self.capped_frame_time * 1000.0,
            self.geometry,
            self.lighting,
            self.pipeline
        )
    }
}

/// GPU statistics for single frame.
#[derive(Copy, Clone)]
pub struct RenderPassStatistics {
    /// Amount of draw calls per frame - lower the better.
    pub draw_calls: usize,
    /// Amount of triangles per frame.
    pub triangles_rendered: usize,
}

impl Display for RenderPassStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Draw Calls: {}\n\
            Triangles Rendered: {}",
            self.draw_calls, self.triangles_rendered
        )
    }
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

/// Shadow map precision allows you to select compromise between quality and performance.
#[derive(Copy, Clone, Hash, PartialOrd, PartialEq, Eq, Ord, Debug)]
#[cfg_attr(feature = "serde_integration", derive(Serialize, Deserialize))]
pub enum ShadowMapPrecision {
    /// Shadow map will use 2 times less memory by switching to 16bit pixel format,
    /// but "shadow acne" may occur.
    Half,
    /// Shadow map will use 32bit pixel format. This option gives highest quality,
    /// but could be less performant than `Half`.
    Full,
}

/// Quality settings allows you to find optimal balance between performance and
/// graphics quality.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde_integration", derive(Serialize, Deserialize))]
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
    /// Point shadow map precision. Allows you to select compromise between
    /// quality and performance.
    pub point_shadow_map_precision: ShadowMapPrecision,

    /// Spot shadows
    /// Size of square shadow map texture in pixels
    pub spot_shadow_map_size: usize,
    /// Use or not percentage close filtering (smoothing) for spot shadows.
    pub spot_soft_shadows: bool,
    /// Spot shadows enabled or not.
    pub spot_shadows_enabled: bool,
    /// Maximum distance from camera to draw shadows.
    pub spot_shadows_distance: f32,
    /// Spot shadow map precision. Allows you to select compromise between
    /// quality and performance.
    pub spot_shadow_map_precision: ShadowMapPrecision,

    /// Whether to use screen space ambient occlusion or not.
    pub use_ssao: bool,
    /// Radius of sampling hemisphere used in SSAO, it defines much ambient
    /// occlusion will be in your scene.
    pub ssao_radius: f32,

    /// Global switch to enable or disable light scattering. Each light can have
    /// its own scatter switch, but this one is able to globally disable scatter.
    pub light_scatter_enabled: bool,

    /// Whether to use Fast Approximate AntiAliasing or not.
    pub fxaa: bool,

    /// Whether to use Parallax Mapping or not.
    pub use_parallax_mapping: bool,
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self::high()
    }
}

impl QualitySettings {
    /// Highest possible graphics quality. Requires very powerful GPU.
    pub fn ultra() -> Self {
        Self {
            point_shadow_map_size: 2048,
            point_shadows_distance: 20.0,
            point_shadows_enabled: true,
            point_soft_shadows: true,

            spot_shadow_map_size: 2048,
            spot_shadows_distance: 20.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: true,

            use_ssao: true,
            ssao_radius: 0.5,

            light_scatter_enabled: true,

            point_shadow_map_precision: ShadowMapPrecision::Full,
            spot_shadow_map_precision: ShadowMapPrecision::Full,

            fxaa: true,

            use_parallax_mapping: false, // TODO: Enable when it is fixed!
        }
    }

    /// High graphics quality, includes all graphical effects. Requires powerful GPU.
    pub fn high() -> Self {
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

            light_scatter_enabled: true,

            point_shadow_map_precision: ShadowMapPrecision::Full,
            spot_shadow_map_precision: ShadowMapPrecision::Full,

            fxaa: true,

            use_parallax_mapping: false, // TODO: Enable when it is fixed!
        }
    }

    /// Medium graphics quality, some of effects are disabled, shadows will have sharp edges.
    pub fn medium() -> Self {
        Self {
            point_shadow_map_size: 512,
            point_shadows_distance: 5.0,
            point_shadows_enabled: true,
            point_soft_shadows: false,

            spot_shadow_map_size: 512,
            spot_shadows_distance: 5.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: false,

            use_ssao: true,
            ssao_radius: 0.5,

            light_scatter_enabled: false,

            point_shadow_map_precision: ShadowMapPrecision::Half,
            spot_shadow_map_precision: ShadowMapPrecision::Half,

            fxaa: true,

            use_parallax_mapping: false,
        }
    }

    /// Lowest graphics quality, all effects are disabled.
    pub fn low() -> Self {
        Self {
            point_shadow_map_size: 1, // Zero is unsupported.
            point_shadows_distance: 0.0,
            point_shadows_enabled: false,
            point_soft_shadows: false,

            spot_shadow_map_size: 1,
            spot_shadows_distance: 0.0,
            spot_shadows_enabled: false,
            spot_soft_shadows: false,

            use_ssao: false,
            ssao_radius: 0.5,

            light_scatter_enabled: false,

            point_shadow_map_precision: ShadowMapPrecision::Half,
            spot_shadow_map_precision: ShadowMapPrecision::Half,

            fxaa: false,

            use_parallax_mapping: false,
        }
    }
}

impl Statistics {
    /// Must be called before render anything.
    fn begin_frame(&mut self) {
        self.frame_start_time = instant::Instant::now();
        self.geometry = Default::default();
        self.lighting = Default::default();
    }

    /// Must be called before SwapBuffers but after all rendering is done.
    fn end_frame(&mut self) {
        let current_time = instant::Instant::now();

        self.pure_frame_time = current_time
            .duration_since(self.frame_start_time)
            .as_secs_f32();
        self.frame_counter += 1;

        if current_time
            .duration_since(self.last_fps_commit_time)
            .as_secs_f32()
            >= 1.0
        {
            self.last_fps_commit_time = current_time;
            self.frames_per_second = self.frame_counter;
            self.frame_counter = 0;
        }
    }

    /// Must be called after SwapBuffers to get capped frame time.
    fn finalize(&mut self) {
        self.capped_frame_time = instant::Instant::now()
            .duration_since(self.frame_start_time)
            .as_secs_f32();
    }
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            pipeline: Default::default(),
            lighting: Default::default(),
            geometry: Default::default(),
            pure_frame_time: 0.0,
            capped_frame_time: 0.0,
            frames_per_second: 0,
            frame_counter: 0,
            frame_start_time: instant::Instant::now(),
            last_fps_commit_time: instant::Instant::now(),
        }
    }
}

/// A sending point for textures that should be uploaded to GPU memory.
#[derive(Clone)]
pub struct TextureUploadSender {
    sender: Sender<Texture>,
}

impl TextureUploadSender {
    /// Requests an upload of the texture to GPU memory.
    pub fn request_upload(&self, texture: Texture) {
        self.sender
            .send(texture)
            .expect("Texture upload receiver must be alive while renderer is alive")
    }
}

/// See module docs.
pub struct Renderer {
    backbuffer: FrameBuffer,
    deferred_light_renderer: DeferredLightRenderer,
    flat_shader: FlatShader,
    sprite_renderer: SpriteRenderer,
    particle_system_renderer: ParticleSystemRenderer,
    // Dummy white one pixel texture which will be used as stub when rendering
    // something without texture specified.
    white_dummy: Rc<RefCell<GpuTexture>>,
    black_dummy: Rc<RefCell<GpuTexture>>,
    environment_dummy: Rc<RefCell<GpuTexture>>,
    // Dummy one pixel texture with (0, 1, 0) vector is used as stub when rendering
    // something without normal map.
    normal_dummy: Rc<RefCell<GpuTexture>>,
    // Dummy one pixel texture used as stub when rendering something without a
    // specular texture
    specular_dummy: Rc<RefCell<GpuTexture>>,
    ui_renderer: UiRenderer,
    statistics: Statistics,
    quad: SurfaceData,
    frame_size: (u32, u32),
    quality_settings: QualitySettings,
    /// Debug renderer instance can be used for debugging purposes
    pub debug_renderer: DebugRenderer,
    /// Scene -> G-buffer mapping.
    scene_to_gbuffer_map: HashMap<Handle<Scene>, GBuffer>,
    backbuffer_clear_color: Color,
    texture_cache: TextureCache,
    geometry_cache: GeometryCache,
    batch_storage: BatchStorage,
    forward_renderer: ForwardRenderer,
    fxaa_renderer: FxaaRenderer,
    renderer2d: Renderer2d,
    texture_upload_receiver: Receiver<Texture>,
    texture_upload_sender: Sender<Texture>,
    // TextureId -> FrameBuffer mapping. This mapping is used for temporal frame buffers
    // like ones used to render UI instances.
    ui_frame_buffers: HashMap<usize, FrameBuffer>,
    // MUST BE LAST! Otherwise you'll get crash, because other parts of the renderer will
    // contain **pointer** to pipeline state. It must be dropped last!
    state: Box<PipelineState>,
}

fn make_ui_frame_buffer(
    frame_size: Vector2<f32>,
    state: &mut PipelineState,
) -> Result<FrameBuffer, FrameworkError> {
    let color_texture = Rc::new(RefCell::new(GpuTexture::new(
        state,
        GpuTextureKind::Rectangle {
            width: frame_size.x as usize,
            height: frame_size.y as usize,
        },
        PixelKind::RGBA8,
        MinificationFilter::Linear,
        MagnificationFilter::Linear,
        1,
        None,
    )?));

    let depth_stencil = Rc::new(RefCell::new(GpuTexture::new(
        state,
        GpuTextureKind::Rectangle {
            width: frame_size.x as usize,
            height: frame_size.y as usize,
        },
        PixelKind::D24S8,
        MinificationFilter::Nearest,
        MagnificationFilter::Nearest,
        1,
        None,
    )?));

    FrameBuffer::new(
        state,
        Some(Attachment {
            kind: AttachmentKind::DepthStencil,
            texture: depth_stencil,
        }),
        vec![Attachment {
            kind: AttachmentKind::Color,
            texture: color_texture,
        }],
    )
}

impl Renderer {
    pub(in crate) fn new(
        context: glow::Context,
        frame_size: (u32, u32),
    ) -> Result<Self, FrameworkError> {
        let settings = QualitySettings::default();

        let (texture_upload_sender, texture_upload_receiver) = std::sync::mpsc::channel();

        // Box pipeline state because we'll store pointers to it inside framework's entities and
        // it must have constant address.
        let mut state = Box::new(PipelineState::new(context));

        Ok(Self {
            backbuffer: FrameBuffer::backbuffer(&mut state),
            frame_size,
            deferred_light_renderer: DeferredLightRenderer::new(&mut state, frame_size, &settings)?,
            flat_shader: FlatShader::new(&mut state)?,
            sprite_renderer: SpriteRenderer::new(&mut state)?,
            white_dummy: Rc::new(RefCell::new(GpuTexture::new(
                &mut state,
                GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                PixelKind::RGBA8,
                MinificationFilter::Linear,
                MagnificationFilter::Linear,
                1,
                Some(&[255u8, 255u8, 255u8, 255u8]),
            )?)),
            black_dummy: Rc::new(RefCell::new(GpuTexture::new(
                &mut state,
                GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                PixelKind::RGBA8,
                MinificationFilter::Linear,
                MagnificationFilter::Linear,
                1,
                Some(&[0u8, 0u8, 0u8, 255u8]),
            )?)),
            environment_dummy: Rc::new(RefCell::new(GpuTexture::new(
                &mut state,
                GpuTextureKind::Cube {
                    width: 1,
                    height: 1,
                },
                PixelKind::RGBA8,
                MinificationFilter::Linear,
                MagnificationFilter::Linear,
                1,
                Some(&[
                    0u8, 0u8, 0u8, 255u8, // pos-x
                    0u8, 0u8, 0u8, 255u8, // neg-x
                    0u8, 0u8, 0u8, 255u8, // pos-y
                    0u8, 0u8, 0u8, 255u8, // neg-y
                    0u8, 0u8, 0u8, 255u8, // pos-z
                    0u8, 0u8, 0u8, 255u8, // neg-z
                ]),
            )?)),
            normal_dummy: Rc::new(RefCell::new(GpuTexture::new(
                &mut state,
                GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                PixelKind::RGBA8,
                MinificationFilter::Linear,
                MagnificationFilter::Linear,
                1,
                Some(&[128u8, 128u8, 255u8, 255u8]),
            )?)),
            specular_dummy: Rc::new(RefCell::new(GpuTexture::new(
                &mut state,
                GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                PixelKind::RGBA8,
                MinificationFilter::Linear,
                MagnificationFilter::Linear,
                1,
                Some(&[32u8, 32u8, 32u8, 32u8]),
            )?)),
            quad: SurfaceData::make_unit_xy_quad(),
            ui_renderer: UiRenderer::new(&mut state)?,
            particle_system_renderer: ParticleSystemRenderer::new(&mut state)?,
            quality_settings: settings,
            debug_renderer: DebugRenderer::new(&mut state)?,
            scene_to_gbuffer_map: Default::default(),
            backbuffer_clear_color: Color::BLACK,
            texture_cache: Default::default(),
            geometry_cache: Default::default(),
            batch_storage: Default::default(),
            forward_renderer: ForwardRenderer::new(&mut state)?,
            ui_frame_buffers: Default::default(),
            fxaa_renderer: FxaaRenderer::new(&mut state)?,
            statistics: Statistics::default(),
            renderer2d: Renderer2d::new(&mut state)?,
            texture_upload_receiver,
            texture_upload_sender,
            state,
        })
    }

    /// Returns statistics for last frame.
    pub fn get_statistics(&self) -> Statistics {
        self.statistics
    }

    /// Unloads texture from GPU memory.
    pub fn unload_texture(&mut self, texture: Texture) {
        self.texture_cache.unload(texture)
    }

    /// Sets color which will be used to fill screen when there is nothing to render.
    pub fn set_backbuffer_clear_color(&mut self, color: Color) {
        self.backbuffer_clear_color = color;
    }

    pub(in crate) fn upload_sender(&self) -> TextureUploadSender {
        TextureUploadSender {
            sender: self.texture_upload_sender.clone(),
        }
    }

    /// Sets new frame size, should be called when received a Resize event.
    ///
    /// # Notes
    ///
    /// Input values will be set to 1 pixel if new size is 0. Rendering cannot
    /// be performed into 0x0 texture.
    pub fn set_frame_size(&mut self, new_size: (u32, u32)) {
        self.deferred_light_renderer
            .set_frame_size(&mut self.state, new_size)
            .unwrap();
        self.frame_size.0 = new_size.0.max(1);
        self.frame_size.1 = new_size.1.max(1);
    }

    /// Returns current (width, height) pair of back buffer size.
    pub fn get_frame_size(&self) -> (u32, u32) {
        self.frame_size
    }

    /// Returns current bounds of back buffer.
    pub fn get_frame_bounds(&self) -> Vector2<f32> {
        Vector2::new(self.frame_size.0 as f32, self.frame_size.1 as f32)
    }

    /// Sets new quality settings for renderer. Never call this method in a loop, otherwise
    /// you may get **significant** lags. Always check if current quality setting differs
    /// from new!
    pub fn set_quality_settings(
        &mut self,
        settings: &QualitySettings,
    ) -> Result<(), FrameworkError> {
        self.quality_settings = *settings;
        self.deferred_light_renderer
            .set_quality_settings(&mut self.state, settings)
    }

    /// Returns current quality settings.
    pub fn get_quality_settings(&self) -> QualitySettings {
        self.quality_settings
    }

    /// Removes all cached GPU data, forces renderer to re-upload data to GPU.
    /// Do not call this method until you absolutely need! It may cause **significant**
    /// performance lag!
    pub fn flush(&mut self) {
        self.texture_cache.clear();
        self.geometry_cache.clear();
        self.renderer2d.flush();
    }

    /// Renders given UI into specified render target. This method is especially useful if you need
    /// to have off-screen UIs (like interactive touch-screen in Doom 3, Dead Space, etc).
    pub fn render_ui_to_texture<M: MessageData, C: Control<M, C>>(
        &mut self,
        render_target: Texture,
        ui: &mut UserInterface<M, C>,
    ) -> Result<(), FrameworkError> {
        let new_width = ui.screen_size().x as usize;
        let new_height = ui.screen_size().y as usize;

        // Create or reuse existing frame buffer.
        let frame_buffer = match self.ui_frame_buffers.entry(render_target.key()) {
            Entry::Occupied(entry) => {
                let frame_buffer = entry.into_mut();
                let frame = frame_buffer.color_attachments().first().unwrap();
                let color_texture_kind = frame.texture.borrow().kind();
                if let GpuTextureKind::Rectangle { width, height } = color_texture_kind {
                    if width != new_width || height != new_height {
                        *frame_buffer = make_ui_frame_buffer(ui.screen_size(), &mut self.state)?;
                    }
                } else {
                    panic!("ui can be rendered only in rectangle texture!")
                }
                frame_buffer
            }
            Entry::Vacant(entry) => {
                entry.insert(make_ui_frame_buffer(ui.screen_size(), &mut self.state)?)
            }
        };

        let viewport = Rect::new(0, 0, new_width as i32, new_height as i32);

        frame_buffer.clear(
            &mut self.state,
            viewport,
            Some(Color::TRANSPARENT),
            Some(0.0),
            Some(0),
        );

        self.statistics += self.ui_renderer.render(UiRenderContext {
            state: &mut self.state,
            viewport,
            frame_buffer,
            frame_width: ui.screen_size().x,
            frame_height: ui.screen_size().y,
            drawing_context: &ui.draw(),
            white_dummy: self.white_dummy.clone(),
            texture_cache: &mut self.texture_cache,
        })?;

        // Finally register texture in the cache so it will become available as texture in deferred/forward
        // renderer.
        self.texture_cache.map.insert(
            render_target.key(),
            CacheEntry {
                value: frame_buffer
                    .color_attachments()
                    .first()
                    .unwrap()
                    .texture
                    .clone(),
                time_to_live: f32::INFINITY,
                value_hash: 0, // TODO
            },
        );

        Ok(())
    }

    fn update_texture_cache(&mut self, dt: f32) {
        // Maximum amount of textures uploaded to GPU per frame. This defines throughput **only** for
        // requests from resource manager. This is needed to prevent huge lag when there are tons of
        // requests, so this is some kind of work load balancer.
        const THROUGHPUT: usize = 5;

        let mut uploaded = 0;
        while let Ok(texture) = self.texture_upload_receiver.try_recv() {
            // Just "touch" texture in the cache and it will load texture to GPU.
            let _ = self.texture_cache.get(&mut self.state, &texture);

            uploaded += 1;
            if uploaded >= THROUGHPUT {
                break;
            }
        }

        self.texture_cache.update(dt);
    }

    fn render_frame(
        &mut self,
        scenes: &SceneContainer,
        drawing_context: &DrawingContext,
        scenes2d: &Scene2dContainer,
        dt: f32,
    ) -> Result<(), FrameworkError> {
        scope_profile!();

        // We have to invalidate resource bindings cache because some textures or programs,
        // or other GL resources can be destroyed and then on their "names" some new resource
        // are created, but cache still thinks that resource is correctly bound, but it is different
        // object have same name.
        self.state.invalidate_resource_bindings_cache();

        // Update caches - this will remove timed out resources.
        self.update_texture_cache(dt);
        self.geometry_cache.update(dt);

        self.statistics.begin_frame();

        let window_viewport = Rect::new(0, 0, self.frame_size.0 as i32, self.frame_size.1 as i32);
        self.backbuffer.clear(
            &mut self.state,
            window_viewport,
            Some(self.backbuffer_clear_color),
            Some(1.0),
            Some(0),
        );

        let backbuffer_width = self.frame_size.0 as f32;
        let backbuffer_height = self.frame_size.1 as f32;

        for (scene_handle, scene) in scenes.pair_iter().filter(|(_, s)| s.enabled) {
            let graph = &scene.graph;

            let frame_size = scene.render_target.as_ref().map_or_else(
                // Use either backbuffer size
                || Vector2::new(backbuffer_width, backbuffer_height),
                // Or framebuffer size
                |rt| {
                    if let TextureKind::Rectangle { width, height } = rt.data_ref().kind() {
                        Vector2::new(width as f32, height as f32)
                    } else {
                        panic!("only rectangle textures can be used as render target!")
                    }
                },
            );

            let state = &mut self.state;

            self.batch_storage.generate_batches(
                state,
                graph,
                self.black_dummy.clone(),
                self.white_dummy.clone(),
                self.normal_dummy.clone(),
                self.specular_dummy.clone(),
                &mut self.texture_cache,
            );

            let gbuffer = self
                .scene_to_gbuffer_map
                .entry(scene_handle)
                .and_modify(|buf| {
                    if buf.width != frame_size.x as i32 || buf.height != frame_size.y as i32 {
                        let width = (frame_size.x as usize).max(1);
                        let height = (frame_size.y as usize).max(1);
                        *buf = GBuffer::new(state, width, height).unwrap();
                    }
                })
                .or_insert_with(|| {
                    let width = (frame_size.x as usize).max(1);
                    let height = (frame_size.y as usize).max(1);
                    GBuffer::new(state, width, height).unwrap()
                });

            // If we specified a texture to draw to, we have to register it in texture cache
            // so it can be used in later on as texture. This is useful in case if you need
            // to draw something on offscreen and then draw it on some mesh.
            // TODO: However it can be dangerous to use frame texture as it may be bound to
            //  pipeline.
            if let Some(rt) = scene.render_target.clone() {
                self.texture_cache.map.insert(
                    rt.key(),
                    CacheEntry {
                        value: gbuffer.frame_texture(),
                        time_to_live: std::f32::INFINITY,
                        value_hash: 0, // TODO
                    },
                );
            }

            for camera in graph.linear_iter().filter_map(|node| {
                if let Node::Camera(camera) = node {
                    if camera.is_enabled() {
                        Some(camera)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) {
                let viewport = camera.viewport_pixels(frame_size);

                self.statistics += gbuffer.fill(GBufferRenderContext {
                    state,
                    camera,
                    geom_cache: &mut self.geometry_cache,
                    batch_storage: &self.batch_storage,
                    texture_cache: &mut self.texture_cache,
                    environment_dummy: self.environment_dummy.clone(),
                    use_parallax_mapping: self.quality_settings.use_parallax_mapping,
                });

                let (pass_stats, light_stats) =
                    self.deferred_light_renderer
                        .render(DeferredRendererContext {
                            state,
                            scene,
                            camera,
                            gbuffer,
                            white_dummy: self.white_dummy.clone(),
                            ambient_color: scene.ambient_lighting_color,
                            settings: &self.quality_settings,
                            textures: &mut self.texture_cache,
                            geometry_cache: &mut self.geometry_cache,
                            batch_storage: &self.batch_storage,
                        });

                self.statistics.lighting += light_stats;
                self.statistics.geometry += pass_stats;

                let depth = gbuffer.depth();

                self.statistics +=
                    self.particle_system_renderer
                        .render(ParticleSystemRenderContext {
                            state,
                            framebuffer: &mut gbuffer.final_frame,
                            graph,
                            camera,
                            white_dummy: self.white_dummy.clone(),
                            depth,
                            frame_width: frame_size.x,
                            frame_height: frame_size.y,
                            viewport,
                            texture_cache: &mut self.texture_cache,
                        });

                self.statistics += self.sprite_renderer.render(SpriteRenderContext {
                    state,
                    framebuffer: &mut gbuffer.final_frame,
                    graph,
                    camera,
                    white_dummy: self.white_dummy.clone(),
                    viewport,
                    textures: &mut self.texture_cache,
                    geom_map: &mut self.geometry_cache,
                });

                self.statistics += self.forward_renderer.render(ForwardRenderContext {
                    state,
                    camera,
                    geom_cache: &mut self.geometry_cache,
                    batch_storage: &self.batch_storage,
                    framebuffer: &mut gbuffer.final_frame, // TODO: GBuffer **must not** contain final frame.
                    viewport,
                });

                self.statistics += self.debug_renderer.render(
                    state,
                    viewport,
                    &mut gbuffer.final_frame,
                    &scene.drawing_context,
                    camera,
                );

                // Finally render everything into back buffer.
                if scene.render_target.is_none() {
                    if self.quality_settings.fxaa {
                        self.statistics.geometry += self.fxaa_renderer.render(
                            state,
                            viewport,
                            gbuffer.frame_texture(),
                            &mut self.backbuffer,
                            &mut self.geometry_cache,
                        );
                    } else {
                        let shader = &self.flat_shader;
                        self.statistics.geometry += self.backbuffer.draw(
                            self.geometry_cache.get(state, &self.quad),
                            state,
                            viewport,
                            &self.flat_shader.program,
                            &DrawParameters {
                                cull_face: CullFace::Back,
                                culling: false,
                                color_write: Default::default(),
                                depth_write: true,
                                stencil_test: false,
                                depth_test: false,
                                blend: false,
                            },
                            |program_binding| {
                                program_binding
                                    .set_matrix4(&shader.wvp_matrix, &{
                                        Matrix4::new_orthographic(
                                            0.0,
                                            viewport.w() as f32,
                                            viewport.h() as f32,
                                            0.0,
                                            -1.0,
                                            1.0,
                                        ) * Matrix4::new_nonuniform_scaling(&Vector3::new(
                                            viewport.w() as f32,
                                            viewport.h() as f32,
                                            0.0,
                                        ))
                                    })
                                    .set_texture(&shader.diffuse_texture, &gbuffer.frame_texture());
                            },
                        );
                    }
                }
            }
        }

        self.statistics += self.renderer2d.render(
            &mut self.state,
            &mut self.backbuffer,
            Vector2::new(backbuffer_width, backbuffer_height),
            scenes2d,
            &mut self.texture_cache,
            self.white_dummy.clone(),
            dt,
        )?;

        // Render UI on top of everything.
        self.statistics += self.ui_renderer.render(UiRenderContext {
            state: &mut self.state,
            viewport: window_viewport,
            frame_buffer: &mut self.backbuffer,
            frame_width: backbuffer_width,
            frame_height: backbuffer_height,
            drawing_context,
            white_dummy: self.white_dummy.clone(),
            texture_cache: &mut self.texture_cache,
        })?;

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate) fn render_and_swap_buffers(
        &mut self,
        scenes: &SceneContainer,
        drawing_context: &DrawingContext,
        scenes2d: &Scene2dContainer,
        context: &glutin::WindowedContext<glutin::PossiblyCurrent>,
        dt: f32,
    ) -> Result<(), FrameworkError> {
        self.render_frame(scenes, drawing_context, scenes2d, dt)?;
        self.statistics.end_frame();
        context.swap_buffers()?;
        self.state.check_error();
        self.statistics.finalize();
        self.statistics.pipeline = self.state.pipeline_statistics();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub(in crate) fn render_and_swap_buffers(
        &mut self,
        scenes: &SceneContainer,
        drawing_context: &DrawingContext,
        scenes2d: &Scene2dContainer,
        dt: f32,
    ) -> Result<(), FrameworkError> {
        self.render_frame(scenes, drawing_context, scenes2d, dt)?;
        self.statistics.end_frame();
        self.state.check_error();
        self.statistics.finalize();
        self.statistics.pipeline = self.state.pipeline_statistics();
        Ok(())
    }
}
