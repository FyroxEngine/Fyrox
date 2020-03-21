use std::{
    cell::RefCell,
    rc::Rc,
};
use crate::{
    renderer::{
        gpu_texture::{
            GpuTextureKind,
            PixelKind,
            MagnificationFilter,
            MininificationFilter,
            Coordinate,
            WrapMode,
            CubeMapFace,
            GpuTexture,
        },
        framebuffer::{
            FrameBuffer,
            Attachment,
            AttachmentKind,
        },
        gpu_program::{
            GpuProgram,
            UniformLocation,
        },
        TextureCache,
        GeometryCache,
        RenderPassStatistics,
        error::RendererError,
    },
    scene::{
        node::Node,
        graph::Graph,
        base::AsBase,
    },
    core::{
        math::{
            mat4::Mat4,
            vec3::Vec3,
            frustum::Frustum,
            Rect,
        },
        color::Color,
    },
};
use crate::renderer::framebuffer::{DrawParameters, CullFace, FrameBufferTrait};
use crate::renderer::gpu_program::UniformValue;
use crate::renderer::state::State;

struct SpotShadowMapShader {
    program: GpuProgram,
    bone_matrices: UniformLocation,
    world_view_projection_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    diffuse_texture: UniformLocation,
}

impl SpotShadowMapShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/spot_shadow_map_fs.glsl");
        let vertex_source = include_str!("shaders/spot_shadow_map_vs.glsl");
        let mut program = GpuProgram::from_source("SpotShadowMapShader", vertex_source, fragment_source)?;
        Ok(Self {
            bone_matrices: program.get_uniform_location("boneMatrices")?,
            world_view_projection_matrix: program.get_uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.get_uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,

            program,
        })
    }
}

pub struct SpotShadowMapRenderer {
    shader: SpotShadowMapShader,
    framebuffer: FrameBuffer,
    bone_matrices: Vec<Mat4>,
    pub size: usize,
}

impl SpotShadowMapRenderer {
    pub fn new(state: &mut State, size: usize) -> Result<Self, RendererError> {
        let framebuffer = FrameBuffer::new(
            state,
            Attachment {
                kind: AttachmentKind::Depth,
                texture: Rc::new(RefCell::new({
                    let kind = GpuTextureKind::Rectangle { width: size, height: size };
                    let mut texture = GpuTexture::new(kind, PixelKind::D32, None)?;
                    texture.bind_mut(0)
                        .set_magnification_filter(MagnificationFilter::Linear)
                        .set_minification_filter(MininificationFilter::Linear)
                        .set_wrap(Coordinate::T, WrapMode::ClampToBorder)
                        .set_wrap(Coordinate::S, WrapMode::ClampToBorder)
                        .set_border_color(Color::WHITE);
                    texture
                })),
            },
            vec![])?;

        Ok(Self {
            size,
            framebuffer,
            shader: SpotShadowMapShader::new()?,
            bone_matrices: Vec::new(),
        })
    }

    pub fn texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.depth_attachment().texture.clone()
    }

    pub fn render(&mut self,
                  state: &mut State,
                  graph: &Graph,
                  light_view_projection: &Mat4,
                  white_dummy: Rc<RefCell<GpuTexture>>,
                  textures: &mut TextureCache,
                  geom_map: &mut GeometryCache,
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.size as i32, self.size as i32);

        self.framebuffer.clear(state, viewport, None, Some(1.0), None);
        let frustum = Frustum::from(*light_view_projection).unwrap();

        for node in graph.linear_iter() {
            if let Node::Mesh(mesh) = node {
                if !node.base().global_visibility() {
                    continue;
                }

                let global_transform = node.base().global_transform();

                if !frustum.is_intersects_aabb_transform(&mesh.bounding_box(), &global_transform) {
                    continue;
                }

                for surface in mesh.surfaces().iter() {
                    let is_skinned = !surface.bones.is_empty();

                    let world = if is_skinned {
                        Mat4::IDENTITY
                    } else {
                        global_transform
                    };
                    let mvp = *light_view_projection * world;

                    statistics.add_draw_call(self.framebuffer.draw(
                        state,
                        viewport,
                        geom_map.get(&surface.get_data().lock().unwrap()),
                        &mut self.shader.program,
                        DrawParameters {
                            cull_face: CullFace::Back,
                            culling: true,
                            color_write: (false, false, false, false),
                            depth_write: true,
                            stencil_test: false,
                            depth_test: true,
                            blend: false,
                        },
                        &[
                            (self.shader.world_view_projection_matrix, UniformValue::Mat4(mvp)),
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
                            })),
                            (self.shader.diffuse_texture, UniformValue::Sampler {
                                index: 0,
                                texture: if let Some(texture) = surface.get_diffuse_texture() {
                                    if let Some(texture) = textures.get(texture) {
                                        texture
                                    } else {
                                        white_dummy.clone()
                                    }
                                } else {
                                    white_dummy.clone()
                                },
                            })
                        ],
                    ));
                }
            }
        }

        statistics
    }
}

struct PointShadowMapShader {
    program: GpuProgram,
    world_matrix: UniformLocation,
    bone_matrices: UniformLocation,
    world_view_projection_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    diffuse_texture: UniformLocation,
    light_position: UniformLocation,
}

