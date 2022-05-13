//! Renderer is a "workhorse" of the engine, it draws scenes (both 3D and 2D), user interface,
//! debug geometry and has an ability to add user-defined render passes. Current renderer
//! implementation is not very flexible, but should cover 95% of use cases.
//!
//! # Implementation details
//!
//! Renderer is based on OpenGL 3.3+ Core.

#![warn(missing_docs)]
#![deny(unsafe_code)]

// Framework is 100% unsafe internally due to FFI calls.
#[allow(unsafe_code)]
pub mod framework;

pub mod batch;
pub mod cache;
pub mod debug_renderer;
pub mod renderer2d;
pub mod ui_renderer;

mod bloom;
mod flat_shader;
mod forward_renderer;
mod fxaa;
mod gbuffer;
mod hdr;
mod light;
mod light_volume;
mod particle_system_renderer;
mod shadow;
mod skybox_shader;
mod sprite_renderer;
mod ssao;

use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
        inspect::{Inspect, PropertyInfo},
        instant,
        math::Rect,
        pool::Handle,
        scope_profile,
    },
    engine::resource_manager::{container::event::ResourceEvent, ResourceManager},
    gui::{draw::DrawingContext, UserInterface},
    material::{
        shader::{SamplerFallback, Shader},
        Material, PropertyValue,
    },
    renderer::{
        batch::BatchStorage,
        bloom::BloomRenderer,
        cache::{geometry::GeometryCache, shader::ShaderCache, texture::TextureCache, CacheEntry},
        debug_renderer::DebugRenderer,
        flat_shader::FlatShader,
        forward_renderer::{ForwardRenderContext, ForwardRenderer},
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, DrawParameters, FrameBuffer},
            geometry_buffer::{DrawCallStatistics, GeometryBuffer, GeometryBufferKind},
            gpu_program::{BuiltInUniform, GpuProgramBinding},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::{PipelineState, PipelineStatistics},
        },
        fxaa::FxaaRenderer,
        gbuffer::{GBuffer, GBufferRenderContext},
        hdr::HighDynamicRangeRenderer,
        light::{DeferredLightRenderer, DeferredRendererContext, LightingStatistics},
        particle_system_renderer::{ParticleSystemRenderContext, ParticleSystemRenderer},
        renderer2d::Renderer2d,
        sprite_renderer::{SpriteRenderContext, SpriteRenderer},
        ui_renderer::{UiRenderContext, UiRenderer},
    },
    resource::texture::{Texture, TextureKind},
    scene::{camera::Camera, mesh::surface::SurfaceData, Scene, SceneContainer},
    utils::log::{Log, MessageKind},
};
use fxhash::FxHashMap;
use glow::HasContext;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::hash_map::Entry,
    fmt::{Display, Formatter},
    rc::Rc,
    sync::mpsc::Receiver,
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// Renderer statistics for one frame, also includes current frames per second
/// amount.
#[derive(Debug, Copy, Clone)]
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
#[derive(Debug, Copy, Clone, Default)]
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
#[derive(
    Copy,
    Clone,
    Hash,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Debug,
    Serialize,
    Deserialize,
    Inspect,
    AsRefStr,
    EnumString,
    EnumVariantNames,
)]
pub enum ShadowMapPrecision {
    /// Shadow map will use 2 times less memory by switching to 16bit pixel format,
    /// but "shadow acne" may occur.
    Half,
    /// Shadow map will use 32bit pixel format. This option gives highest quality,
    /// but could be less performant than `Half`.
    Full,
}

/// Cascaded-shadow maps settings.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Inspect)]
pub struct CsmSettings {
    /// Whether cascaded shadow maps enabled or not.
    pub enabled: bool,

    /// Size of texture for each cascade.
    pub size: usize,

    /// Bit-wise precision for each cascade, the lower precision the better performance is,
    /// but the more artifacts may occur.
    pub precision: ShadowMapPrecision,

    /// Whether to use Percentage-Closer Filtering or not.
    pub pcf: bool,
}

impl Default for CsmSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            size: 2048,
            precision: ShadowMapPrecision::Full,
            pcf: true,
        }
    }
}

