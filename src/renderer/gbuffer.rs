use crate::renderer::TextureCache;
use crate::scene::mesh::RenderPath;
use crate::{
    core::{
        algebra::{Matrix4, Vector4},
        color::Color,
        math::Rect,
        scope_profile,
    },
    renderer::{
        batch::{BatchStorage, InstanceData, MatrixStorage, BONE_MATRICES_COUNT},
        error::RendererError,
        framework::{
            framebuffer::{
                Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer, FrameBufferTrait,
            },
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::PipelineState,
        },
        GeometryCache, RenderPassStatistics,
    },
    scene::camera::Camera,
};
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
}

impl InstancedShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/gbuffer_fs_instanced.glsl");
        let vertex_source = include_str!("shaders/gbuffer_vs_instanced.glsl");
        let program =
            GpuProgram::from_source("GBufferInstancedShader", vertex_source, fragment_source)?;
        Ok(Self {
            use_skeletal_animation: program.uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            normal_texture: program.uniform_location("normalTexture")?,
            specular_texture: program.uniform_location("specularTexture")?,
            roughness_texture: program.uniform_location("roughnessTexture")?,
            lightmap_texture: program.uniform_location("lightmapTexture")?,
            matrix_buffer_stride: program.uniform_location("matrixBufferStride")?,
            matrix_storage_size: program.uniform_location("matrixStorageSize")?,
            matrix_storage: program.uniform_location("matrixStorage")?,
            environment_map: program.uniform_location("environmentMap")?,
            camera_position: program.uniform_location("cameraPosition")?,
            view_projection_matrix: program.uniform_location("viewProjectionMatrix")?,
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
}

impl Shader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/gbuffer_fs.glsl");
        let vertex_source = include_str!("shaders/gbuffer_vs.glsl");
        let program = GpuProgram::from_source("GBufferShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_matrix: program.uniform_location("worldMatrix")?,
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.uniform_location("useSkeletalAnimation")?,
            bone_matrices: program.uniform_location("boneMatrices")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            normal_texture: program.uniform_location("normalTexture")?,
            specular_texture: program.uniform_location("specularTexture")?,
            roughness_texture: program.uniform_location("roughnessTexture")?,
            lightmap_texture: program.uniform_location("lightmapTexture")?,
            diffuse_color: program.uniform_location("diffuseColor")?,
            environment_map: program.uniform_location("environmentMap")?,
            camera_position: program.uniform_location("cameraPosition")?,
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
    bone_matrices: Vec<Matrix4<f32>>,
}

pub(in crate) struct GBufferRenderContext<'a, 'b> {
    pub state: &'a mut PipelineState,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub batch_storage: &'a BatchStorage,
    pub texture_cache: &'a mut TextureCache,
    pub environment_dummy: Rc<RefCell<GpuTexture>>,
}

impl GBuffer {
    pub fn new(
        state: &mut PipelineState,
        width: usize,
        height: usize,
    ) -> Result<Self, RendererError> {
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
                texture: depth_stencil.clone(),
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

        let frame_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Linear,
            MagnificationFilter::Linear,
            1,
            None,
        )?;

        let opt_framebuffer = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil,
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: Rc::new(RefCell::new(frame_texture)),
            }],
        )?;

        Ok(Self {
            framebuffer,
            instanced_shader: InstancedShader::new()?,
            shader: Shader::new()?,
            width: width as i32,
            height: height as i32,
            final_frame: opt_framebuffer,
            matrix_storage: MatrixStorage::new(state)?,
            instance_data_set: Default::default(),
            bone_matrices: Default::default(),
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
                Some(texture) => texture_cache.get(state, texture.clone()).unwrap(),
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

                    statistics += self.framebuffer.draw(
                        geometry,
                        state,
                        viewport,
                        &self.shader.program,
                        &params,
                        &[
                            (
                                self.shader.diffuse_texture,
                                UniformValue::Sampler {
                                    index: 0,
                                    texture: batch.diffuse_texture.clone(),
                                },
                            ),
                            (
                                self.shader.normal_texture,
                                UniformValue::Sampler {
                                    index: 1,
                                    texture: batch.normal_texture.clone(),
                                },
                            ),
                            (
                                self.shader.specular_texture,
                                UniformValue::Sampler {
                                    index: 2,
                                    texture: batch.specular_texture.clone(),
                                },
                            ),
                            (
                                self.shader.lightmap_texture,
                                UniformValue::Sampler {
                                    index: 3,
                                    texture: batch.lightmap_texture.clone(),
                                },
                            ),
                            (
                                self.shader.camera_position,
                                UniformValue::Vector3(camera.global_position()),
                            ),
                            (
                                self.shader.environment_map,
                                UniformValue::Sampler {
                                    index: 4,
                                    texture: environment.clone(),
                                },
                            ),
                            (
                                self.shader.roughness_texture,
                                UniformValue::Sampler {
                                    index: 5,
                                    texture: batch.roughness_texture.clone(),
                                },
                            ),
                            (
                                self.shader.wvp_matrix,
                                UniformValue::Matrix4(view_projection * instance.world_transform),
                            ),
                            (
                                self.shader.world_matrix,
                                UniformValue::Matrix4(instance.world_transform),
                            ),
                            (
                                self.shader.use_skeletal_animation,
                                UniformValue::Bool(batch.is_skinned),
                            ),
                            (
                                self.shader.diffuse_color,
                                UniformValue::Color(instance.color),
                            ),
                            (
                                self.shader.bone_matrices,
                                UniformValue::Mat4Array({
                                    self.bone_matrices.clear();
                                    self.bone_matrices
                                        .extend_from_slice(instance.bone_matrices.as_slice());
                                    &self.bone_matrices
                                }),
                            ),
                        ],
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

                    statistics += self.framebuffer.draw_instances(
                        self.instance_data_set.len(),
                        geometry,
                        state,
                        viewport,
                        &self.instanced_shader.program,
                        &params,
                        &[
                            (
                                self.instanced_shader.diffuse_texture,
                                UniformValue::Sampler {
                                    index: 0,
                                    texture: batch.diffuse_texture.clone(),
                                },
                            ),
                            (
                                self.instanced_shader.normal_texture,
                                UniformValue::Sampler {
                                    index: 1,
                                    texture: batch.normal_texture.clone(),
                                },
                            ),
                            (
                                self.instanced_shader.specular_texture,
                                UniformValue::Sampler {
                                    index: 2,
                                    texture: batch.specular_texture.clone(),
                                },
                            ),
                            (
                                self.instanced_shader.lightmap_texture,
                                UniformValue::Sampler {
                                    index: 3,
                                    texture: batch.lightmap_texture.clone(),
                                },
                            ),
                            (
                                self.instanced_shader.camera_position,
                                UniformValue::Vector3(camera.global_position()),
                            ),
                            (
                                self.instanced_shader.environment_map,
                                UniformValue::Sampler {
                                    index: 4,
                                    texture: environment.clone(),
                                },
                            ),
                            (
                                self.instanced_shader.roughness_texture,
                                UniformValue::Sampler {
                                    index: 5,
                                    texture: batch.roughness_texture.clone(),
                                },
                            ),
                            (
                                self.instanced_shader.matrix_storage,
                                UniformValue::Sampler {
                                    index: 6,
                                    texture: self.matrix_storage.matrices_storage.clone(),
                                },
                            ),
                            (
                                self.instanced_shader.use_skeletal_animation,
                                UniformValue::Bool(batch.is_skinned),
                            ),
                            (
                                self.instanced_shader.matrix_buffer_stride,
                                UniformValue::Integer(BONE_MATRICES_COUNT as i32),
                            ),
                            (
                                self.instanced_shader.matrix_storage_size,
                                UniformValue::Vector4({
                                    let kind = self.matrix_storage.matrices_storage.borrow().kind();
                                    let (w, h) =
                                        if let GpuTextureKind::Rectangle { width, height } = kind {
                                            (width, height)
                                        } else {
                                            unreachable!()
                                        };
                                    Vector4::new(
                                        1.0 / (w as f32),
                                        1.0 / (h as f32),
                                        w as f32,
                                        h as f32,
                                    )
                                }),
                            ),
                            (
                                self.instanced_shader.view_projection_matrix,
                                UniformValue::Matrix4(camera.view_projection_matrix()),
                            ),
                        ],
                    );
                }
            }
        }

        statistics
    }
}
