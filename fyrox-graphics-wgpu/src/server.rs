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

use crate::buffer::WgpuBuffer;
use crate::format_helpers::is_filterable_format;
use crate::framebuffer::{PipelineKey, WgpuFrameBuffer};
use crate::geometry_buffer::WgpuGeometryBuffer;
use crate::program::{WgpuProgram, WgpuShader};
use crate::query::WgpuQuery;
use crate::read_buffer::WgpuAsyncReadBuffer;
use crate::sampler::WgpuSampler;
use crate::texture::{texture_size, WgpuTexture};
use fyrox_core::futures::executor::block_on;
use fyrox_core::log::Log;
use fyrox_core::math::Rect;
use fyrox_graphics::buffer::{GpuBuffer, GpuBufferDescriptor};
use fyrox_graphics::error::FrameworkError;
use fyrox_graphics::framebuffer::{Attachment, GpuFrameBuffer};
use fyrox_graphics::geometry_buffer::{GpuGeometryBuffer, GpuGeometryBufferDescriptor};
use fyrox_graphics::gpu_program::{GpuProgram, GpuShader, ShaderKind, ShaderResourceDefinition};
use fyrox_graphics::gpu_texture::{GpuTexture, GpuTextureDescriptor};
use fyrox_graphics::query::GpuQuery;
use fyrox_graphics::read_buffer::GpuAsyncReadBuffer;
use fyrox_graphics::sampler::{GpuSampler, GpuSamplerDescriptor};
use fyrox_graphics::server::{
    GraphicsServer, ServerCapabilities, ServerMemoryUsage, SharedGraphicsServer,
};
use fyrox_graphics::stats::PipelineStatistics;
use fyrox_graphics::{PolygonFace, PolygonFillMode, ScissorBox};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

const MIPMAP_SHADER_SRC: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    return vec4<f32>(pos[idx], 0.0, 1.0);
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    return textureLoad(src_tex, vec2<i32>(pos.xy), 0);
}
"#;

/// A single draw call recorded into an [`ActivePass`]. Captures all GPU state
/// needed to replay the call when the pass is flushed.
pub struct DrawCommand {
    /// The compiled render pipeline for this draw.
    pub pipeline: wgpu::RenderPipeline,
    /// Resource bind group (textures, samplers, uniforms).
    pub bind_group: Option<wgpu::BindGroup>,
    /// Vertex buffers bound for this draw.
    pub vertex_buffers: Vec<wgpu::Buffer>,
    /// Number of extra vertex buffer slots filled with the dummy buffer.
    pub extra_verts: u32,
    /// Index buffer for indexed drawing.
    pub index_buffer: wgpu::Buffer,
    /// Viewport rectangle.
    pub viewport: Rect<i32>,
    /// Stencil reference value, if stencil testing is enabled.
    pub stencil_ref: Option<u32>,
    /// Optional scissor clipping rectangle.
    pub scissor_box: Option<ScissorBox>,
    /// Start index in the index buffer.
    pub start_idx: u32,
    /// End index in the index buffer.
    pub end_idx: u32,
    /// Number of instances to draw.
    pub instances: u32,
}

/// A batched render pass that accumulates draw commands for a single framebuffer.
///
/// Created when the first draw call targets a framebuffer, and flushed
/// (encoded into a `wgpu::RenderPass`) when the target changes or the frame ends.
pub struct ActivePass {
    /// Identity of the target framebuffer (pointer-based).
    pub framebuffer_id: usize,
    /// Color attachment views for the render pass.
    pub color_views: Vec<wgpu::TextureView>,
    /// Depth-stencil attachment view, if present.
    pub depth_view: Option<wgpu::TextureView>,
    /// Load operation for color attachments.
    pub color_load: wgpu::LoadOp<wgpu::Color>,
    /// Load operation for the depth attachment.
    pub depth_load: wgpu::LoadOp<f32>,
    /// Load operation for the stencil attachment, if present.
    pub stencil_load: Option<wgpu::LoadOp<u32>>,
    /// Accumulated draw commands to replay when the pass is flushed.
    pub commands: Vec<DrawCommand>,
}