/// Quality settings allows you to find optimal balance between performance and
/// graphics quality.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Inspect)]
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

    /// Cascaded-shadow maps settings.
    pub csm_settings: CsmSettings,

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

    /// Whether to use bloom effect.
    pub use_bloom: bool,
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

            use_bloom: true,

            use_parallax_mapping: false, // TODO: Enable when it is fixed!

            csm_settings: Default::default(),
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

            use_bloom: true,

            use_parallax_mapping: false, // TODO: Enable when it is fixed!

            csm_settings: CsmSettings {
                enabled: true,
                size: 2048,
                precision: ShadowMapPrecision::Full,
                pcf: true,
            },
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

            use_bloom: true,

            use_parallax_mapping: false,

            csm_settings: CsmSettings {
                enabled: true,
                size: 512,
                precision: ShadowMapPrecision::Full,
                pcf: false,
            },
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

            use_bloom: false,

            use_parallax_mapping: false,

            csm_settings: CsmSettings {
                enabled: true,
                size: 512,
                precision: ShadowMapPrecision::Half,
                pcf: false,
            },
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

/// A set of frame buffers, renderers, that contains scene-specific data.
pub struct AssociatedSceneData {
    /// G-Buffer of the scene.
    pub gbuffer: GBuffer,

    /// Intermediate high dynamic range frame buffer.
    pub hdr_scene_framebuffer: FrameBuffer,

    /// Final frame of the scene. Tone mapped + gamma corrected.
    pub ldr_scene_framebuffer: FrameBuffer,

    /// Additional frame buffer for post processing.
    pub ldr_temp_framebuffer: FrameBuffer,

    /// HDR renderer has be created per scene, because it contains
    /// scene luminance.
    pub hdr_renderer: HighDynamicRangeRenderer,

    /// Bloom contains only overly bright pixels that creates light
    /// bleeding effect (glow effect).
    pub bloom_renderer: BloomRenderer,
}

impl AssociatedSceneData {
    /// Creates new scene data.
    pub fn new(
        state: &mut PipelineState,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        let mut depth_stencil_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::D24S8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        depth_stencil_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let depth_stencil = Rc::new(RefCell::new(depth_stencil_texture));

        let hdr_frame_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            // Intermediate scene frame will be rendered in HDR render target.
            PixelKind::RGBA16F,
            MinificationFilter::Linear,
            MagnificationFilter::Linear,
            1,
            None,
        )?;

        let hdr_scene_framebuffer = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil.clone(),
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: Rc::new(RefCell::new(hdr_frame_texture)),
            }],
        )?;

        let ldr_frame_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            // Final scene frame is in standard sRGB space.
            PixelKind::RGBA8,
            MinificationFilter::Linear,
            MagnificationFilter::Linear,
            1,
            None,
        )?;

        let ldr_scene_framebuffer = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil.clone(),
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: Rc::new(RefCell::new(ldr_frame_texture)),
            }],
        )?;

        let ldr_temp_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            // Final scene frame is in standard sRGB space.
            PixelKind::RGBA8,
            MinificationFilter::Linear,
            MagnificationFilter::Linear,
            1,
            None,
        )?;

        let ldr_temp_framebuffer = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil,
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: Rc::new(RefCell::new(ldr_temp_texture)),
            }],
        )?;

        Ok(Self {
            gbuffer: GBuffer::new(state, width, height)?,
            hdr_renderer: HighDynamicRangeRenderer::new(state)?,
            bloom_renderer: BloomRenderer::new(state, width, height)?,
            hdr_scene_framebuffer,
            ldr_scene_framebuffer,
            ldr_temp_framebuffer,
        })
    }

    fn copy_depth_stencil_to_scene_framebuffer(&mut self, state: &mut PipelineState) {
        state.blit_framebuffer(
            self.gbuffer.framebuffer().id(),
            self.hdr_scene_framebuffer.id(),
            0,
            0,
            self.gbuffer.width,
            self.gbuffer.height,
            0,
            0,
            self.gbuffer.width,
            self.gbuffer.height,
            false,
            true,
            true,
        );
    }

    /// Returns high-dynamic range frame buffer texture.
    pub fn hdr_scene_frame_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.hdr_scene_framebuffer.color_attachments()[0]
            .texture
            .clone()
    }

    /// Returns low-dynamic range frame buffer texture (final frame).
    pub fn ldr_scene_frame_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.ldr_scene_framebuffer.color_attachments()[0]
            .texture
            .clone()
    }

    /// Returns low-dynamic range frame buffer texture (accumulation frame).
    pub fn ldr_temp_frame_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.ldr_temp_framebuffer.color_attachments()[0]
            .texture
            .clone()
    }
}

pub(in crate) fn make_viewport_matrix(viewport: Rect<i32>) -> Matrix4<f32> {
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
}

