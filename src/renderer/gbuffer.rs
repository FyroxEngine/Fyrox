use crate::{
    core::{
        algebra::{Matrix4, Vector4},
        color::Color,
        math::Rect,
        scope_profile,
    },
    renderer::{
        error::RendererError,
        framework::{
            framebuffer::{
                Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer, FrameBufferTrait,
            },
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::State,
        },
        surface::SurfaceSharedData,
        GeometryCache, RenderPassStatistics, TextureCache,
    },
    scene::{camera::Camera, graph::Graph, node::Node},
};
use arrayvec::ArrayVec;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Debug, Formatter},
    rc::Rc,
    sync::{Arc, Mutex},
};

struct GBufferShader {
    program: GpuProgram,
    use_skeletal_animation: UniformLocation,
    diffuse_texture: UniformLocation,
    normal_texture: UniformLocation,
    specular_texture: UniformLocation,
    lightmap_texture: UniformLocation,
    matrix_buffer_stride: UniformLocation,
    matrix_storage_size: UniformLocation,
    color_storage_size: UniformLocation,
    matrix_storage: UniformLocation,
    color_storage: UniformLocation,
}

impl GBufferShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/gbuffer_fs.glsl");
        let vertex_source = include_str!("shaders/gbuffer_vs.glsl");
        let program = GpuProgram::from_source("GBufferShader", vertex_source, fragment_source)?;
        Ok(Self {
            use_skeletal_animation: program.uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            normal_texture: program.uniform_location("normalTexture")?,
            specular_texture: program.uniform_location("specularTexture")?,
            lightmap_texture: program.uniform_location("lightmapTexture")?,
            matrix_buffer_stride: program.uniform_location("matrixBufferStride")?,
            matrix_storage_size: program.uniform_location("matrixStorageSize")?,
            color_storage_size: program.uniform_location("colorStorageSize")?,
            color_storage: program.uniform_location("colorStorage")?,
            matrix_storage: program.uniform_location("matrixStorage")?,
            program,
        })
    }
}

struct Storage {
    // Generic storage for instancing, contains all matrices needed for instanced
    // rendering. It has variable size, but it is always multiple of 4. Each pixel
    // has RGBA components as f32 so to store 4x4 matrix we need 4 pixels.
    //
    // Q: Why it uses textures instead of SSBO?
    // A: This could be done with SSBO, but it is not available on macOS because SSBO
    // was added only in OpenGL 4.3, but macOS support up to OpenGL 4.1.
    matrices_storage: Rc<RefCell<GpuTexture>>,
    matrices: Vec<Matrix4<f32>>,
    colors_storage: Rc<RefCell<GpuTexture>>,
    colors: Vec<Color>,
}

impl Storage {
    fn new(state: &mut State) -> Result<Self, RendererError> {
        Ok(Self {
            matrices_storage: Rc::new(RefCell::new(GpuTexture::new(
                state,
                GpuTextureKind::Rectangle {
                    width: 4,
                    height: 1,
                },
                PixelKind::RGBA32F,
                MinificationFilter::Nearest,
                MagnificationFilter::Nearest,
                1,
                None,
            )?)),
            matrices: Default::default(),
            colors_storage: Rc::new(RefCell::new(GpuTexture::new(
                state,
                GpuTextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                PixelKind::RGBA8,
                MinificationFilter::Nearest,
                MagnificationFilter::Nearest,
                1,
                None,
            )?)),
            colors: Default::default(),
        })
    }

