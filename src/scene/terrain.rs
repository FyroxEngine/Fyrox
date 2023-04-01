//! Everything related to terrains.

use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        arrayvec::ArrayVec,
        math::{
            aabb::AxisAlignedBoundingBox, ray::Ray, ray_rect_intersection, Rect, TriangleDefinition,
        },
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::{prelude::*, PodVecView},
    },
    material::SharedMaterial,
    resource::texture::{Texture, TextureKind, TexturePixelKind, TextureWrapMode},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::{
            buffer::{TriangleBuffer, VertexBuffer},
            surface::{SurfaceData, SurfaceSharedData},
            vertex::StaticVertex,
        },
        node::{Node, NodeTrait, TypeUuidProvider},
    },
};
use image::{imageops::FilterType, ImageBuffer, Luma};
use std::{
    cell::Cell,
    cmp::Ordering,
    collections::HashMap,
    ops::{Deref, DerefMut, Range},
};

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
#[derive(Debug, Clone)]
pub struct Chunk {
    heightmap: Vec<f32>,
    position: Vector3<f32>,
    physical_size: Vector2<f32>,
    height_map_size: Vector2<u32>,
    surface_data: SurfaceSharedData,
    grid_position: Vector2<i32>,
    pub layer_masks: Vec<Texture>,
}

// Manual implementation of the trait because we need to serialize heightmap differently.
impl Visit for Chunk {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut view = PodVecView::from_pod_vec(&mut self.heightmap);
        view.visit("Heightmap", &mut region)?;

        self.position.visit("Position", &mut region)?;
        self.physical_size.visit("PhysicalSize", &mut region)?;
        self.height_map_size.visit("HeightMapSize", &mut region)?;
        self.layer_masks.visit("LayerMasks", &mut region)?;
        self.grid_position.visit("GridPosition", &mut region)?;
        // self.surface_data is not serialized.

        if region.is_reading() {
            self.rebuild_geometry();
        }

        Ok(())
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            heightmap: Default::default(),
            position: Default::default(),
            physical_size: Default::default(),
            height_map_size: Default::default(),
            surface_data: make_surface_data(),
            grid_position: Default::default(),
            layer_masks: Default::default(),
        }
    }
}

impl Chunk {
    /// Updates vertex and index buffers needed for rendering. In most cases there is no need
    /// to call this method manually, engine will automatically call it when needed.
    pub fn rebuild_geometry(&mut self) {
        let mut surface_data = self.surface_data.lock();
        surface_data.clear();

        assert!(self.height_map_size.x > 1);
        assert!(self.height_map_size.y > 1);

        let mut vertex_buffer_mut = surface_data.vertex_buffer.modify();
        // Form vertex buffer.
        for iy in 0..self.height_map_size.y {
            let kz = iy as f32 / ((self.height_map_size.y - 1) as f32);
            let pz = self.position.z + kz * self.physical_size.y;

            for x in 0..self.height_map_size.x {
                let index = iy * self.height_map_size.x + x;
                let height = self.heightmap[index as usize];
                let kx = x as f32 / ((self.height_map_size.x - 1) as f32);

                let px = self.position.x + kx * self.physical_size.x;
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
        for iy in 0..self.height_map_size.y - 1 {
            let iy_next = iy + 1;
            for x in 0..self.height_map_size.x - 1 {
                let x_next = x + 1;

                let i0 = iy * self.height_map_size.x + x;
                let i1 = iy_next * self.height_map_size.x + x;
                let i2 = iy_next * self.height_map_size.x + x_next;
                let i3 = iy * self.height_map_size.x + x_next;

                geometry_buffer_mut.push(TriangleDefinition([i0, i1, i2]));
                geometry_buffer_mut.push(TriangleDefinition([i2, i3, i0]));
            }
        }
        drop(geometry_buffer_mut);

        surface_data.calculate_normals().unwrap();
        surface_data.calculate_tangents().unwrap();
    }

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
        self.rebuild_geometry();
    }

    /// Returns data for rendering (vertex and index buffers).
    pub fn data(&self) -> SurfaceSharedData {
        self.surface_data.clone()
    }

    pub fn physical_size(&self) -> Vector2<f32> {
        self.physical_size
    }

    pub fn height_map_size(&self) -> Vector2<u32> {
        self.height_map_size
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
///
/// # Prefab inheritance notes
///
/// There is very limited inheritance possible, only layers, decal layer index and cast shadows flag
/// are inheritable. You cannot inherit width, height, chunks and other things because these cannot
/// be modified at runtime because changing width (for example) will invalidate the entire height
/// map which makes runtime modification useless.  
#[derive(Visit, Debug, Default, Reflect, Clone)]
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
    chunk_size: Vector2<f32>,

    #[reflect(
        step = 1.0,
        description = "Min and max 'coordinate' of chunks along X axis.",
        setter = "set_width_chunks"
    )]
    width_chunks: Range<i32>,