impl PointShadowMapShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/point_shadow_map_fs.glsl");
        let vertex_source = include_str!("shaders/point_shadow_map_vs.glsl");
        let mut program = GpuProgram::from_source("PointShadowMapShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_matrix: program.get_uniform_location("worldMatrix")?,
            bone_matrices: program.get_uniform_location("boneMatrices")?,
            world_view_projection_matrix: program.get_uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.get_uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            light_position: program.get_uniform_location("lightPosition")?,
            program,
        })
    }
}

pub struct PointShadowMapRenderer {
    bone_matrices: Vec<Mat4>,
    shader: PointShadowMapShader,
    framebuffer: FrameBuffer,
    pub size: usize,
}

struct PointShadowCubeMapFace {
    face: CubeMapFace,
    look: Vec3,
    up: Vec3,
}

impl PointShadowMapRenderer {
    const FACES: [PointShadowCubeMapFace; 6] = [
        PointShadowCubeMapFace {
            face: CubeMapFace::PositiveX,
            look: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
            up: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::NegativeX,
            look: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
            up: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::PositiveY,
            look: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
            up: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::NegativeY,
            look: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
            up: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::PositiveZ,
            look: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
            up: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::NegativeZ,
            look: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
            up: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        },
    ];

    pub fn new(state: &mut State, size: usize) -> Result<PointShadowMapRenderer, RendererError> {
        let framebuffer = FrameBuffer::new(
            state,
            Attachment {
                kind: AttachmentKind::Depth,
                texture: Rc::new(RefCell::new({
                    let kind = GpuTextureKind::Rectangle { width: size, height: size };
                    let mut texture = GpuTexture::new(kind, PixelKind::D32, None)?;
                    texture.bind_mut(0)
                        .set_minification_filter(MininificationFilter::Nearest)
                        .set_magnification_filter(MagnificationFilter::Nearest)
                        .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
                        .set_wrap(Coordinate::T, WrapMode::ClampToEdge);
                    texture
                })),
            },
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new({
                        let kind = GpuTextureKind::Cube { width: size, height: size };
                        let mut texture = GpuTexture::new(kind, PixelKind::F32, None)?;
                        texture.bind_mut(0)
                            .set_minification_filter(MininificationFilter::Linear)
                            .set_magnification_filter(MagnificationFilter::Linear)
                            .set_wrap(Coordinate::S, WrapMode::ClampToBorder)
                            .set_wrap(Coordinate::T, WrapMode::ClampToBorder)
                            .set_border_color(Color::WHITE);
                        texture
                    })),
                }
            ])?;

        Ok(Self {
            framebuffer,
            size,
            bone_matrices: Vec::new(),
            shader: PointShadowMapShader::new()?,
        })
    }

    pub fn texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn render(&mut self,
                  state: &mut State,
                  graph: &Graph,
                  white_dummy: Rc<RefCell<GpuTexture>>,
                  light_pos: Vec3,
                  light_radius: f32,
                  texture_cache: &mut TextureCache,
                  geom_cache: &mut GeometryCache,
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.size as i32, self.size as i32);

        let light_projection_matrix = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.01, light_radius);

        for face in Self::FACES.iter() {
            self.framebuffer
                .set_cubemap_face(state, 0, face.face)
                .clear(state, viewport, Some(Color::WHITE), Some(1.0), None);

            let light_look_at = light_pos + face.look;
            let light_view_matrix = Mat4::look_at(light_pos, light_look_at, face.up).unwrap_or_default();
            let light_view_projection_matrix = light_projection_matrix * light_view_matrix;

            let frustum = Frustum::from(light_view_projection_matrix).unwrap();

            for node in graph.linear_iter() {
                if let Node::Mesh(mesh) = node {
                    if !node.base().global_visibility() {
                        continue;
                    }

                    let global_transform = node.base().global_transform();

                    if !frustum.is_intersects_aabb_transform(&mesh.bounding_box(), &global_transform) {
                        continue;
                    }

                    for surface in mesh.surfaces().iter() {
                        let is_skinned = !surface.bones.is_empty();

                        let world = if is_skinned {
                            Mat4::IDENTITY
                        } else {
                            global_transform
                        };
                        let mvp = light_view_projection_matrix * world;

                        statistics.add_draw_call(self.framebuffer.draw(
                            state,
                            viewport,
                            geom_cache.get(&surface.get_data().lock().unwrap()),
                            &mut self.shader.program,
                            DrawParameters {
                                cull_face: CullFace::Back,
                                culling: true,
                                color_write: (true, true, true, true),
                                depth_write: true,
                                stencil_test: false,
                                depth_test: true,
                                blend: false,
                            },
                            &[
                                (self.shader.light_position, UniformValue::Vec3(light_pos)),
                                (self.shader.world_matrix, UniformValue::Mat4(world)),
                                (self.shader.world_view_projection_matrix, UniformValue::Mat4(mvp)),
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
                                })),
                                (self.shader.diffuse_texture, UniformValue::Sampler {
                                    index: 0,
                                    texture: if let Some(texture) = surface.get_diffuse_texture() {
                                        if let Some(texture) = texture_cache.get(texture) {
                                            texture
                                        } else {
                                            white_dummy.clone()
                                        }
                                    } else {
                                        white_dummy.clone()
                                    },
                                })
                            ],
                        ));
                    }
                }
            }
        }

        statistics
    }
}