    fn update(&mut self, state: &mut State, batch: &Batch) {
        self.matrices.clear();
        self.colors.clear();
        for instance in batch.instances.iter() {
            // Push generic matrices first.
            self.matrices.push(instance.world_transform);
            self.matrices.push(instance.wvp_transform);

            // Push bone matrices if any.
            if !instance.bone_matrices.is_empty() {
                for m in instance.bone_matrices.iter() {
                    self.matrices.push(m.clone());
                }

                // Pad rest with zeros because we can't use tight packing in this case.
                for _ in 0..(batch.matrix_buffer_stride
                    - GENERIC_MATRICES_COUNT
                    - instance.bone_matrices.len())
                {
                    self.matrices.push(Default::default());
                }
            }

            self.colors.push(instance.color);
        }

        // Select width for the texture by restricting width at 1024 pixels.
        let matrices_tex_size = 1024;
        let actual_matrices_pixel_count = self.matrices.len() * 4;
        let matrices_w = actual_matrices_pixel_count.min(matrices_tex_size);
        let matrices_h = (actual_matrices_pixel_count as f32 / matrices_w as f32)
            .ceil()
            .max(1.0) as usize;
        // Pad data to actual size.
        for _ in 0..(((matrices_w * matrices_h) - actual_matrices_pixel_count) / 4) {
            self.matrices.push(Default::default());
        }

        // Upload to GPU.
        self.matrices_storage
            .borrow_mut()
            .bind_mut(state, 0)
            .set_data(
                state,
                GpuTextureKind::Rectangle {
                    width: matrices_w,
                    height: matrices_h,
                },
                PixelKind::RGBA32F,
                1,
                Some(unsafe {
                    std::slice::from_raw_parts(
                        self.matrices.as_slice() as *const _ as *const u8,
                        self.matrices.len() * std::mem::size_of::<Matrix4<f32>>(),
                    )
                }),
            )
            .unwrap();

        // Select width for the texture by restricting width at 1024 pixels.
        let colors_tex_size = 256;
        let actual_colors_pixel_count = self.colors.len();
        let colors_w = actual_colors_pixel_count.min(colors_tex_size);
        let colors_h = (actual_colors_pixel_count as f32 / colors_w as f32)
            .ceil()
            .max(1.0) as usize;
        // Pad data to actual size.
        for _ in 0..((colors_w * colors_h) - actual_colors_pixel_count) {
            self.colors.push(Default::default());
        }
        self.colors_storage
            .borrow_mut()
            .bind_mut(state, 0)
            .set_data(
                state,
                GpuTextureKind::Rectangle {
                    width: colors_w,
                    height: colors_h,
                },
                PixelKind::RGBA8,
                1,
                Some(unsafe {
                    std::slice::from_raw_parts(
                        self.colors.as_slice() as *const _ as *const u8,
                        self.colors.len() * std::mem::size_of::<Color>(),
                    )
                }),
            )
            .unwrap();
    }
}

pub struct GBuffer {
    framebuffer: FrameBuffer,
    pub final_frame: FrameBuffer,
    shader: GBufferShader,
    pub width: i32,
    pub height: i32,
    batches: HashMap<u64, Batch>,
    storage: Storage,
}

pub(in crate) struct GBufferRenderContext<'a, 'b> {
    pub state: &'a mut State,
    pub graph: &'b Graph,
    pub camera: &'b Camera,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub specular_dummy: Rc<RefCell<GpuTexture>>,
    pub texture_cache: &'a mut TextureCache,
    pub geom_cache: &'a mut GeometryCache,
}

const GENERIC_MATRICES_COUNT: usize = 2;
const BONE_MATRICES_COUNT: usize = 62;

struct Instance {
    world_transform: Matrix4<f32>,
    wvp_transform: Matrix4<f32>,
    bone_matrices: ArrayVec<[Matrix4<f32>; BONE_MATRICES_COUNT]>,
    color: Color,
}

struct Batch {
    data: Arc<Mutex<SurfaceSharedData>>,
    instances: Vec<Instance>,
    diffuse_texture: Rc<RefCell<GpuTexture>>,
    normal_texture: Rc<RefCell<GpuTexture>>,
    specular_texture: Rc<RefCell<GpuTexture>>,
    lightmap_texture: Rc<RefCell<GpuTexture>>,
    matrix_buffer_stride: usize,
    skinned: bool,
}

