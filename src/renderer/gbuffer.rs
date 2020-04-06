use std::{
    rc::Rc,
    cell::RefCell,
};
use crate::{
    renderer::{
        framework::{
            gpu_program::{
                GpuProgram,
                UniformLocation,
                UniformValue,
            },
            framebuffer::{
                FrameBuffer,
                Attachment,
                AttachmentKind,
                CullFace,
                DrawParameters,
                FrameBufferTrait,
            },
            gpu_texture::{
                GpuTextureKind,
                PixelKind,
                GpuTexture,
                Coordinate,
                WrapMode,
            },
            state::State,
        },
        error::RendererError,
        RenderPassStatistics,
        TextureCache,
        GeometryCache,
    },
    scene::{
        node::Node,
        graph::Graph,
        camera::Camera,
        base::AsBase,
    },
    core::{
        scope_profile,
        math::{
            Rect,
            mat4::Mat4,
            frustum::Frustum,
        },
        color::Color,
    },
};

struct GBufferShader {
    program: GpuProgram,
    world_matrix: UniformLocation,
    wvp_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    bone_matrices: UniformLocation,
    diffuse_texture: UniformLocation,
    normal_texture: UniformLocation,
}

impl GBufferShader {
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
            program,
        })
    }
}

pub struct GBuffer {
    framebuffer: FrameBuffer,
    pub final_frame: FrameBuffer,
    shader: GBufferShader,
    bone_matrices: Vec<Mat4>,
    pub width: i32,
    pub height: i32,
}

pub struct GBufferRenderContext<'a, 'b> {
    pub state: &'a mut State,
    pub graph: &'b Graph,
    pub camera: &'b Camera,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub texture_cache: &'a mut TextureCache,
    pub geom_cache: &'a mut GeometryCache,
}

impl GBuffer {
    pub fn new(state: &mut State, width: usize, height: usize) -> Result<Self, RendererError> {
        let mut depth_stencil_texture = GpuTexture::new(state, GpuTextureKind::Rectangle { width, height }, PixelKind::D24S8, None)?;
        depth_stencil_texture.bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let depth_stencil = Rc::new(RefCell::new(depth_stencil_texture));

        let mut diffuse_texture = GpuTexture::new(state, GpuTextureKind::Rectangle { width, height }, PixelKind::RGBA8, None)?;
        diffuse_texture.bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let mut normal_texture = GpuTexture::new(state, GpuTextureKind::Rectangle { width, height }, PixelKind::RGBA8, None)?;
        normal_texture.bind_mut(state, 0)
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
            ])?;

        let frame_texture = GpuTexture::new(state, GpuTextureKind::Rectangle { width, height }, PixelKind::RGBA8, None)?;

        let opt_framebuffer = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil,
            }),
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(frame_texture)),
                }
            ])?;

        Ok(GBuffer {
            framebuffer,
            shader: GBufferShader::new()?,
            bone_matrices: Vec::new(),
            width: width as i32,
            height: height as i32,
            final_frame: opt_framebuffer,
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

    #[must_use]
    pub fn fill(&mut self, args: GBufferRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let GBufferRenderContext {
            state, graph, camera,
            white_dummy, normal_dummy,
            texture_cache, geom_cache
        } = args;

        let frustum = Frustum::from(camera.view_projection_matrix()).unwrap();

        let viewport = Rect::new(0, 0, self.width, self.height);
        self.framebuffer.clear(state, viewport, Some(Color::from_rgba(0, 0, 0, 0)), Some(1.0), Some(0));

        let view_projection = camera.view_projection_matrix();

        for mesh in graph.linear_iter().filter_map(|node| {
            if let Node::Mesh(mesh) = node { Some(mesh) } else { None }
        }) {
            let global_transform = mesh.base().global_transform();

            if !frustum.is_intersects_aabb_transform(&mesh.bounding_box(), &global_transform) {
                continue;
            }

            if !mesh.base().global_visibility() {
                continue;
            }

            for surface in mesh.surfaces().iter() {
                let is_skinned = !surface.bones.is_empty();

                let world = if is_skinned {
                    Mat4::IDENTITY
                } else {
                    mesh.base().global_transform()
                };
                let mvp = view_projection * world;

                let diffuse_texture = if let Some(texture) = surface.get_diffuse_texture() {
                    if let Some(texture) = texture_cache.get(state, texture) {
                        texture
                    } else {
                        white_dummy.clone()
                    }
                } else {
                    white_dummy.clone()
                };

                let normal_texture = if let Some(texture) = surface.get_normal_texture() {
                    if let Some(texture) = texture_cache.get(state, texture) {
                        texture
                    } else {
                        normal_dummy.clone()
                    }
                } else {
                    normal_dummy.clone()
                };

                statistics += self.framebuffer.draw(
                    geom_cache.get(state,&surface.get_data().lock().unwrap()),
                    state,
                    viewport,
                    &self.shader.program,
                    DrawParameters {
                        cull_face: CullFace::Back,
                        culling: true,
                        color_write: Default::default(),
                        depth_write: true,
                        stencil_test: false,
                        depth_test: true,
                        blend: false,
                    },
                    &[
                        (self.shader.diffuse_texture, UniformValue::Sampler {
                            index: 0,
                            texture: diffuse_texture,
                        }),
                        (self.shader.normal_texture, UniformValue::Sampler {
                            index: 1,
                            texture: normal_texture,
                        }),
                        (self.shader.wvp_matrix, UniformValue::Mat4(mvp)),
                        (self.shader.world_matrix, UniformValue::Mat4(world)),
                        (self.shader.use_skeletal_animation, UniformValue::Bool(is_skinned)),
                        (self.shader.bone_matrices, UniformValue::Mat4Array({
                            self.bone_matrices.clear();
                            for bone_handle in surface.bones.iter() {
                                let bone_node = graph.get(*bone_handle);
                                self.bone_matrices.push(
                                    bone_node.base().global_transform() *
                                        bone_node.base().inv_bind_pose_transform());
                            }
                            &self.bone_matrices
                        }))
                    ],
                );
            }
        }

        statistics
    }
}