/// Core wgpu objects shared across the server.
///
/// Holds the wgpu instance, adapter, device, and queue. Shared via [`Arc`] so that
/// GPU commands can be submitted from any resource type.
pub struct WgpuState {
    /// The wgpu instance (entry point for creating adapters and surfaces).
    pub instance: wgpu::Instance,
    /// The selected physical device.
    pub adapter: wgpu::Adapter,
    /// The logical device for creating GPU resources.
    pub device: wgpu::Device,
    /// The command queue for submitting GPU work.
    pub queue: wgpu::Queue,
}

/// The main wgpu-based graphics server.
///
/// Implements [`GraphicsServer`] and serves
/// as the entry point for all GPU resource creation. Manages the wgpu device, surface,
/// pipeline cache, and memory usage tracking.
///
/// # Pipeline Cache
///
/// Render pipelines are cached by a hash of [`PipelineKey`] to avoid recreating
/// identical pipelines across draw calls. The cache is stored in a [`RefCell<HashMap>`]
/// and grows monotonically during the session.
///
/// # Bind Group Cache
///
/// Bind groups are cached by a hash of resource pointers and texture formats to avoid
/// recreating identical bind groups across draw calls. This is critical for performance
/// since bind group creation is expensive.
///
/// # Backbuffer Lifecycle
///
/// The backbuffer acquires a surface texture on the first draw call per frame.
/// Multiple draw calls to the backbuffer reuse the same texture. The frame is
/// presented via [`swap_buffers`](Self::swap_buffers), which also sets the
/// clear flag for the next frame.
pub struct WgpuGraphicsServer {
    /// Core wgpu objects (instance, adapter, device, queue).
    pub state: Arc<WgpuState>,
    /// The rendering surface (window or canvas).
    pub surface: wgpu::Surface<'static>,
    /// Current surface configuration (format, size, present mode).
    pub surface_config: RwLock<wgpu::SurfaceConfiguration>,
    /// Whether to set debug labels on GPU objects.
    pub named_objects: bool,
    /// MSAA sample count (currently forced to 1).
    pub msaa_sample_count: u32,
    /// Hash-based cache of render pipelines, keyed by [`PipelineKey`].
    pub pipeline_cache: RefCell<HashMap<PipelineKey, wgpu::RenderPipeline>>,
    /// Hash-based cache of bind groups, keyed by resource pointers and texture formats.
    pub bind_group_cache: RefCell<HashMap<u64, wgpu::BindGroup>>,
    weak_self: RefCell<Option<Weak<WgpuGraphicsServer>>>,
    /// Tracked GPU memory usage (buffers + textures).
    pub memory_usage: RefCell<ServerMemoryUsage>,
    pipeline_statistics: RefCell<PipelineStatistics>,
    /// Small buffer bound to extra vertex slots when geometry lacks attributes the shader expects.
    pub dummy_vertex_buffer: wgpu::Buffer,
    /// Non-filtering sampler for textures with non-filterable formats (e.g. R32Float).
    non_filtering_sampler: wgpu::Sampler,
    /// Holds the acquired surface frame between do_draw and swap_buffers.
    pub current_frame: RefCell<Option<wgpu::SurfaceTexture>>,
    /// Whether the backbuffer needs clearing at the start of the next frame.
    pub backbuffer_needs_clear: Cell<bool>,
    /// Cached depth-stencil texture for the backbuffer, with its (width, height).
    backbuffer_depth_stencil: RefCell<Option<(u32, u32, GpuTexture)>>,
    /// Per-frame command encoder. Lazily created on first draw, submitted in swap_buffers.
    /// Storing it on the server (not per-framebuffer) because multiple framebuffers
    /// (G-Buffer, HDR, backbuffer) share the same frame and benefit from a single submit.
    pub frame_encoder: RefCell<Option<wgpu::CommandEncoder>>,
    /// Currently accumulating render pass. Draw commands are batched here and
    /// flushed when the target changes or the frame ends.
    pub active_pass: RefCell<Option<ActivePass>>,
    /// Sampler used for mipmap generation (linear filtering).
    mipmap_sampler: wgpu::Sampler,
    /// Bind group layout for the mipmap generation shader.
    mipmap_bind_group_layout: wgpu::BindGroupLayout,
    /// Cached mipmap render pipelines, keyed by texture format.
    mipmap_pipeline_cache: RefCell<HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>>,
    /// Shared shader module for mipmap generation (created once).
    mipmap_shader: OnceCell<wgpu::ShaderModule>,
    /// Shared pipeline layout for mipmap generation (created once).
    mipmap_pipeline_layout: OnceCell<wgpu::PipelineLayout>,
    /// Current polygon fill mode (Fill, Line, or Point). Baked into new pipelines.
    polygon_fill_mode: Cell<PolygonFillMode>,
}

