use crate::{
    fyrox::{
        core::{
            algebra::{Matrix4, Vector3},
            color::Color,
            math::Matrix4Ext,
            pool::Handle,
            sstorage::ImmutableString,
        },
        fxhash::FxHashSet,
        graph::{BaseSceneGraph, SceneGraph},
        renderer::{
            apply_material,
            bundle::{RenderContext, RenderDataBundleStorage},
            framework::{
                error::FrameworkError,
                framebuffer::{
                    Attachment, AttachmentKind, BlendParameters, DrawParameters, FrameBuffer,
                },
                geometry_buffer::{ElementRange, GeometryBuffer, GeometryBufferKind},
                gpu_program::{GpuProgram, UniformLocation},
                gpu_texture::{
                    Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter,
                    MinificationFilter, PixelKind, WrapMode,
                },
                state::{BlendFactor, BlendFunc, PipelineState},
            },
            MaterialContext, RenderPassStatistics, SceneRenderPass, SceneRenderPassContext,
        },
        scene::{mesh::surface::SurfaceData, node::Node, Scene},
    },
    Editor,
};
use std::{any::TypeId, cell::RefCell, rc::Rc};

struct EdgeDetectShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    frame_texture: UniformLocation,
    color: UniformLocation,
}

impl EdgeDetectShader {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = r#"
layout (location = 0) out vec4 outColor;

uniform sampler2D frameTexture;
uniform vec4 color;

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

uniform mat4 worldViewProjection;

out vec2 texCoord;

void main()
{
    texCoord = vertexTexCoord;
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
}"#;

        let program =
            GpuProgram::from_source(state, "EdgeDetectShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            frame_texture: program
                .uniform_location(state, &ImmutableString::new("frameTexture"))?,
            color: program.uniform_location(state, &ImmutableString::new("color"))?,
            program,
        })
    }
}

pub struct HighlightRenderPass {
    framebuffer: FrameBuffer,
    quad: GeometryBuffer,
    edge_detect_shader: EdgeDetectShader,
    pub scene_handle: Handle<Scene>,
    pub nodes_to_highlight: FxHashSet<Handle<Node>>,
}

impl HighlightRenderPass {
    fn create_frame_buffer(state: &PipelineState, width: usize, height: usize) -> FrameBuffer {
        let mut depth_stencil_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::D24S8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )
        .unwrap();
        depth_stencil_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let depth_stencil = Rc::new(RefCell::new(depth_stencil_texture));

        let mut frame_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Linear,
            MagnificationFilter::Linear,
            1,
            None,
        )
        .unwrap();
        frame_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil,
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: Rc::new(RefCell::new(frame_texture)),
            }],
        )
        .unwrap()
    }

    pub fn new_raw(state: &PipelineState, width: usize, height: usize) -> Self {
        Self {
            framebuffer: Self::create_frame_buffer(state, width, height),
            quad: GeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                GeometryBufferKind::StaticDraw,
                state,
            )
            .unwrap(),
            edge_detect_shader: EdgeDetectShader::new(state).unwrap(),
            scene_handle: Default::default(),
            nodes_to_highlight: Default::default(),
        }
    }

    pub fn new(state: &PipelineState, width: usize, height: usize) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new_raw(state, width, height)))
    }

    pub fn resize(&mut self, state: &PipelineState, width: usize, height: usize) {
        self.framebuffer = Self::create_frame_buffer(state, width, height);
    }
}

impl SceneRenderPass for HighlightRenderPass {
    fn on_ldr_render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        if self.scene_handle != ctx.scene_handle {
            return Ok(Default::default());
        }

