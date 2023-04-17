//! Everything related to terrains.

use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        arrayvec::ArrayVec,
        math::{aabb::AxisAlignedBoundingBox, ray::Ray, ray_rect_intersection, Rect},
        pool::Handle,
        reflect::prelude::*,
        sstorage::ImmutableString,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::{prelude::*, PodVecView},
    },
    material::{PropertyValue, SharedMaterial},
    renderer::{
        self,
        batch::{RenderContext, SurfaceInstanceData},
        framework::geometry_buffer::ElementRange,
    },
    resource::texture::{Texture, TextureKind, TexturePixelKind, TextureWrapMode},
    scene::{
        base::{Base, BaseBuilder},
        debug::SceneDrawingContext,
        graph::Graph,
        mesh::RenderPath,
        node::{Node, NodeTrait, TypeUuidProvider},
        terrain::{geometry::TerrainGeometry, quadtree::QuadTree},
    },
    utils::log::{Log, MessageKind},
};
use image::{imageops::FilterType, ImageBuffer, Luma};
use std::{
    cell::Cell,
    cmp::Ordering,
    collections::HashMap,
    ops::{Deref, DerefMut, Range},
};

mod geometry;
mod quadtree;

/// Current implementation version marker.
pub const VERSION: u8 = 1;

/// Layers is a set of textures for rendering + mask texture to exclude some pixels from
/// rendering. Terrain can have as many layers as you want, but each layer slightly decreases
/// performance, so keep amount of layers on reasonable level (1 - 5 should be enough for most
/// cases).
#[derive(Default, Debug, Clone, Visit, Reflect)]
pub struct Layer {
    /// Material of the layer.
    pub material: SharedMaterial,

    /// Name of the mask sampler in the material.
    ///
    /// # Implementation details
    ///
    /// It will be used in the renderer to set appropriate chunk mask to the copy of the material.
    pub mask_property_name: String,
}

impl PartialEq for Layer {
    fn eq(&self, other: &Self) -> bool {
        self.mask_property_name == other.mask_property_name && self.material == other.material
    }
}

/// Chunk is smaller block of a terrain. Terrain can have as many chunks as you need.
/// Can't we just use one big chunk? Well, potentially yes. However in practice, it
/// is very limiting because you need to have very huge mask texture and most of wide-spread
/// GPUs have 16k texture size limit. Multiple chunks provide different LODs to renderer
/// so distant chunks can be rendered with low details reducing GPU load.
#[derive(Debug, Reflect, PartialEq)]
pub struct Chunk {
    #[reflect(hidden)]
    quad_tree: QuadTree,
    #[reflect(hidden)]
    version: u8,
    #[reflect(hidden)]
    heightmap: Vec<f32>,
    #[reflect(hidden)]
    position: Vector3<f32>,
    #[reflect(hidden)]
    physical_size: Vector2<f32>,
    #[reflect(hidden)]
    height_map_size: Vector2<u32>,
    #[reflect(hidden)]
    block_size: Vector2<u32>,
    #[reflect(hidden)]
    grid_position: Vector2<i32>,
    /// Layer blending masks of the chunk.
    #[reflect(hidden)]
    pub layer_masks: Vec<Texture>,
}

impl Clone for Chunk {
    // Deep cloning.
    fn clone(&self) -> Self {
        Self {
            version: self.version,
            heightmap: self.heightmap.clone(),
            position: self.position,
            physical_size: self.physical_size,
            height_map_size: self.height_map_size,
            block_size: self.block_size,
            grid_position: self.grid_position,
            layer_masks: self
                .layer_masks
                .iter()
                .map(|m| {
                    let data = m.data_ref();
                    Texture::from_bytes(
                        data.kind(),
                        data.pixel_kind(),
                        data.data().to_vec(),
                        data.is_serializing_content(),
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>(),
            quad_tree: QuadTree::new(&self.heightmap, self.height_map_size, self.block_size),
        }
    }
}

// Manual implementation of the trait because we need to serialize heightmap differently.
impl Visit for Chunk {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut version = if region.is_reading() {
            0u8
        } else {
            self.version
        };
        let _ = version.visit("Version", &mut region);

        match version {
            0 => {
                let mut view = PodVecView::from_pod_vec(&mut self.heightmap);
                view.visit("Heightmap", &mut region)?;
                self.position.visit("Position", &mut region)?;

                let mut width = 0.0f32;
                width.visit("Width", &mut region)?;
                let mut length = 0.0f32;
                length.visit("Length", &mut region)?;
                self.physical_size = Vector2::new(width, length);

                let mut width_point_count = 0u32;
                width_point_count.visit("WidthPointCount", &mut region)?;
                let mut length_point_count = 0u32;
                length_point_count.visit("LengthPointCount", &mut region)?;
                self.height_map_size = Vector2::new(width_point_count, length_point_count);

                self.grid_position = Vector2::new(
                    (self.position.x / width) as i32,
                    (self.position.y / length) as i32,
                );
            }
            VERSION => {
                let mut view = PodVecView::from_pod_vec(&mut self.heightmap);
                view.visit("Heightmap", &mut region)?;

                self.position.visit("Position", &mut region)?;
                self.physical_size.visit("PhysicalSize", &mut region)?;
                self.height_map_size.visit("HeightMapSize", &mut region)?;
                self.layer_masks.visit("LayerMasks", &mut region)?;
                self.grid_position.visit("GridPosition", &mut region)?;
                let _ = self.block_size.visit("BlockSize", &mut region);
                // self.surface_data is not serialized.
            }
            _ => (),
        }

        self.quad_tree = QuadTree::new(&self.heightmap, self.height_map_size, self.block_size);

        Ok(())
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            quad_tree: Default::default(),
            version: VERSION,
            heightmap: Default::default(),
            position: Default::default(),
            physical_size: Default::default(),
            height_map_size: Default::default(),
            block_size: Default::default(),
            grid_position: Default::default(),
            layer_masks: Default::default(),
        }
    }
}

impl Chunk {
    /// Returns position of the chunk in local 2D coordinates relative to origin of the
    /// terrain.
    pub fn local_position(&self) -> Vector2<f32> {
        map_to_local(self.position)
    }

