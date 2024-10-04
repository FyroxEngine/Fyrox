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

use crate::{
    core::{
        algebra::{Isometry3, Matrix4, Point3, Translation, Vector3},
        math::Rect,
        pool::Handle,
        sstorage::ImmutableString,
    },
    renderer::{
        cache::uniform::UniformBufferCache,
        flat_shader::FlatShader,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{FrameBuffer, ResourceBindGroup, ResourceBinding},
            geometry_buffer::GeometryBuffer,
            gl::server::GlGraphicsServer,
            gpu_program::{GpuProgram, UniformLocation},
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            BlendFactor, BlendFunc, BlendParameters, ColorMask, CompareFunc, DrawParameters,
            ElementRange, GeometryBufferExt, StencilAction, StencilFunc, StencilOp,
        },
        gbuffer::GBuffer,
        RenderPassStatistics,
    },
    scene::{
        graph::Graph,
        light::{point::PointLight, spot::SpotLight},
        mesh::surface::SurfaceData,
        node::Node,
    },
};

struct SpotLightShader {
    program: GpuProgram,
    depth_sampler: UniformLocation,
    uniform_block_binding: usize,
}

impl SpotLightShader {
    fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/spot_volumetric_fs.glsl");
        let vertex_source = include_str!("shaders/spot_volumetric_vs.glsl");
        let program = GpuProgram::from_source(
            server,
            "SpotVolumetricLight",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            depth_sampler: program
                .uniform_location(server, &ImmutableString::new("depthSampler"))?,
            uniform_block_binding: program
                .uniform_block_index(server, &ImmutableString::new("Uniforms"))?,
            program,
        })
    }
}

struct PointLightShader {
    program: GpuProgram,
    depth_sampler: UniformLocation,
    uniform_block_binding: usize,
}

impl PointLightShader {
    fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/point_volumetric_fs.glsl");
        let vertex_source = include_str!("shaders/point_volumetric_vs.glsl");
        let program = GpuProgram::from_source(
            server,
            "PointVolumetricLight",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            depth_sampler: program
                .uniform_location(server, &ImmutableString::new("depthSampler"))?,
            uniform_block_binding: program
                .uniform_block_index(server, &ImmutableString::new("Uniforms"))?,
            program,
        })
    }
}

pub struct LightVolumeRenderer {
    spot_light_shader: SpotLightShader,
    point_light_shader: PointLightShader,
    flat_shader: FlatShader,
    cone: GeometryBuffer,
    sphere: GeometryBuffer,
}

impl LightVolumeRenderer {
    pub fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            spot_light_shader: SpotLightShader::new(server)?,
            point_light_shader: PointLightShader::new(server)?,
            flat_shader: FlatShader::new(server)?,
            cone: GeometryBuffer::from_surface_data(
                &SurfaceData::make_cone(
                    16,
                    1.0,
                    1.0,
                    &Matrix4::new_translation(&Vector3::new(0.0, -1.0, 0.0)),
                ),
                BufferUsage::StaticDraw,
                server,
            )?,
            sphere: GeometryBuffer::from_surface_data(
                &SurfaceData::make_sphere(8, 8, 1.0, &Matrix4::identity()),
                BufferUsage::StaticDraw,
                server,
            )?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_volume(
        &mut self,
        server: &dyn GraphicsServer,
        light: &Node,
        light_handle: Handle<Node>,
        gbuffer: &mut GBuffer,
        quad: &GeometryBuffer,
        view: Matrix4<f32>,
        inv_proj: Matrix4<f32>,
        view_proj: Matrix4<f32>,
        viewport: Rect<i32>,
        graph: &Graph,
        frame_buffer: &mut dyn FrameBuffer,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let frame_matrix = Matrix4::new_orthographic(
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
        ));

        let position = view
            .transform_point(&Point3::from(light.global_position()))
            .coords;