impl WgpuGraphicsServer {
    /// Creates a new wgpu graphics server with a window and GPU device.
    ///
    /// Initializes wgpu with the primary backend (Vulkan/Metal/DX12 on native,
    /// WebGL2 on WASM). Prefers non-sRGB surface formats to avoid double gamma
    /// correction (the engine applies its own in the HDR tone-mapping pass).
    ///
    /// Returns the created [`Window`] and a shared [`GraphicsServer`] handle.
    ///
    /// # Arguments
    ///
    /// * `vsync` — enable vertical sync (`PresentMode::AutoVsync` vs `AutoNoVsync`)
    /// * `_msaa_sample_count` — currently ignored (MSAA not yet implemented)
    /// * `window_target` — the winit event loop for window/surface creation
    /// * `window_attributes` — initial window configuration
    /// * `named_objects` — whether to set debug labels on GPU objects
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        vsync: bool,
        _msaa_sample_count: Option<u8>,
        window_target: &ActiveEventLoop,
        window_attributes: WindowAttributes,
        named_objects: bool,
    ) -> Result<(Window, SharedGraphicsServer), FrameworkError> {
        let window = window_target
            .create_window(window_attributes)
            .map_err(|e| FrameworkError::Custom(format!("Failed to create window: {e}")))?;
        let size = window.inner_size();

        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
                window_target.owned_display_handle(),
            ))
        });

        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        #[cfg(not(target_arch = "wasm32"))]
        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::from_window(&window)
                .map_err(|e| FrameworkError::Custom(format!("Failed to get window handle: {e}")))?;
            instance
                .create_surface_unsafe(target)
                .map_err(|e| FrameworkError::Custom(format!("Failed to create surface: {e}")))?
        };

        #[cfg(target_arch = "wasm32")]
        let surface = {
            use fyrox_core::wasm_bindgen::JsCast;
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().unwrap();
            let web_window = fyrox_core::web_sys::window().unwrap();
            let document = web_window.document().unwrap();
            let body = document.body().unwrap();
            body.append_child(&canvas)
                .expect("Append canvas to HTML body");
            instance
                .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
                .map_err(|e| FrameworkError::Custom(format!("Failed to create surface: {e}")))?
        };

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|e| FrameworkError::Custom(format!("No suitable WGPU adapter found: {e}")))?;

        let adapter_features = adapter.features();
        let mut required_features = wgpu::Features::empty();

        if adapter_features.contains(wgpu::Features::POLYGON_MODE_LINE) {
            required_features |= wgpu::Features::POLYGON_MODE_LINE;
        }

        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features,
            required_limits: if cfg!(target_arch = "wasm32") {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                adapter.limits()
            },
            memory_hints: wgpu::MemoryHints::Performance,
            ..Default::default()
        }))
        .map_err(|e| FrameworkError::Custom(format!("Failed to request device: {e}")))?;

        let surface_caps = surface.get_capabilities(&adapter);
        // Prefer linear (non-sRGB) formats to avoid double gamma correction.
        // The engine applies its own gamma correction in the HDR tone-mapping pass.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or_else(|| surface_caps.formats.first().copied())
            .ok_or_else(|| FrameworkError::Custom("Surface has no supported formats".into()))?;

        let present_mode = if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: Default::default(),
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // TODO: Force `msaa_sample_count` to 1 in the wgpu backend. Full MSAA support requires creating multisampled render targets and resolve targets, which is a larger feature.
        let msaa = 1u32; // msaa_sample_count.unwrap_or(1).max(1) as u32;

        let non_filtering_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("NonFilteringSampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let mipmap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("MipmapSampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let mipmap_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MipmapBGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let dummy_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DummyVB"),
            size: 16, // enough for vec4f
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let server = Rc::new(Self {
            state: Arc::new(WgpuState {
                instance,
                adapter,
                device,
                queue,
            }),
            surface,
            surface_config: RwLock::new(surface_config),
            named_objects,
            msaa_sample_count: msaa,
            pipeline_cache: RefCell::new(HashMap::new()),
            bind_group_cache: RefCell::new(HashMap::new()),
            weak_self: RefCell::new(None),
            memory_usage: RefCell::new(ServerMemoryUsage::default()),
            pipeline_statistics: RefCell::new(PipelineStatistics::default()),
            dummy_vertex_buffer,
            non_filtering_sampler,
            current_frame: RefCell::new(None),
            backbuffer_needs_clear: Cell::new(true),
            backbuffer_depth_stencil: RefCell::new(None),
            frame_encoder: RefCell::new(None),
            active_pass: RefCell::new(None),
            mipmap_sampler,
            mipmap_bind_group_layout,
            mipmap_pipeline_cache: RefCell::new(HashMap::new()),
            mipmap_shader: OnceCell::new(),
            mipmap_pipeline_layout: OnceCell::new(),
            polygon_fill_mode: Cell::new(PolygonFillMode::Fill),
        });

        *server.weak_self.borrow_mut() = Some(Rc::downgrade(&server));

        Ok((window, server))
    }

    /// Returns a [`Weak`] reference to this server.
    ///
    /// Used by resource types to avoid reference cycles. Resources store a weak
    /// reference and call [`upgrade`](Weak::upgrade) when they need the server.
    pub fn weak_ref(&self) -> Weak<WgpuGraphicsServer> {
        self.weak_self.borrow().clone().unwrap()
    }
    /// Returns the non-filtering sampler used for textures with non-filterable formats
    /// (e.g. `R32Float`, `R32Uint`). These formats require
    /// [`SamplerBindingType::NonFiltering`](wgpu::SamplerBindingType::NonFiltering).
    pub fn non_filtering_sampler(&self) -> &wgpu::Sampler {
        &self.non_filtering_sampler
    }
    /// Returns the current polygon fill mode. Baked into new render pipelines.
    pub fn polygon_fill_mode(&self) -> PolygonFillMode {
        self.polygon_fill_mode.get()
    }

    /// Encodes the current [`ActivePass`] into the frame command encoder and clears it.
    ///
    /// All buffered draw commands are replayed into a `wgpu::RenderPass`, which is
    /// then dropped (ending the pass). The encoder is stored back for subsequent
    /// operations. Does nothing if there is no active pass.
    pub fn flush_active_pass(&self) {
        let Some(pass) = self.active_pass.borrow_mut().take() else {
            return;
        };

        let mut encoder = self.frame_encoder.borrow_mut().take().unwrap_or_else(|| {
            self.state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
        });

        let color_attachments: Vec<_> = pass
            .color_views
            .iter()
            .map(|view| {
                Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: pass.color_load,
                        store: wgpu::StoreOp::Store,
                    },
                })
            })
            .collect();

        let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: pass.depth_view.as_ref().map(|v| {
                wgpu::RenderPassDepthStencilAttachment {
                    view: v,
                    depth_ops: Some(wgpu::Operations {
                        load: pass.depth_load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: pass.stencil_load.map(|load| wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    }),
                }
            }),
            ..Default::default()
        });

        for cmd in pass.commands {
            rp.set_viewport(
                cmd.viewport.x() as f32,
                cmd.viewport.y() as f32,
                cmd.viewport.w() as f32,
                cmd.viewport.h() as f32,
                0.0,
                1.0,
            );
            rp.set_pipeline(&cmd.pipeline);
            if let Some(bg) = &cmd.bind_group {
                rp.set_bind_group(0, bg, &[]);
            }
            if let Some(st) = cmd.stencil_ref {
                rp.set_stencil_reference(st);
            }
            match cmd.scissor_box {
                Some(sb) => {
                    // The ScissorBox Y is computed for OpenGL (origin at bottom-left):
                    //   y_gl = viewport_h - (pos_y + size_h)
                    // wgpu uses top-left origin (same as UI coords), so convert:
                    //   y_wgpu = viewport_h - y_gl - height = pos_y
                    let rt_h = cmd.viewport.h();
                    let wgpu_y = (rt_h - sb.y - sb.height).max(0);
                    rp.set_scissor_rect(
                        sb.x.max(0) as u32,
                        wgpu_y as u32,
                        sb.width.max(0) as u32,
                        sb.height.max(0) as u32,
                    );
                }
                None => rp.set_scissor_rect(
                    cmd.viewport.x().max(0) as u32,
                    cmd.viewport.y().max(0) as u32,
                    cmd.viewport.w().max(0) as u32,
                    cmd.viewport.h().max(0) as u32,
                ),
            }
            for (i, vb) in cmd.vertex_buffers.iter().enumerate() {
                rp.set_vertex_buffer(i as u32, vb.slice(..));
            }
            let geo_buf_count = cmd.vertex_buffers.len() as u32;
            for i in 0..cmd.extra_verts {
                rp.set_vertex_buffer(geo_buf_count + i, self.dummy_vertex_buffer.slice(..));
            }
            rp.set_index_buffer(cmd.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(cmd.start_idx..cmd.end_idx, 0, 0..cmd.instances);
        }

        drop(rp);
        *self.frame_encoder.borrow_mut() = Some(encoder);
    }
}

