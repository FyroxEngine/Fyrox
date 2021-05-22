//! Everything related to terrains.

#![allow(missing_docs)] // Temporary

use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        math::{aabb::AxisAlignedBoundingBox, Rect, TriangleDefinition},
        pool::Handle,
        visitor::prelude::*,
    },
    resource::texture::{Texture, TextureKind, TexturePixelKind},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::{
            buffer::{GeometryBuffer, VertexBuffer},
            surface::SurfaceData,
            vertex::StaticVertex,
        },
        node::Node,
    },
};
use std::{
    cell::Cell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone, Visit)]
pub struct Layer {
    pub diffuse_texture: Option<Texture>,
    pub normal_texture: Option<Texture>,
    pub specular_texture: Option<Texture>,
    pub roughness_texture: Option<Texture>,
    pub height_texture: Option<Texture>,
    pub mask: Option<Texture>,
    pub tile_factor: Vector2<f32>,
}

impl Layer {
    pub fn batch_id(&self, data_key: u64) -> u64 {
        let mut hasher = DefaultHasher::new();

        data_key.hash(&mut hasher);

        for texture in [
            self.diffuse_texture.as_ref(),
            self.normal_texture.as_ref(),
            self.specular_texture.as_ref(),
            self.roughness_texture.as_ref(),
            self.height_texture.as_ref(),
        ]
        .iter()
        .filter_map(|t| *t)
        {
            texture.key().hash(&mut hasher);
        }

        hasher.finish()
    }
}

#[derive(Default, Debug, Clone, Visit)]
pub struct Chunk {
    heightmap: Vec<f32>,
    layers: Vec<Layer>,
    position: Vector3<f32>,
    width: f32,
    length: f32,
    width_point_count: u32,
    length_point_count: u32,
    // No need to save surface data, it will be regenerated automatically from
    // other data.
    #[visit(skip)]
    surface_data: Arc<RwLock<SurfaceData>>,
    #[visit(skip)]
    dirty: Cell<bool>,
}

impl Chunk {
    pub fn update(&mut self) {
        if self.dirty.get() {
            let mut surface_data = self.surface_data.write().unwrap();
            surface_data.clear();

            assert_eq!(self.width_point_count & 1, 0);
            assert_eq!(self.length_point_count & 1, 0);

            let mut vertex_buffer_mut = surface_data.vertex_buffer.modify();
            // Form vertex buffer.
            for z in 0..self.length_point_count {
                let kz = z as f32 / ((self.length_point_count - 1) as f32);
                let pz = self.position.z + kz * self.length;

                for x in 0..self.width_point_count {
                    let index = z * self.width_point_count + x;
                    let height = self.heightmap[index as usize];
                    let kx = x as f32 / ((self.width_point_count - 1) as f32);

                    let px = self.position.x + kx * self.width;
                    let py = self.position.y + height;

                    vertex_buffer_mut
                        .push_vertex(&StaticVertex {
                            position: Vector3::new(px, py, pz),
                            tex_coord: Vector2::new(kx, kz),
                            // Normals and tangents will be calculated later.
                            normal: Default::default(),
                            tangent: Default::default(),
                        })
                        .unwrap();
                }
            }
            drop(vertex_buffer_mut);

            let mut geometry_buffer_mut = surface_data.geometry_buffer.modify();
            // Form index buffer.
            // TODO: Generate LODs.
            for z in 0..self.length_point_count - 1 {
                let z_next = z + 1;
                for x in 0..self.width_point_count - 1 {
                    let x_next = x + 1;

                    let i0 = z * self.width_point_count + x;
                    let i1 = z_next * self.width_point_count + x;
                    let i2 = z_next * self.width_point_count + x_next;
                    let i3 = z * self.width_point_count + x_next;

                    geometry_buffer_mut.push(TriangleDefinition([i0, i1, i2]));
                    geometry_buffer_mut.push(TriangleDefinition([i2, i3, i0]));
                }
            }
            drop(geometry_buffer_mut);

            surface_data.calculate_normals().unwrap();
            surface_data.calculate_tangents().unwrap();

            self.dirty.set(false);
        }
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn local_position(&self) -> Vector2<f32> {
        map_to_local(self.position)
    }

    pub fn data(&self) -> Arc<RwLock<SurfaceData>> {
        self.surface_data.clone()
    }
}

fn map_to_local(v: Vector3<f32>) -> Vector2<f32> {
    // Terrain is a XZ oriented surface so we can map X -> X, Z -> Y
    Vector2::new(v.x, v.z)
}

#[derive(Visit, Debug, Default)]
pub struct Terrain {
    width: f32,
    length: f32,
    base: Base,
    chunks: Vec<Chunk>,
    bounding_box_dirty: Cell<bool>,
    bounding_box: Cell<AxisAlignedBoundingBox>,
}

impl Deref for Terrain {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Terrain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

fn project(global_transform: Matrix4<f32>, p: Vector3<f32>) -> Option<Vector2<f32>> {
    // Transform point in coordinate system of the terrain.
    if let Some(inv_global_transform) = global_transform.try_inverse() {
        let local_p = inv_global_transform
            .transform_point(&Point3::from(p))
            .coords;
        Some(map_to_local(local_p))
    } else {
        None
    }
}

impl Terrain {
    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    pub fn chunks_ref(&self) -> &[Chunk] {
        &self.chunks
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            width: self.width,
            length: self.length,
            base: self.base.raw_copy(),
            chunks: self.chunks.clone(),
            bounding_box_dirty: Cell::new(true),
            bounding_box: Default::default(),
        }
    }