    #[reflect(
        step = 1.0,
        description = "Min and max 'coordinate' of chunks along Y axis.",
        setter = "set_length_chunks"
    )]
    length_chunks: Range<i32>,

    #[reflect(
        min_value = 2.0,
        step = 1.0,
        description = "Size of the height map per chunk, in pixels. Warning: any change to this value will result in resampling!",
        setter = "set_height_map_size"
    )]
    height_map_size: Vector2<u32>,

    #[reflect(
        min_value = 1.0,
        step = 1.0,
        description = "Size of the blending mask per chunk, in pixels. Warning: any change to this value will result in resampling!"
    )]
    mask_size: Vector2<u32>,

    #[reflect(hidden)]
    chunks: Vec<Chunk>,

    #[reflect(hidden)]
    bounding_box_dirty: Cell<bool>,

    #[reflect(hidden)]
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

impl TypeUuidProvider for Terrain {
    fn type_uuid() -> Uuid {
        uuid!("4b0a7927-bcd8-41a3-949a-dd10fba8e16a")
    }
}

impl Terrain {
    pub fn chunk_size(&self) -> Vector2<f32> {
        self.chunk_size
    }

    pub fn set_chunk_size(&mut self, chunk_size: Vector2<f32>) {
        self.chunk_size = chunk_size;

        // Re-position each chunk according to its position on the grid.
        for (z, iy) in self.length_chunks.clone().zip(0..self.length_chunks.len()) {
            for (x, ix) in self.width_chunks.clone().zip(0..self.width_chunks.len()) {
                let position = Vector3::new(
                    x as f32 * self.chunk_size.x,
                    0.0,
                    z as f32 * self.chunk_size.y,
                );

                let chunk = &mut self.chunks[iy * self.width_chunks.len() + ix];
                chunk.position = position;
                chunk.physical_size = chunk_size;
                chunk.rebuild_geometry();
            }
        }
    }

    pub fn width_chunks(&self) -> Range<i32> {
        self.width_chunks.clone()
    }

    pub fn length_chunks(&self) -> Range<i32> {
        self.length_chunks.clone()
    }

    pub fn height_map_size(&self) -> Vector2<u32> {
        self.height_map_size
    }

    pub fn set_height_map_size(&mut self, height_map_size: Vector2<u32>) {
        self.resample_height_maps(height_map_size);
    }

    pub fn mask_size(&self) -> Vector2<u32> {
        self.mask_size
    }

    pub fn set_mask_size(&mut self, mask_size: Vector2<u32>) {
        self.mask_size = mask_size;
        self.resample_masks();
    }

    pub fn set_width_chunks(&mut self, chunks: Range<i32>) {
        self.resize(chunks, self.length_chunks.clone());
    }

    pub fn set_length_chunks(&mut self, chunks: Range<i32>) {
        self.resize(self.width_chunks.clone(), chunks);
    }

