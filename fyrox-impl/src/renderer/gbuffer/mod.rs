//! GBuffer Layout:
//!
//! RT0: sRGBA8 - Diffuse color (xyz)
//! RT1: RGBA8 - Normal (xyz)
//! RT2: RGBA16F - Ambient light + emission (both in xyz)
//! RT3: RGBA8 - Metallic (x) + Roughness (y) + Ambient Occlusion (z)
//! RT4: R8UI - Decal mask (x)
//!
//! Every alpha channel is used for layer blending for terrains. This is inefficient, but for
//! now I don't know better solution.

use crate::{
    core::{
        algebra::{Matrix4, Vector2},
        color::Color,
        math::Rect,
        scope_profile,
        sstorage::ImmutableString,
    },
    renderer::{
        apply_material,
        bundle::RenderDataBundleStorage,
        cache::shader::ShaderCache,
        framework::{
            error::FrameworkError,
            framebuffer::{
                Attachment, AttachmentKind, BlendParameters, DrawParameters, FrameBuffer,
            },
            geometry_buffer::{ElementRange, GeometryBuffer, GeometryBufferKind},
            gpu_program::GpuProgramBinding,
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::{BlendFactor, BlendFunc, PipelineState},
        },
        gbuffer::decal::DecalShader,
        storage::MatrixStorageCache,
        GeometryCache, MaterialContext, RenderPassStatistics, TextureCache,
    },
    scene::{
        camera::Camera,
        decal::Decal,
        graph::Graph,
        mesh::{surface::SurfaceData, RenderPath},
    },
};
use fyrox_core::math::Matrix4Ext;
use std::{cell::RefCell, rc::Rc};

mod decal;

pub struct GBuffer {
    framebuffer: FrameBuffer,
    decal_framebuffer: FrameBuffer,
    pub width: i32,
    pub height: i32,
    cube: GeometryBuffer,
    decal_shader: DecalShader,
    render_pass_name: ImmutableString,
}

pub(crate) struct GBufferRenderContext<'a, 'b> {
    pub state: &'a PipelineState,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub bundle_storage: &'a RenderDataBundleStorage,
    pub texture_cache: &'a mut TextureCache,
    pub shader_cache: &'a mut ShaderCache,
    #[allow(dead_code)]
    pub environment_dummy: Rc<RefCell<GpuTexture>>,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub black_dummy: Rc<RefCell<GpuTexture>>,
    pub volume_dummy: Rc<RefCell<GpuTexture>>,
    pub use_parallax_mapping: bool,
    pub graph: &'b Graph,
    pub matrix_storage: &'a mut MatrixStorageCache,
}

impl GBuffer {
    pub fn new(state: &PipelineState, width: usize, height: usize) -> Result<Self, FrameworkError> {
        scope_profile!();

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

        let mut diffuse_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        diffuse_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);
        let diffuse_texture = Rc::new(RefCell::new(diffuse_texture));

