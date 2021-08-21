//! GBuffer Layout:
//!
//! RT0: sRGBA8 - Diffuse color.
//! RT1: RGBA8 - Normal (xyz) + specular (w)
//! RT2: RGBA8 - Ambient light + emission
//! RT3: R8UI - Decal mask

use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector4},
        arrayvec::ArrayVec,
        color::Color,
        math::Rect,
        scope_profile,
    },
    renderer::{
        batch::{BatchStorage, InstanceData, MatrixStorage, BONE_MATRICES_COUNT},
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer},
            gpu_program::GpuProgramBinding,
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::PipelineState,
        },
        gbuffer::{
            decal::DecalShader,
            uber_shader::{UberShader, UberShaderFeatures},
        },
        GeometryCache, RenderPassStatistics, TextureCache,
    },
    scene::{
        camera::Camera, graph::Graph, mesh::surface::SurfaceData, mesh::RenderPath, node::Node,
    },
};
use std::{cell::RefCell, rc::Rc};

mod decal;
mod uber_shader;

pub struct GBuffer {
    shaders: ArrayVec<UberShader, 9>,
    framebuffer: FrameBuffer,
    decal_framebuffer: FrameBuffer,
    pub width: i32,
    pub height: i32,
    matrix_storage: MatrixStorage,
    instance_data_set: Vec<InstanceData>,
    cube: SurfaceData,
    decal_shader: DecalShader,
}

pub(in crate) struct GBufferRenderContext<'a, 'b> {
    pub state: &'a mut PipelineState,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub batch_storage: &'a BatchStorage,
    pub texture_cache: &'a mut TextureCache,
    pub environment_dummy: Rc<RefCell<GpuTexture>>,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub use_parallax_mapping: bool,
    pub graph: &'b Graph,
}