    /// Returns a reference to height map.
    pub fn heightmap(&self) -> &[f32] {
        &self.heightmap
    }

    /// Sets new height map. New height map must be equal with size of current.
    pub fn set_heightmap(&mut self, heightmap: Vec<f32>) {
        assert_eq!(self.heightmap.len(), heightmap.len());
        self.heightmap = heightmap;
    }

    /// Returns the size of the chunk in meters.
    pub fn physical_size(&self) -> Vector2<f32> {
        self.physical_size
    }

    /// Returns amount of pixels in the height map along each dimension.
    pub fn height_map_size(&self) -> Vector2<u32> {
        self.height_map_size
    }

    pub fn debug_draw(&self, transform: &Matrix4<f32>, ctx: &mut SceneDrawingContext) {
        let transform = *transform * Matrix4::new_translation(&self.position);

        self.quad_tree
            .debug_draw(&transform, self.height_map_size, self.physical_size, ctx)
    }
}

fn map_to_local(v: Vector3<f32>) -> Vector2<f32> {
    // Terrain is a XZ oriented surface so we can map X -> X, Z -> Y
    Vector2::new(v.x, v.z)
}

/// Ray-terrain intersection result.
#[derive(Debug)]
pub struct TerrainRayCastResult {
    /// World-space position of impact point.
    pub position: Vector3<f32>,
    /// World-space normal of triangle at impact point.
    pub normal: Vector3<f32>,
    /// Index of a chunk that was hit.
    pub chunk_index: usize,
    /// Time of impact. Usually in [0; 1] range where 0 - origin of a ray, 1 - its end.
    pub toi: f32,
}

/// Terrain is a height field where each point has fixed coordinates in XZ plane, but variable
/// Y coordinate. It can be used to create landscapes. It supports multiple layers, where each
/// layer has its own material and mask.
#[derive(Debug, Default, Reflect, Clone)]
pub struct Terrain {
    base: Base,

    #[reflect(setter = "set_layers")]
    layers: InheritableVariable<Vec<Layer>>,

    #[reflect(setter = "set_decal_layer_index")]
    decal_layer_index: InheritableVariable<u8>,

    #[reflect(
        min_value = 0.001,
        description = "Size of the chunk, in meters.",
        setter = "set_chunk_size"
    )]
    chunk_size: InheritableVariable<Vector2<f32>>,

    #[reflect(
        step = 1.0,
        description = "Min and max 'coordinate' of chunks along X axis.",
        setter = "set_width_chunks"
    )]
    width_chunks: InheritableVariable<Range<i32>>,

    #[reflect(
        step = 1.0,
        description = "Min and max 'coordinate' of chunks along Y axis.",
        setter = "set_length_chunks"
    )]
    length_chunks: InheritableVariable<Range<i32>>,

    #[reflect(
        min_value = 2.0,
        step = 1.0,
        description = "Size of the height map per chunk, in pixels. Warning: any change to this value will result in resampling!",
        setter = "set_height_map_size"
    )]
    height_map_size: InheritableVariable<Vector2<u32>>,

    #[reflect(min_value = 8.0, step = 1.0)]
    block_size: InheritableVariable<Vector2<u32>>,

    #[reflect(
        min_value = 1.0,
        step = 1.0,
        description = "Size of the blending mask per chunk, in pixels. Warning: any change to this value will result in resampling!",
        setter = "set_mask_size"
    )]
    mask_size: InheritableVariable<Vector2<u32>>,

    #[reflect(read_only)]
    chunks: InheritableVariable<Vec<Chunk>>,

    #[reflect(hidden)]
    bounding_box_dirty: Cell<bool>,

    #[reflect(hidden)]
    bounding_box: Cell<AxisAlignedBoundingBox>,

    #[reflect(hidden)]
    geometry: TerrainGeometry,

    #[reflect(hidden)]
    version: u8,
}

