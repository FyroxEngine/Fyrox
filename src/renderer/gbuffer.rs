use crate::{
    core::{algebra::Vector4, color::Color, math::Rect, scope_profile},
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer},
        gpu_program::{GpuProgram, UniformLocation},
        gpu_texture::{
            Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
            PixelKind, WrapMode,
        },
        state::PipelineState,
    },
    renderer::{
        batch::{BatchStorage, InstanceData, MatrixStorage, BONE_MATRICES_COUNT},
        GeometryCache, RenderPassStatistics, TextureCache,
    },
    scene::{camera::Camera, mesh::RenderPath},
};
use glow::HasContext;
use std::{cell::RefCell, rc::Rc};

struct InstancedShader {
    program: GpuProgram,
    use_skeletal_animation: UniformLocation,
    diffuse_texture: UniformLocation,
    normal_texture: UniformLocation,
    specular_texture: UniformLocation,
    roughness_texture: UniformLocation,
    lightmap_texture: UniformLocation,
    matrix_buffer_stride: UniformLocation,
    matrix_storage_size: UniformLocation,
    matrix_storage: UniformLocation,
    environment_map: UniformLocation,
    camera_position: UniformLocation,
    view_projection_matrix: UniformLocation,
    use_pom: UniformLocation,
    height_texture: UniformLocation,
}

impl InstancedShader {
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/gbuffer_fs_instanced.glsl");
        let vertex_source = include_str!("shaders/gbuffer_vs_instanced.glsl");
        let program = GpuProgram::from_source(
            state,
            "GBufferInstancedShader",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            use_skeletal_animation: program.uniform_location(state, "useSkeletalAnimation")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            normal_texture: program.uniform_location(state, "normalTexture")?,
            specular_texture: program.uniform_location(state, "specularTexture")?,
            roughness_texture: program.uniform_location(state, "roughnessTexture")?,
            lightmap_texture: program.uniform_location(state, "lightmapTexture")?,
            matrix_buffer_stride: program.uniform_location(state, "matrixBufferStride")?,
            matrix_storage_size: program.uniform_location(state, "matrixStorageSize")?,
            matrix_storage: program.uniform_location(state, "matrixStorage")?,
            environment_map: program.uniform_location(state, "environmentMap")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
            view_projection_matrix: program.uniform_location(state, "viewProjectionMatrix")?,
            use_pom: program.uniform_location(state, "usePOM")?,
            height_texture: program.uniform_location(state, "heightTexture")?,
            program,
        })
    }
}

struct Shader {
    program: GpuProgram,
    world_matrix: UniformLocation,
    wvp_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    bone_matrices: UniformLocation,
    diffuse_texture: UniformLocation,
    normal_texture: UniformLocation,
    specular_texture: UniformLocation,
    roughness_texture: UniformLocation,
    lightmap_texture: UniformLocation,
    diffuse_color: UniformLocation,
    environment_map: UniformLocation,
    camera_position: UniformLocation,
    use_pom: UniformLocation,
    height_texture: UniformLocation,
}

impl Shader {
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/gbuffer_fs.glsl");
        let vertex_source = include_str!("shaders/gbuffer_vs.glsl");
        let program =
            GpuProgram::from_source(state, "GBufferShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_matrix: program.uniform_location(state, "worldMatrix")?,
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            use_skeletal_animation: program.uniform_location(state, "useSkeletalAnimation")?,
            bone_matrices: program.uniform_location(state, "boneMatrices")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            normal_texture: program.uniform_location(state, "normalTexture")?,
            specular_texture: program.uniform_location(state, "specularTexture")?,
            roughness_texture: program.uniform_location(state, "roughnessTexture")?,
            lightmap_texture: program.uniform_location(state, "lightmapTexture")?,
            diffuse_color: program.uniform_location(state, "diffuseColor")?,
            environment_map: program.uniform_location(state, "environmentMap")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
            use_pom: program.uniform_location(state, "usePOM")?,
            height_texture: program.uniform_location(state, "heightTexture")?,
            program,
        })
    }
}

pub struct GBuffer {
    framebuffer: FrameBuffer,
    pub final_frame: FrameBuffer,
    instanced_shader: InstancedShader,
    shader: Shader,
    pub width: i32,
    pub height: i32,
    matrix_storage: MatrixStorage,
    instance_data_set: Vec<InstanceData>,
}

pub(in crate) struct GBufferRenderContext<'a, 'b> {
    pub state: &'a mut PipelineState,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub batch_storage: &'a BatchStorage,
    pub texture_cache: &'a mut TextureCache,
    pub environment_dummy: Rc<RefCell<GpuTexture>>,
    pub use_parallax_mapping: bool,
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