/// See module docs.
pub struct Renderer {
    backbuffer: FrameBuffer,
    scene_render_passes: Vec<Rc<RefCell<dyn SceneRenderPass>>>,
    deferred_light_renderer: DeferredLightRenderer,
    flat_shader: FlatShader,
    sprite_renderer: SpriteRenderer,
    particle_system_renderer: ParticleSystemRenderer,
    /// Dummy white one pixel texture which will be used as stub when rendering
    /// something without texture specified.
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    black_dummy: Rc<RefCell<GpuTexture>>,
    environment_dummy: Rc<RefCell<GpuTexture>>,
    // Dummy one pixel texture with (0, 1, 0) vector is used as stub when rendering
    // something without normal map.
    normal_dummy: Rc<RefCell<GpuTexture>>,
    // Dummy one pixel texture used as stub when rendering something without a
    // metallic texture. Default metalness is 0.0
    metallic_dummy: Rc<RefCell<GpuTexture>>,
    /// User interface renderer.
    pub ui_renderer: UiRenderer,
    statistics: Statistics,
    quad: GeometryBuffer,
    frame_size: (u32, u32),
    quality_settings: QualitySettings,
    /// Debug renderer instance can be used for debugging purposes
    pub debug_renderer: DebugRenderer,
    /// A set of associated data for each scene that was rendered.
    pub scene_data_map: FxHashMap<Handle<Scene>, AssociatedSceneData>,
    backbuffer_clear_color: Color,
    /// Texture cache with GPU textures.
    pub texture_cache: TextureCache,
    shader_cache: ShaderCache,
    geometry_cache: GeometryCache,
    batch_storage: BatchStorage,
    forward_renderer: ForwardRenderer,
    fxaa_renderer: FxaaRenderer,
    renderer2d: Renderer2d,
    texture_event_receiver: Receiver<ResourceEvent<Texture>>,
    shader_event_receiver: Receiver<ResourceEvent<Shader>>,
    // TextureId -> FrameBuffer mapping. This mapping is used for temporal frame buffers
    // like ones used to render UI instances.
    ui_frame_buffers: FxHashMap<usize, FrameBuffer>,
    // MUST BE LAST! Otherwise you'll get crash, because other parts of the renderer will
    // contain **pointer** to pipeline state. It must be dropped last!
    /// Pipeline state.
    pub state: Box<PipelineState>,
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

/// A context for custom scene render passes.
pub struct SceneRenderPassContext<'a, 'b> {
    /// A pipeline state that is used as a wrapper to underlying graphics API.
    pub pipeline_state: &'a mut PipelineState,

    /// A texture cache that uploads engine's `Texture` as internal `GpuTexture` to GPU.
    /// Use this to get a corresponding GPU texture by an instance of a `Texture`.
    pub texture_cache: &'a mut TextureCache,

    /// A geometry cache that uploads engine's `SurfaceData` as internal `GeometryBuffer` to GPU.
    /// Use this to get a corresponding GPU geometry buffer (essentially it is just a VAO) by an
    /// instance of a `SurfaceData`.
    pub geometry_cache: &'a mut GeometryCache,

    /// A storage that contains "pre-compiled" groups of render data (batches).
    pub batch_storage: &'a BatchStorage,

    /// Current quality settings of the renderer.
    pub quality_settings: &'a QualitySettings,

    /// Current framebuffer to which scene is being rendered to.
    pub framebuffer: &'a mut FrameBuffer,

    /// A scene being rendered.
    pub scene: &'b Scene,

    /// A camera from the scene that is used as "eyes".
    pub camera: &'b Camera,

    /// A viewport of the camera.
    pub viewport: Rect<i32>,

    /// A handle of the scene being rendered.
    pub scene_handle: Handle<Scene>,

    /// An 1x1 white pixel texture that could be used a stub when there is no texture.
    pub white_dummy: Rc<RefCell<GpuTexture>>,

    /// An 1x1 pixel texture with (0, 1, 0) vector that could be used a stub when
    /// there is no normal map.
    pub normal_dummy: Rc<RefCell<GpuTexture>>,

    /// An 1x1 pixel with 0.0 metalness factor texture that could be used a stub when
    /// there is no metallic map.
    pub metallic_dummy: Rc<RefCell<GpuTexture>>,

    /// An 1x1 black cube map texture that could be used a stub when there is no environment
    /// texture.
    pub environment_dummy: Rc<RefCell<GpuTexture>>,

    /// An 1x1 black pixel texture that could be used a stub when there is no texture.
    pub black_dummy: Rc<RefCell<GpuTexture>>,