#[derive(Visit, Default)]
struct OldLayer {
    pub material: SharedMaterial,
    pub mask_property_name: String,
    pub chunk_masks: Vec<Texture>,
}

impl Visit for Terrain {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut version = if region.is_reading() {
            0u8
        } else {
            self.version
        };
        let _ = version.visit("Version", &mut region);

        match version {
            0 => {
                // Old version.
                self.base.visit("Base", &mut region)?;
                self.decal_layer_index
                    .visit("DecalLayerIndex", &mut region)?;

                let mut layers = InheritableVariable::<Vec<OldLayer>>::new(Default::default());
                layers.visit("Layers", &mut region)?;

                let mut width = 0.0f32;
                width.visit("Width", &mut region)?;
                let mut length = 0.0f32;
                length.visit("Length", &mut region)?;

                let mut mask_resolution = 0.0f32;
                mask_resolution.visit("MaskResolution", &mut region)?;

                let mut height_map_resolution = 0.0f32;
                height_map_resolution.visit("HeightMapResolution", &mut region)?;

                let mut chunks = Vec::<Chunk>::new();
                chunks.visit("Chunks", &mut region)?;

                let mut width_chunks = 0u32;
                width_chunks.visit("WidthChunks", &mut region)?;
                self.width_chunks = (0..(width_chunks as i32)).into();

                let mut length_chunks = 0u32;
                length_chunks.visit("LengthChunks", &mut region)?;
                self.length_chunks = (0..(length_chunks as i32)).into();

                self.chunk_size =
                    Vector2::new(width / width_chunks as f32, length / length_chunks as f32).into();

                self.mask_size = Vector2::new(
                    (self.chunk_size.x * mask_resolution) as u32,
                    (self.chunk_size.y * mask_resolution) as u32,
                )
                .into();
                self.height_map_size = Vector2::new(
                    (self.chunk_size.x * height_map_resolution) as u32,
                    (self.chunk_size.y * height_map_resolution) as u32,
                )
                .into();

                // Convert to new format.
                for mut layer in layers.take() {
                    for chunk in chunks.iter_mut().rev() {
                        chunk.layer_masks.push(layer.chunk_masks.pop().unwrap());
                    }

                    self.layers.push(Layer {
                        material: layer.material,
                        mask_property_name: layer.mask_property_name,
                    })
                }

                self.chunks = chunks.into();
            }
            VERSION => {
                // Current version
                self.base.visit("Base", &mut region)?;
                self.layers.visit("Layers", &mut region)?;
                self.decal_layer_index
                    .visit("DecalLayerIndex", &mut region)?;
                self.chunk_size.visit("ChunkSize", &mut region)?;
                self.width_chunks.visit("WidthChunks", &mut region)?;
                self.length_chunks.visit("LengthChunks", &mut region)?;
                self.height_map_size.visit("HeightMapSize", &mut region)?;
                let _ = self.block_size.visit("BlockSize", &mut region);
                self.mask_size.visit("MaskSize", &mut region)?;
                self.chunks.visit("Chunks", &mut region)?;
            }
            _ => (),
        }

        if region.is_reading() {
            self.geometry = TerrainGeometry::new(*self.block_size);
        }

        Ok(())
    }
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

impl TypeUuidProvider for Terrain {
    fn type_uuid() -> Uuid {
        uuid!("4b0a7927-bcd8-41a3-949a-dd10fba8e16a")
    }
}

impl Terrain {
    /// Returns chunk size in meters.
    pub fn chunk_size(&self) -> Vector2<f32> {
        *self.chunk_size
    }

    /// Sets new chunk size of the terrain (in meters). All chunks in the terrain will be repositioned and their
    /// geometry will be rebuilt.
    pub fn set_chunk_size(&mut self, chunk_size: Vector2<f32>) -> Vector2<f32> {
        let old = *self.chunk_size;
        self.chunk_size.set_value_and_mark_modified(chunk_size);

        // Re-position each chunk according to its position on the grid.
        for (z, iy) in (*self.length_chunks)
            .clone()
            .zip(0..self.length_chunks.len())
        {
            for (x, ix) in (*self.width_chunks).clone().zip(0..self.width_chunks.len()) {
                let position = Vector3::new(
                    x as f32 * self.chunk_size.x,
                    0.0,
                    z as f32 * self.chunk_size.y,
                );

                let chunk = &mut self.chunks[iy * self.width_chunks.len() + ix];
                chunk.position = position;
                chunk.physical_size = chunk_size;
            }
        }
        old
    }

