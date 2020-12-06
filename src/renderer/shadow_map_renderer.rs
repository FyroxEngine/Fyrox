#![warn(clippy::too_many_arguments)]

use crate::renderer::ShadowMapPrecision;
use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector3},
        color::Color,
        math::{frustum::Frustum, Rect},
        scope_profile,
    },
    renderer::{
        batch::BatchStorage,
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
            state::{ColorMask, PipelineState},
        },
        GeometryCache, RenderPassStatistics,
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
    precision: ShadowMapPrecision,
    shader: SpotShadowMapShader,
    // Three "cascades" for various use cases:
    //  0 - largest, for lights close to camera.
    //  1 - medium, for lights with medium distance to camera.
    //  2 - small, for farthest lights.
    cascades: [FrameBuffer; 3],
    bone_matrices: Vec<Matrix4<f32>>,
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
    pub fn new(
        state: &mut PipelineState,
        size: usize,
        precision: ShadowMapPrecision,
    ) -> Result<Self, RendererError> {
        fn make_cascade(
            state: &mut PipelineState,
            size: usize,
            precision: ShadowMapPrecision,
        ) -> Result<FrameBuffer, RendererError> {
            let depth = {
                let kind = GpuTextureKind::Rectangle {
                    width: size,
                    height: size,
                };
                let mut texture = GpuTexture::new(
                    state,
                    kind,
                    match precision {
                        ShadowMapPrecision::Full => PixelKind::D32,
                        ShadowMapPrecision::Half => PixelKind::D16,
                    },
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
            precision,
            size,
            cascades: [
                make_cascade(state, cascade_size(size, 0), precision)?,
                make_cascade(state, cascade_size(size, 1), precision)?,
                make_cascade(state, cascade_size(size, 2), precision)?,
            ],
            shader: SpotShadowMapShader::new()?,
            bone_matrices: Vec::new(),
        })
    }

    pub fn base_size(&self) -> usize {
        self.size
    }

    pub fn precision(&self) -> ShadowMapPrecision {
        self.precision
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
        state: &mut PipelineState,
        graph: &Graph,
        light_view_projection: &Matrix4<f32>,
        batches: &BatchStorage,
        geom_cache: &mut GeometryCache,
        cascade: usize,
    ) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let framebuffer = &mut self.cascades[cascade];
        let cascade_size = cascade_size(self.size, cascade);

        let viewport = Rect::new(0, 0, cascade_size as i32, cascade_size as i32);

        framebuffer.clear(state, viewport, None, Some(1.0), None);
        let frustum = Frustum::from(*light_view_projection).unwrap();

        for batch in batches.batches.iter() {
            let geometry = geom_cache.get(state, &batch.data.read().unwrap());

            for instance in batch.instances.iter() {
                let node = &graph[instance.owner];

                let visible = node.global_visibility() && {
                    if let Node::Mesh(mesh) = node {
                        mesh.cast_shadows() && mesh.is_intersect_frustum(graph, &frustum)
                    } else {
                        false
                    }
                };

                if visible {
                    statistics += framebuffer.draw(
                        geometry,
                        state,
                        viewport,
                        &self.shader.program,
                        &DrawParameters {
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
                                UniformValue::Matrix4(
                                    light_view_projection * instance.world_transform,
                                ),
                            ),
                            (
                                self.shader.use_skeletal_animation,
                                UniformValue::Bool(batch.is_skinned),
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
                            (
                                self.shader.diffuse_texture,
                                UniformValue::Sampler {
                                    index: 0,
                                    texture: batch.diffuse_texture.clone(),
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
    precision: ShadowMapPrecision,
    bone_matrices: Vec<Matrix4<f32>>,
    shader: PointShadowMapShader,
    cascades: [FrameBuffer; 3],
    size: usize,
    faces: [PointShadowCubeMapFace; 6],
}

struct PointShadowCubeMapFace {
    face: CubeMapFace,
    look: Vector3<f32>,
    up: Vector3<f32>,
}

pub(in crate) struct PointShadowMapRenderContext<'a, 'c> {
    pub state: &'a mut PipelineState,
    pub graph: &'c Graph,
    pub light_pos: Vector3<f32>,
    pub light_radius: f32,
    pub geom_cache: &'a mut GeometryCache,
    pub cascade: usize,
    pub batch_storage: &'a BatchStorage,
}

impl PointShadowMapRenderer {
    pub fn new(
        state: &mut PipelineState,
        size: usize,
        precision: ShadowMapPrecision,
    ) -> Result<Self, RendererError> {
        fn make_cascade(
            state: &mut PipelineState,
            size: usize,
            precision: ShadowMapPrecision,
        ) -> Result<FrameBuffer, RendererError> {
            let depth = {
                let kind = GpuTextureKind::Rectangle {
                    width: size,
                    height: size,
                };
                let mut texture = GpuTexture::new(
                    state,
                    kind,
                    match precision {
                        ShadowMapPrecision::Half => PixelKind::D16,
                        ShadowMapPrecision::Full => PixelKind::D32,
                    },
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
            precision,
            bone_matrices: Default::default(),
            cascades: [
                make_cascade(state, cascade_size(size, 0), precision)?,
                make_cascade(state, cascade_size(size, 1), precision)?,
                make_cascade(state, cascade_size(size, 2), precision)?,
            ],
            size,
            shader: PointShadowMapShader::new()?,
            faces: [
                PointShadowCubeMapFace {
                    face: CubeMapFace::PositiveX,
                    look: Vector3::new(1.0, 0.0, 0.0),
                    up: Vector3::new(0.0, -1.0, 0.0),
                },
                PointShadowCubeMapFace {
                    face: CubeMapFace::NegativeX,
                    look: Vector3::new(-1.0, 0.0, 0.0),
                    up: Vector3::new(0.0, -1.0, 0.0),
                },
                PointShadowCubeMapFace {
                    face: CubeMapFace::PositiveY,
                    look: Vector3::new(0.0, 1.0, 0.0),
                    up: Vector3::new(0.0, 0.0, 1.0),
                },
                PointShadowCubeMapFace {
                    face: CubeMapFace::NegativeY,
                    look: Vector3::new(0.0, -1.0, 0.0),
                    up: Vector3::new(0.0, 0.0, -1.0),
                },
                PointShadowCubeMapFace {
                    face: CubeMapFace::PositiveZ,
                    look: Vector3::new(0.0, 0.0, 1.0),
                    up: Vector3::new(0.0, -1.0, 0.0),
                },
                PointShadowCubeMapFace {
                    face: CubeMapFace::NegativeZ,
                    look: Vector3::new(0.0, 0.0, -1.0),
                    up: Vector3::new(0.0, -1.0, 0.0),
                },
            ],
        })
    }

    pub fn base_size(&self) -> usize {
        self.size
    }

    pub fn precision(&self) -> ShadowMapPrecision {
        self.precision
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
            light_pos,
            light_radius,
            geom_cache,
            cascade,
            batch_storage,
        } = args;

        let framebuffer = &mut self.cascades[cascade];
        let cascade_size = cascade_size(self.size, cascade);

        let viewport = Rect::new(0, 0, cascade_size as i32, cascade_size as i32);

        let light_projection_matrix =
            Matrix4::new_perspective(1.0, std::f32::consts::FRAC_PI_2, 0.01, light_radius);

        for face in self.faces.iter() {
            framebuffer.set_cubemap_face(state, 0, face.face).clear(
                state,
                viewport,
                Some(Color::WHITE),
                Some(1.0),
                None,
            );

            let light_look_at = light_pos + face.look;
            let light_view_matrix = Matrix4::look_at_rh(
                &Point3::from(light_pos),
                &Point3::from(light_look_at),
                &face.up,
            );
            let light_view_projection_matrix = light_projection_matrix * light_view_matrix;

            let frustum = Frustum::from(light_view_projection_matrix).unwrap();

            for batch in batch_storage.batches.iter() {
                let geometry = geom_cache.get(state, &batch.data.read().unwrap());

                for instance in batch.instances.iter() {
                    let node = &graph[instance.owner];

                    let visible = node.global_visibility() && {
                        if let Node::Mesh(mesh) = node {
                            mesh.cast_shadows() && mesh.is_intersect_frustum(graph, &frustum)
                        } else {
                            false
                        }
                    };

                    if visible {
                        statistics += framebuffer.draw(
                            geometry,
                            state,
                            viewport,
                            &self.shader.program,
                            &DrawParameters {
                                cull_face: CullFace::Back,
                                culling: true,
                                color_write: Default::default(),
                                depth_write: true,
                                stencil_test: false,
                                depth_test: true,
                                blend: false,
                            },
                            &[
                                (self.shader.light_position, UniformValue::Vector3(light_pos)),
                                (
                                    self.shader.world_matrix,
                                    UniformValue::Matrix4(instance.world_transform),
                                ),
                                (
                                    self.shader.world_view_projection_matrix,
                                    UniformValue::Matrix4(
                                        light_view_projection_matrix * instance.world_transform,
                                    ),
                                ),
                                (
                                    self.shader.use_skeletal_animation,
                                    UniformValue::Bool(batch.is_skinned),
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
                                (
                                    self.shader.diffuse_texture,
                                    UniformValue::Sampler {
                                        index: 0,
                                        texture: batch.diffuse_texture.clone(),
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
