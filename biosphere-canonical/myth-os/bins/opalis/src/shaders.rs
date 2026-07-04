use std::sync::Arc;

use egui_wgpu;
use egui::mutex::Mutex;

/// Uniform data passed to the shader every frame
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderUniforms {
    pub time: f32,
    pub resolution_x: f32,
    pub resolution_y: f32,
    /// Control parameter (0.0-1.0) — can be driven by MIDI, mouse, neural net, etc.
    pub intensity: f32,
    /// Color tint RGBA (0.0-1.0 each)
    pub tint_r: f32,
    pub tint_g: f32,
    pub tint_b: f32,
    pub tint_a: f32,
}

impl Default for ShaderUniforms {
    fn default() -> Self {
        Self {
            time: 0.0,
            resolution_x: 1280.0,
            resolution_y: 800.0,
            intensity: 0.6,
            tint_r: 0.6,
            tint_g: 0.4,
            tint_b: 1.0,
            tint_a: 1.0,
        }
    }
}

/// The WGSL shader source — a procedural nebula/fractal
pub const NEBULA_SHADER: &str = r#"
struct Uniforms {
    time: f32,
    resolution_x: f32,
    resolution_y: f32,
    intensity: f32,
    tint_r: f32,
    tint_g: f32,
    tint_b: f32,
    tint_a: f32,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full-screen triangle
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = pos[vertex_index] * 0.5 + 0.5;
    return out;
}

// Simplex-like noise
fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u_smooth = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u_smooth.x), mix(c, d, u_smooth.x), u_smooth.y);
}

fn fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var value = 0.0;
    var amplitude = 0.5;
    for (var i = 0; i < 5; i++) {
        value += amplitude * noise(p);
        p *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let t = u.time * 0.15;
    let intensity = u.intensity;

    // Centered coordinates
    let centered = (uv - 0.5) * 2.0;
    let dist = length(centered);

    // Layered fractal noise for nebula effect
    let n1 = fbm(centered * 3.0 + vec2<f32>(t, t * 0.7));
    let n2 = fbm(centered * 5.0 - vec2<f32>(t * 0.5, t * 0.3));
    let n3 = fbm(centered * 8.0 + vec2<f32>(t * 0.3, -t * 0.5));

    // Combine into nebula density
    let density = (n1 * 0.5 + n2 * 0.3 + n3 * 0.2) * intensity;

    // Color channels with tint
    let r = density * u.tint_r * (0.8 + 0.2 * sin(t + uv.x * 3.0));
    let g = density * u.tint_g * (0.7 + 0.3 * cos(t * 0.7 + uv.y * 2.0));
    let b = density * u.tint_b * (0.9 + 0.1 * sin(t * 1.3));

    // Vignette — darker at edges
    let vignette = 1.0 - smoothstep(0.3, 1.2, dist);

    // Occasional bright spots (stars / electrical sparks)
    let spark = smoothstep(0.92, 0.95, noise(centered * 20.0 + vec2<f32>(t * 2.0, 0.0)));

    let final_r = (r * vignette + spark * 0.8) * u.tint_a;
    let final_g = (g * vignette + spark * 0.6) * u.tint_a;
    let final_b = (b * vignette + spark * 1.0) * u.tint_a;

    return vec4<f32>(final_r, final_g, final_b, 1.0);
}
"#;

/// GPU resources for rendering a shader layer
pub struct ShaderRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub uniforms: ShaderUniforms,
}

impl ShaderRenderer {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nebula_shader"),
            source: wgpu::ShaderSource::Wgsl(NEBULA_SHADER.into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader_uniforms"),
            size: std::mem::size_of::<ShaderUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shader_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shader_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shader_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nebula_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            uniforms: ShaderUniforms::default(),
        }
    }
}

/// Callback that egui uses to render our shader into a rect
pub struct ShaderCallback {
    pub renderer: Arc<Mutex<ShaderRenderer>>,
}

impl egui_wgpu::CallbackTrait for ShaderCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let renderer = self.renderer.lock();
        queue.write_buffer(
            &renderer.uniform_buffer,
            0,
            bytemuck::bytes_of(&renderer.uniforms),
        );
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let renderer = self.renderer.lock();
        render_pass.set_pipeline(&renderer.pipeline);
        render_pass.set_bind_group(0, &renderer.bind_group, &[]);
        render_pass.draw(0..3, 0..1); // full-screen triangle
    }
}
