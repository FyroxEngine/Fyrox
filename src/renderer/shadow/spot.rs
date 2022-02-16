use crate::{
    core::{
        algebra::Matrix4,
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
            framebuffer::{Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::{ColorMask, PipelineState},
        },
        shadow::{cascade_size, should_cast_shadows},
        GeometryCache, MaterialContext, RenderPassStatistics, ShadowMapPrecision,
    },
};
use std::{cell::RefCell, rc::Rc};

pub struct SpotShadowMapRenderer {
    precision: ShadowMapPrecision,
    // Three "cascades" for various use cases:
    //  0 - largest, for lights close to camera.
    //  1 - medium, for lights with medium distance to camera.
    //  2 - small, for farthest lights.
    cascades: [FrameBuffer; 3],
    size: usize,
    render_pass_name: ImmutableString,
}

impl SpotShadowMapRenderer {
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
                    MinificationFilter::Linear,
                    MagnificationFilter::Linear,
                    1,
                    None,
                )?;
                texture
                    .bind_mut(state, 0)
                    .set_wrap(Coordinate::T, WrapMode::ClampToEdge)
                    .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
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
            render_pass_name: ImmutableString::new("SpotShadow"),
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
        light_view_projection: &Matrix4<f32>,
        batches: &BatchStorage,
        geom_cache: &mut GeometryCache,
        cascade: usize,
        shader_cache: &mut ShaderCache,
        texture_cache: &mut TextureCache,
        normal_dummy: Rc<RefCell<GpuTexture>>,
        white_dummy: Rc<RefCell<GpuTexture>>,
        black_dummy: Rc<RefCell<GpuTexture>>,
    ) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let framebuffer = &mut self.cascades[cascade];
        let cascade_size = cascade_size(self.size, cascade);

        let viewport = Rect::new(0, 0, cascade_size as i32, cascade_size as i32);

        framebuffer.clear(state, viewport, None, Some(1.0), None);
        let frustum = Frustum::from(*light_view_projection).unwrap_or_default();

        for batch in batches.batches.iter() {
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
                            &DrawParameters {
                                cull_face: Some(CullFace::Back),
                                color_write: ColorMask::all(false),
                                depth_write: true,
                                stencil_test: None,
                                depth_test: true,
                                blend: None,
                                stencil_op: Default::default(),
                            },
                            |mut program_binding| {
                                apply_material(MaterialContext {
                                    material: &*material,
                                    program_binding: &mut program_binding,
                                    texture_cache,
                                    world_matrix: &instance.world_transform,
                                    wvp_matrix: &(light_view_projection * instance.world_transform),
                                    bone_matrices: &instance.bone_matrices,
                                    use_skeletal_animation: batch.is_skinned,
                                    camera_position: &Default::default(),
                                    use_pom: false,
                                    light_position: &Default::default(),
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

        statistics
    }
}
