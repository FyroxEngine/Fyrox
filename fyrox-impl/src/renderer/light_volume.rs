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
        sstorage::ImmutableString,
    },
    renderer::{
        bundle::{LightSource, LightSourceKind},
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial, RenderPassContainer},
            uniform::UniformBufferCache,
        },
        flat_shader::FlatShader,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{BufferLocation, GpuFrameBuffer, ResourceBindGroup, ResourceBinding},
            geometry_buffer::GpuGeometryBuffer,
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            ColorMask, CompareFunc, DrawParameters, ElementRange, GeometryBufferExt, StencilAction,
            StencilFunc, StencilOp,
        },
        gbuffer::GBuffer,
        make_viewport_matrix, RenderPassStatistics,
    },
    scene::{graph::Graph, mesh::surface::SurfaceData},
};

pub struct LightVolumeRenderer {
    spot_light_shader: RenderPassContainer,
    point_light_shader: RenderPassContainer,
    flat_shader: FlatShader,
    cone: GpuGeometryBuffer,
    sphere: GpuGeometryBuffer,
}

impl LightVolumeRenderer {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            spot_light_shader: RenderPassContainer::from_str(
                server,
                include_str!("shaders/spot_volumetric.shader"),
            )?,
            point_light_shader: RenderPassContainer::from_str(
                server,
                include_str!("shaders/point_volumetric.shader"),
            )?,
            flat_shader: FlatShader::new(server)?,
            cone: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_cone(
                    16,
                    1.0,
                    1.0,
                    &Matrix4::new_translation(&Vector3::new(0.0, -1.0, 0.0)),
                ),
                BufferUsage::StaticDraw,
                server,
            )?,
            sphere: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_sphere(8, 8, 1.0, &Matrix4::identity()),
                BufferUsage::StaticDraw,
                server,
            )?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_volume(
        &mut self,
        light: &LightSource,
        gbuffer: &GBuffer,
        quad: &GpuGeometryBuffer,
        view: Matrix4<f32>,
        inv_proj: Matrix4<f32>,
        view_proj: Matrix4<f32>,
        viewport: Rect<i32>,
        graph: &Graph,
        frame_buffer: &GpuFrameBuffer,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let frame_matrix = make_viewport_matrix(viewport);
        let position = view.transform_point(&Point3::from(light.position)).coords;
        let color = light.color.srgb_to_linear_f32().xyz();

        match light.kind {
            LightSourceKind::Spot {
                distance,
                full_cone_angle,
                ..
            } => {
                let direction = view.transform_vector(
                    &(-light
                        .up_vector
                        .try_normalize(f32::EPSILON)
                        .unwrap_or_else(Vector3::z)),
                );

                // Draw cone into stencil buffer - it will mark pixels for further volumetric light
                // calculations, it will significantly reduce amount of pixels for far lights thus
                // significantly improve performance.

                let k = (full_cone_angle * 0.5 + 1.0f32.to_radians()).tan() * distance;
                let light_shape_matrix = Isometry3 {
                    rotation: graph.global_rotation(light.handle),
                    translation: Translation {
                        vector: light.position,
                    },
                }
                .to_homogeneous()
                    * Matrix4::new_nonuniform_scaling(&Vector3::new(k, distance, k));
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
                                .write(StaticUniformBuffer::<256>::new().with(&mvp))?,
                            binding: BufferLocation::Auto {
                                shader_location: self.flat_shader.uniform_buffer_binding,
                            },
                            data_usage: Default::default(),
                        }],
                    }],
                    ElementRange::Full,
                )?;

                // Finally draw fullscreen quad, GPU will calculate scattering only on pixels that were
                // marked in stencil buffer. For distant lights it will be very low amount of pixels and
                // so distant lights won't impact performance.
                let cone_angle_cos = (full_cone_angle * 0.5).cos();

                let properties = PropertyGroup::from([
                    property("worldViewProjection", &frame_matrix),
                    property("invProj", &inv_proj),
                    property("lightPosition", &position),
                    property("lightDirection", &direction),
                    property("lightColor", &color),
                    property("scatterFactor", &light.scatter),
                    property("intensity", &light.intensity),
                    property("coneAngleCos", &cone_angle_cos),
                ]);
                let material = RenderMaterial::from([
                    binding("depthSampler", gbuffer.depth()),
                    binding("properties", &properties),
                ]);

                stats += self.spot_light_shader.run_pass(
                    &ImmutableString::new("Primary"),
                    frame_buffer,
                    quad,
                    viewport,
                    &material,
                    uniform_buffer_cache,
                    Default::default(),
                    None,
                )?;
            }
            LightSourceKind::Point { radius, .. } => {
                frame_buffer.clear(viewport, None, None, Some(0));

                // Radius bias is used to slightly increase sphere radius to add small margin
                // for fadeout effect. It is set to 5%.
                let bias = 1.05;
                let k = bias * radius;
                let light_shape_matrix = Matrix4::new_translation(&light.position)
                    * Matrix4::new_nonuniform_scaling(&Vector3::new(k, k, k));
                let mvp = view_proj * light_shape_matrix;

                let uniform_buffer =
                    uniform_buffer_cache.write(StaticUniformBuffer::<256>::new().with(&mvp))?;

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
                            binding: BufferLocation::Auto {
                                shader_location: self.flat_shader.uniform_buffer_binding,
                            },
                            data_usage: Default::default(),
                        }],
                    }],
                    ElementRange::Full,
                )?;

                // Finally draw fullscreen quad, GPU will calculate scattering only on pixels that were
                // marked in stencil buffer. For distant lights it will be very low amount of pixels and
                // so distant lights won't impact performance.
                let properties = PropertyGroup::from([
                    property("worldViewProjection", &frame_matrix),
                    property("invProj", &inv_proj),
                    property("lightPosition", &position),
                    property("lightColor", &color),
                    property("scatterFactor", &light.scatter),
                    property("intensity", &light.intensity),
                    property("lightRadius", &radius),
                ]);
                let material = RenderMaterial::from([
                    binding("depthSampler", gbuffer.depth()),
                    binding("properties", &properties),
                ]);

                stats += self.point_light_shader.run_pass(
                    &ImmutableString::new("Primary"),
                    frame_buffer,
                    quad,
                    viewport,
                    &material,
                    uniform_buffer_cache,
                    Default::default(),
                    None,
                )?;
            }
            _ => (),
        }

        Ok(stats)
    }
}