    /// A texture with depth values from G-Buffer.
    ///
    /// # Important notes
    ///
    /// Keep in mind that G-Buffer cannot be modified in custom render passes, so you don't
    /// have an ability to write to this texture. However you can still write to depth of
    /// the frame buffer as you'd normally do.
    pub depth_texture: Rc<RefCell<GpuTexture>>,

    /// A texture with world-space normals from G-Buffer.
    ///
    /// # Important notes
    ///
    /// Keep in mind that G-Buffer cannot be modified in custom render passes, so you don't
    /// have an ability to write to this texture.
    pub normal_texture: Rc<RefCell<GpuTexture>>,

    /// A texture with ambient lighting values from G-Buffer.
    ///
    /// # Important notes
    ///
    /// Keep in mind that G-Buffer cannot be modified in custom render passes, so you don't
    /// have an ability to write to this texture.
    pub ambient_texture: Rc<RefCell<GpuTexture>>,

    /// User interface renderer.
    pub ui_renderer: &'a mut UiRenderer,
}

/// A trait for custom scene rendering pass. It could be used to add your own rendering techniques.
pub trait SceneRenderPass {
    /// Renders scene into high dynamic range target. It will be called for **each** scene
    /// registered in the engine, but you are able to filter out scene by its handle.
    fn on_hdr_render(
        &mut self,
        _ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        Ok(RenderPassStatistics::default())
    }

    /// Renders scene into low dynamic range target. It will be called for **each** scene
    /// registered in the engine, but you are able to filter out scene by its handle.
    fn on_ldr_render(
        &mut self,
        _ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        Ok(RenderPassStatistics::default())
    }
}

fn blit_pixels(
    state: &mut PipelineState,
    framebuffer: &mut FrameBuffer,
    texture: Rc<RefCell<GpuTexture>>,
    shader: &FlatShader,
    viewport: Rect<i32>,
    quad: &GeometryBuffer,
) -> DrawCallStatistics {
    framebuffer.draw(
        quad,
        state,
        viewport,
        &shader.program,
        &DrawParameters {
            cull_face: None,
            color_write: Default::default(),
            depth_write: true,
            stencil_test: None,
            depth_test: false,
            blend: None,
            stencil_op: Default::default(),
        },
        |mut program_binding| {
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
                .set_texture(&shader.diffuse_texture, &texture);
        },
    )
}

pub(in crate) struct MaterialContext<'a, 'b, 'c> {
    pub material: &'a Material,
    pub program_binding: &'a mut GpuProgramBinding<'b, 'c>,
    pub texture_cache: &'a mut TextureCache,

    // Built-in uniforms.
    pub world_matrix: &'a Matrix4<f32>,
    pub wvp_matrix: &'a Matrix4<f32>,
    pub bone_matrices: &'a [Matrix4<f32>],
    pub use_skeletal_animation: bool,
    pub camera_position: &'a Vector3<f32>,
    pub use_pom: bool,
    pub light_position: &'a Vector3<f32>,

    // Fallback samplers.
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub black_dummy: Rc<RefCell<GpuTexture>>,
}

