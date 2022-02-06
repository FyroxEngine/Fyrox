//! Everything related to terrains.

use crate::engine::resource_manager::ResourceManager;
use crate::scene::variable::{InheritError, TemplateVariable};
use crate::scene::DirectlyInheritableEntity;
use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        arrayvec::ArrayVec,
        inspect::{Inspect, PropertyInfo},
        math::{
            aabb::AxisAlignedBoundingBox, ray::Ray, ray_rect_intersection, Rect, TriangleDefinition,
        },
        parking_lot::Mutex,
        pool::Handle,
        visitor::{prelude::*, PodVecView},
    },
    impl_directly_inheritable_entity_trait,
    material::Material,
    resource::texture::{Texture, TextureKind, TexturePixelKind, TextureWrapMode},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::{
            buffer::{TriangleBuffer, VertexBuffer},
            surface::SurfaceData,
            vertex::StaticVertex,
        },
        node::Node,
    },
};
use std::{
    cell::Cell,
    cmp::Ordering,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// Layers is a set of textures for rendering + mask texture to exclude some pixels from
/// rendering. Terrain can have as many layers as you want, but each layer slightly decreases
/// performance, so keep amount of layers on reasonable level (1 - 5 should be enough for most
/// cases).
#[derive(Default, Debug, Clone, Visit, Inspect)]
pub struct Layer {
    /// Material of the layer.
    pub material: Arc<Mutex<Material>>,

    /// Name of the mask sampler in the material.
    ///
    /// # Implementation details
    ///
    /// It will be used in the renderer to set appropriate chunk mask to the copy of the material.
    pub mask_property_name: String,

    #[inspect(skip)]
    pub(in crate) chunk_masks: Vec<Texture>,
}

impl Layer {
    /// Returns a slice containing a set of masks for every chunk in the layer.
    pub fn chunk_masks(&self) -> &[Texture] {
        &self.chunk_masks
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
    width: f32,
    length: f32,
    width_point_count: u32,
    length_point_count: u32,
    surface_data: Arc<Mutex<SurfaceData>>,
    dirty: Cell<bool>,
}

// Manual implementation of the trait because we need to serialize heightmap differently.
impl Visit for Chunk {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut view = PodVecView::from_pod_vec(&mut self.heightmap);
        view.visit("Heightmap", visitor)?;

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
    /// Updates vertex and index buffers needed for rendering. In most cases there is no need
    /// to call this method manually, engine will automatically call it when needed.
    pub fn update(&mut self) {
        if self.dirty.get() {
            let mut surface_data = self.surface_data.lock();
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
        self.dirty.set(true);
    }

    /// Returns data for rendering (vertex and index buffers).
    pub fn data(&self) -> Arc<Mutex<SurfaceData>> {
        self.surface_data.clone()
    }

    /// Returns width of height map in dots.
    pub fn width_point_count(&self) -> u32 {
        self.width_point_count
    }

    /// Returns length of height map in dots.
    pub fn length_point_count(&self) -> u32 {
        self.length_point_count
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
#[derive(Visit, Debug, Default, Inspect)]
pub struct Terrain {
    base: Base,

    #[inspect(getter = "Deref::deref")]
    layers: TemplateVariable<Vec<Layer>>,

    #[visit(optional)] // Backward compatibility
    #[inspect(getter = "Deref::deref")]
    decal_layer_index: TemplateVariable<u8>,

    #[inspect(getter = "Deref::deref")]
    cast_shadows: TemplateVariable<bool>,

    #[inspect(read_only)]
    width: f32,
    #[inspect(read_only)]
    length: f32,
    #[inspect(read_only)]
    mask_resolution: f32,
    #[inspect(read_only)]
    height_map_resolution: f32,
    #[inspect(skip)]
    chunks: Vec<Chunk>,
    #[inspect(read_only)]
    width_chunks: u32,
    #[inspect(read_only)]
    length_chunks: u32,
    #[inspect(skip)]
    bounding_box_dirty: Cell<bool>,
    #[inspect(skip)]
    bounding_box: Cell<AxisAlignedBoundingBox>,
}

impl_directly_inheritable_entity_trait!(Terrain;
    layers,
    decal_layer_index,
    cast_shadows
);

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
    /// Returns width of the terrain in local coordinates.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Returns amount of chunks along X axis.
    pub fn width_chunk_count(&self) -> usize {
        self.width_chunks as usize
    }

    /// Returns length of the terrain in local coordinates.
    pub fn length(&self) -> f32 {
        self.length
    }

    /// Returns amount of chunks along Z axis
    pub fn length_chunk_count(&self) -> usize {
        self.length_chunks as usize
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
    pub fn set_decal_layer_index(&mut self, index: u8) {
        self.decal_layer_index.set(index);
    }

    /// Returns current decal index.
    pub fn decal_layer_index(&self) -> u8 {
        *self.decal_layer_index
    }

    /// Returns true if terrain should cast shadows.
    pub fn cast_shadows(&self) -> bool {
        *self.cast_shadows
    }

    /// Sets whether terrain should cast shadows or not.
    pub fn set_cast_shadows(&mut self, value: bool) {
        self.cast_shadows.set(value);
    }

    /// Creates raw copy of the terrain. Do not use this method directly, use
    /// Graph::copy_node.
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
            decal_layer_index: self.decal_layer_index.clone(),
            layers: self.layers.clone(),
            cast_shadows: self.cast_shadows.clone(),
        }
    }

    /// Returns pre-cached bounding axis-aligned bounding box of the terrain. Keep in mind that
    /// if you're modified terrain, bounding box will be recalculated and it is not fast.
    pub fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
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
                Default::default(),
                Vector3::new(self.width, max_height, self.length),
            );
            self.bounding_box.set(bounding_box);
            self.bounding_box_dirty.set(false);

            bounding_box
        } else {
            self.bounding_box.get()
        }
    }

    /// Returns current **world-space** bounding box.
    pub fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
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
                    for z in 0..chunk.length_point_count {
                        let kz = z as f32 / (chunk.length_point_count - 1) as f32;
                        for x in 0..chunk.width_point_count {
                            let kx = x as f32 / (chunk.width_point_count - 1) as f32;

                            let pixel_position = chunk.local_position()
                                + Vector2::new(kx * chunk.width, kz * chunk.length);

                            let k = match brush.shape {
                                BrushShape::Circle { radius } => {
                                    1.0 - ((center - pixel_position).norm() / radius).powf(2.0)
                                }
                                BrushShape::Rectangle { .. } => 1.0,
                            };

                            if brush.shape.contains(center, pixel_position) {
                                chunk.heightmap[(z * chunk.width_point_count + x) as usize] +=
                                    k * amount;

                                chunk.dirty.set(true);
                            }
                        }
                    }
                }
            }
            BrushMode::DrawOnMask { layer, alpha } => {
                let alpha = alpha.clamp(-1.0, 1.0);

                for (chunk_index, chunk) in self.chunks.iter_mut().enumerate() {
                    let chunk_position = chunk.local_position();
                    let layer = &mut self.layers.get_mut()[layer];
                    let mut texture_data = layer.chunk_masks[chunk_index].data_ref();
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

                            let k = match brush.shape {
                                BrushShape::Circle { radius } => {
                                    1.0 - ((center - pixel_position).norm() / radius).powf(4.0)
                                }
                                BrushShape::Rectangle { .. } => 1.0,
                            };

                            if brush.shape.contains(center, pixel_position) {
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

    /// Updates terrain's chunks. There is no need to call this method in normal circumstances,
    /// engine will automatically call this method when needed.
    pub fn update(&mut self) {
        for chunk in self.chunks.iter_mut() {
            chunk.update();
        }
    }

    /// Returns a reference to a slice with layers of the terrain.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Returns a mutable reference to a slice with layers of the terrain.
    pub fn layers_mut(&mut self) -> &mut [Layer] {
        self.layers.get_mut()
    }

    /// Adds new layer to the chunk. It is possible to have different layer count per chunk
    /// in the same terrain, however it seems to not have practical usage, so try to keep
    /// equal layer count per each chunk in your terrains.
    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.get_mut().push(layer);
    }

    /// Removes given layers from the terrain.
    pub fn remove_layer(&mut self, layer: usize) -> Layer {
        self.layers.get_mut().remove(layer)
    }

    /// Tries to remove last layer from the terrain.
    pub fn pop_layer(&mut self) -> Option<Layer> {
        self.layers.get_mut().pop()
    }

    /// Inserts new layer at given position in the terrain.
    pub fn insert_layer(&mut self, layer: Layer, index: usize) {
        self.layers.get_mut().insert(index, layer)
    }

    /// Creates new layer with given parameters, but does **not** add it to the terrain.
    pub fn create_layer(
        &self,
        value: u8,
        material: Arc<Mutex<Material>>,
        mask_property_name: String,
    ) -> Layer {
        Layer {
            material,
            mask_property_name,
            chunk_masks: self
                .chunks
                .iter()
                .map(|c| {
                    create_layer_mask(
                        (c.width * self.mask_resolution) as u32,
                        (c.length * self.mask_resolution) as u32,
                        value,
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn restore_resources(&mut self, resource_manager: ResourceManager) {
        for layer in self.layers() {
            layer.material.lock().resolve(resource_manager.clone());
        }
    }

    // Prefab inheritance resolving.
    pub(crate) fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        self.base.inherit_properties(parent)?;
        if let Node::Terrain(parent) = parent {
            self.try_inherit_self_properties(parent)?;
        }
        Ok(())
    }

    pub(crate) fn reset_inheritable_properties(&mut self) {
        self.base.reset_inheritable_properties();
        self.reset_self_inheritable_properties();
    }
}

/// Shape of a brush.
#[derive(Copy, Clone, Inspect, Debug)]
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
#[derive(Clone, PartialEq, PartialOrd, Inspect, Debug)]
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
#[derive(Clone, Inspect, Debug)]
pub struct Brush {
    /// Center of the brush.
    #[inspect(skip)]
    pub center: Vector3<f32>,
    /// Shape of the brush.
    pub shape: BrushShape,
    /// Paint mode of the brush.
    pub mode: BrushMode,
}

/// Layer definition for a terrain builder.
pub struct LayerDefinition {
    /// Material of the layer.
    pub material: Arc<Mutex<Material>>,

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
    width: f32,
    length: f32,
    mask_resolution: f32,
    width_chunks: usize,
    length_chunks: usize,
    height_map_resolution: f32,
    layers: Vec<LayerDefinition>,
    decal_layer_index: u8,
    cast_shadows: bool,
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

fn make_surface_data() -> Arc<Mutex<SurfaceData>> {
    Arc::new(Mutex::new(SurfaceData::new(
        VertexBuffer::new::<StaticVertex>(0, StaticVertex::layout(), vec![]).unwrap(),
        TriangleBuffer::default(),
        false,
    )))
}

impl TerrainBuilder {
    /// Creates new builder instance.
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
            decal_layer_index: 0,
            cast_shadows: true,
        }
    }

    /// Sets desired terrain height in local coordinates.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets desired terrain length in local coordinates.
    pub fn with_length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }

    /// Sets desired mask resolution in pixels per unit. For example you have width = height = 16
    /// and you set resolution to 4 - then mask will have width = height = 4*16 = 64x64 pixels.
    pub fn with_mask_resolution(mut self, resolution: f32) -> Self {
        self.mask_resolution = resolution;
        self
    }

    /// Sets desired terrain width subdivision. The value passed in should correlate with desired
    /// width of the terrain. For example if you have small terrain, 2 chunks will be enough, however
    /// if you have huge terrain, the value should be 8+.
    pub fn with_width_chunks(mut self, count: usize) -> Self {
        self.width_chunks = count.max(1);
        self
    }

    /// Sets desired terrain length subdivision. The value passed in should correlate with desired
    /// length of the terrain. For example if you have small terrain, 2 chunks will be enough, however
    /// if you have huge terrain, the value should be 8+.
    pub fn with_length_chunks(mut self, count: usize) -> Self {
        self.length_chunks = count.max(1);
        self
    }

    /// Sets desired height map resolution in dots per unit. For example you have width = height = 16
    /// and you set resolution to 4 - then height map will have width = height = 4*16 = 64x64 dots.
    pub fn with_height_map_resolution(mut self, resolution: f32) -> Self {
        self.height_map_resolution = resolution;
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

    /// Sets whether terrain should cast shadows or not.
    pub fn with_cast_shadows(mut self, value: bool) -> Self {
        self.cast_shadows = value;
        self
    }

    /// Build terrain node.
    pub fn build_node(self) -> Node {
        let mut chunks = Vec::new();
        let chunk_length = self.length / self.length_chunks as f32;
        let chunk_width = self.width / self.width_chunks as f32;
        let chunk_length_points =
            make_divisible_by_2((chunk_length * self.height_map_resolution) as u32);
        let chunk_width_points =
            make_divisible_by_2((chunk_width * self.height_map_resolution) as u32);
        for z in 0..self.length_chunks {
            for x in 0..self.width_chunks {
                chunks.push(Chunk {
                    width_point_count: chunk_width_points,
                    length_point_count: chunk_length_points,
                    heightmap: vec![0.0; (chunk_length_points * chunk_width_points) as usize],
                    position: Vector3::new(x as f32 * chunk_width, 0.0, z as f32 * chunk_length),
                    width: chunk_width,
                    surface_data: make_surface_data(),
                    dirty: Cell::new(true),
                    length: chunk_length,
                });
            }
        }

        let mask_resolution = self.mask_resolution;

        let terrain = Terrain {
            width: self.width,
            length: self.length,
            base: self.base_builder.build_base(),
            layers: self
                .layers
                .into_iter()
                .enumerate()
                .map(|(layer_index, definition)| {
                    Layer {
                        material: definition.material,
                        mask_property_name: definition.mask_property_name,
                        chunk_masks: chunks
                            .iter()
                            .map(|c| {
                                create_layer_mask(
                                    (c.width * mask_resolution) as u32,
                                    (c.length * mask_resolution) as u32,
                                    // Base layer is opaque, every other by default - transparent.
                                    if layer_index == 0 { 255 } else { 0 },
                                )
                            })
                            .collect(),
                    }
                })
                .collect::<Vec<_>>()
                .into(),
            chunks,
            bounding_box_dirty: Cell::new(true),
            bounding_box: Default::default(),
            mask_resolution: self.mask_resolution,
            height_map_resolution: self.height_map_resolution,
            width_chunks: self.width_chunks as u32,
            length_chunks: self.length_chunks as u32,
            decal_layer_index: self.decal_layer_index.into(),
            cast_shadows: self.cast_shadows.into(),
        };

        Node::Terrain(terrain)
    }

    /// Builds terrain node and adds it to given graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
