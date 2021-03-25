use crate::core::arrayvec::ArrayVec;
use crate::scene::mesh::RenderPath;
use crate::{
    core::{algebra::Matrix4, color::Color, pool::Handle},
    renderer::{
        error::RendererError,
        framework::gpu_texture::{
            GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
        },
        framework::{gpu_texture::GpuTexture, state::PipelineState},
        surface::SurfaceSharedData,
        TextureCache,
    },
    scene::{graph::Graph, node::Node},
};
use std::sync::RwLock;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Debug, Formatter},
    rc::Rc,
    sync::Arc,
};

pub const BONE_MATRICES_COUNT: usize = 64;

#[repr(C)]
#[doc(hidden)]
pub struct InstanceData {
    pub color: Color,
    pub world: Matrix4<f32>,
    pub depth_offset: f32, // Does NOT include bone matrices, they simply won't fit into vertex attributes
                           // limit and they'll be passed using texture.
}

pub struct SurfaceInstance {
    pub owner: Handle<Node>,
    pub world_transform: Matrix4<f32>,
    pub bone_matrices: ArrayVec<[Matrix4<f32>; BONE_MATRICES_COUNT]>,
    pub color: Color,
    pub depth_offset: f32,
}

pub struct Batch {
    pub data: Arc<RwLock<SurfaceSharedData>>,
    pub instances: Vec<SurfaceInstance>,
    pub diffuse_texture: Rc<RefCell<GpuTexture>>,
    pub normal_texture: Rc<RefCell<GpuTexture>>,
    pub specular_texture: Rc<RefCell<GpuTexture>>,
    pub roughness_texture: Rc<RefCell<GpuTexture>>,
    pub lightmap_texture: Rc<RefCell<GpuTexture>>,
    pub is_skinned: bool,
    pub render_path: RenderPath,
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

#[derive(Default)]
pub struct BatchStorage {
    buffers: Vec<Vec<SurfaceInstance>>,
    inner: HashMap<u64, usize>,
    /// Sorted list of batches.
    pub batches: Vec<Batch>,
}

impl BatchStorage {
    pub(in crate) fn generate_batches(
        &mut self,
        state: &mut PipelineState,
        graph: &Graph,
        black_dummy: Rc<RefCell<GpuTexture>>,
        white_dummy: Rc<RefCell<GpuTexture>>,
        normal_dummy: Rc<RefCell<GpuTexture>>,
        specular_dummy: Rc<RefCell<GpuTexture>>,
        texture_cache: &mut TextureCache,
    ) {
        for batch in self.batches.iter_mut() {
            batch.instances.clear();
            self.buffers.push(std::mem::take(&mut batch.instances));
        }

        self.batches.clear();
        self.inner.clear();

        for (handle, mesh) in graph.pair_iter().filter_map(|(handle, node)| {
            if let Node::Mesh(mesh) = node {
                Some((handle, mesh))
            } else {
                None
            }
        }) {
            for surface in mesh.surfaces().iter() {
                let is_skinned = !surface.bones.is_empty();

                let world = if is_skinned {
                    Matrix4::identity()
                } else {
                    mesh.global_transform()
                };

                let data = surface.data();
                let key = surface.batch_id();

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

                let roughness_texture = surface
                    .roughness_texture()
                    .and_then(|texture| texture_cache.get(state, texture))
                    .unwrap_or_else(|| black_dummy.clone());

                let lightmap_texture = surface
                    .lightmap_texture()
                    .and_then(|texture| texture_cache.get(state, texture))
                    .unwrap_or_else(|| black_dummy.clone());

                let batch = if let Some(&batch_index) = self.inner.get(&key) {
                    self.batches.get_mut(batch_index).unwrap()
                } else {
                    self.inner.insert(key, self.batches.len());
                    self.batches.push(Batch {
                        data,
                        instances: self.buffers.pop().unwrap_or_default(),
                        diffuse_texture: diffuse_texture.clone(),
                        normal_texture: normal_texture.clone(),
                        specular_texture: specular_texture.clone(),
                        roughness_texture: roughness_texture.clone(),
                        lightmap_texture: lightmap_texture.clone(),
                        is_skinned: !surface.bones.is_empty(),
                        render_path: mesh.render_path(),
                    });
                    self.batches.last_mut().unwrap()
                };

                // Update textures.
                batch.diffuse_texture = diffuse_texture;
                batch.normal_texture = normal_texture;
                batch.specular_texture = specular_texture;
                batch.roughness_texture = roughness_texture;
                batch.lightmap_texture = lightmap_texture;

                batch.instances.push(SurfaceInstance {
                    world_transform: world,
                    bone_matrices: surface
                        .bones
                        .iter()
                        .map(|&bone_handle| {
                            let bone_node = &graph[bone_handle];
                            bone_node.global_transform() * bone_node.inv_bind_pose_transform()
                        })
                        .collect(),
                    color: surface.color(),
                    owner: handle,
                    depth_offset: mesh.depth_offset_factor(),
                });
            }
        }

        // Sort by diffuse texture, this will significantly decrease texture pipeline
        // state changes during the rendering.
        self.batches
            .sort_unstable_by_key(|b| (&*b.diffuse_texture.borrow()) as *const _ as u64);
    }
}

pub struct MatrixStorage {
    // Generic storage for instancing, contains all matrices needed for instanced
    // rendering. It has variable size, but it is always multiple of 4. Each pixel
    // has RGBA components as f32 so to store 4x4 matrix we need 4 pixels.
    //
    // Q: Why it uses textures instead of SSBO?
    // A: This could be done with SSBO, but it is not available on macOS because SSBO
    // was added only in OpenGL 4.3, but macOS support up to OpenGL 4.1.
    pub matrices_storage: Rc<RefCell<GpuTexture>>,
    matrices: Vec<Matrix4<f32>>,
}

impl MatrixStorage {
    pub fn new(state: &mut PipelineState) -> Result<Self, RendererError> {
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
        })
    }

    pub fn clear(&mut self) {
        self.matrices.clear();
    }

    pub fn push_slice(&mut self, matrices: &[Matrix4<f32>]) {
        self.matrices.extend_from_slice(matrices);

        // Pad rest with zeros because we can't use tight packing in this case.
        for _ in 0..(BONE_MATRICES_COUNT - matrices.len()) {
            self.matrices.push(Default::default());
        }
    }

    pub fn update(&mut self, state: &mut PipelineState) {
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
    }
}
