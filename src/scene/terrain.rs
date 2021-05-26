//! Everything related to terrains.

#![allow(missing_docs)] // Temporary

use crate::core::arrayvec::ArrayVec;
use crate::core::math::ray::Ray;
use crate::core::math::ray_rect_intersection;
use crate::core::visitor::PodVecView;
use crate::resource::texture::TextureWrapMode;
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
use std::cmp::Ordering;
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

#[derive(Debug, Clone)]
pub struct Chunk {
    heightmap: Vec<f32>,
    layers: Vec<Layer>,
    position: Vector3<f32>,
    width: f32,
    length: f32,
    width_point_count: u32,
    length_point_count: u32,
    surface_data: Arc<RwLock<SurfaceData>>,
    dirty: Cell<bool>,
}

// Manual implementation of the trait because we need to serialize heightmap differently.
impl Visit for Chunk {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut view = PodVecView::from_pod_vec(&mut self.heightmap);
        view.visit("Heightmap", visitor)?;

        self.layers.visit("Layers", visitor)?;
        self.position.visit("Position", visitor)?;
        self.width.visit("Width", visitor)?;
        self.length.visit("Length", visitor)?;
        self.width_point_count.visit("WidthPointCount", visitor)?;
        self.length_point_count.visit("LengthPointCount", visitor)?;
        // self.surface_data, self.dirty is are not serialized.

        visitor.leave_region()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            heightmap: Default::default(),
            layers: Default::default(),
            position: Default::default(),
            width: 0.0,
            length: 0.0,
            width_point_count: 0,
            length_point_count: 0,
            surface_data: make_surface_data(),
            dirty: Cell::new(true),
        }
    }
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

    pub fn layers_mut(&mut self) -> &mut [Layer] {
        &mut self.layers
    }

    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
        self.dirty.set(true);
    }

    pub fn remove_layer(&mut self, layer: usize) -> Layer {
        let layer = self.layers.remove(layer);
        self.dirty.set(true);
        layer
    }

    pub fn pop_layer(&mut self) -> Option<Layer> {
        let layer = self.layers.pop();
        self.dirty.set(true);
        layer
    }

    pub fn insert_layer(&mut self, layer: Layer, index: usize) {
        self.layers.insert(index, layer);
        self.dirty.set(true);
    }

    pub fn local_position(&self) -> Vector2<f32> {
        map_to_local(self.position)
    }

    pub fn heightmap(&self) -> &[f32] {
        &self.heightmap
    }

    pub fn set_heightmap(&mut self, heightmap: Vec<f32>) {
        assert_eq!(self.heightmap.len(), heightmap.len());
        self.heightmap = heightmap;
        self.dirty.set(true);
    }

    pub fn data(&self) -> Arc<RwLock<SurfaceData>> {
        self.surface_data.clone()
    }
}

fn map_to_local(v: Vector3<f32>) -> Vector2<f32> {
    // Terrain is a XZ oriented surface so we can map X -> X, Z -> Y
    Vector2::new(v.x, v.z)
}

#[derive(Debug)]
pub struct TerrainRayCastResult {
    pub position: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub chunk_index: usize,
    pub toi: f32,
}

#[derive(Visit, Debug, Default)]
pub struct Terrain {
    width: f32,
    length: f32,
    mask_resolution: f32,
    height_map_resolution: f32,
    base: Base,
    chunks: Vec<Chunk>,
    width_chunks: u32,
    length_chunks: u32,
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

