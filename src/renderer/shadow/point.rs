use crate::renderer::shadow::cascade_size;
use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector3},
        color::Color,
        math::{frustum::Frustum, Rect},
        scope_profile,
    },
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer},
        gpu_program::{GpuProgram, UniformLocation},
        gpu_texture::{
            Coordinate, CubeMapFace, GpuTexture, GpuTextureKind, MagnificationFilter,
            MinificationFilter, PixelKind, WrapMode,
        },
        state::PipelineState,
    },
    renderer::{batch::BatchStorage, GeometryCache, RenderPassStatistics, ShadowMapPrecision},
    scene::{graph::Graph, node::Node},
};
use std::{cell::RefCell, rc::Rc};

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
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/point_shadow_map_fs.glsl");
        let vertex_source = include_str!("../shaders/point_shadow_map_vs.glsl");
        let program = GpuProgram::from_source(
            state,
            "PointShadowMapShader",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            world_matrix: program.uniform_location(state, "worldMatrix")?,
            bone_matrices: program.uniform_location(state, "boneMatrices")?,
            world_view_projection_matrix: program.uniform_location(state, "worldViewProjection")?,
            use_skeletal_animation: program.uniform_location(state, "useSkeletalAnimation")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            light_position: program.uniform_location(state, "lightPosition")?,
            program,
        })
    }
}

pub struct PointShadowMapRenderer {
    precision: ShadowMapPrecision,
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
    ) -> Result<Self, FrameworkError> {
        fn make_cascade(
            state: &mut PipelineState,
            size: usize,
            precision: ShadowMapPrecision,
        ) -> Result<FrameBuffer, FrameworkError> {
            let depth = {
                let kind = GpuTextureKind::Rectangle {
                    width: size,
                    height: size,
                };
                let mut texture = GpuTexture::new(
                    state,
                    kind,
                    match precision {
                        ShadowMapPrecision::Full => PixelKind::D32F,
                        ShadowMapPrecision::Half => PixelKind::D16,
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
                    MinificationFilter::Linear,
                    MagnificationFilter::Linear,
                    1,
                    None,
                )?;
                texture
                    .bind_mut(state, 0)
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
        }

        Ok(Self {
            precision,
            cascades: [
                make_cascade(state, cascade_size(size, 0), precision)?,
                make_cascade(state, cascade_size(size, 1), precision)?,
                make_cascade(state, cascade_size(size, 2), precision)?,
            ],
            size,
            shader: PointShadowMapShader::new(state)?,
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

            let frustum = Frustum::from(light_view_projection_matrix).unwrap_or_default();

            for batch in batch_storage.batches.iter() {
                let geometry = geom_cache.get(state, &batch.data.read().unwrap());

                for instance in batch.instances.iter() {
                    let node = &graph[instance.owner];

                    let visible = node.global_visibility() && {
                        match node {
                            Node::Mesh(mesh) => {
                                mesh.cast_shadows() && mesh.is_intersect_frustum(graph, &frustum)
                            }
                            Node::Terrain(_) => {
                                // https://github.com/rg3dengine/rg3d/issues/117
                                true
                            }
                            _ => false,
                        }
                    };

                    if visible {
                        let shader = &self.shader;
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
                            |program_binding| {
                                program_binding
                                    .set_vector3(&shader.light_position, &light_pos)
                                    .set_matrix4(&shader.world_matrix, &instance.world_transform)
                                    .set_matrix4(
                                        &shader.world_view_projection_matrix,
                                        &(light_view_projection_matrix * instance.world_transform),
                                    )
                                    .set_bool(&shader.use_skeletal_animation, batch.is_skinned)
                                    .set_matrix4_array(
                                        &shader.bone_matrices,
                                        instance.bone_matrices.as_slice(),
                                    )
                                    .set_texture(&shader.diffuse_texture, &batch.diffuse_texture);
                            },
                        );
                    }
                }
            }
        }

        statistics
    }
}