impl GraphicsServer for WgpuGraphicsServer {
    fn create_buffer(&self, desc: GpuBufferDescriptor) -> Result<GpuBuffer, FrameworkError> {
        Ok(GpuBuffer(Rc::new(WgpuBuffer::new(self, desc)?)))
    }
    fn create_texture(&self, desc: GpuTextureDescriptor) -> Result<GpuTexture, FrameworkError> {
        Ok(GpuTexture(Rc::new(WgpuTexture::new(self, desc)?)))
    }
    fn create_sampler(&self, desc: GpuSamplerDescriptor) -> Result<GpuSampler, FrameworkError> {
        Ok(GpuSampler(Rc::new(WgpuSampler::new(self, desc)?)))
    }
    fn create_frame_buffer(
        &self,
        depth: Option<Attachment>,
        colors: Vec<Attachment>,
    ) -> Result<GpuFrameBuffer, FrameworkError> {
        Ok(GpuFrameBuffer(Rc::new(WgpuFrameBuffer::new(
            self, depth, colors,
        )?)))
    }
    fn back_buffer(&self) -> GpuFrameBuffer {
        let config = self.surface_config.read().unwrap();
        let (w, h) = (config.width, config.height);
        drop(config);

        let mut cache = self.backbuffer_depth_stencil.borrow_mut();
        let needs_recreate = match cache.as_ref() {
            Some((cw, ch, _)) => *cw != w || *ch != h,
            None => true,
        };
        if needs_recreate {
            if w > 0 && h > 0 {
                match self.create_2d_render_target(
                    "BackbufferDepthStencil",
                    fyrox_graphics::gpu_texture::PixelKind::D24S8,
                    w as usize,
                    h as usize,
                ) {
                    Ok(tex) => {
                        *cache = Some((w, h, tex));
                    }
                    Err(e) => {
                        Log::warn(format!("Failed to create backbuffer depth-stencil: {e}"));
                        *cache = None;
                    }
                }
            } else {
                *cache = None;
            }
        }
        let depth_attachment = cache
            .as_ref()
            .map(|(_, _, tex)| Attachment::depth_stencil(tex.clone()));
        GpuFrameBuffer(Rc::new(WgpuFrameBuffer::backbuffer(self, depth_attachment)))
    }
    fn create_query(&self) -> Result<GpuQuery, FrameworkError> {
        Ok(GpuQuery(Rc::new(WgpuQuery::new(self)?)))
    }
    fn create_shader(
        &self,
        name: String,
        kind: ShaderKind,
        source: String,
        resources: &[ShaderResourceDefinition],
        line_offset: isize,
    ) -> Result<GpuShader, FrameworkError> {
        Ok(GpuShader(Rc::new(WgpuShader::new(
            self,
            name,
            kind,
            source,
            resources,
            line_offset,
        )?)))
    }
    fn create_program(
        &self,
        name: &str,
        vs: String,
        vs_offset: isize,
        fs: String,
        fs_offset: isize,
        resources: &[ShaderResourceDefinition],
    ) -> Result<GpuProgram, FrameworkError> {
        Ok(GpuProgram(Rc::new(WgpuProgram::from_source(
            self, name, vs, vs_offset, fs, fs_offset, resources,
        )?)))
    }
    fn create_program_from_shaders(
        &self,
        name: &str,
        vs: &GpuShader,
        fs: &GpuShader,
        resources: &[ShaderResourceDefinition],
    ) -> Result<GpuProgram, FrameworkError> {
        Ok(GpuProgram(Rc::new(WgpuProgram::from_shaders(
            self, name, vs, fs, resources,
        )?)))
    }
    fn create_async_read_buffer(
        &self,
        name: &str,
        pixel_size: usize,
        pixel_count: usize,
    ) -> Result<GpuAsyncReadBuffer, FrameworkError> {
        Ok(GpuAsyncReadBuffer(Rc::new(WgpuAsyncReadBuffer::new(
            self,
            name,
            pixel_size,
            pixel_count,
        )?)))
    }
    fn create_geometry_buffer(
        &self,
        desc: GpuGeometryBufferDescriptor,
    ) -> Result<GpuGeometryBuffer, FrameworkError> {
        Ok(GpuGeometryBuffer(Rc::new(WgpuGeometryBuffer::new(
            self, desc,
        )?)))
    }
    fn weak(&self) -> Weak<dyn GraphicsServer> {
        self.weak_ref() as Weak<dyn GraphicsServer>
    }
    fn flush(&self) {
        // flush in Fyrox means "send the accumulated commands to the video card right now."
        // An empty submit is not needed here, just close and send the encoder, if there is one.
        self.flush_active_pass();
        if let Some(encoder) = self.frame_encoder.borrow_mut().take() {
            self.state.queue.submit(std::iter::once(encoder.finish()));
        }
    }
    fn finish(&self) {
        self.state
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();
    }
    fn invalidate_resource_bindings_cache(&self) {
        *self.pipeline_statistics.borrow_mut() = Default::default();
    }
    fn pipeline_statistics(&self) -> PipelineStatistics {
        *self.pipeline_statistics.borrow()
    }
    fn swap_buffers(&self) -> Result<(), FrameworkError> {
        // Submit all batched draw commands from this frame
        self.flush_active_pass();
        if let Some(encoder) = self.frame_encoder.borrow_mut().take() {
            self.state.queue.submit(std::iter::once(encoder.finish()));
        }

        if let Some(frame) = self.current_frame.borrow_mut().take() {
            self.state.queue.present(frame);
        }

        self.backbuffer_needs_clear.replace(true);
        Ok(())
    }
    fn set_frame_size(&self, new_size: (u32, u32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            let mut config = self.surface_config.write().unwrap();

            if config.width == new_size.0 && config.height == new_size.1 {
                return;
            }

            config.width = new_size.0;
            config.height = new_size.1;

            self.flush_active_pass();
            if let Some(encoder) = self.frame_encoder.borrow_mut().take() {
                self.state.queue.submit(std::iter::once(encoder.finish()));
            }

            self.current_frame.borrow_mut().take();

            self.surface.configure(&self.state.device, &config);
        }
    }
    fn capabilities(&self) -> ServerCapabilities {
        let limits = self.state.device.limits();
        ServerCapabilities {
            max_uniform_buffer_binding_size: limits.max_uniform_buffer_binding_size as usize,
            uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment as usize,
            max_lod_bias: 16.0,
        }
    }
    fn set_polygon_fill_mode(&self, _face: PolygonFace, mode: PolygonFillMode) {
        self.polygon_fill_mode.set(mode);
    }
    fn generate_mipmap(&self, texture: &GpuTexture) {
        let Some(wtex) = texture.as_any().downcast_ref::<WgpuTexture>() else {
            return;
        };
        let format = wtex.format();
        if !is_filterable_format(format) {
            Log::warn("generate_mipmap: format is not filterable, skipping");
            return;
        }

        let (width, height, _depth) = texture_size(texture.kind());
        let mip_count = wtex.wgpu_texture().mip_level_count();
        if mip_count <= 1 || width <= 1 || height <= 1 {
            return;
        }

        self.flush_active_pass();

        let shader = self.mipmap_shader.get_or_init(|| {
            self.state
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("MipmapShader"),
                    source: wgpu::ShaderSource::Wgsl(MIPMAP_SHADER_SRC.into()),
                })
        });
        let layout = self.mipmap_pipeline_layout.get_or_init(|| {
            self.state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("MipmapPL"),
                    bind_group_layouts: &[Some(&self.mipmap_bind_group_layout)],
                    ..Default::default()
                })
        });

        if !self.mipmap_pipeline_cache.borrow().contains_key(&format) {
            let pipeline =
                self.state
                    .device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("MipmapPipeline"),
                        layout: Some(layout),
                        vertex: wgpu::VertexState {
                            module: shader,
                            entry_point: Some("vs_main"),
                            buffers: &[],
                            compilation_options: Default::default(),
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: shader,
                            entry_point: Some("fs_main"),
                            targets: &[Some(wgpu::ColorTargetState {
                                format,
                                blend: None,
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                            compilation_options: Default::default(),
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            ..Default::default()
                        },
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState {
                            count: 1,
                            mask: !0,
                            alpha_to_coverage_enabled: false,
                        },
                        multiview_mask: None,
                        cache: None,
                    });
            self.mipmap_pipeline_cache
                .borrow_mut()
                .insert(format, pipeline);
        }

        let pipeline_cache = self.mipmap_pipeline_cache.borrow();
        let pipeline = pipeline_cache.get(&format).unwrap();

        let mut encoder = self.frame_encoder.borrow_mut().take().unwrap_or_else(|| {
            self.state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
        });

        let mut mip_w = width;
        let mut mip_h = height;

        for level in 1..mip_count {
            mip_w = (mip_w / 2).max(1);
            mip_h = (mip_h / 2).max(1);

            let src_view = wtex
                .wgpu_texture()
                .create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_mip_level: level - 1,
                    mip_level_count: Some(1),
                    ..Default::default()
                });

            let dst_view = wtex
                .wgpu_texture()
                .create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    ..Default::default()
                });

            let bind_group = self
                .state
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.mipmap_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&src_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.mipmap_sampler),
                        },
                    ],
                });

            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &dst_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            rp.set_viewport(0.0, 0.0, mip_w as f32, mip_h as f32, 0.0, 1.0);
            rp.set_pipeline(pipeline);
            rp.set_bind_group(0, &bind_group, &[]);
            rp.draw(0..3, 0..1);
            drop(rp);
        }

        *self.frame_encoder.borrow_mut() = Some(encoder);
    }
    fn memory_usage(&self) -> ServerMemoryUsage {
        self.memory_usage.borrow().clone()
    }
    fn push_debug_group(&self, _name: &str) {}
    fn pop_debug_group(&self) {}
}
