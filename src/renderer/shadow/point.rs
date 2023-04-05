use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector3},
        color::Color,
        math::{frustum::Frustum, Rect},
        scope_profile,
        sstorage::ImmutableString,
    },
    renderer::{
        apply_material,
        batch::BatchStorage,
        cache::{shader::ShaderCache, texture::TextureCache},
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, FrameBuffer},
            gpu_texture::{
                Coordinate, CubeMapFace, GpuTexture, GpuTextureKind, MagnificationFilter,
                MinificationFilter, PixelKind, WrapMode,
            },
            state::PipelineState,
        },
        shadow::{cascade_size, should_cast_shadows},
        storage::MatrixStorage,
        GeometryCache, MaterialContext, RenderPassStatistics, ShadowMapPrecision,
    },
};
use std::{cell::RefCell, rc::Rc};

pub struct PointShadowMapRenderer {
    precision: ShadowMapPrecision,
    cascades: [FrameBuffer; 3],
    size: usize,
    faces: [PointShadowCubeMapFace; 6],
    render_pass_name: ImmutableString,
}

struct PointShadowCubeMapFace {
    face: CubeMapFace,
    look: Vector3<f32>,
    up: Vector3<f32>,
}

pub(crate) struct PointShadowMapRenderContext<'a> {
    pub state: &'a mut PipelineState,
    pub light_pos: Vector3<f32>,
    pub light_radius: f32,
    pub geom_cache: &'a mut GeometryCache,
    pub cascade: usize,
    pub batch_storage: &'a BatchStorage,
    pub shader_cache: &'a mut ShaderCache,
    pub texture_cache: &'a mut TextureCache,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub black_dummy: Rc<RefCell<GpuTexture>>,
    pub matrix_storage: &'a mut MatrixStorage,
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
            render_pass_name: ImmutableString::new("PointShadow"),
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

    pub(crate) fn render(&mut self, args: PointShadowMapRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let PointShadowMapRenderContext {
            state,
            light_pos,
            light_radius,
            geom_cache,
            cascade,
            batch_storage,
            shader_cache,
            texture_cache,
            normal_dummy,
            white_dummy,
            black_dummy,
            matrix_storage,
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
                let material = batch.material.lock();
                let geometry = geom_cache.get(state, &batch.data);

                if let Some(render_pass) = shader_cache
                    .get(state, material.shader())
                    .and_then(|shader_set| shader_set.render_passes.get(&self.render_pass_name))
                {
                    for instance in batch.instances.iter() {
                        if should_cast_shadows(instance, &frustum) {
                            statistics += framebuffer.draw(
                                geometry,
                                state,
                                viewport,
                                &render_pass.program,
                                &render_pass.draw_params,
                                |mut program_binding| {
                                    apply_material(MaterialContext {
                                        material: &material,
                                        program_binding: &mut program_binding,
                                        texture_cache,
                                        matrix_storage,
                                        world_matrix: &instance.world_transform,
                                        wvp_matrix: &(light_view_projection_matrix
                                            * instance.world_transform),
                                        bone_matrices: &instance.bone_matrices,
                                        use_skeletal_animation: batch.is_skinned,
                                        camera_position: &Default::default(),
                                        use_pom: false,
                                        light_position: &light_pos,
                                        normal_dummy: normal_dummy.clone(),
                                        white_dummy: white_dummy.clone(),
                                        black_dummy: black_dummy.clone(),
                                    });
                                },
                            );
                        }
                    }
                }
            }
        }

        statistics
    }
}
