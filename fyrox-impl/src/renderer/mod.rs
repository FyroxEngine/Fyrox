// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Renderer is a "workhorse" of the engine, it draws scenes (both 3D and 2D), user interface,
//! debug geometry and has an ability to add user-defined render passes. Current renderer
//! implementation is not very flexible, but should cover 95% of use cases.
//!
//! # Implementation details
//!
//! Renderer is based on OpenGL 3.3+ Core.

#![warn(missing_docs)]

pub mod framework;

pub mod bundle;
pub mod cache;
pub mod debug_renderer;
pub mod storage;
pub mod ui_renderer;
pub mod visibility;

mod bloom;
mod forward_renderer;
mod fxaa;
mod gbuffer;
mod hdr;
mod light;
mod light_volume;
mod occlusion;
mod shadow;
mod ssao;
mod stats;

use crate::{
    asset::{event::ResourceEvent, manager::ResourceManager},
    core::{
        algebra::{Matrix4, Vector2, Vector3, Vector4},
        array_as_u8_slice,
        color::Color,
        instant,
        log::{Log, MessageKind},
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        sstorage::ImmutableString,
        uuid_provider,
    },
    engine::error::EngineError,
    graph::SceneGraph,
    gui::draw::DrawingContext,
    material::shader::{Shader, ShaderDefinition},
    renderer::{
        bloom::BloomRenderer,
        bundle::{ObserverInfo, RenderDataBundleStorage, RenderDataBundleStorageOptions},
        cache::{
            geometry::GeometryCache,
            shader::{
                binding, property, PropertyGroup, RenderMaterial, RenderPassContainer, ShaderCache,
            },
            texture::TextureCache,
            uniform::{UniformBufferCache, UniformMemoryAllocator},
        },
        debug_renderer::DebugRenderer,
        forward_renderer::{ForwardRenderContext, ForwardRenderer},
        framework::{
            buffer::{BufferKind, BufferUsage, GpuBuffer},
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, DrawCallStatistics, GpuFrameBuffer},
            geometry_buffer::GpuGeometryBuffer,
            gpu_program::SamplerFallback,
            gpu_texture::{GpuTexture, GpuTextureDescriptor, GpuTextureKind, PixelKind},
            server::{GraphicsServer, SharedGraphicsServer},
            GeometryBufferExt, PolygonFace, PolygonFillMode,
        },
        fxaa::FxaaRenderer,
        gbuffer::{GBuffer, GBufferRenderContext},
        hdr::HighDynamicRangeRenderer,
        light::{DeferredLightRenderer, DeferredRendererContext},
        ui_renderer::{UiRenderContext, UiRenderer},
        visibility::VisibilityCache,
    },
    resource::texture::{Texture, TextureKind, TextureResource},
    scene::{
        camera::Camera,
        mesh::{
            buffer::{BytesStorage, TriangleBuffer, VertexAttributeDescriptor, VertexBuffer},
            surface::{SurfaceData, SurfaceResource},
        },
        Scene, SceneContainer,
    },
};
use fxhash::FxHashMap;
use fyrox_graphics::{
    sampler::{GpuSampler, GpuSamplerDescriptor},
    sampler::{MagnificationFilter, MinificationFilter, WrapMode},
};
use fyrox_resource::untyped::ResourceKind;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
pub use stats::*;
use std::{
    any::TypeId, cell::RefCell, collections::hash_map::Entry, hash::Hash, rc::Rc,
    sync::mpsc::Receiver,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};
use uuid::Uuid;
use winit::window::Window;

lazy_static! {
    static ref GBUFFER_PASS_NAME: ImmutableString = ImmutableString::new("GBuffer");
    static ref DIRECTIONAL_SHADOW_PASS_NAME: ImmutableString =
        ImmutableString::new("DirectionalShadow");
    static ref SPOT_SHADOW_PASS_NAME: ImmutableString = ImmutableString::new("SpotShadow");
    static ref POINT_SHADOW_PASS_NAME: ImmutableString = ImmutableString::new("PointShadow");
}

/// Checks whether the provided render pass name is one of the names of built-in shadow render passes.
pub fn is_shadow_pass(render_pass_name: &str) -> bool {
    render_pass_name == &**DIRECTIONAL_SHADOW_PASS_NAME
        || render_pass_name == &**SPOT_SHADOW_PASS_NAME
        || render_pass_name == &**POINT_SHADOW_PASS_NAME
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
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
)]
pub enum ShadowMapPrecision {
    /// Shadow map will use 2 times less memory by switching to 16bit pixel format,
    /// but "shadow acne" may occur.
    Half,
    /// Shadow map will use 32bit pixel format. This option gives highest quality,
    /// but could be less performant than `Half`.
    Full,
}

uuid_provider!(ShadowMapPrecision = "f9b2755b-248e-46ba-bcab-473eac1acdb8");