pub(in crate) fn apply_material(ctx: MaterialContext) {
    let built_in_uniforms = &ctx.program_binding.program.built_in_uniform_locations;

    // Apply values for built-in uniforms.
    if let Some(location) = &built_in_uniforms[BuiltInUniform::WorldMatrix as usize] {
        ctx.program_binding.set_matrix4(location, ctx.world_matrix);
    }
    if let Some(location) = &built_in_uniforms[BuiltInUniform::WorldViewProjectionMatrix as usize] {
        ctx.program_binding.set_matrix4(location, ctx.wvp_matrix);
    }
    if let Some(location) = &built_in_uniforms[BuiltInUniform::BoneMatrices as usize] {
        ctx.program_binding
            .set_matrix4_array(location, ctx.bone_matrices);
    }
    if let Some(location) = &built_in_uniforms[BuiltInUniform::UseSkeletalAnimation as usize] {
        ctx.program_binding
            .set_bool(location, ctx.use_skeletal_animation);
    }
    if let Some(location) = &built_in_uniforms[BuiltInUniform::CameraPosition as usize] {
        ctx.program_binding
            .set_vector3(location, ctx.camera_position);
    }
    if let Some(location) = &built_in_uniforms[BuiltInUniform::UsePOM as usize] {
        ctx.program_binding.set_bool(location, ctx.use_pom);
    }
    if let Some(location) = &built_in_uniforms[BuiltInUniform::LightPosition as usize] {
        ctx.program_binding
            .set_vector3(location, ctx.light_position);
    }

    // Apply material properties.
    for (name, value) in ctx.material.properties() {
        if let Some(uniform) = ctx.program_binding.uniform_location(name) {
            match value {
                PropertyValue::Float(v) => {
                    ctx.program_binding.set_f32(&uniform, *v);
                }
                PropertyValue::Int(v) => {
                    ctx.program_binding.set_i32(&uniform, *v);
                }
                PropertyValue::UInt(v) => {
                    ctx.program_binding.set_u32(&uniform, *v);
                }
                PropertyValue::Vector2(v) => {
                    ctx.program_binding.set_vector2(&uniform, v);
                }
                PropertyValue::Vector3(v) => {
                    ctx.program_binding.set_vector3(&uniform, v);
                }
                PropertyValue::Vector4(v) => {
                    ctx.program_binding.set_vector4(&uniform, v);
                }
                PropertyValue::Matrix2(v) => {
                    ctx.program_binding.set_matrix2(&uniform, v);
                }
                PropertyValue::Matrix3(v) => {
                    ctx.program_binding.set_matrix3(&uniform, v);
                }
                PropertyValue::Matrix4(v) => {
                    ctx.program_binding.set_matrix4(&uniform, v);
                }
                PropertyValue::Color(v) => {
                    ctx.program_binding.set_srgb_color(&uniform, v);
                }
                PropertyValue::Bool(v) => {
                    ctx.program_binding.set_bool(&uniform, *v);
                }
                PropertyValue::Sampler { value, fallback } => {
                    let texture = value
                        .as_ref()
                        .and_then(|t| ctx.texture_cache.get(ctx.program_binding.state, t))
                        .unwrap_or_else(|| match fallback {
                            SamplerFallback::White => ctx.white_dummy.clone(),
                            SamplerFallback::Normal => ctx.normal_dummy.clone(),
                            SamplerFallback::Black => ctx.black_dummy.clone(),
                        });

                    ctx.program_binding.set_texture(&uniform, &texture);
                }
                PropertyValue::FloatArray(v) => {
                    ctx.program_binding.set_f32_slice(&uniform, v);
                }
                PropertyValue::IntArray(v) => {
                    ctx.program_binding.set_i32_slice(&uniform, v);
                }
                PropertyValue::UIntArray(v) => {
                    ctx.program_binding.set_u32_slice(&uniform, v);
                }
                PropertyValue::Vector2Array(v) => {
                    ctx.program_binding.set_vector2_slice(&uniform, v);
                }
                PropertyValue::Vector3Array(v) => {
                    ctx.program_binding.set_vector3_slice(&uniform, v);
                }
                PropertyValue::Vector4Array(v) => {
                    ctx.program_binding.set_vector4_slice(&uniform, v);
                }
                PropertyValue::Matrix2Array(v) => {
                    ctx.program_binding.set_matrix2_array(&uniform, v);
                }
                PropertyValue::Matrix3Array(v) => {
                    ctx.program_binding.set_matrix3_array(&uniform, v);
                }
                PropertyValue::Matrix4Array(v) => {
                    ctx.program_binding.set_matrix4_array(&uniform, v);
                }
            }
        }
    }
}