        let mut normal_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        normal_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);
        let normal_texture = Rc::new(RefCell::new(normal_texture));

        let mut ambient_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA16F,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        ambient_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let mut decal_mask_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::R8UI,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        decal_mask_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let mut material_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        material_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let framebuffer = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil,
            }),
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: diffuse_texture.clone(),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: normal_texture.clone(),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(ambient_texture)),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(material_texture)),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(decal_mask_texture)),
                },
            ],
        )?;

        let decal_framebuffer = FrameBuffer::new(
            state,
            None,
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: diffuse_texture,
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: normal_texture,
                },
            ],
        )?;

        Ok(Self {
            framebuffer,
            width: width as i32,
            height: height as i32,
            decal_shader: DecalShader::new(state)?,
            cube: GeometryBuffer::from_surface_data(
                &SurfaceData::make_cube(Matrix4::identity()),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
            decal_framebuffer,
            render_pass_name: ImmutableString::new("GBuffer"),
        })
    }

    pub fn framebuffer(&self) -> &FrameBuffer {
        &self.framebuffer
    }

    pub fn depth(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.depth_attachment().unwrap().texture.clone()
    }

    pub fn diffuse_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn normal_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[1].texture.clone()
    }

    pub fn ambient_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[2].texture.clone()
    }

    pub fn material_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[3].texture.clone()
    }

    pub fn decal_mask_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[4].texture.clone()
    }

    pub(crate) fn fill(
        &mut self,
        args: GBufferRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let GBufferRenderContext {
            state,
            camera,
            geom_cache,
            bundle_storage,
            texture_cache,
            shader_cache,
            use_parallax_mapping,
            white_dummy,
            normal_dummy,
            black_dummy,
            volume_dummy,
            graph,
            matrix_storage,
            ..
        } = args;

        let viewport = Rect::new(0, 0, self.width, self.height);
        self.framebuffer.clear(
            state,
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            Some(0),
        );

        let initial_view_projection = camera.view_projection_matrix();

        let inv_view = camera.inv_view_matrix().unwrap();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        for bundle in bundle_storage
            .bundles
            .iter()
            .filter(|b| b.render_path == RenderPath::Deferred)
        {
            let mut material_state = bundle.material.state();

            let Some(material) = material_state.data() else {
                continue;
            };

            let Some(geometry) = geom_cache.get(state, &bundle.data, bundle.time_to_live) else {
                continue;
            };

            let blend_shapes_storage = bundle
                .data
                .data_ref()
                .blend_shapes_container
                .as_ref()
                .and_then(|c| c.blend_shape_storage.clone());

            let Some(render_pass) = shader_cache
                .get(state, material.shader())
                .and_then(|shader_set| shader_set.render_passes.get(&self.render_pass_name))
            else {
                continue;
            };

            for instance in bundle.instances.iter() {
                let apply_uniforms = |mut program_binding: GpuProgramBinding| {
                    let view_projection = if instance.depth_offset != 0.0 {
                        let mut projection = camera.projection_matrix();
                        projection[14] -= instance.depth_offset;
                        projection * camera.view_matrix()
                    } else {
                        initial_view_projection
                    };

                    apply_material(MaterialContext {
                        material,
                        program_binding: &mut program_binding,
                        texture_cache,
                        matrix_storage,
                        world_matrix: &instance.world_transform,
                        view_projection_matrix: &view_projection,
                        wvp_matrix: &(view_projection * instance.world_transform),
                        bone_matrices: &instance.bone_matrices,
                        use_skeletal_animation: bundle.is_skinned,
                        camera_position: &camera.global_position(),
                        camera_up_vector: &camera_up,
                        camera_side_vector: &camera_side,
                        z_near: camera.projection().z_near(),
                        use_pom: use_parallax_mapping,
                        light_position: &Default::default(),
                        blend_shapes_storage: blend_shapes_storage.as_ref(),
                        blend_shapes_weights: &instance.blend_shapes_weights,
                        normal_dummy: &normal_dummy,
                        white_dummy: &white_dummy,
                        black_dummy: &black_dummy,
                        volume_dummy: &volume_dummy,
                        persistent_identifier: instance.persistent_identifier,
                        light_data: None,
                        ambient_light: Color::WHITE, // TODO
                        scene_depth: None,           // TODO. Add z-pre-pass.
                        z_far: camera.projection().z_far(),
                    });
                };

                statistics += self.framebuffer.draw(
                    geometry,
                    state,
                    viewport,
                    &render_pass.program,
                    &render_pass.draw_params,
                    instance.element_range,
                    apply_uniforms,
                )?;
            }
        }

        let inv_view_proj = initial_view_projection.try_inverse().unwrap_or_default();
        let depth = self.depth();
        let decal_mask = self.decal_mask_texture();
        let resolution = Vector2::new(self.width as f32, self.height as f32);

        // Render decals after because we need to modify diffuse texture of G-Buffer and use depth texture
        // for rendering. We'll render in the G-Buffer, but depth will be used from final frame, since
        // decals do not modify depth (only diffuse and normal maps).
        let unit_cube = &self.cube;
        for decal in graph.linear_iter().filter_map(|n| n.cast::<Decal>()) {
            let shader = &self.decal_shader;
            let program = &self.decal_shader.program;

            let world_view_proj = initial_view_projection * decal.global_transform();

            statistics += self.decal_framebuffer.draw(
                unit_cube,
                state,
                viewport,
                program,
                &DrawParameters {
                    cull_face: None,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: None,
                    depth_test: false,
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                        ..Default::default()
                    }),
                    stencil_op: Default::default(),
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding
                        .set_matrix4(&shader.world_view_projection, &world_view_proj)
                        .set_matrix4(&shader.inv_view_proj, &inv_view_proj)
                        .set_matrix4(
                            &shader.inv_world_decal,
                            &decal.global_transform().try_inverse().unwrap_or_default(),
                        )
                        .set_vector2(&shader.resolution, &resolution)
                        .set_texture(&shader.scene_depth, &depth)
                        .set_texture(
                            &shader.diffuse_texture,
                            decal
                                .diffuse_texture()
                                .and_then(|t| texture_cache.get(state, t))
                                .unwrap_or(&white_dummy),
                        )
                        .set_texture(
                            &shader.normal_texture,
                            decal
                                .normal_texture()
                                .and_then(|t| texture_cache.get(state, t))
                                .unwrap_or(&normal_dummy),
                        )
                        .set_texture(&shader.decal_mask, &decal_mask)
                        .set_u32(&shader.layer_index, decal.layer() as u32)
                        .set_linear_color(&shader.color, &decal.color());
                },
            )?;
        }

        Ok(statistics)
    }
}