impl Debug for Batch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Batch {}: {} instances",
            &*self.data as *const _ as u64,
            self.instances.len()
        )
    }
}

impl Batch {
    fn clear(&mut self) {
        self.instances.clear();
    }
}

impl GBuffer {
    fn generate_batches(
        &mut self,
        state: &mut State,
        graph: &Graph,
        camera: &Camera,
        white_dummy: Rc<RefCell<GpuTexture>>,
        normal_dummy: Rc<RefCell<GpuTexture>>,
        specular_dummy: Rc<RefCell<GpuTexture>>,
        texture_cache: &mut TextureCache,
    ) {
        for batch in self.batches.values_mut() {
            batch.clear();
        }

        let initial_view_projection = camera.view_projection_matrix();

        for mesh in graph.pair_iter().filter_map(|(handle, node)| {
            if let (Node::Mesh(mesh), true) = (node, camera.visibility_cache.is_visible(handle)) {
                Some(mesh)
            } else {
                None
            }
        }) {
            let view_projection = if mesh.depth_offset_factor() != 0.0 {
                let mut projection = camera.projection_matrix();
                projection[14] -= mesh.depth_offset_factor();
                projection * camera.view_matrix()
            } else {
                initial_view_projection
            };

            for surface in mesh.surfaces().iter() {
                let is_skinned = !surface.bones.is_empty();

                let world = if is_skinned {
                    Matrix4::identity()
                } else {
                    mesh.global_transform()
                };
                let mvp = view_projection * world;

                let diffuse_texture = surface
                    .diffuse_texture()
                    .and_then(|texture| texture_cache.get(state, texture))
                    .unwrap_or_else(|| white_dummy.clone());

                let normal_texture = surface
                    .normal_texture()
                    .and_then(|texture| texture_cache.get(state, texture))
                    .unwrap_or_else(|| normal_dummy.clone());

                let specular_texture = surface
                    .specular_texture()
                    .and_then(|texture| texture_cache.get(state, texture))
                    .unwrap_or_else(|| specular_dummy.clone());

                let lightmap_texture = surface
                    .lightmap_texture()
                    .and_then(|texture| texture_cache.get(state, texture))
                    .unwrap_or_else(|| white_dummy.clone());

                let data = surface.data();
                let key = &*data as *const _ as u64;
                let batch = self.batches.entry(key).or_insert(Batch {
                    data,
                    instances: Default::default(),
                    diffuse_texture,
                    normal_texture,
                    specular_texture,
                    lightmap_texture,
                    matrix_buffer_stride: if surface.bones.is_empty() {
                        // Non-skinned mesh will hold only GENERIC_MATRICES_COUNT per instance.
                        GENERIC_MATRICES_COUNT
                    } else {
                        // Skinned mesh requires additional matrices per instances for bones.
                        GENERIC_MATRICES_COUNT + BONE_MATRICES_COUNT
                    },
                    skinned: !surface.bones.is_empty(),
                });
                let mut instance = Instance {
                    world_transform: world,
                    wvp_transform: mvp,
                    bone_matrices: Default::default(),
                    color: surface.color(),
                };
                for &bone_handle in surface.bones.iter() {
                    let bone_node = &graph[bone_handle];
                    instance
                        .bone_matrices
                        .push(bone_node.global_transform() * bone_node.inv_bind_pose_transform());
                }
                batch.instances.push(instance);
            }
        }
    }

