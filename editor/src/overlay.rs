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
    fyrox::{
        core::{algebra::Matrix4, math::Matrix4Ext, sstorage::ImmutableString},
        renderer::{
            framework::{
                buffer::BufferUsage,
                error::FrameworkError,
                framebuffer::{ResourceBindGroup, ResourceBinding},
                geometry_buffer::GeometryBuffer,
                gpu_program::{GpuProgram, UniformLocation},
                server::GraphicsServer,
                uniform::StaticUniformBuffer,
                BlendFactor, BlendFunc, BlendParameters, CompareFunc, DrawParameters, ElementRange,
                GeometryBufferExt,
            },
            RenderPassStatistics, SceneRenderPass, SceneRenderPassContext,
        },
        resource::texture::{
            CompressionOptions, TextureImportOptions, TextureMinificationFilter, TextureResource,
            TextureResourceExtension,
        },
        scene::mesh::surface::SurfaceData,
    },
    Editor,
};
use fyrox::renderer::framework::framebuffer::BufferLocation;
use std::{any::TypeId, cell::RefCell, rc::Rc};

struct OverlayShader {
    program: Box<dyn GpuProgram>,
    diffuse_texture: UniformLocation,
    uniform_buffer_binding: usize,
}

impl OverlayShader {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../resources/shaders/overlay_fs.glsl");
        let vertex_source = include_str!("../resources/shaders/overlay_vs.glsl");
        let program = server.create_program("OverlayShader", vertex_source, fragment_source)?;
        Ok(Self {
            uniform_buffer_binding: program
                .uniform_block_index(&ImmutableString::new("Uniforms"))?,
            diffuse_texture: program.uniform_location(&ImmutableString::new("diffuseTexture"))?,
            program,
        })
    }
}

pub struct OverlayRenderPass {
    quad: Box<dyn GeometryBuffer>,
    shader: OverlayShader,
    sound_icon: TextureResource,
    light_icon: TextureResource,
    pub pictogram_size: f32,
}

impl OverlayRenderPass {
    pub fn new(server: &dyn GraphicsServer) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            quad: <dyn GeometryBuffer>::from_surface_data(
                &SurfaceData::make_collapsed_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )
            .unwrap(),
            shader: OverlayShader::new(server).unwrap(),
            sound_icon: TextureResource::load_from_memory(
                "../resources/sound_source.png".into(),
                include_bytes!("../resources/sound_source.png"),
                TextureImportOptions::default()
                    .with_compression(CompressionOptions::NoCompression)
                    .with_minification_filter(TextureMinificationFilter::Linear),
            )
            .unwrap(),
            light_icon: TextureResource::load_from_memory(
                "../resources/light_source.png".into(),
                include_bytes!("../resources/light_source.png"),
                TextureImportOptions::default()
                    .with_compression(CompressionOptions::NoCompression)
                    .with_minification_filter(TextureMinificationFilter::Linear),
            )
            .unwrap(),
            pictogram_size: 0.33,
        }))
    }
}

impl SceneRenderPass for OverlayRenderPass {
    fn on_hdr_render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let view_projection = ctx.camera.view_projection_matrix();
        let shader = &self.shader;
        let inv_view = ctx.camera.inv_view_matrix().unwrap();
        let camera_up = -inv_view.up();
        let camera_side = inv_view.side();
        let sound_icon = ctx
            .texture_cache
            .get(ctx.server, &self.sound_icon)
            .cloned()
            .unwrap();
        let light_icon = ctx.texture_cache.get(ctx.server, &self.light_icon).unwrap();

        for node in ctx.scene.graph.linear_iter() {
            let icon =
                if node.is_directional_light() || node.is_spot_light() || node.is_point_light() {
                    light_icon.clone()
                } else if node.is_sound() {
                    sound_icon.clone()
                } else {
                    continue;
                };

            let position = node.global_position();
            let world_matrix = Matrix4::new_translation(&position);

            ctx.framebuffer.draw(
                &*self.quad,
                ctx.viewport,
                &*shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: None,
                    depth_test: Some(CompareFunc::Less),
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                        ..Default::default()
                    }),
                    stencil_op: Default::default(),
                    scissor_box: None,
                },
                &[ResourceBindGroup {
                    bindings: &[
                        ResourceBinding::texture(&icon, &shader.diffuse_texture),
                        ResourceBinding::Buffer {
                            buffer: ctx.uniform_buffer_cache.write(
                                StaticUniformBuffer::<256>::new()
                                    .with(&view_projection)
                                    .with(&world_matrix)
                                    .with(&camera_side)
                                    .with(&camera_up)
                                    .with(&self.pictogram_size),
                            )?,
                            binding: BufferLocation::Auto {
                                shader_location: shader.uniform_buffer_binding,
                            },
                            data_usage: Default::default(),
                        },
                    ],
                }],
                ElementRange::Full,
            )?;
        }

        Ok(Default::default())
    }

    fn source_type_id(&self) -> TypeId {
        TypeId::of::<Editor>()
    }
}