        let mut ambient_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        ambient_texture
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
                    texture: Rc::new(RefCell::new(diffuse_texture)),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(normal_texture)),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(ambient_texture)),
                },
            ],
        )?;

        let mut final_frame_depth_stencil_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::D24S8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        final_frame_depth_stencil_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let final_frame_depth_stencil = Rc::new(RefCell::new(final_frame_depth_stencil_texture));

        let frame_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Linear,
            MagnificationFilter::Linear,
            1,
            None,
        )?;

        let final_frame = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: final_frame_depth_stencil,
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: Rc::new(RefCell::new(frame_texture)),
            }],
        )?;

        Ok(Self {
            framebuffer,
            instanced_shader: InstancedShader::new(state)?,
            shader: Shader::new(state)?,
            width: width as i32,
            height: height as i32,
            final_frame,
            matrix_storage: MatrixStorage::new(state)?,
            instance_data_set: Default::default(),
        })
    }

    pub fn frame_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.final_frame.color_attachments()[0].texture.clone()
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
        } = args;

        let viewport = Rect::new(0, 0, self.width, self.height);
        self.framebuffer.clear(
            state,
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            Some(0),
        );

        let params = DrawParameters {
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

            let environment = match camera.environment_ref() {
                Some(texture) => texture_cache.get(state, texture).unwrap(),
                None => environment_dummy.clone(),
            };

            if batch.instances.len() == 1 {
                // Draw single instances the usual way, there is no need to spend time to
                // pass additional data via textures on GPU just to draw single instance.

                let instance = batch.instances.first().unwrap();
                if camera.visibility_cache.is_visible(instance.owner) {
                    let view_projection = if instance.depth_offset != 0.0 {
                        let mut projection = camera.projection_matrix();
                        projection[14] -= instance.depth_offset;
                        projection * camera.view_matrix()
                    } else {
                        initial_view_projection
                    };

                    let shader = &self.shader;
                    statistics += self.framebuffer.draw(
                        geometry,
                        state,
                        viewport,
                        &shader.program,
                        &params,
                        |program_binding| {
                            program_binding
                                .set_sampler(&shader.diffuse_texture, 0, &batch.diffuse_texture)
                                .set_sampler(&shader.normal_texture, 1, &batch.normal_texture)
                                .set_sampler(&shader.specular_texture, 2, &batch.specular_texture)
                                .set_sampler(&shader.lightmap_texture, 3, &batch.lightmap_texture)
                                .set_sampler(&shader.environment_map, 4, &environment)
                                .set_sampler(&shader.roughness_texture, 5, &batch.roughness_texture)
                                .set_sampler(&shader.height_texture, 6, &batch.height_texture)
                                .set_vector3(&shader.camera_position, &camera.global_position())
                                .set_bool(&shader.use_pom, batch.use_pom && use_parallax_mapping)
                                .set_matrix4(
                                    &shader.wvp_matrix,
                                    &(view_projection * instance.world_transform),
                                )
                                .set_matrix4(&shader.world_matrix, &instance.world_transform)
                                .set_bool(&shader.use_skeletal_animation, batch.is_skinned)
                                .set_color(&shader.diffuse_color, &instance.color)
                                .set_matrix4_array(
                                    &shader.bone_matrices,
                                    instance.bone_matrices.as_slice(),
                                );
                        },
                    );
                }
            } else {
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

                if !self.instance_data_set.is_empty() {
                    self.matrix_storage.update(state);
                    geometry.set_buffer_data(state, 1, self.instance_data_set.as_slice());

                    let shader = &self.instanced_shader;
                    let matrix_storage = &self.matrix_storage;
                    statistics += self.framebuffer.draw_instances(
                        self.instance_data_set.len(),
                        geometry,
                        state,
                        viewport,
                        &shader.program,
                        &params,
                        |program_binding| {
                            program_binding
                                .set_sampler(&shader.diffuse_texture, 0, &batch.diffuse_texture)
                                .set_sampler(&shader.normal_texture, 1, &batch.normal_texture)
                                .set_sampler(&shader.specular_texture, 2, &batch.specular_texture)
                                .set_sampler(&shader.lightmap_texture, 3, &batch.lightmap_texture)
                                .set_sampler(&shader.environment_map, 4, &environment)
                                .set_sampler(&shader.roughness_texture, 5, &batch.roughness_texture)
                                .set_sampler(
                                    &shader.matrix_storage,
                                    6,
                                    &matrix_storage.matrices_storage,
                                )
                                .set_sampler(&shader.height_texture, 7, &batch.height_texture)
                                .set_vector3(&shader.camera_position, &camera.global_position())
                                .set_bool(&shader.use_pom, batch.use_pom && use_parallax_mapping)
                                .set_bool(&shader.use_skeletal_animation, batch.is_skinned)
                                .set_integer(
                                    &shader.matrix_buffer_stride,
                                    BONE_MATRICES_COUNT as i32,
                                )
                                .set_vector4(&shader.matrix_storage_size, {
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
                                    &shader.view_projection_matrix,
                                    &camera.view_projection_matrix(),
                                );
                        },
                    );
                }
            }
        }

        // Copy depth-stencil from gbuffer to final frame buffer.
        unsafe {
            state
                .gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.framebuffer.id()));
            state
                .gl
                .bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.final_frame.id()));
            state.gl.blit_framebuffer(
                0,
                0,
                self.width,
                self.height,
                0,
                0,
                self.width,
                self.height,
                glow::DEPTH_BUFFER_BIT | glow::STENCIL_BUFFER_BIT,
                glow::NEAREST,
            );
        }

        statistics
    }
}
