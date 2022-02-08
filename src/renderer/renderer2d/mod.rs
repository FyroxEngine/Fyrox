//! A renderer responsible for drawing 2D scenes.

use crate::{
    core::{
        algebra::{Vector2, Vector3, Vector4},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, Rect},
        sstorage::ImmutableString,
    },
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::{DrawParameters, FrameBuffer},
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::GpuTexture,
            state::{BlendFactor, BlendFunc, PipelineState},
        },
        renderer2d::cache::{GeometryCache, InstanceData, Mesh},
        RenderPassStatistics, TextureCache,
    },
    scene::{camera::Camera, graph::Graph, light::Light, node::Node},
    utils::value_as_u8_slice,
};
use fxhash::{FxHashMap, FxHasher};
use std::{cell::RefCell, cmp::Ordering, hash::Hasher, rc::Rc};

mod cache;

struct SpriteShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
    light_count: UniformLocation,
    light_color_radius: UniformLocation,
    light_position: UniformLocation,
    light_direction: UniformLocation,
    light_parameters: UniformLocation,
    ambient_light_color: UniformLocation,
}

impl SpriteShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/sprite_fs.glsl");
        let vertex_source = include_str!("shaders/sprite_vs.glsl");

        let program =
            GpuProgram::from_source(state, "RectangleShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, &ImmutableString::new("viewProjection"))?,
            diffuse_texture: program
                .uniform_location(state, &ImmutableString::new("diffuseTexture"))?,
            light_count: program.uniform_location(state, &ImmutableString::new("lightCount"))?,
            light_color_radius: program
                .uniform_location(state, &ImmutableString::new("lightColorRadius"))?,
            light_direction: program
                .uniform_location(state, &ImmutableString::new("lightDirection"))?,
            light_position: program
                .uniform_location(state, &ImmutableString::new("lightPosition"))?,
            light_parameters: program
                .uniform_location(state, &ImmutableString::new("lightParameters"))?,
            ambient_light_color: program
                .uniform_location(state, &ImmutableString::new("ambientLightColor"))?,
            program,
        })
    }
}

pub(in crate) struct Renderer2d {
    sprite_shader: SpriteShader,
    quad: Mesh,
    geometry_cache: GeometryCache,
    batch_storage: SpriteBatchStorage,
    instance_data_set: Vec<InstanceData>,
}

#[derive(Default)]
struct SpriteBatchStorage {
    batches: Vec<Batch>,
    index_map: FxHashMap<u64, usize>,
}

impl SpriteBatchStorage {
    fn generate_batches(
        &mut self,
        state: &mut PipelineState,
        graph: &Graph,
        texture_cache: &mut TextureCache,
        white_dummy: Rc<RefCell<GpuTexture>>,
    ) {
        self.index_map.clear();
        for batch in self.batches.iter_mut() {
            batch.instances.clear();
        }

        let mut batch_index = 0;
        for node in graph.linear_iter() {
            if let Node::Rectangle(rectangle) = node {
                if !rectangle.global_visibility() {
                    continue;
                }

                let texture = rectangle.texture().map_or_else(
                    || white_dummy.clone(),
                    |t| {
                        texture_cache
                            .get(state, t)
                            .unwrap_or_else(|| white_dummy.clone())
                    },
                );

                let z = rectangle.global_position().z;

                let mut hasher = FxHasher::default();
                // Objects with different Z coordinate will go into separate batches.
                hasher.write(value_as_u8_slice(&z));
                // Objects with different textures will go into separate batches.
                hasher.write_u64(&*texture.borrow() as *const _ as u64);
                let batch_id = hasher.finish();

                let index = *self.index_map.entry(batch_id).or_insert_with(|| {
                    let index = batch_index;
                    batch_index += 1;
                    index as usize
                });

                // Reuse old batches to prevent redundant memory allocations
                let batch = if let Some(batch) = self.batches.get_mut(index) {
                    batch.texture = texture.clone();
                    batch.z = z;
                    batch
                } else {
                    self.batches.push(Batch {
                        instances: Default::default(),
                        texture: texture.clone(),
                        z,
                    });
                    self.batches.last_mut().unwrap()
                };

                batch.instances.push(Instance {
                    gpu_data: InstanceData {
                        color: rectangle.color().srgb_to_linear(),
                        world_matrix: rectangle.global_transform(),
                    },
                    aabb: rectangle.world_bounding_box(),
                });
            }
        }

        // Sort back-to-front for correct blending.
        self.batches.sort_by(|a, b| {
            if a.z < b.z {
                Ordering::Greater
            } else if a.z > b.z {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
    }
}

struct Instance {
    gpu_data: InstanceData,
    aabb: AxisAlignedBoundingBox,
}

struct Batch {
    instances: Vec<Instance>,
    texture: Rc<RefCell<GpuTexture>>,
    z: f32,
}

impl Renderer2d {
    pub(in crate) fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            sprite_shader: SpriteShader::new(state)?,
            quad: Mesh::new_unit_quad(),
            geometry_cache: Default::default(),
            batch_storage: Default::default(),
            instance_data_set: Default::default(),
        })
    }