    pub fn resize(&mut self, width_chunks: Range<i32>, length_chunks: Range<i32>) {
        let mut chunks = self
            .chunks
            .drain(..)
            .map(|c| (c.grid_position, c))
            .collect::<HashMap<_, _>>();

        self.width_chunks = width_chunks;
        self.length_chunks = length_chunks;

        for z in self.length_chunks.clone() {
            for x in self.width_chunks.clone() {
                let chunk = if let Some(existing_chunk) = chunks.remove(&Vector2::new(x, z)) {
                    // Put existing chunk back at its position.
                    existing_chunk
                } else {
                    // Create new chunk.
                    let mut new_chunk = Chunk {
                        heightmap: vec![
                            0.0;
                            (self.height_map_size.x * self.height_map_size.y) as usize
                        ],
                        position: Vector3::new(
                            x as f32 * self.chunk_size.x,
                            0.0,
                            z as f32 * self.chunk_size.y,
                        ),
                        physical_size: self.chunk_size,
                        height_map_size: self.height_map_size,
                        surface_data: make_surface_data(),
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
                    };

                    new_chunk.rebuild_geometry();

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
                    let mut modified = false;

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

                                modified = true;
                            }
                        }
                    }

                    if modified {
                        chunk.rebuild_geometry();
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
    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.get_value_mut_and_mark_modified().push(layer);
    }

    /// Removes given layers from the terrain.
    pub fn remove_layer(&mut self, layer: usize) -> Layer {
        self.layers.get_value_mut_and_mark_modified().remove(layer)
    }

    /// Tries to remove last layer from the terrain.
    pub fn pop_layer(&mut self) -> Option<Layer> {
        self.layers.get_value_mut_and_mark_modified().pop()
    }

    /// Inserts new layer at given position in the terrain.
    pub fn insert_layer(&mut self, layer: Layer, index: usize) {
        self.layers
            .get_value_mut_and_mark_modified()
            .insert(index, layer)
    }

    fn resample_masks(&mut self) {
        // TODO
    }

    fn resample_height_maps(&mut self, mut new_size: Vector2<u32>) {
        new_size = new_size.sup(&Vector2::repeat(2));

        for chunk in self.chunks.iter_mut() {
            let mut heightmap = std::mem::take(&mut chunk.heightmap);

            let mut max = -f32::MAX;
            for &height in &heightmap {
                if height > max {
                    max = height;
                }
            }

            for height in &mut heightmap {
                *height /= max;
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
            chunk.rebuild_geometry();
        }

        self.height_map_size = new_size;
        self.bounding_box_dirty.set(true);
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

/// Layer definition for a terrain builder.
pub struct LayerDefinition {
    /// Material of the layer.
    pub material: SharedMaterial,

    /// Name of the mask sampler in the material. It should be `maskTexture` if standard material shader
    /// is used.
    ///
    /// # Implementation details
    ///
    /// It will be used in the renderer to set appropriate chunk mask to the copy of the material.
    pub mask_property_name: String,
}

/// Terrain builder allows you to quickly build a terrain with required features.
pub struct TerrainBuilder {
    base_builder: BaseBuilder,
    chunk_size: Vector2<f32>,
    mask_size: Vector2<u32>,
    width_chunks: Range<i32>,
    length_chunks: Range<i32>,
    height_map_size: Vector2<u32>,
    layers: Vec<LayerDefinition>,
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

fn make_surface_data() -> SurfaceSharedData {
    SurfaceSharedData::new(SurfaceData::new(
        VertexBuffer::new::<StaticVertex>(0, StaticVertex::layout(), vec![]).unwrap(),
        TriangleBuffer::default(),
        false,
    ))
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
            layers: Default::default(),
            decal_layer_index: 0,
        }
    }

    pub fn with_chunk_size(mut self, size: Vector2<f32>) -> Self {
        self.chunk_size = size;
        self
    }

    pub fn with_mask_size(mut self, size: Vector2<u32>) -> Self {
        self.mask_size = size;
        self
    }

    pub fn with_width_chunks(mut self, width_chunks: Range<i32>) -> Self {
        self.width_chunks = width_chunks;
        self
    }

    pub fn with_length_chunks(mut self, length_chunks: Range<i32>) -> Self {
        self.length_chunks = length_chunks;
        self
    }

    pub fn with_height_map_size(mut self, size: Vector2<u32>) -> Self {
        self.height_map_size = size;
        self
    }

    /// Sets desired layers that will be used for each chunk in the terrain.
    pub fn with_layers(mut self, layers: Vec<LayerDefinition>) -> Self {
        self.layers = layers;
        self
    }

    /// Sets desired decal layer index.
    pub fn with_decal_layer_index(mut self, decal_layer_index: u8) -> Self {
        self.decal_layer_index = decal_layer_index;
        self
    }

    /// Build terrain node.
    pub fn build_node(self) -> Node {
        let mut chunks = Vec::new();
        for z in self.length_chunks.clone() {
            for x in self.width_chunks.clone() {
                let mut chunk = Chunk {
                    height_map_size: self.height_map_size,
                    heightmap: vec![
                        0.0;
                        (self.height_map_size.x * self.height_map_size.y) as usize
                    ],
                    position: Vector3::new(
                        x as f32 * self.chunk_size.x,
                        0.0,
                        z as f32 * self.chunk_size.y,
                    ),
                    physical_size: self.chunk_size,
                    surface_data: make_surface_data(),
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
                };

                chunk.rebuild_geometry();

                chunks.push(chunk);
            }
        }

        let terrain = Terrain {
            chunk_size: self.chunk_size,
            base: self.base_builder.build_base(),
            layers: self
                .layers
                .into_iter()
                .map(|definition| Layer {
                    material: definition.material,
                    mask_property_name: definition.mask_property_name,
                })
                .collect::<Vec<_>>()
                .into(),
            chunks,
            bounding_box_dirty: Cell::new(true),
            bounding_box: Default::default(),
            mask_size: self.mask_size,
            height_map_size: self.height_map_size,
            width_chunks: self.width_chunks,
            length_chunks: self.length_chunks,
            decal_layer_index: self.decal_layer_index.into(),
        };

        Node::new(terrain)
    }

    /// Builds terrain node and adds it to given graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
