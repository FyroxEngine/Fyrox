use crate::{
    core::{
        algebra::{Matrix4, Vector2},
        arrayvec::ArrayVec,
        color::Color,
        pool::Handle,
        scope_profile,
    },
    material::Material,
    renderer::framework::{
        error::FrameworkError,
        gpu_texture::{
            GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
        },
        state::PipelineState,
    },
    renderer::TextureCache,
    scene::{
        graph::Graph,
        mesh::{surface::SurfaceData, RenderPath},
        node::Node,
    },
    utils::array_as_u8_slice,
};
use std::sync::Mutex;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Debug, Formatter},
    rc::Rc,
    sync::{Arc, RwLock},
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
    pub bone_matrices: ArrayVec<Matrix4<f32>, BONE_MATRICES_COUNT>,
    pub color: Color,
    pub depth_offset: f32,
}

pub struct Batch {
    pub data: Arc<RwLock<SurfaceData>>,
    pub instances: Vec<SurfaceInstance>,
    pub material: Arc<Mutex<Material>>,
    pub mask_texture: Rc<RefCell<GpuTexture>>,
    pub is_skinned: bool,
    pub render_path: RenderPath,
    pub is_terrain: bool,
    pub blend: bool,
    pub tex_coord_scale: Vector2<f32>,
    pub decal_layer_index: u8,
    sort_index: u64,
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
    batch_map: HashMap<u64, usize>,
    /// Sorted list of batches.
    pub batches: Vec<Batch>,
}

impl BatchStorage {
    pub(in crate) fn generate_batches(
        &mut self,
        state: &mut PipelineState,
        graph: &Graph,
        white_dummy: Rc<RefCell<GpuTexture>>,
        texture_cache: &mut TextureCache,
    ) {
        scope_profile!();

        for batch in self.batches.iter_mut() {
            batch.instances.clear();
            self.buffers.push(std::mem::take(&mut batch.instances));
        }

        self.batches.clear();
        self.batch_map.clear();

        for (handle, node) in graph.pair_iter() {
            match node {
                Node::Mesh(mesh) => {
                    for surface in mesh.surfaces().iter() {
                        let is_skinned = !surface.bones.is_empty();

                        let world = if is_skinned {
                            Matrix4::identity()
                        } else {
                            mesh.global_transform()
                        };

                        let data = surface.data();
                        let key = surface.batch_id();

                        let batch = if let Some(&batch_index) = self.batch_map.get(&key) {
                            self.batches.get_mut(batch_index).unwrap()
                        } else {
                            self.batch_map.insert(key, self.batches.len());
                            self.batches.push(Batch {
                                data,
                                // Batches from meshes will be sorted using diffuse textures.
                                // This will significantly reduce pipeline state changes.
                                sort_index: surface.batch_id(),
                                instances: self.buffers.pop().unwrap_or_default(),
                                material: surface.material().clone(),
                                mask_texture: white_dummy.clone(),
                                is_skinned: !surface.bones.is_empty(),
                                render_path: mesh.render_path(),
                                is_terrain: false,
                                blend: false,
                                decal_layer_index: mesh.decal_layer_index(),
                                tex_coord_scale: Vector2::new(1.0, 1.0),
                            });
                            self.batches.last_mut().unwrap()
                        };

                        batch.sort_index = surface.batch_id();

                        // Update textures.
                        batch.material = surface.material().clone();

                        batch.instances.push(SurfaceInstance {
                            world_transform: world,
                            bone_matrices: surface
                                .bones
                                .iter()
                                .map(|&bone_handle| {
                                    let bone_node = &graph[bone_handle];
                                    bone_node.global_transform()
                                        * bone_node.inv_bind_pose_transform()
                                })
                                .collect(),
                            color: Default::default(), //TODO surface.color(),
                            owner: handle,
                            depth_offset: mesh.depth_offset_factor(),
                        });
                    }
                }
                Node::Terrain(terrain) => {
                    for chunk in terrain.chunks_ref().iter() {
                        let data = chunk.data();
                        let data_key = &*data as *const _ as u64;

                        for (layer_index, layer) in chunk.layers().iter().enumerate() {
                            let key = layer.batch_id(data_key);

                            let mask_texture = layer
                                .mask
                                .as_ref()
                                .and_then(|texture| texture_cache.get(state, texture))
                                .unwrap_or_else(|| white_dummy.clone());

                            // TODO. Add support for lightmaps for terrains.

                            let batch = if let Some(&batch_index) = self.batch_map.get(&key) {
                                self.batches.get_mut(batch_index).unwrap()
                            } else {
                                self.batch_map.insert(key, self.batches.len());
                                self.batches.push(Batch {
                                    data: data.clone(),
                                    instances: self.buffers.pop().unwrap_or_default(),
                                    material: layer.material.clone(),
                                    mask_texture: mask_texture.clone(),
                                    is_skinned: false,
                                    render_path: RenderPath::Deferred,
                                    sort_index: layer_index as u64,
                                    is_terrain: true,
                                    blend: layer_index != 0,
                                    tex_coord_scale: layer.tile_factor,
                                    decal_layer_index: terrain.decal_layer_index(),
                                });
                                self.batches.last_mut().unwrap()
                            };

                            batch.sort_index = layer_index as u64;

                            // Update textures.
                            batch.material = layer.material.clone();
                            batch.mask_texture = mask_texture;

                            batch.instances.push(SurfaceInstance {
                                world_transform: terrain.global_transform(),
                                bone_matrices: Default::default(),
                                color: Color::WHITE,
                                owner: handle,
                                depth_offset: terrain.depth_offset_factor(),
                            });
                        }
                    }
                }
                _ => (),
            }
        }

        self.batches.sort_unstable_by_key(|b| b.sort_index);
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
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
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
                GpuTextureKind::Rectangle {
                    width: matrices_w,
                    height: matrices_h,
                },
                PixelKind::RGBA32F,
                1,
                Some(array_as_u8_slice(&self.matrices)),
            )
            .unwrap();
    }
}
