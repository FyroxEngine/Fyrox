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

use crate::renderer::make_viewport_matrix;
use crate::{
    core::{
        algebra::{Matrix3, Matrix4, Vector2, Vector3},
        color::Color,
        math::{lerpf, Rect},
        sstorage::ImmutableString,
    },
    rand::Rng,
    renderer::{
        cache::uniform::UniformBufferCache,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{
                Attachment, AttachmentKind, FrameBuffer, ResourceBindGroup, ResourceBinding,
            },
            geometry_buffer::GeometryBuffer,
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{
                GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
            },
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            DrawParameters, ElementRange, GeometryBufferExt,
        },
        gbuffer::GBuffer,
        ssao::blur::Blur,
        RenderPassStatistics,
    },
    scene::mesh::surface::SurfaceData,
};
use fyrox_graphics::framebuffer::BufferLocation;
use fyrox_graphics::gpu_texture::GpuTextureDescriptor;
use std::{cell::RefCell, rc::Rc};

mod blur;

// Keep in sync with shader define.
const KERNEL_SIZE: usize = 32;

// Size of noise texture.
const NOISE_SIZE: usize = 4;

struct Shader {
    program: Box<dyn GpuProgram>,
    depth_sampler: UniformLocation,
    normal_sampler: UniformLocation,
    noise_sampler: UniformLocation,
    uniform_block_index: usize,
}

impl Shader {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/ssao_fs.glsl");
        let vertex_source = include_str!("../shaders/ssao_vs.glsl");
        let program = server.create_program("SsaoShader", vertex_source, fragment_source)?;
        Ok(Self {
            depth_sampler: program.uniform_location(&ImmutableString::new("depthSampler"))?,
            normal_sampler: program.uniform_location(&ImmutableString::new("normalSampler"))?,
            noise_sampler: program.uniform_location(&ImmutableString::new("noiseSampler"))?,
            uniform_block_index: program.uniform_block_index(&ImmutableString::new("Uniforms"))?,
            program,
        })
    }
}

pub struct ScreenSpaceAmbientOcclusionRenderer {
    blur: Blur,
    shader: Shader,
    framebuffer: Box<dyn FrameBuffer>,
    quad: Box<dyn GeometryBuffer>,
    width: i32,
    height: i32,
    noise: Rc<RefCell<dyn GpuTexture>>,
    kernel: [Vector3<f32>; KERNEL_SIZE],
    radius: f32,
}

impl ScreenSpaceAmbientOcclusionRenderer {
    pub fn new(
        server: &dyn GraphicsServer,
        frame_width: usize,
        frame_height: usize,
    ) -> Result<Self, FrameworkError> {
        // It is good balance between quality and performance, no need to do SSAO in full resolution.
        // This SSAO map size reduction was taken from DOOM (2016).
        let width = (frame_width / 2).max(1);
        let height = (frame_height / 2).max(1);

        let occlusion = server.create_2d_render_target(PixelKind::R32F, width, height)?;

        let mut rng = crate::rand::thread_rng();

        Ok(Self {
            blur: Blur::new(server, width, height)?,
            shader: Shader::new(server)?,
            framebuffer: server.create_frame_buffer(
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: occlusion,
                }],
            )?,
            quad: <dyn GeometryBuffer>::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
            width: width as i32,
            height: height as i32,
            kernel: {
                let mut kernel = [Default::default(); KERNEL_SIZE];
                for (i, v) in kernel.iter_mut().enumerate() {
                    let k = i as f32 / KERNEL_SIZE as f32;
                    let scale = lerpf(0.1, 1.0, k * k);
                    *v = Vector3::new(
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(0.0..1.0),
                    )
                    // Make sphere
                    .try_normalize(f32::EPSILON)
                    .unwrap()
                    // Use non-uniform distribution to shuffle points inside hemisphere.
                    .scale(scale);
                }
                kernel
            },
            noise: {
                const RGB_PIXEL_SIZE: usize = 3;
                let mut pixels = [0u8; RGB_PIXEL_SIZE * NOISE_SIZE * NOISE_SIZE];
                for pixel in pixels.chunks_exact_mut(RGB_PIXEL_SIZE) {
                    pixel[0] = rng.gen_range(0u8..255u8); // R
                    pixel[1] = rng.gen_range(0u8..255u8); // G
                    pixel[2] = 0u8; // B
                }
                server.create_texture(GpuTextureDescriptor {
                    kind: GpuTextureKind::Rectangle {
                        width: NOISE_SIZE,
                        height: NOISE_SIZE,
                    },
                    pixel_kind: PixelKind::RGB8,
                    min_filter: MinificationFilter::Nearest,
                    mag_filter: MagnificationFilter::Nearest,
                    data: Some(&pixels),
                    ..Default::default()
                })?
            },
            radius: 0.5,
        })
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.abs();
    }

    fn raw_ao_map(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn ao_map(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.blur.result()
    }

    pub(crate) fn render(
        &mut self,
        gbuffer: &GBuffer,
        projection_matrix: Matrix4<f32>,
        view_matrix: Matrix3<f32>,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.width, self.height);

        let frame_matrix = make_viewport_matrix(viewport);

        self.framebuffer.clear(
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            None,
        );

        let noise_scale = Vector2::new(
            self.width as f32 / NOISE_SIZE as f32,
            self.height as f32 / NOISE_SIZE as f32,
        );

        let uniform_buffer = uniform_buffer_cache.write(
            StaticUniformBuffer::<1024>::new()
                .with(&frame_matrix)
                .with(&projection_matrix.try_inverse().unwrap_or_default())
                .with(&projection_matrix)
                .with_slice(&self.kernel)
                .with(&noise_scale)
                .with(&view_matrix)
                .with(&self.radius),
        )?;

        stats += self.framebuffer.draw(
            &*self.quad,
            viewport,
            &*self.shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: None,
                blend: None,
                stencil_op: Default::default(),
                scissor_box: None,
            },
            &[ResourceBindGroup {
                bindings: &[
                    ResourceBinding::texture(&gbuffer.depth(), &self.shader.depth_sampler),
                    ResourceBinding::texture(
                        &gbuffer.normal_texture(),
                        &self.shader.normal_sampler,
                    ),
                    ResourceBinding::texture(&self.noise, &self.shader.noise_sampler),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer,
                        binding: BufferLocation::Auto {
                            shader_location: self.shader.uniform_block_index,
                        },
                        data_usage: Default::default(),
                    },
                ],
            }],
            ElementRange::Full,
        )?;

        self.blur.render(self.raw_ao_map(), uniform_buffer_cache)?;

        Ok(stats)
    }
}
