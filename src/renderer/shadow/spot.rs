use crate::renderer::shadow::cascade_size;
use crate::{
    core::{
        algebra::Matrix4,
        color::Color,
        math::{frustum::Frustum, Rect},
        scope_profile,
    },
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer},
        gpu_program::{GpuProgram, UniformLocation},
        gpu_texture::{
            Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
            PixelKind, WrapMode,
        },
        state::{ColorMask, PipelineState},
    },
    renderer::{batch::BatchStorage, GeometryCache, RenderPassStatistics, ShadowMapPrecision},
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
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/spot_shadow_map_fs.glsl");
        let vertex_source = include_str!("../shaders/spot_shadow_map_vs.glsl");
        let program =
            GpuProgram::from_source(state, "SpotShadowMapShader", vertex_source, fragment_source)?;
        Ok(Self {
            bone_matrices: program.uniform_location(state, "boneMatrices")?,
            world_view_projection_matrix: program.uniform_location(state, "worldViewProjection")?,
            use_skeletal_animation: program.uniform_location(state, "useSkeletalAnimation")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,

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
    size: usize,
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
            shader: SpotShadowMapShader::new(state)?,
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
        let frustum = Frustum::from(*light_view_projection).unwrap_or_default();

        for batch in batches.batches.iter() {
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
                            color_write: ColorMask::all(false),
                            depth_write: true,
                            stencil_test: false,
                            depth_test: true,
                            blend: false,
                        },
                        |program_binding| {
                            program_binding
                                .set_matrix4(
                                    &shader.world_view_projection_matrix,
                                    &(light_view_projection * instance.world_transform),
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

        statistics
    }
}