    pub fn bounding_box(&self) -> AxisAlignedBoundingBox {
        if self.bounding_box_dirty.get() {
            let mut max_height = -f32::MAX;
            for chunk in self.chunks.iter() {
                for &height in chunk.heightmap.iter() {
                    if height > max_height {
                        max_height = height;
                    }
                }
            }

            let bounding_box = AxisAlignedBoundingBox::from_min_max(
                self.global_position(),
                self.global_position() + Vector3::new(self.width, max_height, self.length),
            );
            self.bounding_box.set(bounding_box);
            self.bounding_box_dirty.set(false);

            bounding_box
        } else {
            self.bounding_box.get()
        }
    }

    /// Projects given 3D point on the surface of terrain and returns 2D vector
    /// expressed in local 2D coordinate system of terrain.
    pub fn project(&self, p: Vector3<f32>) -> Option<Vector2<f32>> {
        project(self.global_transform(), p)
    }

    pub fn draw(&mut self, brush: &Brush) {
        let center = project(self.global_transform(), brush.center).unwrap();

        match brush.mode {
            BrushMode::AlternateHeightMap { amount } => {
                for chunk in self.chunks.iter_mut() {
                    for z in 0..chunk.length_point_count {
                        let kz = z as f32 / (chunk.length_point_count - 1) as f32;
                        for x in 0..chunk.width_point_count {
                            let kx = x as f32 / (chunk.width_point_count - 1) as f32;

                            let pixel_position = chunk.local_position()
                                + Vector2::new(kx * chunk.width, kz * chunk.length);

                            if brush.kind.contains(center, pixel_position) {
                                chunk.heightmap[(z * chunk.width_point_count + x) as usize] +=
                                    amount;

                                chunk.dirty.set(true);
                            }
                        }
                    }
                }
            }
            BrushMode::DrawOnMask { layer } => {
                for chunk in self.chunks.iter_mut() {
                    let chunk_position = chunk.local_position();
                    let layer = &mut chunk.layers[layer];
                    let mut texture_data = layer.mask.as_ref().unwrap().data_ref();
                    let mut texture_data_mut = texture_data.modify();

                    let (texture_width, texture_height) =
                        if let TextureKind::Rectangle { width, height } = texture_data_mut.kind() {
                            (width as usize, height as usize)
                        } else {
                            unreachable!("Mask must be a 2D greyscale image!")
                        };

                    for z in 0..texture_height {
                        let kz = z as f32 / (texture_height - 1) as f32;
                        for x in 0..texture_width {
                            let kx = x as f32 / (texture_width - 1) as f32;

                            let pixel_position =
                                chunk_position + Vector2::new(kx * chunk.width, kz * chunk.length);

                            if brush.kind.contains(center, pixel_position) {
                                // We can draw on mask directly, without any problems because it has R8 pixel format.
                                texture_data_mut.data_mut()[(z * texture_width + x) as usize] = 255;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update(&mut self) {
        for chunk in self.chunks.iter_mut() {
            chunk.update();
        }
    }
}

#[derive(Copy, Clone)]
pub enum BrushKind {
    Circle { radius: f32 },
    Rectangle { width: f32, length: f32 },
}

impl BrushKind {
    pub fn contains(&self, brush_center: Vector2<f32>, pixel_position: Vector2<f32>) -> bool {
        match *self {
            BrushKind::Circle { radius } => (brush_center - pixel_position).norm() < radius,
            BrushKind::Rectangle { width, length } => Rect::new(
                brush_center.x - width * 0.5,
                brush_center.y - length * 0.5,
                width,
                length,
            )
            .contains(pixel_position),
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub enum BrushMode {
    AlternateHeightMap { amount: f32 },
    DrawOnMask { layer: usize },
}

#[derive(Clone)]
pub struct Brush {
    pub center: Vector3<f32>,
    pub kind: BrushKind,
    pub mode: BrushMode,
}

pub struct LayerDefinition {
    pub diffuse_texture: Option<Texture>,
    pub normal_texture: Option<Texture>,
    pub specular_texture: Option<Texture>,
    pub roughness_texture: Option<Texture>,
    pub height_texture: Option<Texture>,
    pub tile_factor: Vector2<f32>,
}

pub struct TerrainBuilder {
    base_builder: BaseBuilder,
    width: f32,
    length: f32,
    mask_resolution: f32,
    width_chunks: usize,
    length_chunks: usize,
    resolution: f32,
    layers: Vec<LayerDefinition>,
}

fn make_divisible_by_2(n: u32) -> u32 {
    if n & 1 == 1 {
        n + 1
    } else {
        n
    }
}

impl TerrainBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            width: 64.0,
            length: 64.0,
            width_chunks: 2,
            length_chunks: 2,
            mask_resolution: 16.0,
            resolution: 8.0,
            layers: Default::default(),
        }
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }

    pub fn with_mask_resolution(mut self, resolution: f32) -> Self {
        self.mask_resolution = resolution;
        self
    }

    pub fn with_width_chunks(mut self, count: usize) -> Self {
        self.width_chunks = count;
        self
    }

    pub fn with_length_chunks(mut self, count: usize) -> Self {
        self.length_chunks = count;
        self
    }

    pub fn with_resolution(mut self, resolution: f32) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn with_layers(mut self, layers: Vec<LayerDefinition>) -> Self {
        self.layers = layers;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        let mut chunks = Vec::new();
        let chunk_length = self.length / self.length_chunks as f32;
        let chunk_width = self.width / self.width_chunks as f32;
        let chunk_length_points = make_divisible_by_2((chunk_length * self.resolution) as u32);
        let chunk_width_points = make_divisible_by_2((chunk_width * self.resolution) as u32);
        let chunk_mask_width = (chunk_width * self.mask_resolution) as u32;
        let chunk_mask_height = (chunk_length * self.mask_resolution) as u32;
        for z in 0..self.length_chunks {
            for x in 0..self.width_chunks {
                chunks.push(Chunk {
                    width_point_count: chunk_width_points,
                    length_point_count: chunk_length_points,
                    heightmap: vec![0.0; (chunk_length_points * chunk_width_points) as usize],
                    layers: self
                        .layers
                        .iter()
                        .enumerate()
                        .map(|(layer_index, definition)| Layer {
                            diffuse_texture: definition.diffuse_texture.clone(),
                            normal_texture: definition.normal_texture.clone(),
                            specular_texture: definition.specular_texture.clone(),
                            roughness_texture: definition.roughness_texture.clone(),
                            height_texture: definition.height_texture.clone(),
                            mask: Texture::from_bytes(
                                TextureKind::Rectangle {
                                    width: chunk_mask_width,
                                    height: chunk_mask_height,
                                },
                                TexturePixelKind::R8,
                                vec![
                                    // Base layer is opaque, every other by default - transparent.
                                    if layer_index == 0 { 255 } else { 0 };
                                    (chunk_mask_width * chunk_mask_height) as usize
                                ],
                                // Content of mask will be explicitly serialized.
                                true,
                            ),
                            tile_factor: definition.tile_factor,
                        })
                        .collect(),
                    position: Vector3::new(x as f32 * chunk_width, 0.0, z as f32 * chunk_length),
                    width: chunk_width,
                    surface_data: Arc::new(RwLock::new(SurfaceData::new(
                        VertexBuffer::new::<StaticVertex>(0, StaticVertex::layout(), vec![])
                            .unwrap(),
                        GeometryBuffer::default(),
                        false,
                    ))),
                    dirty: Cell::new(true),
                    length: chunk_length,
                });
            }
        }

        let terrain = Terrain {
            width: self.width,
            length: self.length,
            base: self.base_builder.build_base(),
            chunks,
            bounding_box_dirty: Cell::new(true),
            bounding_box: Default::default(),
        };

        graph.add_node(Node::Terrain(terrain))
    }
}