impl GBuffer {
    pub fn new(
        state: &mut PipelineState,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
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
            PixelKind::SRGBA8,
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

        let mut shaders = ArrayVec::<UberShader, 9>::new();
        for i in 0..(UberShaderFeatures::COUNT.bits() as usize) {
            shaders.push(UberShader::new(
                state,
                UberShaderFeatures::from_bits(i as u32).unwrap(),
            )?);
        }

        Ok(Self {
            framebuffer,
            shaders,
            width: width as i32,
            height: height as i32,
            matrix_storage: MatrixStorage::new(state)?,
            instance_data_set: Default::default(),
            decal_shader: DecalShader::new(state)?,
            cube: SurfaceData::make_cube(Matrix4::identity()),
            decal_framebuffer,
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

    pub fn decal_mask_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[3].texture.clone()
    }

    #[must_use]
    pub(in crate) fn fill(&mut self, args: GBufferRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let GBufferRenderContext {
            state,
            camera,
            geom_cache,
            batch_storage,
            texture_cache,
            environment_dummy,
            use_parallax_mapping,
            white_dummy,
            normal_dummy,
            graph,
        } = args;

        let viewport = Rect::new(0, 0, self.width, self.height);
        self.framebuffer.clear(
            state,
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            Some(0),
        );

        let mut params = DrawParameters {
            cull_face: CullFace::Back,
            culling: true,
            color_write: Default::default(),
            depth_write: true,
            stencil_test: false,
            depth_test: true,
            blend: false,
        };

        let initial_view_projection = camera.view_projection_matrix();

        for batch in batch_storage
            .batches
            .iter()
            .filter(|b| b.render_path == RenderPath::Deferred)
        {
            let data = batch.data.read().unwrap();
            let geometry = geom_cache.get(state, &data);
            let use_instanced_rendering = batch.instances.len() > 1;

            let environment = match camera.environment_ref() {
                Some(texture) => texture_cache.get(state, texture).unwrap(),
                None => environment_dummy.clone(),
            };

            // Prepare batch info storage in case if we're rendering multiple objects
            // at once.
            if use_instanced_rendering {
                self.matrix_storage.clear();
                self.instance_data_set.clear();
                for instance in batch.instances.iter() {
                    if camera.visibility_cache.is_visible(instance.owner) {
                        self.instance_data_set.push(InstanceData {
                            color: instance.color,
                            world: instance.world_transform,
                            depth_offset: instance.depth_offset,
                        });
                        self.matrix_storage
                            .push_slice(instance.bone_matrices.as_slice());
                    }
                }
                // Every object from batch might be clipped.
                if !self.instance_data_set.is_empty() {
                    self.matrix_storage.update(state);
                    geometry.set_buffer_data(state, 1, self.instance_data_set.as_slice());
                }
            }

            // Select shader
            let mut required_features = UberShaderFeatures::DEFAULT;
            required_features.set(UberShaderFeatures::LIGHTMAP, batch.use_lightmapping);
            required_features.set(UberShaderFeatures::TERRAIN, batch.is_terrain);
            required_features.set(UberShaderFeatures::INSTANCING, use_instanced_rendering);

            let shader = &self.shaders[required_features.bits() as usize];

            let need_render = if use_instanced_rendering {
                !self.instance_data_set.is_empty()
            } else {
                camera
                    .visibility_cache
                    .is_visible(batch.instances.first().unwrap().owner)
            };

            if need_render {
                let matrix_storage = &self.matrix_storage;

                let apply_uniforms = |program_binding: GpuProgramBinding| {
                    let program_binding = program_binding
                        .set_texture(&shader.diffuse_texture, &batch.diffuse_texture)
                        .set_texture(&shader.normal_texture, &batch.normal_texture)
                        .set_texture(&shader.specular_texture, &batch.specular_texture)
                        .set_texture(&shader.environment_map, &environment)
                        .set_texture(&shader.roughness_texture, &batch.roughness_texture)
                        .set_texture(&shader.height_texture, &batch.height_texture)
                        .set_texture(&shader.emission_texture, &batch.emission_texture)
                        .set_vector3(&shader.camera_position, &camera.global_position())
                        .set_bool(&shader.use_pom, batch.use_pom && use_parallax_mapping)
                        .set_bool(&shader.use_skeletal_animation, batch.is_skinned)
                        .set_vector2(&shader.tex_coord_scale, &batch.tex_coord_scale)
                        .set_f32(&shader.emission_strength, batch.emission_strength)
                        .set_u32(&shader.layer_index, batch.decal_layer_index as u32);

                    let program_binding = if batch.use_lightmapping {
                        program_binding.set_texture(
                            shader.lightmap_texture.as_ref().unwrap(),
                            &batch.lightmap_texture,
                        )
                    } else {
                        program_binding
                    };

                    let program_binding = if batch.is_terrain {
                        program_binding
                            .set_texture(shader.mask_texture.as_ref().unwrap(), &batch.mask_texture)
                    } else {
                        program_binding
                    };

                    if use_instanced_rendering {
                        program_binding
                            .set_texture(
                                shader.matrix_storage.as_ref().unwrap(),
                                &matrix_storage.matrices_storage,
                            )
                            .set_i32(
                                shader.matrix_buffer_stride.as_ref().unwrap(),
                                BONE_MATRICES_COUNT as i32,
                            )
                            .set_vector4(shader.matrix_storage_size.as_ref().unwrap(), {
                                let kind = matrix_storage.matrices_storage.borrow().kind();
                                let (w, h) =
                                    if let GpuTextureKind::Rectangle { width, height } = kind {
                                        (width, height)
                                    } else {
                                        unreachable!()
                                    };
                                &Vector4::new(
                                    1.0 / (w as f32),
                                    1.0 / (h as f32),
                                    w as f32,
                                    h as f32,
                                )
                            })
                            .set_matrix4(
                                shader.view_projection_matrix.as_ref().unwrap(),
                                &camera.view_projection_matrix(),
                            );
                    } else {
                        let instance = batch.instances.first().unwrap();

                        let view_projection = if instance.depth_offset != 0.0 {
                            let mut projection = camera.projection_matrix();
                            projection[14] -= instance.depth_offset;
                            projection * camera.view_matrix()
                        } else {
                            initial_view_projection
                        };
                        program_binding
                            .set_linear_color(
                                shader.diffuse_color.as_ref().unwrap(),
                                &instance.color,
                            )
                            .set_matrix4(
                                shader.wvp_matrix.as_ref().unwrap(),
                                &(view_projection * instance.world_transform),
                            )
                            .set_matrix4_array(
                                shader.bone_matrices.as_ref().unwrap(),
                                instance.bone_matrices.as_slice(),
                            )
                            .set_matrix4(
                                shader.world_matrix.as_ref().unwrap(),
                                &instance.world_transform,
                            );
                    }
                };

                params.blend = batch.blend;
                state.set_blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

                statistics += if use_instanced_rendering {
                    self.framebuffer.draw_instances(
                        self.instance_data_set.len(),
                        geometry,
                        state,
                        viewport,
                        &shader.program,
                        &params,
                        apply_uniforms,
                    )
                } else {
                    self.framebuffer.draw(
                        geometry,
                        state,
                        viewport,
                        &shader.program,
                        &params,
                        apply_uniforms,
                    )
                };
            }
        }

        let inv_view_proj = initial_view_projection.try_inverse().unwrap_or_default();
        let depth = self.depth();
        let decal_mask = self.decal_mask_texture();
        let resolution = Vector2::new(self.width as f32, self.height as f32);

        // Render decals after because we need to modify diffuse texture of G-Buffer and use depth texture
        // for rendering. We'll render in the G-Buffer, but depth will be used from final frame, since
        // decals do not modify depth (only diffuse and normal maps).
        let unit_cube = geom_cache.get(state, &self.cube);
        for decal in graph.linear_iter().filter_map(|n| {
            if let Node::Decal(d) = n {
                Some(d)
            } else {
                None
            }
        }) {
            let shader = &self.decal_shader;
            let program = &self.decal_shader.program;

            let diffuse_texture = decal
                .diffuse_texture()
                .and_then(|t| texture_cache.get(state, t))
                .unwrap_or_else(|| white_dummy.clone());

            let normal_texture = decal
                .normal_texture()
                .and_then(|t| texture_cache.get(state, t))
                .unwrap_or_else(|| normal_dummy.clone());

            let world_view_proj = initial_view_projection * decal.global_transform();

            statistics += self.decal_framebuffer.draw(
                unit_cube,
                state,
                viewport,
                program,
                &DrawParameters {
                    cull_face: CullFace::Back,
                    culling: false,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: false,
                    depth_test: false,
                    blend: true,
                },
                |program_binding| {
                    program_binding
                        .set_matrix4(&shader.world_view_projection, &world_view_proj)
                        .set_matrix4(&shader.inv_view_proj, &inv_view_proj)
                        .set_matrix4(
                            &shader.inv_world_decal,
                            &decal.global_transform().try_inverse().unwrap_or_default(),
                        )
                        .set_vector2(&shader.resolution, &resolution)
                        .set_texture(&shader.scene_depth, &depth)
                        .set_texture(&shader.diffuse_texture, &diffuse_texture)
                        .set_texture(&shader.normal_texture, &normal_texture)
                        .set_texture(&shader.decal_mask, &decal_mask)
                        .set_u32(&shader.layer_index, decal.layer() as u32)
                        .set_linear_color(&shader.color, &decal.color());
                },
            );
        }

        statistics
    }
}
