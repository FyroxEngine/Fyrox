use crate::{
    core::{
        algebra::{Matrix4, Vector3},
        color::Color,
        math::Rect,
        scope_profile,
    },
    renderer::{
        apply_material,
        bundle::{ObserverInfo, RenderDataBundleStorage},
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
        shadow::cascade_size,
        storage::MatrixStorageCache,
        GeometryCache, MaterialContext, RenderPassStatistics, ShadowMapPrecision,
        SPOT_SHADOW_PASS_NAME,
    },
    scene::graph::Graph,
};
use fyrox_core::math::Matrix4Ext;
use std::{cell::RefCell, rc::Rc};

pub struct SpotShadowMapRenderer {
    precision: ShadowMapPrecision,
    // Three "cascades" for various use cases:
    //  0 - largest, for lights close to camera.
    //  1 - medium, for lights with medium distance to camera.
    //  2 - small, for farthest lights.
    cascades: [FrameBuffer; 3],
    size: usize,
}

impl SpotShadowMapRenderer {
    pub fn new(
        state: &PipelineState,
        size: usize,
        precision: ShadowMapPrecision,
    ) -> Result<Self, FrameworkError> {
        fn make_cascade(
            state: &PipelineState,
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
    pub(crate) fn render(
        &mut self,
        state: &PipelineState,
        graph: &Graph,
        light_position: Vector3<f32>,
        light_view_matrix: Matrix4<f32>,
        z_near: f32,
        z_far: f32,
        light_projection_matrix: Matrix4<f32>,
        geom_cache: &mut GeometryCache,
        cascade: usize,
        shader_cache: &mut ShaderCache,
        texture_cache: &mut TextureCache,
        normal_dummy: Rc<RefCell<GpuTexture>>,
        white_dummy: Rc<RefCell<GpuTexture>>,
        black_dummy: Rc<RefCell<GpuTexture>>,
        volume_dummy: Rc<RefCell<GpuTexture>>,
        matrix_storage: &mut MatrixStorageCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let framebuffer = &mut self.cascades[cascade];
        let cascade_size = cascade_size(self.size, cascade);

        let viewport = Rect::new(0, 0, cascade_size as i32, cascade_size as i32);

        framebuffer.clear(state, viewport, None, Some(1.0), None);

        let light_view_projection = light_projection_matrix * light_view_matrix;
        let bundle_storage = RenderDataBundleStorage::from_graph(
            graph,
            ObserverInfo {
                observer_position: light_position,
                z_near,
                z_far,
                view_matrix: light_view_matrix,
                projection_matrix: light_projection_matrix,
            },
            SPOT_SHADOW_PASS_NAME.clone(),
        );

        let inv_view = light_view_matrix.try_inverse().unwrap();
        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        for bundle in bundle_storage.bundles.iter() {
            let mut material_state = bundle.material.state();
            let Some(material) = material_state.data() else {
                continue;
            };

            let Some(geometry) = geom_cache.get(state, &bundle.data, bundle.time_to_live) else {
                continue;
            };

            let blend_shapes_storage = bundle
                .data
                .data_ref()
                .blend_shapes_container
                .as_ref()
                .and_then(|c| c.blend_shape_storage.clone());

            let Some(render_pass) = shader_cache
                .get(state, material.shader())
                .and_then(|shader_set| shader_set.render_passes.get(&SPOT_SHADOW_PASS_NAME))
            else {
                continue;
            };

            for instance in bundle.instances.iter() {
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
                    instance.element_range,
                    |mut program_binding| {
                        apply_material(MaterialContext {
                            material,
                            program_binding: &mut program_binding,
                            texture_cache,
                            matrix_storage,
                            world_matrix: &instance.world_transform,
                            view_projection_matrix: &light_view_projection,
                            wvp_matrix: &(light_view_projection * instance.world_transform),
                            bone_matrices: &instance.bone_matrices,
                            use_skeletal_animation: bundle.is_skinned,
                            camera_position: &Default::default(),
                            camera_up_vector: &camera_up,
                            camera_side_vector: &camera_side,
                            z_near,
                            use_pom: false,
                            light_position: &Default::default(),
                            blend_shapes_storage: blend_shapes_storage.as_ref(),
                            blend_shapes_weights: &instance.blend_shapes_weights,
                            normal_dummy: &normal_dummy,
                            white_dummy: &white_dummy,
                            black_dummy: &black_dummy,
                            volume_dummy: &volume_dummy,
                            persistent_identifier: instance.persistent_identifier,
                            light_data: None,            // TODO
                            ambient_light: Color::WHITE, // TODO
                            scene_depth: None,
                            z_far,
                        });
                    },
                )?;
            }
        }

        Ok(statistics)
    }
}