/// Cascaded-shadow maps settings.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect, Eq)]
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
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
    /// Point shadows fade out range.
    /// Specifies the distance from the camera at which point shadows start to fade out.
    /// Shadows beyond this distance will gradually become less visible.
    pub point_shadows_fade_out_range: f32,

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
    /// Specifies the distance from the camera at which spot shadows start to fade out.
    /// Shadows beyond this distance will gradually become less visible.
    pub spot_shadows_fade_out_range: f32,

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

    /// Whether to use occlusion culling for geometry or not. Warning: this is experimental feature
    /// that may have bugs and unstable behavior. Disabled by default.
    #[serde(default)]
    pub use_occlusion_culling: bool,

    /// Whether to use occlusion culling for light sources or not. Warning: this is experimental
    /// feature that may have bugs and unstable behavior. Disabled by default.
    #[serde(default)]
    pub use_light_occlusion_culling: bool,
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
            point_shadows_fade_out_range: 1.0,

            spot_shadow_map_size: 2048,
            spot_shadows_distance: 20.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: true,
            spot_shadows_fade_out_range: 1.0,

            use_ssao: true,
            ssao_radius: 0.5,

            light_scatter_enabled: true,

            point_shadow_map_precision: ShadowMapPrecision::Full,
            spot_shadow_map_precision: ShadowMapPrecision::Full,

            fxaa: true,

            use_bloom: true,

            use_parallax_mapping: true,

            csm_settings: Default::default(),

            use_occlusion_culling: false,
            use_light_occlusion_culling: false,
        }
    }

    /// High graphics quality, includes all graphical effects. Requires powerful GPU.
    pub fn high() -> Self {
        Self {
            point_shadow_map_size: 1024,
            point_shadows_distance: 15.0,
            point_shadows_enabled: true,
            point_soft_shadows: true,
            point_shadows_fade_out_range: 1.0,

            spot_shadow_map_size: 1024,
            spot_shadows_distance: 15.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: true,
            spot_shadows_fade_out_range: 1.0,

            use_ssao: true,
            ssao_radius: 0.5,

            light_scatter_enabled: true,

            point_shadow_map_precision: ShadowMapPrecision::Full,
            spot_shadow_map_precision: ShadowMapPrecision::Full,

            fxaa: true,

            use_bloom: true,

            use_parallax_mapping: true,

            csm_settings: CsmSettings {
                enabled: true,
                size: 2048,
                precision: ShadowMapPrecision::Full,
                pcf: true,
            },

            use_occlusion_culling: false,
            use_light_occlusion_culling: false,
        }
    }

    /// Medium graphics quality, some of effects are disabled, shadows will have sharp edges.
    pub fn medium() -> Self {
        Self {
            point_shadow_map_size: 512,
            point_shadows_distance: 5.0,
            point_shadows_enabled: true,
            point_soft_shadows: false,
            point_shadows_fade_out_range: 1.0,

            spot_shadow_map_size: 512,
            spot_shadows_distance: 5.0,
            spot_shadows_enabled: true,
            spot_soft_shadows: false,
            spot_shadows_fade_out_range: 1.0,

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

            use_occlusion_culling: false,
            use_light_occlusion_culling: false,
        }
    }

    /// Lowest graphics quality, all effects are disabled.
    pub fn low() -> Self {
        Self {
            point_shadow_map_size: 1, // Zero is unsupported.
            point_shadows_distance: 0.0,
            point_shadows_enabled: false,
            point_soft_shadows: false,
            point_shadows_fade_out_range: 1.0,

            spot_shadow_map_size: 1,
            spot_shadows_distance: 0.0,
            spot_shadows_enabled: false,
            spot_soft_shadows: false,
            spot_shadows_fade_out_range: 1.0,

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

            use_occlusion_culling: false,
            use_light_occlusion_culling: false,
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
            texture_cache_size: 0,
            geometry_cache_size: 0,
            shader_cache_size: 0,
            uniform_buffer_cache_size: 0,
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
    pub hdr_scene_framebuffer: GpuFrameBuffer,

    /// Final frame of the scene. Tone mapped + gamma corrected.
    pub ldr_scene_framebuffer: GpuFrameBuffer,

    /// Additional frame buffer for post processing.
    pub ldr_temp_framebuffer: GpuFrameBuffer,

    /// HDR renderer has be created per scene, because it contains
    /// scene luminance.
    pub hdr_renderer: HighDynamicRangeRenderer,

    /// Bloom contains only overly bright pixels that creates light
    /// bleeding effect (glow effect).
    pub bloom_renderer: BloomRenderer,

    /// Rendering statistics for a scene.
    pub statistics: SceneStatistics,
}

impl AssociatedSceneData {
    /// Creates new scene data.
    pub fn new(
        server: &dyn GraphicsServer,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        let depth_stencil = server.create_2d_render_target(PixelKind::D24S8, width, height)?;
        // Intermediate scene frame will be rendered in HDR render target.
        let hdr_frame_texture =
            server.create_2d_render_target(PixelKind::RGBA16F, width, height)?;

        let hdr_scene_framebuffer = server.create_frame_buffer(
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil.clone(),
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: hdr_frame_texture,
            }],
        )?;

        let ldr_frame_texture = server.create_texture(GpuTextureDescriptor {
            kind: GpuTextureKind::Rectangle { width, height },
            // Final scene frame is in standard sRGB space.
            pixel_kind: PixelKind::RGBA8,
            ..Default::default()
        })?;

        let ldr_scene_framebuffer = server.create_frame_buffer(
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil.clone(),
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: ldr_frame_texture,
            }],
        )?;

        let ldr_temp_texture = server.create_texture(GpuTextureDescriptor {
            kind: GpuTextureKind::Rectangle { width, height },
            // Final scene frame is in standard sRGB space.
            pixel_kind: PixelKind::RGBA8,
            ..Default::default()
        })?;

        let ldr_temp_framebuffer = server.create_frame_buffer(
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil,
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: ldr_temp_texture,
            }],
        )?;

        Ok(Self {
            gbuffer: GBuffer::new(server, width, height)?,
            hdr_renderer: HighDynamicRangeRenderer::new(server)?,
            bloom_renderer: BloomRenderer::new(server, width, height)?,
            hdr_scene_framebuffer,
            ldr_scene_framebuffer,
            ldr_temp_framebuffer,
            statistics: Default::default(),
        })
    }

    fn copy_depth_stencil_to_scene_framebuffer(&mut self) {
        self.gbuffer.framebuffer().blit_to(
            &self.hdr_scene_framebuffer,
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
    pub fn hdr_scene_frame_texture(&self) -> &GpuTexture {
        &self.hdr_scene_framebuffer.color_attachments()[0].texture
    }

    /// Returns low-dynamic range frame buffer texture (final frame).
    pub fn ldr_scene_frame_texture(&self) -> &GpuTexture {
        &self.ldr_scene_framebuffer.color_attachments()[0].texture
    }

    /// Returns low-dynamic range frame buffer texture (accumulation frame).
    pub fn ldr_temp_frame_texture(&self) -> &GpuTexture {
        &self.ldr_temp_framebuffer.color_attachments()[0].texture
    }
}

/// Creates a view-projection matrix that projects unit quad a screen with the specified viewport.
pub fn make_viewport_matrix(viewport: Rect<i32>) -> Matrix4<f32> {
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

/// A set of textures of certain kinds that could be used as a stub in cases when you don't have
/// your own texture of this kind.
pub struct FallbackResources {
    /// White, one pixel, texture which will be used as stub when rendering something without
    /// a texture specified.
    pub white_dummy: GpuTexture,
    /// Black, one pixel, texture.
    pub black_dummy: GpuTexture,
    /// A cube map with 6 textures of 1x1 black pixel in size.
    pub environment_dummy: GpuTexture,
    /// One pixel texture with (0, 1, 0) vector is used as stub when rendering something without a
    /// normal map.
    pub normal_dummy: GpuTexture,
    /// One pixel texture used as stub when rendering something without a  metallic texture. Default
    /// metalness is 0.0
    pub metallic_dummy: GpuTexture,
    /// One pixel volume texture.
    pub volume_dummy: GpuTexture,
    /// A stub uniform buffer for situation when there's no actual bone matrices.
    pub bone_matrices_stub_uniform_buffer: GpuBuffer,
    /// A sampler with the linear filtration that clamps incoming UVs to `[0;1]` range.
    pub linear_clamp_sampler: GpuSampler,
    /// A sampler with the linear filtration.
    pub linear_wrap_sampler: GpuSampler,
    /// A sampler with the nearest filtration that clamps incoming UVs to `[0;1]` range.
    pub nearest_clamp_sampler: GpuSampler,
    /// A sampler with the nearest filtration.
    pub nearest_wrap_sampler: GpuSampler,
}

impl FallbackResources {
    /// Picks a texture that corresponds to the actual value of the given sampler fallback.
    pub fn sampler_fallback(&self, sampler_fallback: SamplerFallback) -> &GpuTexture {
        match sampler_fallback {
            SamplerFallback::White => &self.white_dummy,
            SamplerFallback::Normal => &self.normal_dummy,
            SamplerFallback::Black => &self.black_dummy,
            SamplerFallback::Volume => &self.volume_dummy,
        }
    }
}

/// A cache for dynamic surfaces, the content of which changes every frame. The main purpose of this
/// cache is to keep associated GPU buffers alive a long as the surfaces in the cache and thus prevent
/// redundant resource reallocation on every frame. This is very important for dynamic drawing, such
/// as 2D sprites, tile maps, etc.
#[derive(Default)]
pub struct DynamicSurfaceCache {
    cache: FxHashMap<u64, SurfaceResource>,
}

impl DynamicSurfaceCache {
    /// Creates a new empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Tries to get an existing surface from the cache using its unique id or creates a new one and
    /// returns it.
    pub fn get_or_create(
        &mut self,
        unique_id: u64,
        layout: &[VertexAttributeDescriptor],
    ) -> SurfaceResource {
        if let Some(surface) = self.cache.get(&unique_id) {
            surface.clone()
        } else {
            let default_capacity = 4096;

            // Initialize empty vertex buffer.
            let vertex_buffer = VertexBuffer::new_with_layout(
                layout,
                0,
                BytesStorage::with_capacity(default_capacity),
            )
            .unwrap();

            // Initialize empty triangle buffer.
            let triangle_buffer = TriangleBuffer::new(Vec::with_capacity(default_capacity * 3));

            let surface = SurfaceResource::new_ok(
                Uuid::new_v4(),
                ResourceKind::Embedded,
                SurfaceData::new(vertex_buffer, triangle_buffer),
            );

            self.cache.insert(unique_id, surface.clone());

            surface
        }
    }

    /// Clears the surfaces in the cache, does **not** clear the cache itself.
    pub fn clear(&mut self) {
        for surface in self.cache.values_mut() {
            let mut surface_data = surface.data_ref();
            surface_data.vertex_buffer.modify().clear();
            surface_data.geometry_buffer.modify().clear();
        }
    }
}

/// See module docs.
pub struct Renderer {
    backbuffer: GpuFrameBuffer,
    scene_render_passes: Vec<Rc<RefCell<dyn SceneRenderPass>>>,
    deferred_light_renderer: DeferredLightRenderer,
    blit_shader: RenderPassContainer,
    /// A set of textures of certain kinds that could be used as a stub in cases when you don't have
    /// your own texture of this kind.
    pub fallback_resources: FallbackResources,
    /// User interface renderer.
    pub ui_renderer: UiRenderer,
    statistics: Statistics,
    quad: GpuGeometryBuffer,
    frame_size: (u32, u32),
    quality_settings: QualitySettings,
    /// Debug renderer instance can be used for debugging purposes
    pub debug_renderer: DebugRenderer,
    /// Screen space debug renderer instance can be used for debugging purposes to draw lines directly
    /// on screen. It is useful to debug some rendering algorithms.
    pub screen_space_debug_renderer: DebugRenderer,
    /// A set of associated data for each scene that was rendered.
    pub scene_data_map: FxHashMap<Handle<Scene>, AssociatedSceneData>,
    backbuffer_clear_color: Color,
    /// Texture cache with GPU textures.
    pub texture_cache: TextureCache,
    /// Uniform buffer cache.
    pub uniform_buffer_cache: UniformBufferCache,
    shader_cache: ShaderCache,
    geometry_cache: GeometryCache,
    forward_renderer: ForwardRenderer,
    fxaa_renderer: FxaaRenderer,
    texture_event_receiver: Receiver<ResourceEvent>,
    shader_event_receiver: Receiver<ResourceEvent>,
    // TextureId -> FrameBuffer mapping. This mapping is used for temporal frame buffers
    // like ones used to render UI instances.
    ui_frame_buffers: FxHashMap<u64, GpuFrameBuffer>,
    uniform_memory_allocator: UniformMemoryAllocator,
    /// Dynamic surface cache. See [`DynamicSurfaceCache`] docs for more info.
    pub dynamic_surface_cache: DynamicSurfaceCache,
    /// Visibility cache based on occlusion query.
    pub visibility_cache: VisibilityCache,
    /// Graphics server.
    pub server: SharedGraphicsServer,
}

fn make_ui_frame_buffer(
    frame_size: Vector2<f32>,
    server: &dyn GraphicsServer,
    pixel_kind: PixelKind,
) -> Result<GpuFrameBuffer, FrameworkError> {
    let color_texture = server.create_texture(GpuTextureDescriptor {
        kind: GpuTextureKind::Rectangle {
            width: frame_size.x as usize,
            height: frame_size.y as usize,
        },
        pixel_kind,
        ..Default::default()
    })?;

    let depth_stencil = server.create_2d_render_target(
        PixelKind::D24S8,
        frame_size.x as usize,
        frame_size.y as usize,
    )?;

    server.create_frame_buffer(
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
    /// Amount of time (in seconds) that passed from creation of the engine. Keep in mind, that
    /// this value is **not** guaranteed to match real time. A user can change delta time with
    /// which the engine "ticks" and this delta time affects elapsed time.
    pub elapsed_time: f32,
    /// A graphics server that is used as a wrapper to underlying graphics API.
    pub server: &'a dyn GraphicsServer,

    /// A texture cache that uploads engine's `Texture` as internal `GpuTexture` to GPU.
    /// Use this to get a corresponding GPU texture by an instance of a `Texture`.
    pub texture_cache: &'a mut TextureCache,

    /// A geometry cache that uploads engine's `SurfaceData` as internal `GeometryBuffer` to GPU.
    /// Use this to get a corresponding GPU geometry buffer (essentially it is just a VAO) by an
    /// instance of a `SurfaceData`.
    pub geometry_cache: &'a mut GeometryCache,

    /// A cache that stores all native shaders associated with a shader resource. You can use it
    /// to get a ready-to-use set of shaders for your shader resource, which could be obtained
    /// from a material.
    pub shader_cache: &'a mut ShaderCache,

    /// A storage that contains "pre-compiled" groups of render data (batches).
    pub bundle_storage: &'a RenderDataBundleStorage,

    /// Current quality settings of the renderer.
    pub quality_settings: &'a QualitySettings,

    /// Current framebuffer to which scene is being rendered to.
    pub framebuffer: &'a GpuFrameBuffer,

    /// A scene being rendered.
    pub scene: &'b Scene,

    /// A camera from the scene that is used as "eyes".
    pub camera: &'b Camera,

    /// A viewport of the camera.
    pub viewport: Rect<i32>,

    /// A handle of the scene being rendered.
    pub scene_handle: Handle<Scene>,

    /// A set of textures of certain kinds that could be used as a stub in cases when you don't have
    /// your own texture of this kind.
    pub fallback_resources: &'a FallbackResources,

    /// A texture with depth values from G-Buffer.
    ///
    /// # Important notes
    ///
    /// Keep in mind that G-Buffer cannot be modified in custom render passes, so you don't
    /// have an ability to write to this texture. However, you can still write to depth of
    /// the frame buffer as you'd normally do.
    pub depth_texture: &'a GpuTexture,

    /// A texture with world-space normals from G-Buffer.
    ///
    /// # Important notes
    ///
    /// Keep in mind that G-Buffer cannot be modified in custom render passes, so you don't
    /// have an ability to write to this texture.
    pub normal_texture: &'a GpuTexture,

    /// A texture with ambient lighting values from G-Buffer.
    ///
    /// # Important notes
    ///
    /// Keep in mind that G-Buffer cannot be modified in custom render passes, so you don't
    /// have an ability to write to this texture.
    pub ambient_texture: &'a GpuTexture,

    /// User interface renderer.
    pub ui_renderer: &'a mut UiRenderer,

    /// A cache of uniform buffers.
    pub uniform_buffer_cache: &'a mut UniformBufferCache,

    /// Memory allocator for uniform buffers that tries to pack uniforms densely into large uniform
    /// buffers, giving you offsets to the data.
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,

    /// Dynamic surface cache. See [`DynamicSurfaceCache`] docs for more info.
    pub dynamic_surface_cache: &'a mut DynamicSurfaceCache,
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

    /// Should return type id of a plugin, that holds this render pass. **WARNING:** Setting incorrect
    /// (anything else, than a real plugin's type id) value here will result in hard crash with happy
    /// debugging times.
    fn source_type_id(&self) -> TypeId;
}

fn blit_pixels(
    uniform_buffer_cache: &mut UniformBufferCache,
    framebuffer: &GpuFrameBuffer,
    texture: &GpuTexture,
    blit_shader: &RenderPassContainer,
    viewport: Rect<i32>,
    quad: &GpuGeometryBuffer,
    fallback_resources: &FallbackResources,
) -> Result<DrawCallStatistics, FrameworkError> {
    let wvp = make_viewport_matrix(viewport);
    let properties = PropertyGroup::from([property("worldViewProjection", &wvp)]);
    let material = RenderMaterial::from([
        binding(
            "diffuseTexture",
            (texture, &fallback_resources.linear_clamp_sampler),
        ),
        binding("properties", &properties),
    ]);
    blit_shader.run_pass(
        1,
        &ImmutableString::new("Primary"),
        framebuffer,
        quad,
        viewport,
        &material,
        uniform_buffer_cache,
        Default::default(),
        None,
    )
}

#[allow(missing_docs)] // TODO
pub struct LightData<const N: usize = 16> {
    pub count: usize,
    pub color_radius: [Vector4<f32>; N],
    pub position: [Vector3<f32>; N],
    pub direction: [Vector3<f32>; N],
    pub parameters: [Vector2<f32>; N],
}

impl<const N: usize> Default for LightData<N> {
    fn default() -> Self {
        Self {
            count: 0,
            color_radius: [Default::default(); N],
            position: [Default::default(); N],
            direction: [Default::default(); N],
            parameters: [Default::default(); N],
        }
    }
}

impl Renderer {
    /// Creates a new renderer with the given graphics server.
    pub fn new(
        server: Rc<dyn GraphicsServer>,
        frame_size: (u32, u32),
        resource_manager: &ResourceManager,
    ) -> Result<Self, EngineError> {
        let settings = QualitySettings::default();

        let (texture_event_sender, texture_event_receiver) = std::sync::mpsc::channel();

        resource_manager
            .state()
            .event_broadcaster
            .add(texture_event_sender);

        let (shader_event_sender, shader_event_receiver) = std::sync::mpsc::channel();

        resource_manager
            .state()
            .event_broadcaster
            .add(shader_event_sender);

        let caps = server.capabilities();
        Log::info(format!("Graphics Server Capabilities\n{caps:?}",));

        let shader_cache = ShaderCache::default();

        let one_megabyte = 1024 * 1024;
        let uniform_memory_allocator = UniformMemoryAllocator::new(
            // Clamp max uniform block size from the upper bound, to prevent allocating huge
            // uniform buffers when GPU supports it. Some AMD GPUs are able to allocate ~500 Mb
            // uniform buffers, which will lead to ridiculous VRAM consumption.
            caps.max_uniform_block_size.min(one_megabyte),
            caps.uniform_buffer_offset_alignment,
        );

        let fallback_resources = FallbackResources {
            white_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[255u8, 255u8, 255u8, 255u8]),
                ..Default::default()
            })?,
            black_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[0u8, 0u8, 0u8, 255u8]),
                ..Default::default()
            })?,
            environment_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Cube {
                    width: 1,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[
                    0u8, 0u8, 0u8, 255u8, // pos-x
                    0u8, 0u8, 0u8, 255u8, // neg-x
                    0u8, 0u8, 0u8, 255u8, // pos-y
                    0u8, 0u8, 0u8, 255u8, // neg-y
                    0u8, 0u8, 0u8, 255u8, // pos-z
                    0u8, 0u8, 0u8, 255u8, // neg-z
                ]),
                ..Default::default()
            })?,
            normal_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[128u8, 128u8, 255u8, 255u8]),
                ..Default::default()
            })?,
            metallic_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[0u8, 0u8, 0u8, 0u8]),
                ..Default::default()
            })?,
            volume_dummy: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Volume {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                pixel_kind: PixelKind::RGBA8,
                data: Some(&[0u8, 0u8, 0u8, 0u8]),
                ..Default::default()
            })?,
            bone_matrices_stub_uniform_buffer: {
                let buffer = server.create_buffer(
                    ShaderDefinition::MAX_BONE_MATRICES * size_of::<Matrix4<f32>>(),
                    BufferKind::Uniform,
                    BufferUsage::StaticDraw,
                )?;
                const SIZE: usize = ShaderDefinition::MAX_BONE_MATRICES * size_of::<Matrix4<f32>>();
                let zeros = [0.0; SIZE];
                buffer.write_data(array_as_u8_slice(&zeros))?;
                buffer
            },
            linear_clamp_sampler: server.create_sampler(GpuSamplerDescriptor {
                min_filter: MinificationFilter::Linear,
                mag_filter: MagnificationFilter::Linear,
                s_wrap_mode: WrapMode::ClampToEdge,
                t_wrap_mode: WrapMode::ClampToEdge,
                r_wrap_mode: WrapMode::ClampToEdge,
                ..Default::default()
            })?,
            linear_wrap_sampler: server.create_sampler(GpuSamplerDescriptor {
                min_filter: MinificationFilter::Linear,
                mag_filter: MagnificationFilter::Linear,
                ..Default::default()
            })?,
            nearest_clamp_sampler: server.create_sampler(GpuSamplerDescriptor {
                min_filter: MinificationFilter::Nearest,
                mag_filter: MagnificationFilter::Nearest,
                s_wrap_mode: WrapMode::ClampToEdge,
                t_wrap_mode: WrapMode::ClampToEdge,
                r_wrap_mode: WrapMode::ClampToEdge,
                ..Default::default()
            })?,
            nearest_wrap_sampler: server.create_sampler(GpuSamplerDescriptor {
                min_filter: MinificationFilter::Nearest,
                mag_filter: MagnificationFilter::Nearest,
                ..Default::default()
            })?,
        };

        Ok(Self {
            backbuffer: server.back_buffer(),
            frame_size,
            deferred_light_renderer: DeferredLightRenderer::new(&*server, frame_size, &settings)?,
            blit_shader: RenderPassContainer::from_str(
                &*server,
                include_str!("shaders/blit.shader"),
            )?,
            fallback_resources,
            quad: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                &*server,
            )?,
            ui_renderer: UiRenderer::new(&*server)?,
            quality_settings: settings,
            debug_renderer: DebugRenderer::new(&*server)?,
            screen_space_debug_renderer: DebugRenderer::new(&*server)?,
            scene_data_map: Default::default(),
            backbuffer_clear_color: Color::BLACK,
            texture_cache: Default::default(),
            geometry_cache: Default::default(),
            forward_renderer: ForwardRenderer::new(),
            ui_frame_buffers: Default::default(),
            fxaa_renderer: FxaaRenderer::new(&*server)?,
            statistics: Statistics::default(),
            shader_event_receiver,
            texture_event_receiver,
            shader_cache,
            scene_render_passes: Default::default(),
            uniform_buffer_cache: UniformBufferCache::new(server.clone()),
            server,
            visibility_cache: Default::default(),
            uniform_memory_allocator,
            dynamic_surface_cache: DynamicSurfaceCache::new(),
        })
    }

    /// Adds a custom render pass.
    pub fn add_render_pass(&mut self, pass: Rc<RefCell<dyn SceneRenderPass>>) {
        self.scene_render_passes.push(pass);
    }

    /// Removes specified render pass.
    pub fn remove_render_pass(&mut self, pass: Rc<RefCell<dyn SceneRenderPass>>) {
        if let Some(index) = self
            .scene_render_passes
            .iter()
            .position(|p| Rc::ptr_eq(p, &pass))
        {
            self.scene_render_passes.remove(index);
        }
    }

    /// Returns a slice with every registered render passes.
    pub fn render_passes(&self) -> &[Rc<RefCell<dyn SceneRenderPass>>] {
        &self.scene_render_passes
    }

    /// Removes all render passes from the renderer.
    pub fn clear_render_passes(&mut self) {
        self.scene_render_passes.clear()
    }

    /// Returns statistics for last frame.
    pub fn get_statistics(&self) -> Statistics {
        self.statistics
    }

    /// Unloads texture from GPU memory.
    pub fn unload_texture(&mut self, texture: TextureResource) {
        self.texture_cache.unload(texture)
    }

    /// Sets color which will be used to fill screen when there is nothing to render.
    pub fn set_backbuffer_clear_color(&mut self, color: Color) {
        self.backbuffer_clear_color = color;
    }

    /// Returns a reference to current graphics server.
    pub fn graphics_server(&self) -> &dyn GraphicsServer {
        &*self.server
    }

    /// Sets new frame size. You should call the same method on [`crate::engine::Engine`]
    /// instead, which will update the size for the user interface and rendering context
    /// as well as this one.
    ///
    /// # Notes
    ///
    /// Input values will be set to 1 pixel if new size is 0. Rendering cannot
    /// be performed into 0x0 texture.
    pub(crate) fn set_frame_size(&mut self, new_size: (u32, u32)) -> Result<(), FrameworkError> {
        self.frame_size.0 = new_size.0.max(1);
        self.frame_size.1 = new_size.1.max(1);

        self.deferred_light_renderer
            .set_frame_size(&*self.server, new_size)?;

        self.graphics_server().set_frame_size(new_size);

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
            .set_quality_settings(&*self.server, settings)
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
    }

    /// Renders given UI into specified render target. This method is especially useful if you need
    /// to have off-screen UIs (like interactive touch-screen in Doom 3, Dead Space, etc).
    pub fn render_ui_to_texture(
        &mut self,
        render_target: TextureResource,
        screen_size: Vector2<f32>,
        drawing_context: &DrawingContext,
        clear_color: Color,
        pixel_kind: PixelKind,
    ) -> Result<(), FrameworkError> {
        let new_width = screen_size.x as usize;
        let new_height = screen_size.y as usize;

        // Create or reuse existing frame buffer.
        let frame_buffer = match self.ui_frame_buffers.entry(render_target.key()) {
            Entry::Occupied(entry) => {
                let frame_buffer = entry.into_mut();
                let frame = frame_buffer.color_attachments().first().unwrap();
                let color_texture_kind = frame.texture.kind();
                if let GpuTextureKind::Rectangle { width, height } = color_texture_kind {
                    if width != new_width
                        || height != new_height
                        || frame.texture.pixel_kind() != pixel_kind
                    {
                        *frame_buffer =
                            make_ui_frame_buffer(screen_size, &*self.server, pixel_kind)?;
                    }
                } else {
                    panic!("ui can be rendered only in rectangle texture!")
                }
                frame_buffer
            }
            Entry::Vacant(entry) => entry.insert(make_ui_frame_buffer(
                screen_size,
                &*self.server,
                pixel_kind,
            )?),
        };

        let viewport = Rect::new(0, 0, new_width as i32, new_height as i32);

        frame_buffer.clear(viewport, Some(clear_color), Some(0.0), Some(0));

        self.statistics += self.ui_renderer.render(UiRenderContext {
            server: &*self.server,
            viewport,
            frame_buffer,
            frame_width: screen_size.x,
            frame_height: screen_size.y,
            drawing_context,
            fallback_resources: &self.fallback_resources,
            texture_cache: &mut self.texture_cache,
            uniform_buffer_cache: &mut self.uniform_buffer_cache,
        })?;

        // Finally register texture in the cache so it will become available as texture in deferred/forward
        // renderer.
        self.texture_cache.try_register(
            &*self.server,
            &render_target,
            frame_buffer
                .color_attachments()
                .first()
                .unwrap()
                .texture
                .clone(),
        )
    }

    fn update_texture_cache(&mut self, dt: f32) {
        // Maximum amount of textures uploaded to GPU per frame. This defines throughput **only** for
        // requests from resource manager. This is needed to prevent huge lag when there are tons of
        // requests, so this is some kind of work load balancer.
        const THROUGHPUT: usize = 5;

        let mut uploaded = 0;
        while let Ok(event) = self.texture_event_receiver.try_recv() {
            if let ResourceEvent::Loaded(resource) | ResourceEvent::Reloaded(resource) = event {
                if let Some(texture) = resource.try_cast::<Texture>() {
                    match self.texture_cache.upload(&*self.server, &texture) {
                        Ok(_) => {
                            uploaded += 1;
                            if uploaded >= THROUGHPUT {
                                break;
                            }
                        }
                        Err(e) => {
                            Log::writeln(
                                MessageKind::Error,
                                format!("Failed to upload texture to GPU. Reason: {e:?}"),
                            );
                        }
                    }
                }
            }
        }

        self.texture_cache.update(dt);
    }

    fn update_shader_cache(&mut self, dt: f32) {
        while let Ok(event) = self.shader_event_receiver.try_recv() {
            if let ResourceEvent::Loaded(resource) | ResourceEvent::Reloaded(resource) = event {
                if let Some(shader) = resource.try_cast::<Shader>() {
                    // Remove and immediately "touch" the shader cache to force upload shader.
                    self.shader_cache.remove(&shader);
                    let _ = self.shader_cache.get(&*self.server, &shader);
                }
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
    }

    /// Unconditionally renders a scene and returns a reference to a [`AssociatedSceneData`] instance
    /// that contains rendered data (including intermediate data, such as G-Buffer content, etc.).
    pub fn render_scene(
        &mut self,
        scene_handle: Handle<Scene>,
        scene: &Scene,
        elapsed_time: f32,
        dt: f32,
    ) -> Result<&AssociatedSceneData, FrameworkError> {
        let graph = &scene.graph;

        let backbuffer_width = self.frame_size.0 as f32;
        let backbuffer_height = self.frame_size.1 as f32;

        let window_viewport = Rect::new(0, 0, self.frame_size.0 as i32, self.frame_size.1 as i32);

        let frame_size = scene
            .rendering_options
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

        let server = &*self.server;

        let scene_associated_data = self
            .scene_data_map
            .entry(scene_handle)
            .and_modify(|data| {
                if data.gbuffer.width != frame_size.x as i32
                    || data.gbuffer.height != frame_size.y as i32
                {
                    let width = frame_size.x as usize;
                    let height = frame_size.y as usize;

                    Log::info(format!(
                        "Associated scene rendering data was re-created for scene {}, because render frame size was changed. Old is {}x{}, new {}x{}!",
                        scene_handle,
                        data.gbuffer.width,data.gbuffer.height,width,height
                    ));

                    *data = AssociatedSceneData::new(server, width, height).unwrap();
                }
            })
            .or_insert_with(|| {
                let width = frame_size.x as usize;
                let height = frame_size.y as usize;

                Log::info(format!(
                    "A new associated scene rendering data was created for scene {scene_handle}!"
                ));

                AssociatedSceneData::new(server, width, height).unwrap()
            });

        let pipeline_stats = server.pipeline_statistics();
        scene_associated_data.statistics = Default::default();

        // If we specified a texture to draw to, we have to register it in texture cache
        // so it can be used in later on as texture. This is useful in case if you need
        // to draw something on offscreen and then draw it on some mesh.
        if let Some(rt) = scene.rendering_options.render_target.clone() {
            self.texture_cache.try_register(
                server,
                &rt,
                scene_associated_data.ldr_scene_frame_texture().clone(),
            )?;
        }

        for (camera_handle, camera) in graph.pair_iter().filter_map(|(handle, node)| {
            if node.is_globally_enabled() {
                if let Some(camera) = node.cast::<Camera>() {
                    if camera.is_enabled() {
                        return Some((handle, camera));
                    }
                }
            }
            None
        }) {
            let visibility_cache = self.visibility_cache.get_or_register(graph, camera_handle);

            let viewport = camera.viewport_pixels(frame_size);

            let bundle_storage = RenderDataBundleStorage::from_graph(
                graph,
                elapsed_time,
                ObserverInfo {
                    observer_position: camera.global_position(),
                    z_near: camera.projection().z_near(),
                    z_far: camera.projection().z_far(),
                    view_matrix: camera.view_matrix(),
                    projection_matrix: camera.projection_matrix(),
                },
                GBUFFER_PASS_NAME.clone(),
                RenderDataBundleStorageOptions {
                    collect_lights: true,
                },
                &mut self.dynamic_surface_cache,
            );

            server.set_polygon_fill_mode(
                PolygonFace::FrontAndBack,
                scene.rendering_options.polygon_rasterization_mode,
            );

            scene_associated_data.statistics +=
                scene_associated_data.gbuffer.fill(GBufferRenderContext {
                    server,
                    camera,
                    geom_cache: &mut self.geometry_cache,
                    bundle_storage: &bundle_storage,
                    texture_cache: &mut self.texture_cache,
                    shader_cache: &mut self.shader_cache,
                    quality_settings: &self.quality_settings,
                    fallback_resources: &self.fallback_resources,
                    graph,
                    uniform_buffer_cache: &mut self.uniform_buffer_cache,
                    uniform_memory_allocator: &mut self.uniform_memory_allocator,
                    screen_space_debug_renderer: &mut self.screen_space_debug_renderer,
                    unit_quad: &self.quad,
                })?;

            server.set_polygon_fill_mode(PolygonFace::FrontAndBack, PolygonFillMode::Fill);

            scene_associated_data.copy_depth_stencil_to_scene_framebuffer();

            scene_associated_data.hdr_scene_framebuffer.clear(
                viewport,
                Some(
                    scene
                        .rendering_options
                        .clear_color
                        .unwrap_or(self.backbuffer_clear_color),
                ),
                None, // Keep depth, we've just copied valid data in it.
                Some(0),
            );

            let (pass_stats, light_stats) =
                self.deferred_light_renderer
                    .render(DeferredRendererContext {
                        elapsed_time,
                        server,
                        scene,
                        camera,
                        gbuffer: &mut scene_associated_data.gbuffer,
                        ambient_color: scene.rendering_options.ambient_lighting_color,
                        render_data_bundle: &bundle_storage,
                        settings: &self.quality_settings,
                        textures: &mut self.texture_cache,
                        geometry_cache: &mut self.geometry_cache,
                        frame_buffer: &scene_associated_data.hdr_scene_framebuffer,
                        shader_cache: &mut self.shader_cache,
                        fallback_resources: &self.fallback_resources,
                        uniform_buffer_cache: &mut self.uniform_buffer_cache,
                        visibility_cache,
                        uniform_memory_allocator: &mut self.uniform_memory_allocator,
                        dynamic_surface_cache: &mut self.dynamic_surface_cache,
                    })?;

            scene_associated_data.statistics += light_stats;
            scene_associated_data.statistics += pass_stats;

            let depth = scene_associated_data.gbuffer.depth();

            scene_associated_data.statistics +=
                self.forward_renderer.render(ForwardRenderContext {
                    state: server,
                    geom_cache: &mut self.geometry_cache,
                    texture_cache: &mut self.texture_cache,
                    shader_cache: &mut self.shader_cache,
                    bundle_storage: &bundle_storage,
                    framebuffer: &scene_associated_data.hdr_scene_framebuffer,
                    viewport,
                    quality_settings: &self.quality_settings,
                    fallback_resources: &self.fallback_resources,
                    scene_depth: depth,
                    ambient_light: scene.rendering_options.ambient_lighting_color,
                    uniform_memory_allocator: &mut self.uniform_memory_allocator,
                })?;

            for render_pass in self.scene_render_passes.iter() {
                scene_associated_data.statistics +=
                    render_pass
                        .borrow_mut()
                        .on_hdr_render(SceneRenderPassContext {
                            elapsed_time,
                            server,
                            texture_cache: &mut self.texture_cache,
                            geometry_cache: &mut self.geometry_cache,
                            shader_cache: &mut self.shader_cache,
                            quality_settings: &self.quality_settings,
                            bundle_storage: &bundle_storage,
                            viewport,
                            scene,
                            camera,
                            scene_handle,
                            fallback_resources: &self.fallback_resources,
                            depth_texture: scene_associated_data.gbuffer.depth(),
                            normal_texture: scene_associated_data.gbuffer.normal_texture(),
                            ambient_texture: scene_associated_data.gbuffer.ambient_texture(),
                            framebuffer: &scene_associated_data.hdr_scene_framebuffer,
                            ui_renderer: &mut self.ui_renderer,
                            uniform_buffer_cache: &mut self.uniform_buffer_cache,
                            uniform_memory_allocator: &mut self.uniform_memory_allocator,
                            dynamic_surface_cache: &mut self.dynamic_surface_cache,
                        })?;
            }

            let quad = &self.quad;

            // Prepare glow map.
            scene_associated_data.statistics += scene_associated_data.bloom_renderer.render(
                quad,
                scene_associated_data.hdr_scene_frame_texture(),
                &mut self.uniform_buffer_cache,
                &self.fallback_resources,
            )?;

            // Convert high dynamic range frame to low dynamic range (sRGB) with tone mapping and gamma correction.
            scene_associated_data.statistics += scene_associated_data.hdr_renderer.render(
                server,
                scene_associated_data.hdr_scene_frame_texture(),
                scene_associated_data.bloom_renderer.result(),
                &scene_associated_data.ldr_scene_framebuffer,
                viewport,
                quad,
                dt,
                camera.exposure(),
                camera.color_grading_lut_ref(),
                camera.color_grading_enabled(),
                &mut self.texture_cache,
                &mut self.uniform_buffer_cache,
                &self.fallback_resources,
            )?;

            // Apply FXAA if needed.
            if self.quality_settings.fxaa {
                scene_associated_data.statistics += self.fxaa_renderer.render(
                    viewport,
                    scene_associated_data.ldr_scene_frame_texture(),
                    &scene_associated_data.ldr_temp_framebuffer,
                    &mut self.uniform_buffer_cache,
                    &self.fallback_resources,
                )?;

                let quad = &self.quad;
                let temp_frame_texture = scene_associated_data.ldr_temp_frame_texture();
                scene_associated_data.statistics += blit_pixels(
                    &mut self.uniform_buffer_cache,
                    &scene_associated_data.ldr_scene_framebuffer,
                    temp_frame_texture,
                    &self.blit_shader,
                    viewport,
                    quad,
                    &self.fallback_resources,
                )?;
            }

            // Render debug geometry in the LDR frame buffer.
            self.debug_renderer.set_lines(&scene.drawing_context.lines);
            scene_associated_data.statistics += self.debug_renderer.render(
                &mut self.uniform_buffer_cache,
                viewport,
                &scene_associated_data.ldr_scene_framebuffer,
                camera.view_projection_matrix(),
            )?;

            for render_pass in self.scene_render_passes.iter() {
                scene_associated_data.statistics +=
                    render_pass
                        .borrow_mut()
                        .on_ldr_render(SceneRenderPassContext {
                            elapsed_time,
                            server,
                            texture_cache: &mut self.texture_cache,
                            geometry_cache: &mut self.geometry_cache,
                            shader_cache: &mut self.shader_cache,
                            quality_settings: &self.quality_settings,
                            bundle_storage: &bundle_storage,
                            viewport,
                            scene,
                            camera,
                            scene_handle,
                            fallback_resources: &self.fallback_resources,
                            depth_texture: scene_associated_data.gbuffer.depth(),
                            normal_texture: scene_associated_data.gbuffer.normal_texture(),
                            ambient_texture: scene_associated_data.gbuffer.ambient_texture(),
                            framebuffer: &scene_associated_data.ldr_scene_framebuffer,
                            ui_renderer: &mut self.ui_renderer,
                            uniform_buffer_cache: &mut self.uniform_buffer_cache,
                            uniform_memory_allocator: &mut self.uniform_memory_allocator,
                            dynamic_surface_cache: &mut self.dynamic_surface_cache,
                        })?;
            }
        }

        self.visibility_cache.update(graph);

        // Optionally render everything into back buffer.
        if scene.rendering_options.render_target.is_none() {
            scene_associated_data.statistics += blit_pixels(
                &mut self.uniform_buffer_cache,
                &self.backbuffer,
                scene_associated_data.ldr_scene_frame_texture(),
                &self.blit_shader,
                window_viewport,
                &self.quad,
                &self.fallback_resources,
            )?;
        }

        self.statistics += scene_associated_data.statistics;
        scene_associated_data.statistics.pipeline = server.pipeline_statistics() - pipeline_stats;

        Ok(scene_associated_data)
    }

    fn render_frame<'a>(
        &mut self,
        scenes: &SceneContainer,
        elapsed_time: f32,
        drawing_contexts: impl Iterator<Item = &'a DrawingContext>,
    ) -> Result<(), FrameworkError> {
        if self.frame_size.0 == 0 || self.frame_size.1 == 0 {
            return Ok(());
        }

        self.uniform_buffer_cache.mark_all_unused();
        self.uniform_memory_allocator.clear();
        self.dynamic_surface_cache.clear();

        // Make sure to drop associated data for destroyed scenes.
        self.scene_data_map
            .retain(|h, _| scenes.is_valid_handle(*h));

        // We have to invalidate resource bindings cache because some textures or programs,
        // or other GL resources can be destroyed and then on their "names" some new resource
        // are created, but cache still thinks that resource is correctly bound, but it is different
        // object have same name.
        self.server.invalidate_resource_bindings_cache();
        let dt = self.statistics.capped_frame_time;
        self.statistics.begin_frame();

        let window_viewport = Rect::new(0, 0, self.frame_size.0 as i32, self.frame_size.1 as i32);
        self.backbuffer.clear(
            window_viewport,
            Some(self.backbuffer_clear_color),
            Some(1.0),
            Some(0),
        );

        let backbuffer_width = self.frame_size.0 as f32;
        let backbuffer_height = self.frame_size.1 as f32;

        for (scene_handle, scene) in scenes.pair_iter().filter(|(_, s)| *s.enabled) {
            self.render_scene(scene_handle, scene, elapsed_time, dt)?;
        }

        self.graphics_server()
            .set_polygon_fill_mode(PolygonFace::FrontAndBack, PolygonFillMode::Fill);

        // Render UI on top of everything without gamma correction.
        for drawing_context in drawing_contexts {
            self.statistics += self.ui_renderer.render(UiRenderContext {
                server: &*self.server,
                viewport: window_viewport,
                frame_buffer: &self.backbuffer,
                frame_width: backbuffer_width,
                frame_height: backbuffer_height,
                drawing_context,
                fallback_resources: &self.fallback_resources,
                texture_cache: &mut self.texture_cache,
                uniform_buffer_cache: &mut self.uniform_buffer_cache,
            })?;
        }

        let screen_matrix =
            Matrix4::new_orthographic(0.0, backbuffer_width, backbuffer_height, 0.0, -1.0, 1.0);
        self.screen_space_debug_renderer.render(
            &mut self.uniform_buffer_cache,
            window_viewport,
            &self.backbuffer,
            screen_matrix,
        )?;

        self.statistics.geometry_cache_size = self.geometry_cache.alive_count();
        self.statistics.texture_cache_size = self.texture_cache.alive_count();
        self.statistics.shader_cache_size = self.shader_cache.alive_count();
        self.statistics.uniform_buffer_cache_size = self.uniform_buffer_cache.alive_count();

        Ok(())
    }

    pub(crate) fn render_and_swap_buffers<'a>(
        &mut self,
        scenes: &SceneContainer,
        elapsed_time: f32,
        drawing_contexts: impl Iterator<Item = &'a DrawingContext>,
        window: &Window,
    ) -> Result<(), FrameworkError> {
        self.render_frame(scenes, elapsed_time, drawing_contexts)?;
        self.statistics.end_frame();
        window.pre_present_notify();
        self.graphics_server().swap_buffers()?;
        self.statistics.finalize();
        self.statistics.pipeline = self.server.pipeline_statistics();
        Ok(())
    }
}
