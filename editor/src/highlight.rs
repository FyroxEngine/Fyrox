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
        core::{color::Color, pool::Handle, sstorage::ImmutableString},
        fxhash::FxHashSet,
        graph::BaseSceneGraph,
        renderer::{
            bundle::{BundleRenderContext, ObserverInfo, RenderContext, RenderDataBundleStorage},
            framework::{
                buffer::BufferUsage,
                error::FrameworkError,
                framebuffer::{
                    Attachment, AttachmentKind, BufferLocation, FrameBuffer, ResourceBindGroup,
                    ResourceBinding,
                },
                geometry_buffer::GeometryBuffer,
                gpu_program::{GpuProgram, UniformLocation},
                gpu_texture::PixelKind,
                server::GraphicsServer,
                uniform::StaticUniformBuffer,
                BlendFactor, BlendFunc, BlendParameters, CompareFunc, DrawParameters, ElementRange,
                GeometryBufferExt,
            },
            make_viewport_matrix, RenderPassStatistics, SceneRenderPass, SceneRenderPassContext,
        },
        scene::{mesh::surface::SurfaceData, node::Node, Scene},
    },
    Editor,
};
use fyrox::graph::SceneGraph;
use std::{any::TypeId, cell::RefCell, rc::Rc};

struct EdgeDetectShader {
    program: Box<dyn GpuProgram>,
    uniform_buffer_binding: usize,
    frame_texture: UniformLocation,
}

impl EdgeDetectShader {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = r#"
layout (location = 0) out vec4 outColor;

uniform sampler2D frameTexture;

layout(std140) uniform Uniforms {
    mat4 worldViewProjection;
    vec4 color;
};

in vec2 texCoord;

void main() {
	ivec2 size = textureSize(frameTexture, 0);

	float w = 1.0 / float(size.x);
	float h = 1.0 / float(size.y);

    float n[9];
	n[0] = texture(frameTexture, texCoord + vec2(-w, -h)).a;
	n[1] = texture(frameTexture, texCoord + vec2(0.0, -h)).a;
	n[2] = texture(frameTexture, texCoord + vec2(w, -h)).a;
	n[3] = texture(frameTexture, texCoord + vec2( -w, 0.0)).a;
	n[4] = texture(frameTexture, texCoord).a;
	n[5] = texture(frameTexture, texCoord + vec2(w, 0.0)).a;
	n[6] = texture(frameTexture, texCoord + vec2(-w, h)).a;
	n[7] = texture(frameTexture, texCoord + vec2(0.0, h)).a;
	n[8] = texture(frameTexture, texCoord + vec2(w, h)).a;

	float sobel_edge_h = n[2] + (2.0 * n[5]) + n[8] - (n[0] + (2.0 * n[3]) + n[6]);
  	float sobel_edge_v = n[0] + (2.0 * n[1]) + n[2] - (n[6] + (2.0 * n[7]) + n[8]);
	float sobel = sqrt((sobel_edge_h * sobel_edge_h) + (sobel_edge_v * sobel_edge_v));

	outColor = vec4(color.rgb, color.a * sobel);
}"#;

        let vertex_source = r#"
layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;

layout(std140) uniform Uniforms {
    mat4 worldViewProjection;
    vec4 color;
};

out vec2 texCoord;

void main()
{
    texCoord = vertexTexCoord;
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
}"#;

        let program = server.create_program("EdgeDetectShader", vertex_source, fragment_source)?;
        Ok(Self {
            uniform_buffer_binding: program
                .uniform_block_index(&ImmutableString::new("Uniforms"))?,
            frame_texture: program.uniform_location(&ImmutableString::new("frameTexture"))?,
            program,
        })
    }
}

pub struct HighlightRenderPass {
    framebuffer: Box<dyn FrameBuffer>,
    quad: Box<dyn GeometryBuffer>,
    edge_detect_shader: EdgeDetectShader,
    pub scene_handle: Handle<Scene>,
    pub nodes_to_highlight: FxHashSet<Handle<Node>>,
}