    pub fn new(state: &mut State, width: usize, height: usize) -> Result<Self, RendererError> {
        scope_profile!();

        let mut depth_stencil_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::D24S8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        depth_stencil_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let depth_stencil = Rc::new(RefCell::new(depth_stencil_texture));

        let mut diffuse_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        diffuse_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let mut normal_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        normal_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let mut ambient_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        ambient_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let framebuffer = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil.clone(),
            }),
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(diffuse_texture)),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(normal_texture)),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(ambient_texture)),
                },
            ],
        )?;

        let frame_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;

        let opt_framebuffer = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil,
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: Rc::new(RefCell::new(frame_texture)),
            }],
        )?;

        Ok(Self {
            framebuffer,
            shader: GBufferShader::new()?,
            width: width as i32,
            height: height as i32,
            final_frame: opt_framebuffer,
            batches: Default::default(),
            storage: Storage::new(state)?,
        })
    }

    pub fn frame_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.final_frame.color_attachments()[0].texture.clone()
    }

    pub fn depth(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.depth_attachment().unwrap().texture.clone()
    }

    pub fn diffuse_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn normal_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[1].texture.clone()
    }

    pub fn ambient_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[2].texture.clone()
    }

    #[must_use]
    pub(in crate) fn fill(&mut self, args: GBufferRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let GBufferRenderContext {
            state,
            graph,
            camera,
            white_dummy,
            normal_dummy,
            specular_dummy,
            texture_cache,
            geom_cache,
        } = args;

        let viewport = Rect::new(0, 0, self.width, self.height);
        self.framebuffer.clear(
            state,
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            Some(0),
        );

        self.generate_batches(
            state,
            graph,
            camera,
            white_dummy,
            normal_dummy,
            specular_dummy,
            texture_cache,
        );

        for batch in self.batches.values() {
            self.storage.update(state, batch);

            let geometry = geom_cache.get(state, &batch.data.lock().unwrap());
            let params = DrawParameters {
                cull_face: CullFace::Back,
                culling: true,
                color_write: Default::default(),
                depth_write: true,
                stencil_test: false,
                depth_test: true,
                blend: false,
            };

            statistics += self.framebuffer.draw_instances(
                batch.instances.len(),
                geometry,
                state,
                viewport,
                &self.shader.program,
                &params,
                &[
                    (
                        self.shader.diffuse_texture,
                        UniformValue::Sampler {
                            index: 0,
                            texture: batch.diffuse_texture.clone(),
                        },
                    ),
                    (
                        self.shader.normal_texture,
                        UniformValue::Sampler {
                            index: 1,
                            texture: batch.normal_texture.clone(),
                        },
                    ),
                    (
                        self.shader.specular_texture,
                        UniformValue::Sampler {
                            index: 2,
                            texture: batch.specular_texture.clone(),
                        },
                    ),
                    (
                        self.shader.lightmap_texture,
                        UniformValue::Sampler {
                            index: 3,
                            texture: batch.lightmap_texture.clone(),
                        },
                    ),
                    (
                        self.shader.color_storage,
                        UniformValue::Sampler {
                            index: 4,
                            texture: self.storage.colors_storage.clone(),
                        },
                    ),
                    (
                        self.shader.matrix_storage,
                        UniformValue::Sampler {
                            index: 5,
                            texture: self.storage.matrices_storage.clone(),
                        },
                    ),
                    (
                        self.shader.use_skeletal_animation,
                        UniformValue::Bool(batch.skinned),
                    ),
                    (
                        self.shader.matrix_buffer_stride,
                        UniformValue::Integer(batch.matrix_buffer_stride as i32),
                    ),
                    (
                        self.shader.matrix_storage_size,
                        UniformValue::Vector4({
                            let kind = self.storage.matrices_storage.borrow().kind();
                            let (w, h) = if let GpuTextureKind::Rectangle { width, height } = kind {
                                (width, height)
                            } else {
                                unreachable!()
                            };
                            Vector4::new(1.0 / (w as f32), 1.0 / (h as f32), w as f32, h as f32)
                        }),
                    ),
                    (
                        self.shader.color_storage_size,
                        UniformValue::Vector4({
                            let kind = self.storage.colors_storage.borrow().kind();
                            let (w, h) = if let GpuTextureKind::Rectangle { width, height } = kind {
                                (width, height)
                            } else {
                                unreachable!()
                            };
                            Vector4::new(1.0 / (w as f32), 1.0 / (h as f32), w as f32, h as f32)
                        }),
                    ),
                ],
            );
        }

        statistics
    }
}