        // Draw selected nodes in the temporary frame buffer first.
        {
            let render_pass_name = ImmutableString::new("Forward");

            let mut render_bundle_storage = RenderDataBundleStorage::default();

            let frustum = ctx.camera.frustum();
            let mut render_context = RenderContext {
                observer_position: &ctx.camera.global_position(),
                z_near: ctx.camera.projection().z_near(),
                z_far: ctx.camera.projection().z_far(),
                view_matrix: &ctx.camera.view_matrix(),
                projection_matrix: &ctx.camera.projection_matrix(),
                frustum: Some(&frustum),
                storage: &mut render_bundle_storage,
                graph: &ctx.scene.graph,
                render_pass_name: &render_pass_name,
            };

            for &root_node_handle in self.nodes_to_highlight.iter() {
                if ctx.scene.graph.is_valid_handle(root_node_handle) {
                    for node_handle in ctx.scene.graph.traverse_handle_iter(root_node_handle) {
                        if let Some(node) = ctx.scene.graph.try_get(node_handle) {
                            node.collect_render_data(&mut render_context);
                        }
                    }
                }
            }

            render_bundle_storage.sort();

            self.framebuffer.clear(
                ctx.pipeline_state,
                ctx.viewport,
                Some(Color::TRANSPARENT),
                Some(1.0),
                None,
            );

            let initial_view_projection = ctx.camera.view_projection_matrix();
            let inv_view = ctx.camera.inv_view_matrix().unwrap();

            let camera_up = inv_view.up();
            let camera_side = inv_view.side();

            for bundle in render_bundle_storage.bundles.iter() {
                let mut material_state = bundle.material.state();

                let Some(material) = material_state.data() else {
                    continue;
                };

                let Some(geometry) =
                    ctx.geometry_cache
                        .get(ctx.pipeline_state, &bundle.data, bundle.time_to_live)
                else {
                    continue;
                };

                let blend_shapes_storage = bundle
                    .data
                    .data_ref()
                    .blend_shapes_container
                    .as_ref()
                    .and_then(|c| c.blend_shape_storage.clone());

                let Some(render_pass) = ctx
                    .shader_cache
                    .get(ctx.pipeline_state, material.shader())
                    .and_then(|shader_set| shader_set.render_passes.get(&render_pass_name))
                else {
                    continue;
                };

                for instance in bundle.instances.iter() {
                    let view_projection = if instance.depth_offset != 0.0 {
                        let mut projection = ctx.camera.projection_matrix();
                        projection[14] -= instance.depth_offset;
                        projection * ctx.camera.view_matrix()
                    } else {
                        initial_view_projection
                    };

                    self.framebuffer.draw(
                        geometry,
                        ctx.pipeline_state,
                        ctx.viewport,
                        &render_pass.program,
                        &render_pass.draw_params,
                        instance.element_range,
                        |mut program_binding| {
                            apply_material(MaterialContext {
                                material,
                                program_binding: &mut program_binding,
                                texture_cache: ctx.texture_cache,
                                world_matrix: &instance.world_transform,
                                view_projection_matrix: &view_projection,
                                wvp_matrix: &(view_projection * instance.world_transform),
                                bone_matrices: &instance.bone_matrices,
                                use_skeletal_animation: bundle.is_skinned,
                                camera_position: &ctx.camera.global_position(),
                                camera_up_vector: &camera_up,
                                camera_side_vector: &camera_side,
                                z_near: ctx.camera.projection().z_near(),
                                z_far: ctx.camera.projection().z_far(),
                                use_pom: false,
                                light_position: &Default::default(),
                                blend_shapes_storage: blend_shapes_storage.as_ref(),
                                blend_shapes_weights: &instance.blend_shapes_weights,
                                normal_dummy: &ctx.normal_dummy,
                                white_dummy: &ctx.white_dummy,
                                black_dummy: &ctx.black_dummy,
                                volume_dummy: &ctx.volume_dummy,
                                matrix_storage: ctx.matrix_storage,
                                persistent_identifier: instance.persistent_identifier,
                                light_data: None,
                                ambient_light: Default::default(),
                                scene_depth: Some(&ctx.depth_texture),
                            });
                        },
                    )?;
                }
            }
        }

        // Render full screen quad with edge detect shader to draw outline of selected objects.
        {
            let frame_matrix = Matrix4::new_orthographic(
                0.0,
                ctx.viewport.w() as f32,
                ctx.viewport.h() as f32,
                0.0,
                -1.0,
                1.0,
            ) * Matrix4::new_nonuniform_scaling(&Vector3::new(
                ctx.viewport.w() as f32,
                ctx.viewport.h() as f32,
                0.0,
            ));
            let shader = &self.edge_detect_shader;
            let frame_texture = self.framebuffer.color_attachments()[0].texture.clone();
            ctx.framebuffer.draw(
                &self.quad,
                ctx.pipeline_state,
                ctx.viewport,
                &shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: None,
                    depth_test: true,
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                        ..Default::default()
                    }),
                    stencil_op: Default::default(),
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding
                        .set_matrix4(&shader.wvp_matrix, &frame_matrix)
                        .set_texture(&shader.frame_texture, &frame_texture)
                        .set_srgb_color(&shader.color, &Color::ORANGE);
                },
            )?;
        }

        Ok(Default::default())
    }

    fn source_type_id(&self) -> TypeId {
        TypeId::of::<Editor>()
    }
}
