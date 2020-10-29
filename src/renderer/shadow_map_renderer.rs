#![warn(clippy::too_many_arguments)]

use crate::{
    core::{
        color::Color,
        math::{frustum::Frustum, mat4::Mat4, vec3::Vec3, Rect},
        scope_profile,
    },
    renderer::{
        error::RendererError,
        framework::{
            framebuffer::{
                Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer, FrameBufferTrait,
            },
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::{
                Coordinate, CubeMapFace, GpuTexture, GpuTextureKind, MagnificationFilter,
                MinificationFilter, PixelKind, WrapMode,
            },
            state::{ColorMask, State},
        },
        GeometryCache, RenderPassStatistics, TextureCache,
    },
    scene::{graph::Graph, node::Node},
};
use std::{cell::RefCell, rc::Rc};

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
        let program =
            GpuProgram::from_source("SpotShadowMapShader", vertex_source, fragment_source)?;
        Ok(Self {
            bone_matrices: program.uniform_location("boneMatrices")?,
            world_view_projection_matrix: program.uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,

            program,
        })
    }
}

pub struct SpotShadowMapRenderer {
    shader: SpotShadowMapShader,
    // Three "cascades" for various use cases:
    //  0 - largest, for lights close to camera.
    //  1 - medium, for lights with medium distance to camera.
    //  2 - small, for farthest lights.
    cascades: [FrameBuffer; 3],
    bone_matrices: Vec<Mat4>,
    size: usize,
}

fn cascade_size(base_size: usize, cascade: usize) -> usize {
    match cascade {
        0 => base_size,
        1 => (base_size / 2).max(1),
        2 => (base_size / 4).max(1),
        _ => unreachable!(),
    }
}

impl SpotShadowMapRenderer {
    pub fn new(state: &mut State, size: usize) -> Result<Self, RendererError> {
        fn make_cascade(state: &mut State, size: usize) -> Result<FrameBuffer, RendererError> {
            let depth = {
                let kind = GpuTextureKind::Rectangle {
                    width: size,
                    height: size,
                };
                let mut texture = GpuTexture::new(
                    state,
                    kind,
                    PixelKind::D16,
                    MinificationFilter::Nearest,
                    MagnificationFilter::Nearest,
                    1,
                    None,
                )?;
                texture
                    .bind_mut(state, 0)
                    .set_magnification_filter(MagnificationFilter::Linear)
                    .set_minification_filter(MinificationFilter::Linear)
                    .set_wrap(Coordinate::T, WrapMode::ClampToBorder)
                    .set_wrap(Coordinate::S, WrapMode::ClampToBorder)
                    .set_border_color(Color::WHITE);
                texture
            };

            FrameBuffer::new(
                state,
                Some(Attachment {
                    kind: AttachmentKind::Depth,
                    texture: Rc::new(RefCell::new(depth)),
                }),
                vec![],
            )
        }

        Ok(Self {
            size,
            cascades: [
                make_cascade(state, cascade_size(size, 0))?,
                make_cascade(state, cascade_size(size, 1))?,
                make_cascade(state, cascade_size(size, 2))?,
            ],
            shader: SpotShadowMapShader::new()?,
            bone_matrices: Vec::new(),
        })
    }

    pub fn base_size(&self) -> usize {
        self.size
    }

    pub fn cascade_texture(&self, cascade: usize) -> Rc<RefCell<GpuTexture>> {
        self.cascades[cascade]
            .depth_attachment()
            .unwrap()
            .texture
            .clone()
    }

    pub fn cascade_size(&self, cascade: usize) -> usize {
        cascade_size(self.size, cascade)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate) fn render(
        &mut self,
        state: &mut State,
        graph: &Graph,
        light_view_projection: &Mat4,
        white_dummy: Rc<RefCell<GpuTexture>>,
        textures: &mut TextureCache,
        geom_map: &mut GeometryCache,
        cascade: usize,
    ) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let framebuffer = &mut self.cascades[cascade];
        let cascade_size = cascade_size(self.size, cascade);

        let viewport = Rect::new(0, 0, cascade_size as i32, cascade_size as i32);

        framebuffer.clear(state, viewport, None, Some(1.0), None);
        let frustum = Frustum::from(*light_view_projection).unwrap();