    pub fn chunks_mut(&mut self) -> &mut [Chunk] {
        &mut self.chunks
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            width: self.width,
            length: self.length,
            mask_resolution: self.mask_resolution,
            height_map_resolution: self.height_map_resolution,
            base: self.base.raw_copy(),
            chunks: self.chunks.clone(),
            width_chunks: self.width_chunks,
            length_chunks: self.length_chunks,
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
            BrushMode::ModifyHeightMap { amount } => {
                for chunk in self.chunks.iter_mut() {
                    for z in 0..chunk.length_point_count {
                        let kz = z as f32 / (chunk.length_point_count - 1) as f32;
                        for x in 0..chunk.width_point_count {
                            let kx = x as f32 / (chunk.width_point_count - 1) as f32;

                            let pixel_position = chunk.local_position()
                                + Vector2::new(kx * chunk.width, kz * chunk.length);

                            let k = match brush.kind {
                                BrushKind::Circle { radius } => {
                                    1.0 - ((center - pixel_position).norm() / radius).powf(2.0)
                                }
                                BrushKind::Rectangle { .. } => 1.0,
                            };

                            if brush.kind.contains(center, pixel_position) {
                                chunk.heightmap[(z * chunk.width_point_count + x) as usize] +=
                                    k * amount;

                                chunk.dirty.set(true);
                            }
                        }
                    }
                }
            }
            BrushMode::DrawOnMask { layer, alpha } => {
                let alpha = alpha.clamp(0.0, 1.0);

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

                            let k = match brush.kind {
                                BrushKind::Circle { radius } => {
                                    1.0 - ((center - pixel_position).norm() / radius).powf(4.0)
                                }
                                BrushKind::Rectangle { .. } => 1.0,
                            };

                            if brush.kind.contains(center, pixel_position) {
                                // We can draw on mask directly, without any problems because it has R8 pixel format.
                                let data = texture_data_mut.data_mut();
                                let pixel = &mut data[(z * texture_width + x) as usize];
                                *pixel = (*pixel as f32 + k * alpha * 255.0).min(255.0) as u8;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn raycast<const DIM: usize>(
        &self,
        ray: Ray,
        results: &mut ArrayVec<TerrainRayCastResult, DIM>,
        sort_results: bool,
    ) -> bool {
        if let Some(inv_transform) = self.global_transform().try_inverse() {
            // Transform ray into local coordinate system of the terrain.
            let local_ray = ray.transform(inv_transform);

            // Project ray on the terrain's 2D space.
            let origin_proj = map_to_local(
                inv_transform
                    .transform_point(&Point3::from(ray.origin))
                    .coords,
            );
            let dir_proj = map_to_local(inv_transform.transform_vector(&ray.dir));

            // Check each cell of each chunk for intersection in 2D.
            'chunk_loop: for (chunk_index, chunk) in self.chunks.iter().enumerate() {
                let cell_width = chunk.width / (chunk.width_point_count - 1) as f32;
                let cell_length = chunk.length / (chunk.length_point_count - 1) as f32;

                for z in 0..chunk.length_point_count {
                    let kz = z as f32 / (chunk.length_point_count - 1) as f32;
                    let nz = z + 1;

                    for x in 0..chunk.width_point_count {
                        let kx = x as f32 / (chunk.width_point_count - 1) as f32;
                        let nx = x + 1;

                        let pixel_position = chunk.local_position()
                            + Vector2::new(kx * chunk.width, kz * chunk.length);

                        let cell_bounds =
                            Rect::new(pixel_position.x, pixel_position.y, cell_width, cell_length);

                        if ray_rect_intersection(cell_bounds, origin_proj, dir_proj).is_some() {
                            // If we have 2D intersection, go back in 3D and do precise intersection
                            // check.
                            if nx < chunk.width_point_count && nz < chunk.length_point_count {
                                let i0 = (z * chunk.width_point_count + x) as usize;
                                let i1 = ((z + 1) * chunk.width_point_count + x) as usize;
                                let i2 = ((z + 1) * chunk.width_point_count + x + 1) as usize;
                                let i3 = (z * chunk.width_point_count + x + 1) as usize;

                                let v0 = Vector3::new(
                                    pixel_position.x,
                                    chunk.heightmap[i0],
                                    pixel_position.y, // Remember Z -> Y mapping!
                                );
                                let v1 =
                                    Vector3::new(v0.x, chunk.heightmap[i1], v0.z + cell_length);
                                let v2 = Vector3::new(v1.x + cell_width, chunk.heightmap[i2], v1.z);
                                let v3 = Vector3::new(v0.x + cell_width, chunk.heightmap[i3], v0.z);

                                for vertices in &[[v0, v1, v2], [v2, v3, v0]] {
                                    if let Some((toi, intersection)) =
                                        local_ray.triangle_intersection(vertices)
                                    {
                                        let normal = (vertices[2] - vertices[0])
                                            .cross(&(vertices[1] - vertices[0]))
                                            .try_normalize(f32::EPSILON)
                                            .unwrap_or_else(Vector3::y);

                                        let result = TerrainRayCastResult {
                                            position: intersection,
                                            normal,
                                            chunk_index,
                                            toi,
                                        };

                                        if results.try_push(result).is_err() {
                                            break 'chunk_loop;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if sort_results {
            results.sort_unstable_by(|a, b| {
                if a.toi > b.toi {
                    Ordering::Greater
                } else if a.toi < b.toi {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            });
        }

        !results.is_empty()
    }

    pub fn update(&mut self) {
        for chunk in self.chunks.iter_mut() {
            chunk.update();
        }
    }

    pub fn create_layer(&self, tile_factor: Vector2<f32>, value: u8) -> Layer {
        let chunk_length = self.length / self.length_chunks as f32;
        let chunk_width = self.width / self.width_chunks as f32;
        let mask_width = (chunk_width * self.mask_resolution) as u32;
        let mask_height = (chunk_length * self.mask_resolution) as u32;

        Layer {
            diffuse_texture: None,
            normal_texture: None,
            specular_texture: None,
            roughness_texture: None,
            height_texture: None,
            mask: Some(create_layer_mask(mask_width, mask_height, value)),
            tile_factor,
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
    ModifyHeightMap {
        /// An offset for height map.
        amount: f32,
    },
    DrawOnMask {
        /// A layer to draw on.
        layer: usize,
        /// A value to put on mask.
        alpha: f32,
    },
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
    height_map_resolution: f32,
    layers: Vec<LayerDefinition>,
}

fn make_divisible_by_2(n: u32) -> u32 {
    if n & 1 == 1 {
        n + 1
    } else {
        n
    }
}

fn create_layer_mask(width: u32, height: u32, value: u8) -> Texture {
    let mask = Texture::from_bytes(
        TextureKind::Rectangle { width, height },
        TexturePixelKind::R8,
        vec![value; (width * height) as usize],
        // Content of mask will be explicitly serialized.
        true,
    )
    .unwrap();

    let mut data_ref = mask.data_ref();
    data_ref.set_s_wrap_mode(TextureWrapMode::ClampToEdge);
    data_ref.set_t_wrap_mode(TextureWrapMode::ClampToEdge);
    drop(data_ref);

    mask
}

fn make_surface_data() -> Arc<RwLock<SurfaceData>> {
    Arc::new(RwLock::new(SurfaceData::new(
        VertexBuffer::new::<StaticVertex>(0, StaticVertex::layout(), vec![]).unwrap(),
        GeometryBuffer::default(),
        false,
    )))
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
            height_map_resolution: 8.0,
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
        self.width_chunks = count.max(1);
        self
    }

    pub fn with_length_chunks(mut self, count: usize) -> Self {
        self.length_chunks = count.max(1);
        self
    }

    pub fn with_height_map_resolution(mut self, resolution: f32) -> Self {
        self.height_map_resolution = resolution;
        self
    }

    pub fn with_layers(mut self, layers: Vec<LayerDefinition>) -> Self {
        self.layers = layers;
        self
    }

    pub fn build_node(self) -> Node {
        let mut chunks = Vec::new();
        let chunk_length = self.length / self.length_chunks as f32;
        let chunk_width = self.width / self.width_chunks as f32;
        let chunk_length_points =
            make_divisible_by_2((chunk_length * self.height_map_resolution) as u32);
        let chunk_width_points =
            make_divisible_by_2((chunk_width * self.height_map_resolution) as u32);
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
                        .map(|(layer_index, definition)| {
                            Layer {
                                diffuse_texture: definition.diffuse_texture.clone(),
                                normal_texture: definition.normal_texture.clone(),
                                specular_texture: definition.specular_texture.clone(),
                                roughness_texture: definition.roughness_texture.clone(),
                                height_texture: definition.height_texture.clone(),
                                // Base layer is opaque, every other by default - transparent.
                                mask: Some(create_layer_mask(
                                    chunk_mask_width,
                                    chunk_mask_height,
                                    if layer_index == 0 { 255 } else { 0 },
                                )),
                                tile_factor: definition.tile_factor,
                            }
                        })
                        .collect(),
                    position: Vector3::new(x as f32 * chunk_width, 0.0, z as f32 * chunk_length),
                    width: chunk_width,
                    surface_data: make_surface_data(),
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
            mask_resolution: self.mask_resolution,
            height_map_resolution: self.height_map_resolution,
            width_chunks: self.width_chunks as u32,
            length_chunks: self.length_chunks as u32,
        };

        Node::Terrain(terrain)
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