    /// Returns height map dimensions along each axis.
    pub fn height_map_size(&self) -> Vector2<u32> {
        *self.height_map_size
    }

    /// Sets new size of the height map for every chunk. Heightmaps in every chunk will be resampled which may
    /// cause precision loss if the size was decreased.
    pub fn set_height_map_size(&mut self, height_map_size: Vector2<u32>) -> Vector2<u32> {
        let old = *self.height_map_size;
        self.resize_height_maps(height_map_size);
        old
    }

    /// Returns amount of pixels along each axis of the layer blending mask.
    pub fn mask_size(&self) -> Vector2<u32> {
        *self.mask_size
    }

    /// Sets new size of the layer blending mask in pixels. Every layer mask will be resampled which may cause
    /// precision loss if the size was decreased.
    pub fn set_mask_size(&mut self, mask_size: Vector2<u32>) -> Vector2<u32> {
        let old = *self.mask_size;
        self.resize_masks(mask_size);
        old
    }

    /// Returns a numeric range along width axis which defines start and end chunk indices on a chunks grid.
    pub fn width_chunks(&self) -> Range<i32> {
        (*self.width_chunks).clone()
    }

    /// Sets amount of chunks along width axis.
    pub fn set_width_chunks(&mut self, chunks: Range<i32>) -> Range<i32> {
        let old = (*self.width_chunks).clone();
        self.resize(chunks, self.length_chunks());
        old
    }

    /// Returns a numeric range along length axis which defines start and end chunk indices on a chunks grid.
    pub fn length_chunks(&self) -> Range<i32> {
        (*self.length_chunks).clone()
    }

    /// Sets amount of chunks along length axis.
    pub fn set_length_chunks(&mut self, chunks: Range<i32>) -> Range<i32> {
        let old = (*self.length_chunks).clone();
        self.resize(self.width_chunks(), chunks);
        old
    }

    /// Sets new chunks ranges for each axis of the terrain. This function automatically adds new chunks if you're
    /// increasing size of the terrain and removes existing if you shrink the terrain.
    pub fn resize(&mut self, width_chunks: Range<i32>, length_chunks: Range<i32>) {
        let mut chunks = self
            .chunks
            .drain(..)
            .map(|c| (c.grid_position, c))
            .collect::<HashMap<_, _>>();

        self.width_chunks.set_value_and_mark_modified(width_chunks);
        self.length_chunks
            .set_value_and_mark_modified(length_chunks);

        for z in (*self.length_chunks).clone() {
            for x in (*self.width_chunks).clone() {
                let chunk = if let Some(existing_chunk) = chunks.remove(&Vector2::new(x, z)) {
                    // Put existing chunk back at its position.
                    existing_chunk
                } else {
                    // Create new chunk.
                    let heightmap =
                        vec![0.0; (self.height_map_size.x * self.height_map_size.y) as usize];
                    let new_chunk = Chunk {
                        quad_tree: QuadTree::new(&heightmap, *self.block_size, *self.block_size),
                        heightmap,
                        position: Vector3::new(
                            x as f32 * self.chunk_size.x,
                            0.0,
                            z as f32 * self.chunk_size.y,
                        ),
                        physical_size: *self.chunk_size,
                        height_map_size: *self.height_map_size,
                        block_size: *self.block_size,
                        grid_position: Vector2::new(x, z),
                        layer_masks: self
                            .layers
                            .iter()
                            .enumerate()
                            .map(|(i, _)| {
                                create_layer_mask(
                                    self.mask_size.x,
                                    self.mask_size.y,
                                    if i == 0 { 255 } else { 0 },
                                )
                            })
                            .collect::<Vec<_>>(),
                        version: VERSION,
                    };

                    new_chunk
                };

                self.chunks.push(chunk);
            }
        }

        self.bounding_box_dirty.set(true);
    }

    /// Returns a reference to chunks of the terrain.
    pub fn chunks_ref(&self) -> &[Chunk] {
        &self.chunks
    }

    /// Returns a mutable reference to chunks of the terrain.
    pub fn chunks_mut(&mut self) -> &mut [Chunk] {
        &mut self.chunks
    }

    /// Sets new decal layer index. It defines which decals will be applies to the mesh,
    /// for example iff a decal has index == 0 and a mesh has index == 0, then decals will
    /// be applied. This allows you to apply decals only on needed surfaces.
    pub fn set_decal_layer_index(&mut self, index: u8) -> u8 {
        self.decal_layer_index.set_value_and_mark_modified(index)
    }

    /// Returns current decal index.
    pub fn decal_layer_index(&self) -> u8 {
        *self.decal_layer_index
    }

    /// Projects given 3D point on the surface of terrain and returns 2D vector
    /// expressed in local 2D coordinate system of terrain.
    pub fn project(&self, p: Vector3<f32>) -> Option<Vector2<f32>> {
        project(self.global_transform(), p)
    }