impl HighlightRenderPass {
    fn create_frame_buffer(
        server: &dyn GraphicsServer,
        width: usize,
        height: usize,
    ) -> Box<dyn FrameBuffer> {
        let depth_stencil = server
            .create_2d_render_target(PixelKind::D24S8, width, height)
            .unwrap();

        let frame_texture = server
            .create_2d_render_target(PixelKind::RGBA8, width, height)
            .unwrap();

        server
            .create_frame_buffer(
                Some(Attachment {
                    kind: AttachmentKind::DepthStencil,
                    texture: depth_stencil,
                }),
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: frame_texture,
                }],
            )
            .unwrap()
    }

    pub fn new_raw(server: &dyn GraphicsServer, width: usize, height: usize) -> Self {
        Self {
            framebuffer: Self::create_frame_buffer(server, width, height),
            quad: <dyn GeometryBuffer>::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )
            .unwrap(),
            edge_detect_shader: EdgeDetectShader::new(server).unwrap(),
            scene_handle: Default::default(),
            nodes_to_highlight: Default::default(),
        }
    }

    pub fn new(server: &dyn GraphicsServer, width: usize, height: usize) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new_raw(server, width, height)))
    }

    pub fn resize(&mut self, server: &dyn GraphicsServer, width: usize, height: usize) {
        self.framebuffer = Self::create_frame_buffer(server, width, height);
    }
}

impl SceneRenderPass for HighlightRenderPass {
    fn on_ldr_render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        if self.scene_handle != ctx.scene_handle {
            return Ok(Default::default());
        }

        // Draw selected nodes in the temporary frame buffer first.
        {
            let render_pass_name = ImmutableString::new("Forward");

            let observer_info = ObserverInfo {
                observer_position: ctx.camera.global_position(),
                z_near: ctx.camera.projection().z_near(),
                z_far: ctx.camera.projection().z_far(),
                view_matrix: ctx.camera.view_matrix(),
                projection_matrix: ctx.camera.projection_matrix(),
            };

            let mut render_bundle_storage =
                RenderDataBundleStorage::new_empty(observer_info.clone());

            let frustum = ctx.camera.frustum();
            let mut render_context = RenderContext {
                elapsed_time: ctx.elapsed_time,
                observer_info: &observer_info,
                frustum: Some(&frustum),
                storage: &mut render_bundle_storage,
                graph: &ctx.scene.graph,
                render_pass_name: &render_pass_name,
            };

            for &root_node_handle in self.nodes_to_highlight.iter() {
                if ctx.scene.graph.is_valid_handle(root_node_handle) {
                    for (_, node) in ctx.scene.graph.traverse_iter(root_node_handle) {
                        node.collect_render_data(&mut render_context);
                    }
                }
            }

            render_bundle_storage.sort();

            self.framebuffer
                .clear(ctx.viewport, Some(Color::TRANSPARENT), Some(1.0), None);

            stats += render_bundle_storage.render_to_frame_buffer(
                ctx.server,
                ctx.geometry_cache,
                ctx.shader_cache,
                |_| true,
                |_| true,
                BundleRenderContext {
                    texture_cache: ctx.texture_cache,
                    render_pass_name: &render_pass_name,
                    frame_buffer: &mut *self.framebuffer,
                    use_pom: false,
                    light_position: &Default::default(),
                    fallback_resources: ctx.fallback_resources,
                    ambient_light: Default::default(),
                    scene_depth: Some(&ctx.depth_texture),
                    viewport: ctx.viewport,
                    uniform_memory_allocator: ctx.uniform_memory_allocator,
                },
            )?;
        }

        // Render full screen quad with edge detect shader to draw outline of selected objects.
        {
            let frame_matrix = make_viewport_matrix(ctx.viewport);
            let shader = &self.edge_detect_shader;
            let frame_texture = self.framebuffer.color_attachments()[0].texture.clone();
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
                        ResourceBinding::texture(&frame_texture, &shader.frame_texture),
                        ResourceBinding::Buffer {
                            buffer: ctx.uniform_buffer_cache.write(
                                StaticUniformBuffer::<256>::new()
                                    .with(&frame_matrix)
                                    .with(&Color::ORANGE),
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
