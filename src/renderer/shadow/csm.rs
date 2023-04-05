use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, Rect},
        sstorage::ImmutableString,
    },
    renderer::{
        apply_material,
        batch::BatchStorage,
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
        storage::MatrixStorage,
        MaterialContext, RenderPassStatistics, ShadowMapPrecision,
    },
    scene::{
        camera::Camera,
        graph::Graph,
        light::directional::{DirectionalLight, FrustumSplitOptions, CSM_NUM_CASCADES},
        mesh::Mesh,
        terrain::Terrain,
    },
};
use std::{cell::RefCell, rc::Rc};

pub struct Cascade {
    pub frame_buffer: FrameBuffer,
    pub view_proj_matrix: Matrix4<f32>,
    pub z_far: f32,
}

impl Cascade {
    pub fn new(
        state: &mut PipelineState,
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
                MinificationFilter::Linear,
                MagnificationFilter::Linear,
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
    render_pass_name: ImmutableString,
}

pub(crate) struct CsmRenderContext<'a, 'c> {
    pub frame_size: Vector2<f32>,
    pub state: &'a mut PipelineState,
    pub graph: &'c Graph,
    pub light: &'c DirectionalLight,
    pub camera: &'c Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub batch_storage: &'a BatchStorage,
    pub shader_cache: &'a mut ShaderCache,
    pub texture_cache: &'a mut TextureCache,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub black_dummy: Rc<RefCell<GpuTexture>>,
    pub matrix_storage: &'a mut MatrixStorage,
}

impl CsmRenderer {
    pub fn new(
        state: &mut PipelineState,
        size: usize,
        precision: ShadowMapPrecision,
    ) -> Result<Self, FrameworkError> {
        Ok(Self {
            precision,
            size,
            render_pass_name: ImmutableString::new("DirectionalShadow"),
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

    pub(crate) fn render(&mut self, ctx: CsmRenderContext) -> RenderPassStatistics {
        let mut stats = RenderPassStatistics::default();

        let CsmRenderContext {
            frame_size,
            state,
            graph,
            light,
            camera,
            geom_cache,
            batch_storage,
            shader_cache,
            texture_cache,
            normal_dummy,
            white_dummy,
            black_dummy,
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
            let znear = z_values[i];
            let mut zfar = z_values[i + 1];

            if zfar.eq(&znear) {
                zfar += 10.0 * f32::EPSILON;
            }

            let projection_matrix = camera
                .projection()
                .clone()
                .with_z_near(znear)
                .with_z_far(zfar)
                .matrix(frame_size);

            let frustum =
                Frustum::from(projection_matrix * camera.view_matrix()).unwrap_or_default();

            let center = frustum.center();
            let light_view_matrix = Matrix4::look_at_lh(
                &Point3::from(center + light_direction),
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

            let light_view_projection = cascade_projection_matrix * light_view_matrix;
            self.cascades[i].view_proj_matrix = light_view_projection;
            self.cascades[i].z_far = zfar;

            let viewport = Rect::new(0, 0, self.size as i32, self.size as i32);
            let framebuffer = &mut self.cascades[i].frame_buffer;
            framebuffer.clear(state, viewport, None, Some(1.0), None);

            for batch in batch_storage.batches.iter() {
                let material = batch.material.lock();
                let geometry = geom_cache.get(state, &batch.data);

                if let Some(render_pass) = shader_cache
                    .get(state, material.shader())
                    .and_then(|shader_set| shader_set.render_passes.get(&self.render_pass_name))
                {
                    for instance in batch.instances.iter() {
                        let node = &graph[instance.owner];

                        let visible = if let Some(mesh) = node.cast::<Mesh>() {
                            mesh.global_visibility() && mesh.cast_shadows()
                        } else if let Some(terrain) = node.cast::<Terrain>() {
                            terrain.global_visibility() && terrain.cast_shadows()
                        } else {
                            false
                        };

                        if !visible {
                            continue;
                        }

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
                            |mut program_binding| {
                                apply_material(MaterialContext {
                                    material: &material,
                                    program_binding: &mut program_binding,
                                    texture_cache,
                                    matrix_storage,
                                    world_matrix: &instance.world_transform,
                                    wvp_matrix: &(light_view_projection * instance.world_transform),
                                    bone_matrices: &instance.bone_matrices,
                                    use_skeletal_animation: batch.is_skinned,
                                    camera_position: &camera.global_position(),
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

        stats
    }
}