        if let Some(spot) = light.cast::<SpotLight>() {
            if !spot.base_light_ref().is_scatter_enabled() {
                return Ok(stats);
            }

            let direction = view.transform_vector(
                &(-light
                    .up_vector()
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_else(Vector3::z)),
            );

            // Draw cone into stencil buffer - it will mark pixels for further volumetric light
            // calculations, it will significantly reduce amount of pixels for far lights thus
            // significantly improve performance.

            let k = (spot.full_cone_angle() * 0.5 + 1.0f32.to_radians()).tan() * spot.distance();
            let light_shape_matrix = Isometry3 {
                rotation: graph.global_rotation(light_handle),
                translation: Translation {
                    vector: spot.global_position(),
                },
            }
            .to_homogeneous()
                * Matrix4::new_nonuniform_scaling(&Vector3::new(k, spot.distance(), k));
            let mvp = view_proj * light_shape_matrix;

            // Clear stencil only.
            frame_buffer.clear(viewport, None, None, Some(0));

            stats += frame_buffer.draw(
                &self.cone,
                viewport,
                &self.flat_shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Equal,
                        ref_value: 0xFF,
                        mask: 0xFFFF_FFFF,
                    }),
                    depth_test: Some(CompareFunc::Less),
                    blend: None,
                    stencil_op: StencilOp {
                        fail: StencilAction::Replace,
                        zfail: StencilAction::Keep,
                        zpass: StencilAction::Replace,
                        write_mask: 0xFFFF_FFFF,
                    },
                    scissor_box: None,
                },
                &[ResourceBindGroup {
                    bindings: &[ResourceBinding::Buffer {
                        buffer: uniform_buffer_cache
                            .write(server, StaticUniformBuffer::<256>::new().with(&mvp))?,
                        shader_location: self.flat_shader.uniform_buffer_binding,
                    }],
                }],
                ElementRange::Full,
                &mut |_| {},
            )?;

            // Finally draw fullscreen quad, GPU will calculate scattering only on pixels that were
            // marked in stencil buffer. For distant lights it will be very low amount of pixels and
            // so distant lights won't impact performance.
            let shader = &self.spot_light_shader;
            stats += frame_buffer.draw(
                quad,
                viewport,
                &shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Equal,
                        ref_value: 0xFF,
                        mask: 0xFFFF_FFFF,
                    }),
                    depth_test: None,
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                        ..Default::default()
                    }),
                    // Make sure to clean stencil buffer after drawing full screen quad.
                    stencil_op: StencilOp {
                        zpass: StencilAction::Zero,
                        ..Default::default()
                    },
                    scissor_box: None,
                },
                &[ResourceBindGroup {
                    bindings: &[
                        ResourceBinding::texture(&gbuffer.depth(), &shader.depth_sampler),
                        ResourceBinding::Buffer {
                            buffer: uniform_buffer_cache.write(
                                server,
                                StaticUniformBuffer::<256>::new()
                                    .with(&frame_matrix)
                                    .with(&inv_proj)
                                    .with(&position)
                                    .with(&direction)
                                    .with(&spot.base_light_ref().color().srgb_to_linear_f32().xyz())
                                    .with(&spot.base_light_ref().scatter())
                                    .with(&spot.base_light_ref().intensity())
                                    .with(&((spot.full_cone_angle() * 0.5).cos())),
                            )?,
                            shader_location: shader.uniform_block_binding,
                        },
                    ],
                }],
                ElementRange::Full,
                &mut |_| {},
            )?
        } else if let Some(point) = light.cast::<PointLight>() {
            if !point.base_light_ref().is_scatter_enabled() {
                return Ok(stats);
            }

            frame_buffer.clear(viewport, None, None, Some(0));

            // Radius bias is used to slightly increase sphere radius to add small margin
            // for fadeout effect. It is set to 5%.
            let bias = 1.05;
            let k = bias * point.radius();
            let light_shape_matrix = Matrix4::new_translation(&light.global_position())
                * Matrix4::new_nonuniform_scaling(&Vector3::new(k, k, k));
            let mvp = view_proj * light_shape_matrix;

            let uniform_buffer =
                uniform_buffer_cache.write(server, StaticUniformBuffer::<256>::new().with(&mvp))?;

            stats += frame_buffer.draw(
                &self.sphere,
                viewport,
                &self.flat_shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Equal,
                        ref_value: 0xFF,
                        mask: 0xFFFF_FFFF,
                    }),
                    depth_test: Some(CompareFunc::Less),
                    blend: None,
                    stencil_op: StencilOp {
                        fail: StencilAction::Replace,
                        zfail: StencilAction::Keep,
                        zpass: StencilAction::Replace,
                        write_mask: 0xFFFF_FFFF,
                    },
                    scissor_box: None,
                },
                &[ResourceBindGroup {
                    bindings: &[ResourceBinding::Buffer {
                        buffer: uniform_buffer,
                        shader_location: self.flat_shader.uniform_buffer_binding,
                    }],
                }],
                ElementRange::Full,
                &mut |_| {},
            )?;

            // Finally draw fullscreen quad, GPU will calculate scattering only on pixels that were
            // marked in stencil buffer. For distant lights it will be very low amount of pixels and
            // so distant lights won't impact performance.
            let shader = &self.point_light_shader;
            stats += frame_buffer.draw(
                quad,
                viewport,
                &shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Equal,
                        ref_value: 0xFF,
                        mask: 0xFFFF_FFFF,
                    }),
                    depth_test: None,
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                        ..Default::default()
                    }),
                    // Make sure to clean stencil buffer after drawing full screen quad.
                    stencil_op: StencilOp {
                        zpass: StencilAction::Zero,
                        ..Default::default()
                    },
                    scissor_box: None,
                },
                &[ResourceBindGroup {
                    bindings: &[
                        ResourceBinding::texture(&gbuffer.depth(), &shader.depth_sampler),
                        ResourceBinding::Buffer {
                            buffer: uniform_buffer_cache.write(
                                server,
                                StaticUniformBuffer::<256>::new()
                                    .with(&frame_matrix)
                                    .with(&inv_proj)
                                    .with(&position)
                                    .with(
                                        &point.base_light_ref().color().srgb_to_linear_f32().xyz(),
                                    )
                                    .with(&point.base_light_ref().scatter())
                                    .with(&point.base_light_ref().intensity())
                                    .with(&point.radius()),
                            )?,
                            shader_location: shader.uniform_block_binding,
                        },
                    ],
                }],
                ElementRange::Full,
                &mut |_| {},
            )?
        }

        Ok(stats)
    }
}