    pub(in crate) fn update_caches(&mut self, dt: f32) {
        self.geometry_cache.update(dt);
    }

    pub(in crate) fn render(
        &mut self,
        state: &mut PipelineState,
        camera: &Camera,
        frame_buffer: &mut FrameBuffer,
        viewport: Rect<i32>,
        graph: &Graph,
        texture_cache: &mut TextureCache,
        white_dummy: Rc<RefCell<GpuTexture>>,
        ambient_color: Color,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();
        let quad = self.geometry_cache.get(state, &self.quad);

        self.batch_storage
            .generate_batches(state, graph, texture_cache, white_dummy);

        let view_projection = camera.view_projection_matrix();

        let frustum = Frustum::from(camera.view_projection_matrix()).unwrap_or_default();

        const MAX_LIGHTS: usize = 16;
        let mut light_count = 0;
        let mut light_color_radius = [Vector4::default(); MAX_LIGHTS];
        let mut light_position = [Vector3::default(); MAX_LIGHTS];
        let mut light_direction = [Vector3::default(); MAX_LIGHTS];
        let mut light_parameters = [Vector2::default(); MAX_LIGHTS];

        for light in graph.linear_iter().filter_map(|n| {
            if let Node::Light(l) = n {
                Some(l)
            } else {
                None
            }
        }) {
            if !light.global_visibility() || light_count == MAX_LIGHTS {
                continue;
            }

            let (radius, half_cone_angle_cos, half_hotspot_angle_cos) = match light {
                Light::Point(point) => (
                    point.radius(),
                    std::f32::consts::PI.cos(),
                    std::f32::consts::PI.cos(),
                ),
                Light::Spot(spot) => (
                    spot.distance(),
                    (spot.hotspot_cone_angle() * 0.5).cos(),
                    (spot.full_cone_angle() * 0.5).cos(),
                ),
                Light::Directional(_) => (
                    f32::INFINITY,
                    std::f32::consts::PI.cos(),
                    std::f32::consts::PI.cos(),
                ),
            };

            if frustum.is_intersects_aabb(&light.world_bounding_box()) {
                let light_num = light_count as usize;
                let color = light.color().as_frgb();

                light_position[light_num] = light.global_position();
                light_direction[light_num] = light.up_vector();
                light_color_radius[light_num] = Vector4::new(color.x, color.y, color.z, radius);
                light_parameters[light_num] =
                    Vector2::new(half_cone_angle_cos, half_hotspot_angle_cos);

                light_count += 1;
            }
        }

        for batch in self.batch_storage.batches.iter() {
            self.instance_data_set.clear();
            for instance in batch.instances.iter() {
                if frustum.is_intersects_aabb(&instance.aabb) {
                    self.instance_data_set.push(instance.gpu_data.clone());
                }
            }

            quad.set_buffer_data(state, 1, &self.instance_data_set);

            if !self.instance_data_set.is_empty() {
                let shader = &self.sprite_shader;
                stats += frame_buffer.draw_instances(
                    self.instance_data_set.len(),
                    quad,
                    state,
                    viewport,
                    &shader.program,
                    &DrawParameters {
                        cull_face: None,
                        color_write: Default::default(),
                        depth_write: true,
                        stencil_test: None,
                        depth_test: true,
                        blend: Some(BlendFunc {
                            sfactor: BlendFactor::SrcAlpha,
                            dfactor: BlendFactor::OneMinusSrcAlpha,
                        }),
                        stencil_op: Default::default(),
                    },
                    |mut program_binding| {
                        program_binding
                            .set_matrix4(&shader.wvp_matrix, &view_projection)
                            .set_texture(&shader.diffuse_texture, &batch.texture)
                            .set_i32(&shader.light_count, light_count as i32)
                            .set_vector4_slice(&shader.light_color_radius, &light_color_radius)
                            .set_vector3_slice(&shader.light_direction, &light_direction)
                            .set_vector3_slice(&shader.light_position, &light_position)
                            .set_vector2_slice(&shader.light_parameters, &light_parameters)
                            .set_vector3(&shader.ambient_light_color, &ambient_color.as_frgb());
                    },
                );
            }
        }

        Ok(stats)
    }

    pub fn flush(&mut self) {
        self.geometry_cache.clear();
    }
}