        for node in graph.linear_iter() {
            if let Node::Mesh(mesh) = node {
                if !node.global_visibility() {
                    continue;
                }

                let global_transform = node.global_transform();

                if !mesh.is_intersect_frustum(graph, &frustum) {
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

                    let diffuse_texture = if let Some(texture) = surface.diffuse_texture() {
                        if let Some(texture) = textures.get(state, texture) {
                            texture
                        } else {
                            white_dummy.clone()
                        }
                    } else {
                        white_dummy.clone()
                    };

                    statistics += framebuffer.draw(
                        geom_map.get(state, &surface.data().lock().unwrap()),
                        state,
                        viewport,
                        &self.shader.program,
                        DrawParameters {
                            cull_face: CullFace::Back,
                            culling: true,
                            color_write: ColorMask::all(false),
                            depth_write: true,
                            stencil_test: false,
                            depth_test: true,
                            blend: false,
                        },
                        &[
                            (
                                self.shader.world_view_projection_matrix,
                                UniformValue::Mat4(mvp),
                            ),
                            (
                                self.shader.use_skeletal_animation,
                                UniformValue::Bool(is_skinned),
                            ),
                            (
                                self.shader.bone_matrices,
                                UniformValue::Mat4Array({
                                    self.bone_matrices.clear();

                                    for &bone_handle in surface.bones.iter() {
                                        let bone = &graph[bone_handle];
                                        self.bone_matrices.push(
                                            bone.global_transform()
                                                * bone.inv_bind_pose_transform(),
                                        );
                                    }

                                    &self.bone_matrices
                                }),
                            ),
                            (
                                self.shader.diffuse_texture,
                                UniformValue::Sampler {
                                    index: 0,
                                    texture: diffuse_texture,
                                },
                            ),
                        ],
                    );
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
        let program =
            GpuProgram::from_source("PointShadowMapShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_matrix: program.uniform_location("worldMatrix")?,
            bone_matrices: program.uniform_location("boneMatrices")?,
            world_view_projection_matrix: program.uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            light_position: program.uniform_location("lightPosition")?,
            program,
        })
    }
}

pub struct PointShadowMapRenderer {
    bone_matrices: Vec<Mat4>,
    shader: PointShadowMapShader,
    cascades: [FrameBuffer; 3],
    size: usize,
}

struct PointShadowCubeMapFace {
    face: CubeMapFace,
    look: Vec3,
    up: Vec3,
}

pub(in crate) struct PointShadowMapRenderContext<'a, 'c> {
    pub state: &'a mut State,
    pub graph: &'c Graph,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub light_pos: Vec3,
    pub light_radius: f32,
    pub texture_cache: &'a mut TextureCache,
    pub geom_cache: &'a mut GeometryCache,
    pub cascade: usize,
}

impl PointShadowMapRenderer {
    const FACES: [PointShadowCubeMapFace; 6] = [
        PointShadowCubeMapFace {
            face: CubeMapFace::PositiveX,
            look: Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            up: Vec3 {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::NegativeX,
            look: Vec3 {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            up: Vec3 {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::PositiveY,
            look: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            up: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::NegativeY,
            look: Vec3 {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            up: Vec3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::PositiveZ,
            look: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            up: Vec3 {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        },
        PointShadowCubeMapFace {
            face: CubeMapFace::NegativeZ,
            look: Vec3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            up: Vec3 {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        },
    ];

    pub fn new(state: &mut State, size: usize) -> Result<Self, RendererError> {
        fn make_cascade(state: &mut State, size: usize) -> Result<FrameBuffer, RendererError> {
            let depth = {
                let kind = GpuTextureKind::Rectangle {
                    width: size,
                    height: size,
                };
                let mut texture = GpuTexture::new(
                    state,
                    kind,
                    PixelKind::D16,
                    MinificationFilter::Nearest,
                    MagnificationFilter::Nearest,
                    1,
                    None,
                )?;
                texture
                    .bind_mut(state, 0)
                    .set_minification_filter(MinificationFilter::Nearest)
                    .set_magnification_filter(MagnificationFilter::Nearest)
                    .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
                    .set_wrap(Coordinate::T, WrapMode::ClampToEdge);
                texture
            };

            let cube_map = {
                let kind = GpuTextureKind::Cube {
                    width: size,
                    height: size,
                };
                let mut texture = GpuTexture::new(
                    state,
                    kind,
                    PixelKind::F16,
                    MinificationFilter::Nearest,
                    MagnificationFilter::Nearest,
                    1,
                    None,
                )?;
                texture
                    .bind_mut(state, 0)
                    .set_minification_filter(MinificationFilter::Linear)
                    .set_magnification_filter(MagnificationFilter::Linear)
                    .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
                    .set_wrap(Coordinate::T, WrapMode::ClampToEdge)
                    .set_wrap(Coordinate::R, WrapMode::ClampToEdge);
                texture
            };

            FrameBuffer::new(
                state,
                Some(Attachment {
                    kind: AttachmentKind::Depth,
                    texture: Rc::new(RefCell::new(depth)),
                }),
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(cube_map)),
                }],
            )
        };

        Ok(Self {
            cascades: [
                make_cascade(state, cascade_size(size, 0))?,
                make_cascade(state, cascade_size(size, 1))?,
                make_cascade(state, cascade_size(size, 2))?,
            ],
            size,
            bone_matrices: Vec::new(),
            shader: PointShadowMapShader::new()?,
        })
    }

    pub fn base_size(&self) -> usize {
        self.size
    }

    pub fn cascade_texture(&self, cascade: usize) -> Rc<RefCell<GpuTexture>> {
        self.cascades[cascade].color_attachments()[0]
            .texture
            .clone()
    }

    pub(in crate) fn render(&mut self, args: PointShadowMapRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let PointShadowMapRenderContext {
            state,
            graph,
            white_dummy,
            light_pos,
            light_radius,
            texture_cache,
            geom_cache,
            cascade,
        } = args;

        let framebuffer = &mut self.cascades[cascade];
        let cascade_size = cascade_size(self.size, cascade);

        let viewport = Rect::new(0, 0, cascade_size as i32, cascade_size as i32);

        let light_projection_matrix =
            Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.01, light_radius);

        for face in Self::FACES.iter() {
            framebuffer.set_cubemap_face(state, 0, face.face).clear(
                state,
                viewport,
                Some(Color::WHITE),
                Some(1.0),
                None,
            );

            let light_look_at = light_pos + face.look;
            let light_view_matrix =
                Mat4::look_at(light_pos, light_look_at, face.up).unwrap_or_default();
            let light_view_projection_matrix = light_projection_matrix * light_view_matrix;

            let frustum = Frustum::from(light_view_projection_matrix).unwrap();

            for node in graph.linear_iter() {
                if let Node::Mesh(mesh) = node {
                    if !node.global_visibility() {
                        continue;
                    }

                    let global_transform = node.global_transform();

                    if !mesh.is_intersect_frustum(graph, &frustum) {
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

                        let diffuse_texture = surface
                            .diffuse_texture()
                            .and_then(|texture| texture_cache.get(state, texture))
                            .unwrap_or_else(|| white_dummy.clone());

                        statistics += framebuffer.draw(
                            geom_cache.get(state, &surface.data().lock().unwrap()),
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
                                (self.shader.light_position, UniformValue::Vec3(light_pos)),
                                (self.shader.world_matrix, UniformValue::Mat4(world)),
                                (
                                    self.shader.world_view_projection_matrix,
                                    UniformValue::Mat4(mvp),
                                ),
                                (
                                    self.shader.use_skeletal_animation,
                                    UniformValue::Bool(is_skinned),
                                ),
                                (
                                    self.shader.bone_matrices,
                                    UniformValue::Mat4Array({
                                        self.bone_matrices.clear();

                                        for &bone_handle in surface.bones.iter() {
                                            let bone = &graph[bone_handle];
                                            self.bone_matrices.push(
                                                bone.global_transform()
                                                    * bone.inv_bind_pose_transform(),
                                            );
                                        }

                                        &self.bone_matrices
                                    }),
                                ),
                                (
                                    self.shader.diffuse_texture,
                                    UniformValue::Sampler {
                                        index: 0,
                                        texture: diffuse_texture,
                                    },
                                ),
                            ],
                        );
                    }
                }
            }
        }

        statistics
    }
}