impl Renderer {
    pub(in crate) fn new(
        context: glow::Context,
        frame_size: (u32, u32),
        resource_manager: &ResourceManager,
    ) -> Result<Self, FrameworkError> {
        let settings = QualitySettings::default();

        let (texture_event_sender, texture_event_receiver) = std::sync::mpsc::channel();

        resource_manager
            .state()
            .containers_mut()
            .textures
            .event_broadcaster
            .add(texture_event_sender);

        let (shader_event_sender, shader_event_receiver) = std::sync::mpsc::channel();

        resource_manager
            .state()
            .containers_mut()
            .shaders
            .event_broadcaster
            .add(shader_event_sender);

        // Box pipeline state because we'll store pointers to it inside framework's entities and
        // it must have constant address.
        let mut state = Box::new(PipelineState::new(context));

        // Dump available GL extensions to the log, this will help debugging graphical issues.
        Log::info(format!(
            "Supported GL Extensions: {:?}",
            state.gl.supported_extensions()
        ));

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
            metallic_dummy: Rc::new(RefCell::new(GpuTexture::new(
                &mut state,
                GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                PixelKind::RGBA8,
                MinificationFilter::Linear,
                MagnificationFilter::Linear,
                1,
                Some(&[0u8, 0u8, 0u8, 0u8]),
            )?)),
            quad: GeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                GeometryBufferKind::StaticDraw,
                &mut state,
            ),
            ui_renderer: UiRenderer::new(&mut state)?,
            particle_system_renderer: ParticleSystemRenderer::new(&mut state)?,
            quality_settings: settings,
            debug_renderer: DebugRenderer::new(&mut state)?,
            scene_data_map: Default::default(),
            backbuffer_clear_color: Color::BLACK,
            texture_cache: Default::default(),
            geometry_cache: Default::default(),
            batch_storage: Default::default(),
            forward_renderer: ForwardRenderer::new(),
            ui_frame_buffers: Default::default(),
            fxaa_renderer: FxaaRenderer::new(&mut state)?,
            statistics: Statistics::default(),
            renderer2d: Renderer2d::new(&mut state)?,
            shader_event_receiver,
            texture_event_receiver,
            state,
            shader_cache: ShaderCache::default(),
            scene_render_passes: Default::default(),
        })
    }

    /// Adds a custom render pass.
    pub fn add_render_pass(&mut self, pass: Rc<RefCell<dyn SceneRenderPass>>) {
        self.scene_render_passes.push(pass);
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

    /// Returns a reference to current pipeline state.
    pub fn pipeline_state(&mut self) -> &mut PipelineState {
        &mut self.state
    }

    /// Sets new frame size. You should call the same method on [`crate::engine::Engine`]
    /// instead, which will update the size for the user interface and rendering context
    /// as well as this one.
    ///
    /// # Notes
    ///
    /// Input values will be set to 1 pixel if new size is 0. Rendering cannot
    /// be performed into 0x0 texture.
    pub(in crate) fn set_frame_size(&mut self, new_size: (u32, u32)) -> Result<(), FrameworkError> {
        self.frame_size.0 = new_size.0.max(1);
        self.frame_size.1 = new_size.1.max(1);

        self.deferred_light_renderer
            .set_frame_size(&mut self.state, new_size)?;

        Ok(())
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
    pub fn render_ui_to_texture(
        &mut self,
        render_target: Texture,
        ui: &mut UserInterface,
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
            drawing_context: ui.draw(),
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
        while let Ok(event) = self.texture_event_receiver.try_recv() {
            if let ResourceEvent::Loaded(texture) | ResourceEvent::Reloaded(texture) = event {
                match self.texture_cache.upload(&mut self.state, &texture) {
                    Ok(_) => {
                        uploaded += 1;
                        if uploaded >= THROUGHPUT {
                            break;
                        }
                    }
                    Err(e) => {
                        Log::writeln(
                            MessageKind::Error,
                            format!("Failed to upload texture to GPU. Reason: {:?}", e),
                        );
                    }
                }
            }
        }

        self.texture_cache.update(dt);
    }

    fn update_shader_cache(&mut self, dt: f32) {
        while let Ok(event) = self.shader_event_receiver.try_recv() {
            if let ResourceEvent::Loaded(shader) | ResourceEvent::Reloaded(shader) = event {
                // Remove and immediately "touch" the shader cache to force upload shader.
                self.shader_cache.remove(&shader);
                let _ = self.shader_cache.get(&mut self.state, &shader);
            }
        }

        self.shader_cache.update(dt)
    }

    /// Update caches - this will remove timed out resources.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn update_caches(&mut self, dt: f32) {
        self.update_texture_cache(dt);
        self.update_shader_cache(dt);
        self.geometry_cache.update(dt);
        self.renderer2d.update_caches(dt);
    }

    fn render_frame(
        &mut self,
        scenes: &SceneContainer,
        drawing_context: &DrawingContext,
    ) -> Result<(), FrameworkError> {
        scope_profile!();

        // Make sure to drop associated data for destroyed scenes.
        self.scene_data_map
            .retain(|h, _| scenes.is_valid_handle(*h));

        // We have to invalidate resource bindings cache because some textures or programs,
        // or other GL resources can be destroyed and then on their "names" some new resource
        // are created, but cache still thinks that resource is correctly bound, but it is different
        // object have same name.
        self.state.invalidate_resource_bindings_cache();
        let dt = self.statistics.capped_frame_time;
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

            let frame_size = scene
                .render_target
                .as_ref()
                .map_or_else(
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
                )
                // Clamp to [1.0; infinity] range.
                .sup(&Vector2::new(1.0, 1.0));

            let state = &mut self.state;

            self.batch_storage.generate_batches(graph);

            let scene_associated_data = self
                .scene_data_map
                .entry(scene_handle)
                .and_modify(|data| {
                    if data.gbuffer.width != frame_size.x as i32
                        || data.gbuffer.height != frame_size.y as i32
                    {
                        let width = frame_size.x as usize;
                        let height = frame_size.y as usize;
                        *data = AssociatedSceneData::new(state, width, height).unwrap();
                    }
                })
                .or_insert_with(|| {
                    let width = frame_size.x as usize;
                    let height = frame_size.y as usize;
                    AssociatedSceneData::new(state, width, height).unwrap()
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
                        value: scene_associated_data.ldr_scene_frame_texture(),
                        time_to_live: f32::INFINITY,
                        value_hash: 0, // TODO
                    },
                );
            }

            for camera in graph.linear_iter().filter_map(|node| {
                if let Some(camera) = node.cast::<Camera>() {
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

                self.statistics += scene_associated_data.gbuffer.fill(GBufferRenderContext {
                    state,
                    camera,
                    geom_cache: &mut self.geometry_cache,
                    batch_storage: &self.batch_storage,
                    texture_cache: &mut self.texture_cache,
                    shader_cache: &mut self.shader_cache,
                    environment_dummy: self.environment_dummy.clone(),
                    use_parallax_mapping: self.quality_settings.use_parallax_mapping,
                    normal_dummy: self.normal_dummy.clone(),
                    white_dummy: self.white_dummy.clone(),
                    black_dummy: self.black_dummy.clone(),
                    graph,
                });

                scene_associated_data.copy_depth_stencil_to_scene_framebuffer(state);

                scene_associated_data.hdr_scene_framebuffer.clear(
                    state,
                    viewport,
                    Some(Color::from_rgba(0, 0, 0, 255)),
                    None, // Keep depth, we've just copied valid data in it.
                    Some(0),
                );

                let (pass_stats, light_stats) =
                    self.deferred_light_renderer
                        .render(DeferredRendererContext {
                            state,
                            scene,
                            camera,
                            gbuffer: &mut scene_associated_data.gbuffer,
                            white_dummy: self.white_dummy.clone(),
                            ambient_color: scene.ambient_lighting_color,
                            settings: &self.quality_settings,
                            textures: &mut self.texture_cache,
                            geometry_cache: &mut self.geometry_cache,
                            batch_storage: &self.batch_storage,
                            frame_buffer: &mut scene_associated_data.hdr_scene_framebuffer,
                            shader_cache: &mut self.shader_cache,
                            normal_dummy: self.normal_dummy.clone(),
                            black_dummy: self.black_dummy.clone(),
                        });

                self.statistics.lighting += light_stats;
                self.statistics.geometry += pass_stats;

                let depth = scene_associated_data.gbuffer.depth();

                self.statistics +=
                    self.particle_system_renderer
                        .render(ParticleSystemRenderContext {
                            state,
                            framebuffer: &mut scene_associated_data.hdr_scene_framebuffer,
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
                    framebuffer: &mut scene_associated_data.hdr_scene_framebuffer,
                    graph,
                    camera,
                    white_dummy: self.white_dummy.clone(),
                    viewport,
                    textures: &mut self.texture_cache,
                });

                self.statistics += self.renderer2d.render(
                    state,
                    camera,
                    &mut scene_associated_data.hdr_scene_framebuffer,
                    viewport,
                    graph,
                    &mut self.texture_cache,
                    self.white_dummy.clone(),
                    scene.ambient_lighting_color,
                )?;

                self.statistics += self.forward_renderer.render(ForwardRenderContext {
                    state,
                    camera,
                    geom_cache: &mut self.geometry_cache,
                    texture_cache: &mut self.texture_cache,
                    shader_cache: &mut self.shader_cache,
                    batch_storage: &self.batch_storage,
                    framebuffer: &mut scene_associated_data.hdr_scene_framebuffer,
                    viewport,
                    quality_settings: &self.quality_settings,
                    white_dummy: self.white_dummy.clone(),
                    normal_dummy: self.normal_dummy.clone(),
                    black_dummy: self.black_dummy.clone(),
                });

                for render_pass in self.scene_render_passes.iter() {
                    self.statistics +=
                        render_pass
                            .borrow_mut()
                            .on_hdr_render(SceneRenderPassContext {
                                pipeline_state: state,
                                texture_cache: &mut self.texture_cache,
                                geometry_cache: &mut self.geometry_cache,
                                quality_settings: &self.quality_settings,
                                batch_storage: &self.batch_storage,
                                viewport,
                                scene,
                                camera,
                                scene_handle,
                                white_dummy: self.white_dummy.clone(),
                                normal_dummy: self.normal_dummy.clone(),
                                metallic_dummy: self.metallic_dummy.clone(),
                                environment_dummy: self.environment_dummy.clone(),
                                black_dummy: self.black_dummy.clone(),
                                depth_texture: scene_associated_data.gbuffer.depth(),
                                normal_texture: scene_associated_data.gbuffer.normal_texture(),
                                ambient_texture: scene_associated_data.gbuffer.ambient_texture(),
                                framebuffer: &mut scene_associated_data.hdr_scene_framebuffer,
                                ui_renderer: &mut self.ui_renderer,
                            })?;
                }

                let quad = &self.quad;

                // Prepare glow map.
                self.statistics.geometry += scene_associated_data.bloom_renderer.render(
                    state,
                    quad,
                    scene_associated_data.hdr_scene_frame_texture(),
                );

                // Convert high dynamic range frame to low dynamic range (sRGB) with tone mapping and gamma correction.
                self.statistics.geometry += scene_associated_data.hdr_renderer.render(
                    state,
                    scene_associated_data.hdr_scene_frame_texture(),
                    scene_associated_data.bloom_renderer.result(),
                    &mut scene_associated_data.ldr_scene_framebuffer,
                    viewport,
                    quad,
                    dt,
                    camera.exposure(),
                    camera.color_grading_lut_ref(),
                    camera.color_grading_enabled(),
                    &mut self.texture_cache,
                );

                // Apply FXAA if needed.
                if self.quality_settings.fxaa {
                    self.statistics.geometry += self.fxaa_renderer.render(
                        state,
                        viewport,
                        scene_associated_data.ldr_scene_frame_texture(),
                        &mut scene_associated_data.ldr_temp_framebuffer,
                    );

                    let quad = &self.quad;
                    let temp_frame_texture = scene_associated_data.ldr_temp_frame_texture();
                    self.statistics.geometry += blit_pixels(
                        state,
                        &mut scene_associated_data.ldr_scene_framebuffer,
                        temp_frame_texture,
                        &self.flat_shader,
                        viewport,
                        quad,
                    );
                }

                // Render debug geometry in the LDR frame buffer.
                self.statistics += self.debug_renderer.render(
                    state,
                    viewport,
                    &mut scene_associated_data.ldr_scene_framebuffer,
                    &scene.drawing_context,
                    camera,
                );

                for render_pass in self.scene_render_passes.iter() {
                    self.statistics +=
                        render_pass
                            .borrow_mut()
                            .on_ldr_render(SceneRenderPassContext {
                                pipeline_state: state,
                                texture_cache: &mut self.texture_cache,
                                geometry_cache: &mut self.geometry_cache,
                                quality_settings: &self.quality_settings,
                                batch_storage: &self.batch_storage,
                                viewport,
                                scene,
                                camera,
                                scene_handle,
                                white_dummy: self.white_dummy.clone(),
                                normal_dummy: self.normal_dummy.clone(),
                                metallic_dummy: self.metallic_dummy.clone(),
                                environment_dummy: self.environment_dummy.clone(),
                                black_dummy: self.black_dummy.clone(),
                                depth_texture: scene_associated_data.gbuffer.depth(),
                                normal_texture: scene_associated_data.gbuffer.normal_texture(),
                                ambient_texture: scene_associated_data.gbuffer.ambient_texture(),
                                framebuffer: &mut scene_associated_data.ldr_scene_framebuffer,
                                ui_renderer: &mut self.ui_renderer,
                            })?;
                }
            }

            // Optionally render everything into back buffer.
            if scene.render_target.is_none() {
                let quad = &self.quad;
                self.statistics.geometry += blit_pixels(
                    state,
                    &mut self.backbuffer,
                    scene_associated_data.ldr_scene_frame_texture(),
                    &self.flat_shader,
                    window_viewport,
                    quad,
                );
            }
        }

        // Render UI on top of everything without gamma correction.
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
        context: &glutin::WindowedContext<glutin::PossiblyCurrent>,
    ) -> Result<(), FrameworkError> {
        self.render_frame(scenes, drawing_context)?;
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
    ) -> Result<(), FrameworkError> {
        self.render_frame(scenes, drawing_context)?;
        self.statistics.end_frame();
        self.state.check_error();
        self.statistics.finalize();
        self.statistics.pipeline = self.state.pipeline_statistics();
        Ok(())
    }
}
