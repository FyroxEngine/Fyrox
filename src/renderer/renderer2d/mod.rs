//! A renderer responsible for drawing 2D scenes.

use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector4},
        math::Rect,
        pool::Handle,
    },
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, DrawParameters, FrameBuffer},
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{
                GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
            },
            state::PipelineState,
        },
        renderer2d::cache::{GeometryCache, InstanceData, Mesh},
        RenderPassStatistics, TextureCache,
    },
    resource::texture::TextureKind,
    scene2d::{light::Light, node::Node, Scene2d, Scene2dContainer},
    utils::log::{Log, MessageKind},
};
use std::{
    cell::RefCell,
    collections::hash_map::{Entry, HashMap},
    rc::Rc,
};

mod cache;

struct SpriteShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
    light_count: UniformLocation,
    light_color_radius: UniformLocation,
    light_position_direction: UniformLocation,
    light_parameters: UniformLocation,
    ambient_light_color: UniformLocation,
}

impl SpriteShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/sprite_fs.glsl");
        let vertex_source = include_str!("shaders/sprite_vs.glsl");

        let program =
            GpuProgram::from_source(state, "SpriteShader2D", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "viewProjection")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            light_count: program.uniform_location(state, "lightCount")?,
            light_color_radius: program.uniform_location(state, "lightColorRadius")?,
            light_position_direction: program.uniform_location(state, "lightPositionDirection")?,
            light_parameters: program.uniform_location(state, "lightParameters")?,
            ambient_light_color: program.uniform_location(state, "ambientLightColor")?,
            program,
        })
    }
}

struct RenderTarget {
    width: u32,
    height: u32,
    framebuffer: FrameBuffer,
}

impl RenderTarget {
    fn new(state: &mut PipelineState, width: u32, height: u32) -> Result<Self, FrameworkError> {
        let texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle {
                width: width as usize,
                height: height as usize,
            },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;

        let framebuffer = FrameBuffer::new(
            state,
            None,
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: Rc::new(RefCell::new(texture)),
            }],
        )?;

        Ok(Self {
            width,
            height,
            framebuffer,
        })
    }
}

pub(in crate) struct Renderer2d {
    sprite_shader: SpriteShader,
    quad: Mesh,
    geometry_cache: GeometryCache,
    framebuffers: HashMap<Handle<Scene2d>, RenderTarget>,
    batch_storage: BatchStorage,
    instance_data_set: Vec<InstanceData>,
}

#[derive(Default)]
struct BatchStorage {
    batches: Vec<Batch>,
    index_map: HashMap<u64, usize>,
}

impl BatchStorage {
    fn generate_batches(
        &mut self,
        state: &mut PipelineState,
        scene: &Scene2d,
        texture_cache: &mut TextureCache,
        white_dummy: Rc<RefCell<GpuTexture>>,
    ) {
        self.index_map.clear();
        for batch in self.batches.iter_mut() {
            batch.instances.clear();
        }

        let mut batch_index = 0;
        for node in scene.graph.linear_iter() {
            if let Node::Sprite(sprite) = node {
                if !sprite.global_visibility() {
                    continue;
                }

                let texture = sprite.texture().map_or_else(
                    || white_dummy.clone(),
                    |t| {
                        texture_cache
                            .get(state, t)
                            .unwrap_or_else(|| white_dummy.clone())
                    },
                );

                let texture_id = &*texture.borrow() as *const _ as u64;
                let index = *self.index_map.entry(texture_id).or_insert_with(|| {
                    let index = batch_index;
                    batch_index += 1;
                    index as usize
                });

                // Reuse old batches to prevent redundant memory allocations
                let batch = if let Some(batch) = self.batches.get_mut(index) {
                    batch.texture = texture.clone();
                    batch
                } else {
                    self.batches.push(Batch {
                        instances: Default::default(),
                        texture: texture.clone(),
                    });
                    self.batches.last_mut().unwrap()
                };

                batch.instances.push(Instance {
                    gpu_data: InstanceData {
                        color: sprite.color().srgb_to_linear(),
                        world_matrix: sprite.global_transform()
                            * Matrix4::new_scaling(sprite.size()),
                    },
                    bounds: sprite.global_bounds(),
                });
            }
        }
    }
}

struct Instance {
    gpu_data: InstanceData,
    bounds: Rect<f32>,
}

struct Batch {
    instances: Vec<Instance>,
    texture: Rc<RefCell<GpuTexture>>,
}

