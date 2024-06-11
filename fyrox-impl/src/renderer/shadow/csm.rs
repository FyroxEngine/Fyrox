use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, Rect},
    },
    renderer::{
        apply_material,
        bundle::{ObserverInfo, RenderDataBundleStorage},
        cache::{geometry::GeometryCache, shader::ShaderCache, texture::TextureCache},
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::{ColorMask, PipelineState},
        },
        storage::MatrixStorageCache,
        MaterialContext, RenderPassStatistics, ShadowMapPrecision, DIRECTIONAL_SHADOW_PASS_NAME,
    },
    scene::{
        camera::Camera,
        graph::Graph,
        light::directional::{DirectionalLight, FrustumSplitOptions, CSM_NUM_CASCADES},
    },
};
use fyrox_core::color::Color;
use fyrox_core::math::Matrix4Ext;
use std::{cell::RefCell, rc::Rc};

pub struct Cascade {
    pub frame_buffer: FrameBuffer,
    pub view_proj_matrix: Matrix4<f32>,
    pub z_far: f32,
}

impl Cascade {
    pub fn new(
        state: &PipelineState,
        size: usize,
        precision: ShadowMapPrecision,
    ) -> Result<Self, FrameworkError> {
        let depth = {
            let mut texture = GpuTexture::new(
                state,
                GpuTextureKind::Rectangle {
                    width: size,
                    height: size,
                },
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
                .set_wrap(Coordinate::S, WrapMode::ClampToEdge);
            texture
        };

        Ok(Self {
            frame_buffer: FrameBuffer::new(
                state,
                Some(Attachment {
                    kind: AttachmentKind::Depth,
                    texture: Rc::new(RefCell::new(depth)),
                }),
                Default::default(),
            )?,
            view_proj_matrix: Default::default(),
            z_far: 0.0,
        })
    }

    pub fn texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.frame_buffer
            .depth_attachment()
            .unwrap()
            .texture
            .clone()
    }
}

pub struct CsmRenderer {
    cascades: [Cascade; CSM_NUM_CASCADES],
    size: usize,
    precision: ShadowMapPrecision,
}

pub(crate) struct CsmRenderContext<'a, 'c> {
    pub frame_size: Vector2<f32>,
    pub state: &'a PipelineState,
    pub graph: &'c Graph,
    pub light: &'c DirectionalLight,
    pub camera: &'c Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub shader_cache: &'a mut ShaderCache,
    pub texture_cache: &'a mut TextureCache,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub black_dummy: Rc<RefCell<GpuTexture>>,
    pub volume_dummy: Rc<RefCell<GpuTexture>>,
    pub matrix_storage: &'a mut MatrixStorageCache,
}

impl CsmRenderer {
    pub fn new(
        state: &PipelineState,
        size: usize,
        precision: ShadowMapPrecision,
    ) -> Result<Self, FrameworkError> {
        Ok(Self {
            precision,
            size,
            cascades: [
                Cascade::new(state, size, precision)?,
                Cascade::new(state, size, precision)?,
                Cascade::new(state, size, precision)?,
            ],
        })
    }

    pub fn precision(&self) -> ShadowMapPrecision {
        self.precision
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn cascades(&self) -> &[Cascade] {
        &self.cascades
    }

    pub(crate) fn render(
        &mut self,
        ctx: CsmRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let CsmRenderContext {
            frame_size,
            state,
            graph,
            light,
            camera,
            geom_cache,
            shader_cache,
            texture_cache,
            normal_dummy,
            white_dummy,
            black_dummy,
            volume_dummy,
            matrix_storage,
        } = ctx;

        let light_direction = -light
            .up_vector()
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(Vector3::y);

        let light_up_vec = light
            .look_vector()
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(Vector3::z);

        let z_values = match light.csm_options.split_options {
            FrustumSplitOptions::Absolute { far_planes } => [
                camera.projection().z_near(),
                far_planes[0],
                far_planes[1],
                far_planes[2],
            ],
            FrustumSplitOptions::Relative { fractions } => [
                camera.projection().z_near(),
                camera.projection().z_far() * fractions[0],
                camera.projection().z_far() * fractions[1],
                camera.projection().z_far() * fractions[2],
            ],
        };

        for i in 0..CSM_NUM_CASCADES {
            let z_near = z_values[i];
            let mut z_far = z_values[i + 1];

            if z_far.eq(&z_near) {
                z_far += 10.0 * f32::EPSILON;
            }

            let projection_matrix = camera
                .projection()
                .clone()
                .with_z_near(z_near)
                .with_z_far(z_far)
                .matrix(frame_size);

            let frustum =
                Frustum::from_view_projection_matrix(projection_matrix * camera.view_matrix())
                    .unwrap_or_default();

            let center = frustum.center();
            let observer_position = center + light_direction;
            let light_view_matrix = Matrix4::look_at_lh(
                &Point3::from(observer_position),
                &Point3::from(center),
                &light_up_vec,
            );

            let mut aabb = AxisAlignedBoundingBox::default();
            for corner in frustum.corners() {
                let light_space_corner = light_view_matrix
                    .transform_point(&Point3::from(corner))
                    .coords;
                aabb.add_point(light_space_corner);
            }

            // Make sure most of the objects outside of the frustum will cast shadows.
            let z_mult = 10.0;
            if aabb.min.z < 0.0 {
                aabb.min.z *= z_mult;
            } else {
                aabb.min.z /= z_mult;
            }
            if aabb.max.z < 0.0 {
                aabb.max.z /= z_mult;
            } else {
                aabb.max.z *= z_mult;
            }

            let cascade_projection_matrix = Matrix4::new_orthographic(
                aabb.min.x, aabb.max.x, aabb.min.y, aabb.max.y, aabb.min.z, aabb.max.z,
            );

            let inv_view = light_view_matrix.try_inverse().unwrap();
            let camera_up = inv_view.up();
            let camera_side = inv_view.side();

            let light_view_projection = cascade_projection_matrix * light_view_matrix;
            self.cascades[i].view_proj_matrix = light_view_projection;
            self.cascades[i].z_far = z_far;

            let viewport = Rect::new(0, 0, self.size as i32, self.size as i32);
            let framebuffer = &mut self.cascades[i].frame_buffer;
            framebuffer.clear(state, viewport, None, Some(1.0), None);

            let bundle_storage = RenderDataBundleStorage::from_graph(
                graph,
                ObserverInfo {
                    observer_position,
                    z_near,
                    z_far,
                    view_matrix: light_view_matrix,
                    projection_matrix: cascade_projection_matrix,
                },
                DIRECTIONAL_SHADOW_PASS_NAME.clone(),
            );

            for bundle in bundle_storage.bundles.iter() {
                let mut material_state = bundle.material.state();
                let Some(material) = material_state.data() else {
                    continue;
                };

                let Some(geometry) = geom_cache.get(state, &bundle.data, bundle.time_to_live)
                else {
                    continue;
                };

                let blend_shapes_storage = bundle
                    .data
                    .data_ref()
                    .blend_shapes_container
                    .as_ref()
                    .and_then(|c| c.blend_shape_storage.clone());

                let Some(render_pass) =
                    shader_cache
                        .get(state, material.shader())
                        .and_then(|shader_set| {
                            shader_set.render_passes.get(&DIRECTIONAL_SHADOW_PASS_NAME)
                        })
                else {
                    continue;
                };

                for instance in bundle.instances.iter() {
                    stats += framebuffer.draw(
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
                                camera_position: &camera.global_position(),
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
        }

        Ok(stats)
    }
}