    /// Multi-functional drawing method. It uses given brush to modify terrain, see Brush docs for
    /// more info.
    pub fn draw(&mut self, brush: &Brush) {
        let center = project(self.global_transform(), brush.center).unwrap();

        match brush.mode {
            BrushMode::ModifyHeightMap { amount } => {
                for chunk in self.chunks.iter_mut() {
                    for iy in 0..chunk.height_map_size.y {
                        let kz = iy as f32 / (chunk.height_map_size.y - 1) as f32;
                        for ix in 0..chunk.height_map_size.y {
                            let kx = ix as f32 / (chunk.height_map_size.x - 1) as f32;

                            let pixel_position = chunk.local_position()
                                + Vector2::new(
                                    kx * chunk.physical_size.x,
                                    kz * chunk.physical_size.y,
                                );

                            let k = match brush.shape {
                                BrushShape::Circle { radius } => {
                                    1.0 - ((center - pixel_position).norm() / radius).powf(2.0)
                                }
                                BrushShape::Rectangle { .. } => 1.0,
                            };

                            if brush.shape.contains(center, pixel_position) {
                                chunk.heightmap[(iy * chunk.height_map_size.x + ix) as usize] +=
                                    k * amount;
                            }
                        }
                    }
                }
            }
            BrushMode::DrawOnMask { layer, alpha } => {
                let alpha = alpha.clamp(-1.0, 1.0);

                for chunk in self.chunks.iter_mut() {
                    let chunk_position = chunk.local_position();
                    let mut texture_data = chunk.layer_masks[layer].data_ref();
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

                            let pixel_position = chunk_position
                                + Vector2::new(
                                    kx * chunk.physical_size.x,
                                    kz * chunk.physical_size.y,
                                );

                            let k = match brush.shape {
                                BrushShape::Circle { radius } => {
                                    1.0 - ((center - pixel_position).norm() / radius).powf(4.0)
                                }
                                BrushShape::Rectangle { .. } => 1.0,
                            };

                            if brush.shape.contains(center, pixel_position) {
                                // We can draw on mask directly, without any problems because it has R8 pixel format.
                                let data = texture_data_mut.data_mut();
                                let pixel = &mut data[z * texture_width + x];
                                *pixel = (*pixel as f32 + k * alpha * 255.0).min(255.0) as u8;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Casts a ray and looks for intersections with the terrain. This method collects all results in
    /// given array with optional sorting by time-of-impact.
    ///
    /// # Performance
    ///
    /// This method isn't well optimized, it could be optimized 2-5x times. This is a TODO for now.
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
                let cell_width = chunk.physical_size.x / (chunk.height_map_size.x - 1) as f32;
                let cell_length = chunk.physical_size.y / (chunk.height_map_size.y - 1) as f32;

                for iy in 0..chunk.height_map_size.y {
                    let kz = iy as f32 / (chunk.height_map_size.y - 1) as f32;
                    let next_iy = iy + 1;

                    for ix in 0..chunk.height_map_size.x {
                        let kx = ix as f32 / (chunk.height_map_size.x - 1) as f32;
                        let next_ix = ix + 1;

                        let pixel_position = chunk.local_position()
                            + Vector2::new(kx * chunk.physical_size.x, kz * chunk.physical_size.y);

                        let cell_bounds =
                            Rect::new(pixel_position.x, pixel_position.y, cell_width, cell_length);

                        if ray_rect_intersection(cell_bounds, origin_proj, dir_proj).is_some() {
                            // If we have 2D intersection, go back in 3D and do precise intersection
                            // check.
                            if next_ix < chunk.height_map_size.x
                                && next_iy < chunk.height_map_size.y
                            {
                                let i0 = (iy * chunk.height_map_size.x + ix) as usize;
                                let i1 = ((iy + 1) * chunk.height_map_size.x + ix) as usize;
                                let i2 = ((iy + 1) * chunk.height_map_size.x + ix + 1) as usize;
                                let i3 = (iy * chunk.height_map_size.x + ix + 1) as usize;

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

    /// Sets new terrain layers.
    pub fn set_layers(&mut self, layers: Vec<Layer>) -> Vec<Layer> {
        self.layers.set_value_and_mark_modified(layers)
    }

    /// Returns a reference to a slice with layers of the terrain.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Returns a mutable reference to a slice with layers of the terrain.
    pub fn layers_mut(&mut self) -> &mut [Layer] {
        self.layers.get_value_mut_and_mark_modified()
    }

    /// Adds new layer to the chunk. It is possible to have different layer count per chunk
    /// in the same terrain, however it seems to not have practical usage, so try to keep
    /// equal layer count per each chunk in your terrains.
    pub fn add_layer(&mut self, layer: Layer, masks: Vec<Texture>) {
        self.insert_layer(layer, masks, self.layers.len())
    }

    /// Removes a layer at the given index together with its respective blending masks from each chunk.
    pub fn remove_layer(&mut self, layer_index: usize) -> (Layer, Vec<Texture>) {
        let layer = self
            .layers
            .get_value_mut_and_mark_modified()
            .remove(layer_index);
        let mut layer_masks = Vec::new();
        for chunk in self.chunks_mut() {
            layer_masks.push(chunk.layer_masks.remove(layer_index));
        }
        (layer, layer_masks)
    }

    /// Removes last terrain layer together with its respective blending masks from each chunk.
    pub fn pop_layer(&mut self) -> Option<(Layer, Vec<Texture>)> {
        if self.layers.is_empty() {
            None
        } else {
            Some(self.remove_layer(self.layers.len() - 1))
        }
    }

    /// Inserts the layer at the given index together with its blending masks for each chunk.
    pub fn insert_layer(&mut self, layer: Layer, mut masks: Vec<Texture>, index: usize) {
        self.layers
            .get_value_mut_and_mark_modified()
            .insert(index, layer);

        for chunk in self.chunks.iter_mut().rev() {
            if let Some(mask) = masks.pop() {
                chunk.layer_masks.insert(index, mask);
            } else {
                chunk.layer_masks.insert(
                    index,
                    create_layer_mask(
                        self.mask_size.x,
                        self.mask_size.y,
                        if index == 0 { 255 } else { 0 },
                    ),
                )
            }
        }
    }

    fn resize_masks(&mut self, mut new_size: Vector2<u32>) {
        new_size = new_size.sup(&Vector2::repeat(1));

        for chunk in self.chunks.iter_mut() {
            for mask in chunk.layer_masks.iter_mut() {
                let data = mask.data_ref();

                let mask_image = ImageBuffer::<Luma<u8>, Vec<u8>>::from_vec(
                    self.mask_size.x,
                    self.mask_size.y,
                    data.data().to_vec(),
                )
                .unwrap();

                let resampled_mask_image = image::imageops::resize(
                    &mask_image,
                    new_size.x,
                    new_size.y,
                    FilterType::Lanczos3,
                );

                let new_mask = resampled_mask_image.into_raw();
                let new_mask_texture = Texture::from_bytes(
                    TextureKind::Rectangle {
                        width: new_size.x,
                        height: new_size.y,
                    },
                    data.pixel_kind(),
                    new_mask,
                    true,
                )
                .unwrap();

                drop(data);
                *mask = new_mask_texture;
            }
        }

        self.mask_size.set_value_and_mark_modified(new_size);
    }

    fn resize_height_maps(&mut self, mut new_size: Vector2<u32>) {
        new_size = new_size.sup(&Vector2::repeat(2));

        for chunk in self.chunks.iter_mut() {
            let mut heightmap = std::mem::take(&mut chunk.heightmap);

            let mut max = -f32::MAX;
            for &height in &heightmap {
                if height > max {
                    max = height;
                }
            }

            if max != 0.0 {
                for height in &mut heightmap {
                    *height /= max;
                }
            }

            let heightmap_image = ImageBuffer::<Luma<f32>, Vec<f32>>::from_vec(
                chunk.height_map_size.x,
                chunk.height_map_size.y,
                heightmap,
            )
            .unwrap();

            let resampled_heightmap_image = image::imageops::resize(
                &heightmap_image,
                new_size.x,
                new_size.y,
                FilterType::Lanczos3,
            );

            let mut resampled_heightmap = resampled_heightmap_image.into_raw();

            for height in &mut resampled_heightmap {
                *height *= max;
            }

            chunk.height_map_size = new_size;
            chunk.heightmap = resampled_heightmap;
        }

        self.height_map_size.set_value_and_mark_modified(new_size);
        self.bounding_box_dirty.set(true);
    }

    /// Returns data for rendering (vertex and index buffers).
    pub fn geometry(&self) -> &TerrainGeometry {
        &self.geometry
    }
}

impl NodeTrait for Terrain {
    crate::impl_query_component!();

    /// Returns pre-cached bounding axis-aligned bounding box of the terrain. Keep in mind that
    /// if you're modified terrain, bounding box will be recalculated and it is not fast.
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
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
                Vector3::new(
                    self.chunk_size.x * self.width_chunks.start as f32,
                    max_height,
                    self.chunk_size.y * self.length_chunks.start as f32,
                ),
                Vector3::new(
                    self.chunk_size.x * self.width_chunks.end as f32,
                    max_height,
                    self.chunk_size.y * self.length_chunks.end as f32,
                ),
            );
            self.bounding_box.set(bounding_box);
            self.bounding_box_dirty.set(false);

            bounding_box
        } else {
            self.bounding_box.get()
        }
    }

    /// Returns current **world-space** bounding box.
    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) {
        if !self.global_visibility()
            || !self.is_globally_enabled()
            || !ctx.frustum.is_intersects_aabb(&self.world_bounding_box())
        {
            return;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) && !self.cast_shadows() {
            return;
        }

        for (layer_index, layer) in self.layers().iter().enumerate() {
            for chunk in self.chunks_ref().iter() {
                let levels = (0..chunk.quad_tree.max_level)
                    .map(|n| {
                        ctx.z_far
                            * ((chunk.quad_tree.max_level - n) as f32
                                / chunk.quad_tree.max_level as f32)
                                .powf(3.0)
                    })
                    .collect::<Vec<_>>();

                let transform = self.global_transform() * Matrix4::new_translation(&chunk.position);

                let mut selection = Vec::new();
                chunk.quad_tree.select(
                    &transform,
                    self.height_map_size(),
                    self.chunk_size(),
                    ctx.frustum,
                    *ctx.observer_position,
                    &levels,
                    &mut selection,
                );

                let mut material = (*layer.material.lock()).clone();
                match material.set_property(
                    &ImmutableString::new(&layer.mask_property_name),
                    PropertyValue::Sampler {
                        value: Some(chunk.layer_masks[layer_index].clone()),
                        fallback: Default::default(),
                    },
                ) {
                    Ok(_) => {
                        let material = SharedMaterial::new(material);

                        for node in selection {
                            let transform = transform
                                * Matrix4::new_translation(&Vector3::new(
                                    node.position.x as f32 / self.height_map_size.x as f32
                                        * self.chunk_size.x,
                                    0.0,
                                    node.position.y as f32 / self.height_map_size.y as f32
                                        * self.chunk_size.y,
                                ))
                                * Matrix4::new_nonuniform_scaling(&Vector3::new(
                                    node.size.x as f32 / self.height_map_size.x as f32
                                        * self.chunk_size.x,
                                    0.0,
                                    node.size.y as f32 / self.height_map_size.y as f32
                                        * self.chunk_size.y,
                                ));

                            if node.is_draw_full() {
                                ctx.storage.push(
                                    &self.geometry.data,
                                    &material,
                                    RenderPath::Deferred,
                                    self.decal_layer_index(),
                                    layer_index as u64,
                                    SurfaceInstanceData {
                                        world_transform: transform,
                                        bone_matrices: Default::default(),
                                        depth_offset: self.depth_offset_factor(),
                                        blend_shapes_weights: Default::default(),
                                        element_range: ElementRange::Full,
                                    },
                                );
                            } else {
                                for (i, draw_quadrant) in node.active_quadrants.iter().enumerate() {
                                    if *draw_quadrant {
                                        ctx.storage.push(
                                            &self.geometry.data,
                                            &material,
                                            RenderPath::Deferred,
                                            self.decal_layer_index(),
                                            layer_index as u64,
                                            SurfaceInstanceData {
                                                world_transform: transform,
                                                bone_matrices: Default::default(),
                                                depth_offset: self.depth_offset_factor(),
                                                blend_shapes_weights: Default::default(),
                                                element_range: self.geometry.quadrants[i],
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => Log::writeln(
                        MessageKind::Error,
                        format!(
                            "Failed to prepare batch for terrain chunk.\
                                 Unable to set mask texture for terrain material. Reason: {:?}",
                            e
                        ),
                    ),
                }
            }
        }
    }

    fn debug_draw(&self, ctx: &mut SceneDrawingContext) {
        for chunk in self.chunks.iter() {
            chunk.debug_draw(&self.global_transform(), ctx)
        }
    }
}

/// Shape of a brush.
#[derive(Copy, Clone, Reflect, Debug)]
pub enum BrushShape {
    /// Circle with given radius.
    Circle {
        /// Radius of the circle.
        radius: f32,
    },
    /// Rectangle with given width and height.
    Rectangle {
        /// Width of the rectangle.
        width: f32,
        /// Length of the rectangle.
        length: f32,
    },
}

impl BrushShape {
    fn contains(&self, brush_center: Vector2<f32>, pixel_position: Vector2<f32>) -> bool {
        match *self {
            BrushShape::Circle { radius } => (brush_center - pixel_position).norm() < radius,
            BrushShape::Rectangle { width, length } => Rect::new(
                brush_center.x - width * 0.5,
                brush_center.y - length * 0.5,
                width,
                length,
            )
            .contains(pixel_position),
        }
    }
}

/// Paint mode of a brush. It defines operation that will be performed on the terrain.
#[derive(Clone, PartialEq, PartialOrd, Reflect, Debug)]
pub enum BrushMode {
    /// Modifies height map.
    ModifyHeightMap {
        /// An offset for height map.
        amount: f32,
    },
    /// Draws on a given layer.
    DrawOnMask {
        /// A layer to draw on.
        layer: usize,
        /// A value to put on mask. Range is [-1.0; 1.0] where negative values "erase"
        /// values from mask, and positive - paints.
        alpha: f32,
    },
}

/// Brush is used to modify terrain. It supports multiple shapes and modes.
#[derive(Clone, Reflect, Debug)]
pub struct Brush {
    /// Center of the brush.
    #[reflect(hidden)]
    pub center: Vector3<f32>,
    /// Shape of the brush.
    pub shape: BrushShape,
    /// Paint mode of the brush.
    pub mode: BrushMode,
}

/// Terrain builder allows you to quickly build a terrain with required features.
pub struct TerrainBuilder {
    base_builder: BaseBuilder,
    chunk_size: Vector2<f32>,
    mask_size: Vector2<u32>,
    width_chunks: Range<i32>,
    length_chunks: Range<i32>,
    height_map_size: Vector2<u32>,
    block_size: Vector2<u32>,
    layers: Vec<Layer>,
    decal_layer_index: u8,
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

impl TerrainBuilder {
    /// Creates new builder instance.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            chunk_size: Vector2::new(16.0, 16.0),
            width_chunks: 0..2,
            length_chunks: 0..2,
            mask_size: Vector2::new(256, 256),
            height_map_size: Vector2::new(256, 256),
            block_size: Vector2::new(32, 32),
            layers: Default::default(),
            decal_layer_index: 0,
        }
    }

    /// Sets desired chunk size in meters.
    pub fn with_chunk_size(mut self, size: Vector2<f32>) -> Self {
        self.chunk_size = size;
        self
    }

    /// Sets desired mask size in pixels.
    pub fn with_mask_size(mut self, size: Vector2<u32>) -> Self {
        self.mask_size = size;
        self
    }

    /// Sets desired chunk amount along width axis.
    pub fn with_width_chunks(mut self, width_chunks: Range<i32>) -> Self {
        self.width_chunks = width_chunks;
        self
    }

    /// Sets desired chunk amount along length axis.
    pub fn with_length_chunks(mut self, length_chunks: Range<i32>) -> Self {
        self.length_chunks = length_chunks;
        self
    }

    /// Sets desired height map size in pixels.
    pub fn with_height_map_size(mut self, size: Vector2<u32>) -> Self {
        self.height_map_size = size;
        self
    }

    /// Sets desired layers that will be used for each chunk in the terrain.
    pub fn with_layers(mut self, layers: Vec<Layer>) -> Self {
        self.layers = layers;
        self
    }

    /// Sets desired decal layer index.
    pub fn with_decal_layer_index(mut self, decal_layer_index: u8) -> Self {
        self.decal_layer_index = decal_layer_index;
        self
    }

    /// Sets desired block size. Block - is a smallest renderable piece of terrain which will be used for
    /// level-of-detail functionality.
    pub fn with_block_size(mut self, block_size: Vector2<u32>) -> Self {
        self.block_size = block_size;
        self
    }

    /// Build terrain node.
    pub fn build_node(self) -> Node {
        let mut chunks = Vec::new();
        for z in self.length_chunks.clone() {
            for x in self.width_chunks.clone() {
                let heightmap =
                    vec![0.0; (self.height_map_size.x * self.height_map_size.y) as usize];
                let chunk = Chunk {
                    quad_tree: QuadTree::new(&heightmap, self.height_map_size, self.block_size),
                    height_map_size: self.height_map_size,
                    heightmap,
                    position: Vector3::new(
                        x as f32 * self.chunk_size.x,
                        0.0,
                        z as f32 * self.chunk_size.y,
                    ),
                    physical_size: self.chunk_size,
                    grid_position: Vector2::new(x, z),
                    layer_masks: self
                        .layers
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            create_layer_mask(
                                self.mask_size.x,
                                self.mask_size.y,
                                // Base layer is opaque, every other by default - transparent.
                                if i == 0 { 255 } else { 0 },
                            )
                        })
                        .collect::<Vec<_>>(),
                    version: VERSION,
                    block_size: self.block_size,
                };

                chunks.push(chunk);
            }
        }

        let terrain = Terrain {
            chunk_size: self.chunk_size.into(),
            base: self.base_builder.build_base(),
            layers: self.layers.into(),
            chunks: chunks.into(),
            bounding_box_dirty: Cell::new(true),
            bounding_box: Default::default(),
            mask_size: self.mask_size.into(),
            height_map_size: self.height_map_size.into(),
            width_chunks: self.width_chunks.into(),
            length_chunks: self.length_chunks.into(),
            decal_layer_index: self.decal_layer_index.into(),
            version: VERSION,
            geometry: TerrainGeometry::new(self.block_size),
            block_size: self.block_size.into(),
        };
        Node::new(terrain)
    }

    /// Builds terrain node and adds it to given graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