impl Renderer2d {
    pub(in crate) fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            sprite_shader: SpriteShader::new(state)?,
            quad: Mesh::new_unit_quad(),
            geometry_cache: Default::default(),
            framebuffers: Default::default(),
            batch_storage: Default::default(),
            instance_data_set: Default::default(),
        })
    }

    pub(in crate) fn update(&mut self, dt: f32) {
        self.geometry_cache.update(dt);
    }

    pub(in crate) fn render(
        &mut self,
        state: &mut PipelineState,
        final_buffer: &mut FrameBuffer,
        final_buffer_size: Vector2<f32>,
        scenes: &Scene2dContainer,
        texture_cache: &mut TextureCache,
        white_dummy: Rc<RefCell<GpuTexture>>,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();
        let quad = self.geometry_cache.get(state, &self.quad);

        for (scene_handle, scene) in scenes.pair_iter() {
            let (frame_buffer, frame_size): (&mut FrameBuffer, Vector2<f32>) = if let Some(
                render_target,
            ) =
                scene.render_target.as_ref()
            {
                let kind = render_target.data_ref().kind();
                let (width, height) = match kind {
                    TextureKind::Rectangle { width, height } => (width, height),
                    _ => {
                        Log::write(MessageKind::Error, format!("Attempt to use {:?} as render target, only rectangular render targets are supported!", kind));
                        continue;
                    }
                };

                match self.framebuffers.entry(scene_handle) {
                    Entry::Occupied(entry) => {
                        let render_target = entry.into_mut();
                        if render_target.width != width || render_target.height != height {
                            *render_target = RenderTarget::new(state, width, height)?;
                        }
                        (
                            &mut render_target.framebuffer,
                            Vector2::new(width as f32, height as f32),
                        )
                    }
                    Entry::Vacant(entry) => (
                        &mut entry
                            .insert(RenderTarget::new(state, width, height)?)
                            .framebuffer,
                        Vector2::new(width as f32, height as f32),
                    ),
                }
            } else {
                (final_buffer, final_buffer_size)
            };

            self.batch_storage
                .generate_batches(state, scene, texture_cache, white_dummy.clone());

            for camera in scene.graph.linear_iter().filter_map(|n| {
                if let Node::Camera(c) = n {
                    Some(c)
                } else {
                    None
                }
            }) {
                let view_projection = camera.view_projection_matrix();
                let viewport = camera.viewport_pixels(frame_size);
                let viewport_f32 = Rect::new(
                    viewport.position.x as f32,
                    viewport.position.y as f32,
                    viewport.size.x as f32,
                    viewport.size.y as f32,
                );

                const MAX_LIGHTS: usize = 16;
                let mut light_count = 0;
                let mut light_color_radius = [Vector4::default(); MAX_LIGHTS];
                let mut light_position_direction = [Vector4::default(); MAX_LIGHTS];
                let mut light_parameters = [Vector2::default(); MAX_LIGHTS];

                for light in scene.graph.linear_iter().filter_map(|n| {
                    if let Node::Light(l) = n {
                        Some(l)
                    } else {
                        None
                    }
                }) {
                    if !light.global_visibility() || light_count == MAX_LIGHTS {
                        continue;
                    }

                    let (radius, half_cone_angle_cos, half_hotspot_angle_cos, direction) =
                        match light {
                            Light::Point(point) => (
                                point.radius(),
                                std::f32::consts::PI.cos(),
                                std::f32::consts::PI.cos(),
                                Vector2::new(0.0, 1.0),
                            ),
                            Light::Spot(spot) => (
                                spot.radius(),
                                spot.half_hotspot_cone_angle(),
                                spot.half_full_cone_angle_cos(),
                                spot.up(),
                            ),
                        };

                    let position = light.global_position();

                    if viewport_f32.intersects_circle(position, radius) {
                        let light_num = light_count as usize;
                        let color = light.color().as_frgb();

                        light_position_direction[light_num] =
                            Vector4::new(position.x, position.y, direction.x, direction.y);
                        light_color_radius[light_num] =
                            Vector4::new(color.x, color.y, color.z, radius);
                        light_parameters[light_num] =
                            Vector2::new(half_cone_angle_cos, half_hotspot_angle_cos);

                        light_count += 1;
                    }
                }

                for batch in self.batch_storage.batches.iter() {
                    self.instance_data_set.clear();
                    for instance in batch.instances.iter() {
                        if viewport_f32.intersects(instance.bounds) {
                            self.instance_data_set.push(instance.gpu_data.clone());
                        }
                    }

                    quad.set_buffer_data(state, 1, &self.instance_data_set);

                    let shader = &self.sprite_shader;
                    stats += frame_buffer.draw_instances(
                        batch.instances.len(),
                        quad,
                        state,
                        viewport,
                        &shader.program,
                        &DrawParameters {
                            cull_face: None,
                            color_write: Default::default(),
                            depth_write: false,
                            stencil_test: false,
                            depth_test: false,
                            blend: true,
                        },
                        |mut program_binding| {
                            program_binding
                                .set_matrix4(&shader.wvp_matrix, &view_projection)
                                .set_texture(&shader.diffuse_texture, &batch.texture)
                                .set_i32(&shader.light_count, light_count as i32)
                                .set_vector4_slice(&shader.light_color_radius, &light_color_radius)
                                .set_vector4_slice(
                                    &shader.light_position_direction,
                                    &light_position_direction,
                                )
                                .set_vector2_slice(&shader.light_parameters, &light_parameters)
                                .set_vector3(
                                    &shader.ambient_light_color,
                                    &scene.ambient_light_color.as_frgb(),
                                );
                        },
                    );
                }
            }
        }
        Ok(stats)
    }

    pub fn flush(&mut self) {
        self.geometry_cache.clear();
    }
}
