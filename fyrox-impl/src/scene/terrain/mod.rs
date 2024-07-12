//! Everything related to terrains. See [`Terrain`] docs for more info.

use crate::material::MaterialResourceExtension;
use crate::renderer::bundle::PersistentIdentifier;
use crate::scene::node::RdcControlFlow;
use crate::{
    asset::Resource,
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3, Vector4},
        arrayvec::ArrayVec,
        log::Log,
        math::{aabb::AxisAlignedBoundingBox, ray::Ray, ray_rect_intersection, Rect},
        parking_lot::Mutex,
        pool::Handle,
        reflect::prelude::*,
        sstorage::ImmutableString,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::{prelude::*, PodVecView},
        TypeUuidProvider,
    },
    material::{Material, MaterialResource, PropertyValue},
    renderer::{
        self,
        bundle::{RenderContext, SurfaceInstanceData},
        framework::geometry_buffer::ElementRange,
    },
    resource::texture::{
        Texture, TextureKind, TexturePixelKind, TextureResource, TextureResourceExtension,
        TextureWrapMode,
    },
    scene::{
        base::{Base, BaseBuilder},
        debug::SceneDrawingContext,
        graph::Graph,
        mesh::RenderPath,
        node::{Node, NodeTrait},
        terrain::{geometry::TerrainGeometry, quadtree::QuadTree},
    },
};
use fxhash::FxHashMap;
use fyrox_core::uuid_provider;
use fyrox_graph::BaseSceneGraph;
use fyrox_resource::untyped::ResourceKind;
use half::f16;
use image::{imageops::FilterType, ImageBuffer, Luma};
use std::{
    cell::Cell,
    cmp::Ordering,
    collections::HashMap,
    ops::{Deref, DerefMut, Range},
};

pub mod brushstroke;
mod geometry;
mod quadtree;

pub use brushstroke::*;

/// Current implementation version marker.
pub const VERSION: u8 = 1;

/// Position of a single cell within terrain data.
#[derive(Debug, Clone)]
pub struct TerrainRect {
    /// The pixel coordinates of the cell.
    pub grid_position: Vector2<i32>,
    /// The local 2D bounds of the cell.
    pub bounds: Rect<f32>,
}

impl TerrainRect {
    /// Calculate the cell which contains the given local 2D coordinates when cells have the given size.
    /// It is assumed that the (0,0) cell has its origin at local 2D point (0.0, 0.0).
    pub fn from_local(position: Vector2<f32>, cell_size: Vector2<f32>) -> TerrainRect {
        let cell_pos = Vector2::new(position.x / cell_size.x, position.y / cell_size.y);
        let cell_pos = cell_pos.map(f32::floor);
        let min = Vector2::new(cell_pos.x * cell_size.x, cell_pos.y * cell_size.y);
        TerrainRect {
            grid_position: cell_pos.map(|x| x as i32),
            bounds: Rect::new(min.x, min.y, cell_size.x, cell_size.y),
        }
    }
}

/// Layers is a material Terrain can have as many layers as you want, but each layer slightly decreases
/// performance, so keep amount of layers on reasonable level (1 - 5 should be enough for most
/// cases).
#[derive(Debug, Clone, Visit, Reflect, PartialEq)]
pub struct Layer {
    /// Material of the layer.
    pub material: MaterialResource,

    /// Name of the mask sampler property in the material.
    pub mask_property_name: String,

    /// Name of the height map sampler property in the material.
    #[visit(optional)]
    pub height_map_property_name: String,

    /// Name of the node uv offsets property in the material.
    #[visit(optional)]
    pub node_uv_offsets_property_name: String,
}

uuid_provider!(Layer = "7439d5fd-43a9-45f0-bd7c-76cf4d2ec22e");

impl Default for Layer {
    fn default() -> Self {
        Self {
            material: MaterialResource::new_ok(Default::default(), Material::standard_terrain()),
            mask_property_name: "maskTexture".to_string(),
            height_map_property_name: "heightMapTexture".to_string(),
            node_uv_offsets_property_name: "nodeUvOffsets".to_string(),
        }
    }
}

/// Extract the &[f32] from a TextureResource to create a QuadTree, or panic.
fn make_quad_tree(
    texture: &Option<TextureResource>,
    height_map_size: Vector2<u32>,
    block_size: Vector2<u32>,
) -> QuadTree {
    let texture = texture.as_ref().unwrap().data_ref();
    let height_mod_count = texture.modifications_count();
    let height_map = texture.data_of_type::<f32>().unwrap();
    QuadTree::new(height_map, height_map_size, block_size, height_mod_count)
}

/// Create an Ok texture resource of the given size from the given height values.
/// `height_map` should have exactly `size.x * size.y` elements.
/// Returns None if the wrong number of height values are given to fill a height map
/// of the given size.
fn make_height_map_texture_internal(
    height_map: Vec<f32>,
    size: Vector2<u32>,
) -> Option<TextureResource> {
    let mut data = Texture::from_bytes(
        TextureKind::Rectangle {
            width: size.x,
            height: size.y,
        },
        TexturePixelKind::R32F,
        crate::core::transmute_vec_as_bytes(height_map),
    )?;

    data.set_t_wrap_mode(TextureWrapMode::ClampToEdge);
    data.set_s_wrap_mode(TextureWrapMode::ClampToEdge);

    Some(Resource::new_ok(Default::default(), data))
}

/// Create an Ok texture resource of the given size from the given height values.
/// `height_map` should have exactly `size.x * size.y` elements.
/// **Panics** if the wrong number of height values are given to fill a height map
/// of the given size.
fn make_height_map_texture(height_map: Vec<f32>, size: Vector2<u32>) -> TextureResource {
    make_height_map_texture_internal(height_map, size).unwrap()
}

/// Chunk is smaller block of a terrain. Terrain can have as many chunks as you need, which always arranged in a
/// grid. You can add chunks from any side of a terrain. Chunks could be considered as a "sub-terrain", which could
/// use its own set of materials for layers. This could be useful for different biomes, to prevent high amount of
/// layers which could harm the performance.
#[derive(Debug, Reflect)]
pub struct Chunk {
    #[reflect(hidden)]
    quad_tree: Mutex<QuadTree>,
    #[reflect(hidden)]
    version: u8,
    #[reflect(
        setter = "set_height_map",
        description = "Height map of the chunk. You can assign a custom height map image here. Keep in mind, that \
        only Red channel will be used! The assigned texture will be automatically converted to internal format suitable \
        for terrain needs."
    )]
    heightmap: Option<TextureResource>,
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
    pub layer_masks: Vec<TextureResource>,
    #[reflect(hidden)]
    height_map_modifications_count: u64,
}

uuid_provider!(Chunk = "ae996754-69c1-49ba-9c17-a7bd4be072a9");

impl PartialEq for Chunk {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.heightmap == other.heightmap
            && self.height_map_size == other.height_map_size
            && self.grid_position == other.grid_position
            && self.layer_masks == other.layer_masks
    }
}

impl Clone for Chunk {
    // Deep cloning.
    fn clone(&self) -> Self {
        Self {
            version: self.version,
            heightmap: Some(self.heightmap.as_ref().unwrap().deep_clone()),
            position: self.position,
            physical_size: self.physical_size,
            height_map_size: self.height_map_size,
            block_size: self.block_size,
            grid_position: self.grid_position,
            layer_masks: self
                .layer_masks
                .iter()
                .map(|m| m.deep_clone())
                .collect::<Vec<_>>(),
            quad_tree: Mutex::new(make_quad_tree(
                &self.heightmap,
                self.height_map_size,
                self.block_size,
            )),
            height_map_modifications_count: self.height_map_modifications_count,
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
                let mut height_map = Vec::<f32>::new();
                let mut view = PodVecView::from_pod_vec(&mut height_map);
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

                self.heightmap = Some(make_height_map_texture(
                    height_map,
                    Vector2::new(width_point_count, length_point_count),
                ));
            }
            VERSION => {
                self.heightmap.visit("Heightmap", &mut region)?;
                // We do not need to visit position, since its value is implied by grid_position.
                //self.position.visit("Position", &mut region)?;
                self.physical_size.visit("PhysicalSize", &mut region)?;
                self.height_map_size.visit("HeightMapSize", &mut region)?;
                self.layer_masks.visit("LayerMasks", &mut region)?;
                self.grid_position.visit("GridPosition", &mut region)?;
                // Set position to have the value implied by grid_position
                if region.is_reading() {
                    self.position = self.position()
                }
                let _ = self.block_size.visit("BlockSize", &mut region);
            }
            _ => (),
        }

        self.quad_tree = Mutex::new(make_quad_tree(
            &self.heightmap,
            self.height_map_size,
            self.block_size,
        ));

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
            block_size: Vector2::new(32, 32),
            grid_position: Default::default(),
            layer_masks: Default::default(),
            height_map_modifications_count: 0,
        }
    }
}

impl Chunk {
    /// Check the heightmap for modifications and update data as necessary.
    pub fn update(&self) {
        let Some(heightmap) = self.heightmap.as_ref() else {
            return;
        };
        let count = heightmap.data_ref().modifications_count();
        let mut quad_tree = self.quad_tree.lock();
        if count != quad_tree.height_mod_count() {
            *quad_tree = make_quad_tree(&self.heightmap, self.height_map_size, self.block_size);
        }
    }
    /// Returns position of the chunk in local 2D coordinates relative to origin of the
    /// terrain.
    pub fn local_position(&self) -> Vector2<f32> {
        map_to_local(self.position())
    }

    /// The position of the chunk within the terrain based on its `grid_position` and `physical_size`.
    pub fn position(&self) -> Vector3<f32> {
        Vector3::new(
            self.grid_position.x as f32 * self.physical_size.x,
            0.0,
            self.grid_position.y as f32 * self.physical_size.y,
        )
    }

    /// The 2D position of the chunk within the chunk array.
    #[inline]
    pub fn grid_position(&self) -> Vector2<i32> {
        self.grid_position
    }

    /// Returns a reference to height map.
    pub fn heightmap(&self) -> &TextureResource {
        self.heightmap.as_ref().unwrap()
    }

    /// Sets new height map to the chunk.
    /// Tries to create a copy of the given texture and convert the copy into [R32F](TexturePixelKind::R32F) format.
    /// If the conversion is successful, the resulting texture becomes the source for height data of this chunk
    /// and the new texture is returned.
    /// If the conversion fails, the argument texture is returned in its original format and the chunk is not modified.
    ///
    /// Failure can happen if:
    /// * The given texture is None.
    /// * The given texture is not in the [Ok state](crate::asset::state::ResourceState::Ok).
    /// * The given texture is not [TextureKind::Rectangle].
    /// * The width or height is incorrect due to not matching [height_map_size](Self::height_map_size).
    /// * The texture's format is not one of the many formats that this method is capable of converting as identified by its [Texture::pixel_kind].
    pub fn set_height_map(
        &mut self,
        height_map: Option<TextureResource>,
    ) -> Option<TextureResource> {
        if let Some(new_height_map) = height_map {
            let mut state = new_height_map.state();
            if let Some(new_height_map_texture) = state.data() {
                if let TextureKind::Rectangle { width, height } = new_height_map_texture.kind() {
                    if width == self.height_map_size.x && height == self.height_map_size.y {
                        fn convert<T, C>(texture: &Texture, mut mapper: C) -> Option<Vec<f32>>
                        where
                            T: Sized,
                            C: Fn(&T) -> f32,
                        {
                            texture
                                .mip_level_data_of_type::<T>(0)
                                .map(|v| v.iter().map(&mut mapper).collect::<Vec<_>>())
                        }

                        // Try to convert Red component of pixels to R32F format.
                        let pixels = match new_height_map_texture.pixel_kind() {
                            TexturePixelKind::R8 | TexturePixelKind::Luminance8 => {
                                convert::<u8, _>(new_height_map_texture, |v| {
                                    *v as f32 / u8::MAX as f32
                                })
                            }
                            TexturePixelKind::RGB8 => {
                                #[repr(C)]
                                struct Rgb8 {
                                    r: u8,
                                    g: u8,
                                    b: u8,
                                }
                                convert::<Rgb8, _>(new_height_map_texture, |v| {
                                    v.r as f32 / u8::MAX as f32
                                })
                            }
                            TexturePixelKind::RGBA8 => {
                                #[repr(C)]
                                struct Rgba8 {
                                    r: u8,
                                    g: u8,
                                    b: u8,
                                    a: u8,
                                }
                                convert::<Rgba8, _>(new_height_map_texture, |v| {
                                    v.r as f32 / u8::MAX as f32
                                })
                            }
                            TexturePixelKind::RG8 | TexturePixelKind::LuminanceAlpha8 => {
                                #[repr(C)]
                                struct Rg8 {
                                    r: u8,
                                    g: u8,
                                }
                                convert::<Rg8, _>(new_height_map_texture, |v| {
                                    v.r as f32 / u8::MAX as f32
                                })
                            }
                            TexturePixelKind::R16 | TexturePixelKind::Luminance16 => {
                                convert::<u16, _>(new_height_map_texture, |v| {
                                    *v as f32 / u16::MAX as f32
                                })
                            }
                            TexturePixelKind::RG16 | TexturePixelKind::LuminanceAlpha16 => {
                                #[repr(C)]
                                struct Rg16 {
                                    r: u16,
                                    g: u16,
                                }
                                convert::<Rg16, _>(new_height_map_texture, |v| {
                                    v.r as f32 / u16::MAX as f32
                                })
                            }
                            TexturePixelKind::BGR8 => {
                                #[repr(C)]
                                struct Bgr8 {
                                    b: u8,
                                    g: u8,
                                    r: u8,
                                }
                                convert::<Bgr8, _>(new_height_map_texture, |v| {
                                    v.r as f32 / u8::MAX as f32
                                })
                            }
                            TexturePixelKind::BGRA8 => {
                                #[repr(C)]
                                struct Bgra8 {
                                    r: u8,
                                    g: u8,
                                    b: u8,
                                    a: u8,
                                }
                                convert::<Bgra8, _>(new_height_map_texture, |v| {
                                    v.r as f32 / u8::MAX as f32
                                })
                            }
                            TexturePixelKind::RGB16 => {
                                #[repr(C)]
                                struct Rgb16 {
                                    r: u16,
                                    g: u16,
                                    b: u16,
                                }
                                convert::<Rgb16, _>(new_height_map_texture, |v| {
                                    v.r as f32 / u16::MAX as f32
                                })
                            }
                            TexturePixelKind::RGBA16 => {
                                #[repr(C)]
                                struct Rgba16 {
                                    r: u16,
                                    g: u16,
                                    b: u16,
                                    a: u16,
                                }
                                convert::<Rgba16, _>(new_height_map_texture, |v| {
                                    v.r as f32 / u16::MAX as f32
                                })
                            }
                            TexturePixelKind::RGB32F => {
                                #[repr(C)]
                                struct Rgb32F {
                                    r: f32,
                                    g: f32,
                                    b: f32,
                                }
                                convert::<Rgb32F, _>(new_height_map_texture, |v| v.r)
                            }
                            TexturePixelKind::RGBA32F => {
                                #[repr(C)]
                                struct Rgba32F {
                                    r: f32,
                                    g: f32,
                                    b: f32,
                                    a: f32,
                                }
                                convert::<Rgba32F, _>(new_height_map_texture, |v| v.r)
                            }
                            TexturePixelKind::RGB16F => {
                                #[repr(C)]
                                struct Rgb16F {
                                    r: f16,
                                    g: f16,
                                    b: f16,
                                }
                                convert::<Rgb16F, _>(new_height_map_texture, |v| v.r.to_f32())
                            }
                            TexturePixelKind::R32F => {
                                convert::<f32, _>(new_height_map_texture, |v| *v)
                            }
                            TexturePixelKind::R16F => {
                                convert::<f16, _>(new_height_map_texture, |v| v.to_f32())
                            }
                            _ => None,
                        };

                        if let Some(pixels) = pixels {
                            if let Some(texture) =
                                make_height_map_texture_internal(pixels, self.height_map_size)
                            {
                                let prev_texture =
                                    std::mem::replace(&mut self.heightmap, Some(texture));
                                self.update_quad_tree();
                                return prev_texture;
                            }
                        }
                    }
                }
            }
        }

        // In case of any error, ignore the new value and return current height map.
        self.heightmap.clone()
    }

    /// Returns the height map of the terrain as an array of `f32`s.
    pub fn heightmap_owned(&self) -> Vec<f32> {
        self.heightmap
            .as_ref()
            .unwrap()
            .data_ref()
            .data_of_type::<f32>()
            .unwrap()
            .to_vec()
    }

    /// Replaces the current height map with a new one. New height map must be equal with size of current.
    pub fn replace_height_map(
        &mut self,
        heightmap: TextureResource,
    ) -> Result<(), TextureResource> {
        let data = heightmap.data_ref();
        if let TextureKind::Rectangle { width, height } = data.kind() {
            if data.pixel_kind() == TexturePixelKind::R32F
                && self.height_map_size.x == width
                && self.height_map_size.y == height
            {
                drop(data);
                self.heightmap = Some(heightmap);
                self.update_quad_tree();
                return Ok(());
            }
        }
        drop(data);
        Err(heightmap)
    }

    /// Returns the size of the chunk in meters.
    pub fn physical_size(&self) -> Vector2<f32> {
        self.physical_size
    }

    /// Returns amount of pixels in the height map along each dimension.
    pub fn height_map_size(&self) -> Vector2<u32> {
        self.height_map_size
    }

    /// Performs debug drawing of the chunk. It draws internal quad-tree structure for debugging purposes.
    pub fn debug_draw(&self, transform: &Matrix4<f32>, ctx: &mut SceneDrawingContext) {
        let transform = *transform * Matrix4::new_translation(&self.position());

        self.quad_tree
            .lock()
            .debug_draw(&transform, self.height_map_size, self.physical_size, ctx)
    }

    fn set_block_size(&mut self, block_size: Vector2<u32>) {
        self.block_size = block_size;
        self.update_quad_tree();
    }

    /// Recalculates the quad tree for this chunk.
    pub fn update_quad_tree(&self) {
        if self.heightmap.is_none() {
            return;
        }
        *self.quad_tree.lock() =
            make_quad_tree(&self.heightmap, self.height_map_size, self.block_size);
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
    /// Height value at the intersection point (this value could be interpolated between four neighbour pixels
    /// of a height map).
    pub height: f32,
    /// World-space normal of triangle at impact point.
    pub normal: Vector3<f32>,
    /// Index of a chunk that was hit.
    pub chunk_index: usize,
    /// Time of impact. Usually in [0; 1] range where 0 - origin of a ray, 1 - its end.
    pub toi: f32,
}

/// An object representing the state of a terrain brush being used from code.
/// It has methods for starting, stopping, stamping, and smearing.
///
/// Each BrushContext requires some amount of heap allocation, so it may be preferable
/// to reuse a BrushContext for multiple strokes when possible.
///
/// A single brush stroke can include multiple operations across multiple frames, but
/// the terrain's texture resources should not be replaced during a stroke because
/// the BrushContext holds references the the texture resources that the terrain
/// had when the stroke started, and any brush operations will be applied to those
/// textures regardless of replacing the textures in the terrain.
#[derive(Default)]
pub struct BrushContext {
    /// Parameter value for the brush. For flattening, this is the target height.
    /// For flattening, it starts as None and then is given a value based on the first
    /// stamp or smear.
    pub value: Option<f32>,
    /// The pixel and brush data of the in-progress stroke.
    pub stroke: BrushStroke,
}

impl BrushContext {
    /// The current brush. This is immutable access only, because
    /// the brush's target may only be changed through [BrushContext::start_stroke].
    ///
    /// Mutable access to the brush's other properties is available through
    /// [BrushContext::shape], [BrushContext::mode], [BrushContext::hardness],
    /// and [BrushContext::alpha].
    pub fn brush(&self) -> &Brush {
        self.stroke.brush()
    }
    /// Mutable access to the brush's shape. This allows the shape of the brush
    /// to change without starting a new stroke.
    pub fn shape(&mut self) -> &mut BrushShape {
        self.stroke.shape()
    }
    /// Mutable access to the brush's mode. This allows the mode of the brush
    /// to change without starting a new stroke.
    pub fn mode(&mut self) -> &mut BrushMode {
        self.stroke.mode()
    }
    /// Mutable access to the brush's hardness. This allows the hardness of the brush
    /// to change without starting a new stroke.
    pub fn hardness(&mut self) -> &mut f32 {
        self.stroke.hardness()
    }
    /// Mutable access to the brush's alpha. This allows the alpha of the brush
    /// to change without starting a new stroke.
    pub fn alpha(&mut self) -> &mut f32 {
        self.stroke.alpha()
    }
    /// Modify the given BrushStroke so that it is using the given Brush and it is modifying the given terrain.
    /// The BrushContext will now hold references to the textures of this terrain for the target of the given brush,
    /// and so the stroke should not be used with other terrains until the stroke is finished.
    /// - `terrain`: The terrain that this stroke will edit.
    /// - `brush`: The Brush containing the brush shape and painting operation to perform.
    pub fn start_stroke(&mut self, terrain: &Terrain, brush: Brush) {
        self.value = None;
        terrain.start_stroke(brush, &mut self.stroke);
    }
    /// Modify the brushstroke to include a stamp of the brush at the given position.
    /// The location of the stamp relative to the textures is determined based on the global position
    /// of the terrain and the size of each terrain pixel.
    /// - `terrain`: The terrain that will be used to translate the given world-space coordinates into
    /// texture-space coordinates. This should be the same terrain as was given to [BrushContext::start_stroke].
    /// - `position`: The position of the brush in world coordinates.
    pub fn stamp(&mut self, terrain: &Terrain, position: Vector3<f32>) {
        let value = if matches!(self.stroke.brush().mode, BrushMode::Flatten { .. }) {
            self.interpolate_value(terrain, position)
        } else {
            0.0
        };
        terrain.stamp(position, value, &mut self.stroke);
    }
    /// Modify the brushstroke to include a smear of the brush from `start` to `end`.
    /// The location of the smear relative to the textures is determined based on the global position
    /// of the terrain and the size of each terrain pixel.
    /// - `terrain`: The terrain that will be used to translate the given world-space coordinates into
    /// texture-space coordinates. This should be the same terrain as was given to [BrushContext::start_stroke].
    /// - `start`: The start of the brush in world coordinates.
    /// - `end`: The end of the brush in world coordinates.
    pub fn smear(&mut self, terrain: &Terrain, start: Vector3<f32>, end: Vector3<f32>) {
        let value = if matches!(self.stroke.brush().mode, BrushMode::Flatten { .. }) {
            self.interpolate_value(terrain, start)
        } else {
            0.0
        };
        terrain.smear(start, end, value, &mut self.stroke);
    }
    /// Update the terrain's textures to include the latest pixel data without ending the stroke.
    pub fn flush(&mut self) {
        self.stroke.flush();
    }
    /// Update the terrain's textures to include the latest data and clear this context of all pixel data
    /// to prepare for starting another stroke.
    pub fn end_stroke(&mut self) {
        self.stroke.end_stroke();
    }
}

impl BrushContext {
    fn interpolate_value(&mut self, terrain: &Terrain, position: Vector3<f32>) -> f32 {
        if let Some(v) = self.value {
            return v;
        }
        let Some(position) = terrain.project(position) else {
            return 0.0;
        };
        let target = self.stroke.brush().target;
        let v = terrain.interpolate_value(position, target);
        self.value = Some(v);
        v
    }
}

/// Terrain is a height field where each point has fixed coordinates in XZ plane, but variable Y coordinate.
/// It can be used to create landscapes. It supports multiple layers, where each layer has its own material
/// and mask.
///
/// ## Chunking
///
/// Terrain itself does not define any geometry or rendering data, instead it uses one or more chunks for that
/// purpose. Each chunk could be considered as a "sub-terrain". You can "stack" any amount of chunks from any
/// side of the terrain. To do that, you define a range of chunks along each axes. This is very useful if you
/// need to extend your terrain in a particular direction. Imagine that you've created a terrain with just one
/// chunk (`0..1` range on both axes), but suddenly you found that you need to extend the terrain to add some
/// new game locations. In this case you can change the range of chunks at the desired axis. For instance, if
/// you want to add a new location to the right from your single chunk, then you should change `width_chunks`
/// range to `0..2` and leave `length_chunks` as is (`0..1`). This way terrain will be extended and you can
/// start shaping the new location.
///
/// ## Layers
///
/// Layer is a material with a blending mask. Layers helps you to build a terrain with wide variety of details.
/// For example, you can have a terrain with 3 layers: grass, rock, snow. This combination can be used to
/// create a terrain with grassy plateaus, rocky mountains with snowy tops. Each chunk (see above) can have its
/// own set of materials for each layer, however the overall layer count is defined by the terrain itself.
/// An ability to have different set of materials for different chunks is very useful to support various biomes.
///
/// ## Level of detail (LOD)
///
/// Terrain has automatic LOD system, which means that the closest portions of it will be rendered with highest
/// possible quality (defined by the resolution of height map and masks), while the furthest portions will be
/// rendered with lowest quality. This effectively balances GPU load and allows you to render huge terrains with
/// low overhead.
///
/// The main parameter that affects LOD system is `block_size` (`Terrain::set_block_size`), which defines size
/// of the patch that will be used for rendering. It is used to divide the size of the height map into a fixed
/// set of blocks using quad-tree algorithm.
///
/// Current implementation uses modified version of CDLOD algorithm without patch morphing. Apparently it is not
/// needed, since bilinear filtration in vertex shader prevents seams to occur.
///
/// ## Painting
///
/// Painting involves constructing a [BrushStroke] and calling its [BrushStroke::accept_messages] method with
/// a channel receiver, and sending a series of pixel messages into that channel. The BrushStroke will translate
/// those messages into modifications to the Terrain's textures.
///
/// ## Ray casting
///
/// You have two options to perform a ray casting:
///
/// 1) By using ray casting feature of the physics engine. In this case you need to create a `Heighfield` collider
/// and use standard [`crate::scene::graph::physics::PhysicsWorld::cast_ray`] method.
/// 2) By using [`Terrain::raycast`] - this method could provide you more information about intersection point, than
/// physics-based.
///
/// ## Physics
///
/// As usual, to have collisions working you need to create a rigid body and add an appropriate collider to it.
/// In case of terrains you need to create a collider with `Heightfield` shape and specify your terrain as a
/// geometry source.
///
/// ## Coordinate Spaces
///
/// Terrains operate in several systems of coordinates depending upon which aspect of the terrain is being measured.
///
/// - **Local:** These are the 3D `f32` coordinates of the Terrain node that are transformed to world space by the
/// [Base::global_transform]. It is measured in meters.
/// - **Local 2D:** These are the 2D `f32` coordinates formed by taking the (x,y,z) of local coordinates and turning them
/// into (x,z), with y removed and z becoming the new y.
/// The size of chunks in these coordinates is set by [Terrain::chunk_size].
/// - **Grid Position:** These are the 2D `i32` coordinates that represent a chunk's position within the regular grid of
/// chunks that make up a terrain. The *local 2D* position of a chunk can be calculated from its *grid position* by
/// multiplying its x and y coordinates by the x and y of [Terrain::chunk_size].
/// - **Height Pixel Position:** These are the 2D coordinates that measure position across the x and z axes of
/// the terrain using pixels in the height data of each chunk. (0,0) is the position of the Terrain node.
/// The *height pixel position* of a chunk can be calculated from its *grid position* by
/// multiplying its x and y coordinates by (x - 1) and (y - 1) of [Terrain::height_map_size].
/// Subtracting 1 from each dimension is necessary because the height map data of chunks overlaps by one pixel
/// on each edge, so the distance between the origins of two adjacent chunks is one less than height_map_size.
/// - **Mask Pixel Position:** These are the 2D coordinates that measure position across the x and z axes of
/// the terrain using pixels of the mask data of each chunk. (0,0) is the position of the (0,0) pixel of the
/// mask texture of the (0,0) chunk.
/// This means that (0,0) is offset from the position of the Terrain node by a half-pixel in the x direction
/// and a half-pixel in the z direction.
/// The size of each pixel is determined by [Terrain::chunk_size] and [Terrain::mask_size].
///
/// The size of blocks and the size of quad tree nodes is measured in height pixel coordinates, and these measurements
/// count the number of pixels needed to render the vertices of that part of the terrain, which means that they
/// overlap with their neighbors just as chunks overlap. Two adjacent blocks share vertices along their edge,
/// so they also share pixels in the height map data.
#[derive(Debug, Reflect, Clone)]
pub struct Terrain {
    base: Base,

    #[reflect(setter = "set_layers")]
    layers: InheritableVariable<Vec<Layer>>,

    #[reflect(setter = "set_decal_layer_index")]
    decal_layer_index: InheritableVariable<u8>,

    /// Size of the chunk, in meters.
    /// This value becomes the [Chunk::physical_size] of newly created chunks.
    #[reflect(
        min_value = 0.001,
        description = "Size of the chunk, in meters.",
        setter = "set_chunk_size"
    )]
    chunk_size: InheritableVariable<Vector2<f32>>,

    /// Min and max 'coordinate' of chunks along X axis.
    #[reflect(
        step = 1.0,
        description = "Min and max 'coordinate' of chunks along X axis.",
        setter = "set_width_chunks"
    )]
    width_chunks: InheritableVariable<Range<i32>>,

    /// Min and max 'coordinate' of chunks along Y axis.
    #[reflect(
        step = 1.0,
        description = "Min and max 'coordinate' of chunks along Y axis.",
        setter = "set_length_chunks"
    )]
    length_chunks: InheritableVariable<Range<i32>>,

    /// Size of the height map per chunk, in pixels. Warning: any change to this value will result in resampling!
    ///
    /// Each dimension should be one greater than some power of 2, such as 5 = 4 + 1, 9 = 8 + 1, 17 = 16 + 1, and so on.
    /// This is important because when chunks are being split into quadrants for LOD, the splits must always happen
    /// along its vertices, and there should be an equal number of vertices on each side of each split.
    /// If there cannot be an equal number of vertices on each side of the split, then the split will be made
    /// so that the number of vertices is as close to equal as possible, but this may result in vertices not being
    /// properly aligned between adjacent blocks.
    #[reflect(
        min_value = 2.0,
        step = 1.0,
        description = "Size of the height map per chunk, in pixels. Should be a power of 2 plus 1, for example: 5, 9, 17, etc. \
        Warning: any change to this value will result in resampling!",
        setter = "set_height_map_size"
    )]
    height_map_size: InheritableVariable<Vector2<u32>>,

    /// Size of the mesh block that will be scaled to various sizes to render the terrain at various levels of detail,
    /// as measured by counting vertices along each dimension.
    ///
    /// Each dimension should be one greater than some power of 2, such as 5 = 4 + 1, 9 = 8 + 1, 17 = 16 + 1, and so on.
    /// This helps the vertices of the block to align with the pixels of the height data texture, since the height data
    /// texture should also have dimensions that are one greater than some power of 2.
    #[reflect(min_value = 8.0, step = 1.0, setter = "set_block_size")]
    block_size: InheritableVariable<Vector2<u32>>,

    /// Size of the blending mask per chunk, in pixels. Warning: any change to this value will result in resampling!
    #[reflect(
        min_value = 1.0,
        step = 1.0,
        description = "Size of the blending mask per chunk, in pixels. Warning: any change to this value will result in resampling!",
        setter = "set_mask_size"
    )]
    mask_size: InheritableVariable<Vector2<u32>>,

    #[reflect(immutable_collection)]
    chunks: InheritableVariable<Vec<Chunk>>,

    #[reflect(hidden)]
    bounding_box_dirty: Cell<bool>,

    #[reflect(hidden)]
    bounding_box: Cell<AxisAlignedBoundingBox>,

    /// The [SurfaceSharedData](crate::scene::mesh::surface::SurfaceResource) that will be instanced to render
    /// all the chunks of the height map.
    #[reflect(hidden)]
    geometry: TerrainGeometry,

    #[reflect(hidden)]
    version: u8,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            base: Default::default(),
            layers: Default::default(),
            decal_layer_index: Default::default(),
            chunk_size: Vector2::new(16.0, 16.0).into(),
            width_chunks: Default::default(),
            length_chunks: Default::default(),
            height_map_size: Default::default(),
            block_size: Vector2::new(33, 33).into(),
            mask_size: Default::default(),
            chunks: Default::default(),
            bounding_box_dirty: Cell::new(true),
            bounding_box: Cell::new(Default::default()),
            geometry: Default::default(),
            version: VERSION,
        }
    }
}

#[derive(Visit)]
struct OldLayer {
    pub material: MaterialResource,
    pub mask_property_name: String,
    pub chunk_masks: Vec<TextureResource>,
}

impl Default for OldLayer {
    fn default() -> Self {
        Self {
            material: MaterialResource::new_ok(Default::default(), Material::standard_terrain()),
            mask_property_name: "maskTexture".to_string(),
            chunk_masks: Default::default(),
        }
    }
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

                let mut layers =
                    InheritableVariable::<Vec<OldLayer>>::new_modified(Default::default());
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

                    // TODO: Due to the bug in resource system, material properties are not kept in sync
                    // so here we must re-create the material and put every property from the old material
                    // to the new.
                    let mut new_material = Material::standard_terrain();

                    let mut material_state = layer.material.state();
                    if let Some(material) = material_state.data() {
                        for (name, value) in material.properties() {
                            Log::verify(new_material.set_property(name, value.clone()));
                        }
                    }

                    self.layers.push(Layer {
                        material: MaterialResource::new_ok(Default::default(), new_material),
                        mask_property_name: layer.mask_property_name,
                        ..Default::default()
                    });
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

/// Calculate the grid position of the chunk that would contain the given pixel position
/// assuming chunks have the given size.
fn pixel_position_to_grid_position(
    position: Vector2<i32>,
    chunk_size: Vector2<u32>,
) -> Vector2<i32> {
    let chunk_size = chunk_size.map(|x| x as i32);
    let x = position.x / chunk_size.x;
    let y = position.y / chunk_size.y;
    // Correct for the possibility of x or y being negative.
    let x = if position.x < 0 && position.x % chunk_size.x != 0 {
        x - 1
    } else {
        x
    };
    let y = if position.y < 0 && position.y % chunk_size.y != 0 {
        y - 1
    } else {
        y
    };
    Vector2::new(x, y)
}

impl TypeUuidProvider for Terrain {
    fn type_uuid() -> Uuid {
        uuid!("4b0a7927-bcd8-41a3-949a-dd10fba8e16a")
    }
}

impl Terrain {
    /// Returns chunk size in meters. This is equivalent to [Chunk::physical_size].
    pub fn chunk_size(&self) -> Vector2<f32> {
        *self.chunk_size
    }

    /// Sets new chunk size of the terrain (in meters). All chunks in the terrain will be repositioned according
    /// to their positions on the grid. Return the previous chunk size.
    pub fn set_chunk_size(&mut self, chunk_size: Vector2<f32>) -> Vector2<f32> {
        let old = *self.chunk_size;
        self.chunk_size.set_value_and_mark_modified(chunk_size);

        // Re-position each chunk according to its position on the grid.
        for iy in 0..self.length_chunks.len() {
            for ix in 0..self.width_chunks.len() {
                let chunk = &mut self.chunks[iy * self.width_chunks.len() + ix];
                chunk.physical_size = chunk_size;
                chunk.position = chunk.position();
            }
        }

        self.bounding_box_dirty.set(true);

        old
    }

    /// Returns height map dimensions along each axis.
    /// This is measured in *pixels* and gives the size of each chunk,
    /// including the 1 pixel overlap that each chunk shares with its neighbors.
    pub fn height_map_size(&self) -> Vector2<u32> {
        *self.height_map_size
    }

    /// Sets new size of the height map for every chunk. Heightmaps in every chunk will be resampled which may
    /// cause precision loss if the size was decreased. **Warning:** This method is very heavy and should not be
    /// used at every frame!
    pub fn set_height_map_size(&mut self, height_map_size: Vector2<u32>) -> Vector2<u32> {
        let old = *self.height_map_size;
        self.resize_height_maps(height_map_size);
        old
    }

    /// Sets the new block size, measured in height map pixels.
    /// Block size defines "granularity" of the terrain; the minimal terrain patch that
    /// will be used for rendering. It directly affects level-of-detail system of the terrain. **Warning:** This
    /// method is very heavy and should not be used at every frame!
    pub fn set_block_size(&mut self, block_size: Vector2<u32>) -> Vector2<u32> {
        let old = *self.block_size;
        self.block_size.set_value_and_mark_modified(block_size);
        self.geometry = TerrainGeometry::new(*self.block_size);
        for chunk in self.chunks.iter_mut() {
            chunk.set_block_size(*self.block_size);
        }
        old
    }

    /// Returns current block size of the terrain as measured by counting vertices along each axis of the block mesh.
    pub fn block_size(&self) -> Vector2<u32> {
        *self.block_size
    }

    /// Returns the number of pixels along each axis of the layer blending mask.
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
                        quad_tree: Mutex::new(QuadTree::new(
                            &heightmap,
                            *self.height_map_size,
                            *self.block_size,
                            0,
                        )),
                        heightmap: Some(make_height_map_texture(heightmap, self.height_map_size())),
                        height_map_modifications_count: 0,
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
        self.bounding_box_dirty.set(true);
        &mut self.chunks
    }

    /// Return the chunk with the matching [Chunk::grid_position].
    pub fn find_chunk(&self, grid_position: Vector2<i32>) -> Option<&Chunk> {
        self.chunks
            .iter()
            .find(|c| c.grid_position == grid_position)
    }

    /// Return the chunk with the matching [Chunk::grid_position].
    pub fn find_chunk_mut(&mut self, grid_position: Vector2<i32>) -> Option<&mut Chunk> {
        self.chunks
            .iter_mut()
            .find(|c| c.grid_position == grid_position)
    }

    /// Create new quad trees for every chunk in the terrain.
    pub fn update_quad_trees(&mut self) {
        for c in self.chunks.iter_mut() {
            c.update_quad_tree();
        }
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

    /// Convert from local 2D to height pixel position.
    pub fn local_to_height_pixel(&self, p: Vector2<f32>) -> Vector2<f32> {
        let scale = self.height_grid_scale();
        Vector2::new(p.x / scale.x, p.y / scale.y)
    }

    /// Convert from local 2D to mask pixel position.
    pub fn local_to_mask_pixel(&self, p: Vector2<f32>) -> Vector2<f32> {
        let scale = self.mask_grid_scale();
        let half = scale * 0.5;
        let p = p - half;
        Vector2::new(p.x / scale.x, p.y / scale.y)
    }

    /// The size of each cell of the height grid in local 2D units.
    pub fn height_grid_scale(&self) -> Vector2<f32> {
        let cell_width = self.chunk_size.x / (self.height_map_size.x - 1) as f32;
        let cell_length = self.chunk_size.y / (self.height_map_size.y - 1) as f32;
        Vector2::new(cell_width, cell_length)
    }

    /// The size of each cell of the mask grid in local 2D units.
    pub fn mask_grid_scale(&self) -> Vector2<f32> {
        let cell_width = self.chunk_size.x / self.mask_size.x as f32;
        let cell_length = self.chunk_size.y / self.mask_size.y as f32;
        Vector2::new(cell_width, cell_length)
    }

    /// Calculate which cell of the height grid contains the given local 2D position.
    pub fn get_height_grid_square(&self, position: Vector2<f32>) -> TerrainRect {
        TerrainRect::from_local(position, self.height_grid_scale())
    }

    /// Calculate which cell of the mask grid contains the given local 2D position.
    pub fn get_mask_grid_square(&self, position: Vector2<f32>) -> TerrainRect {
        let cell_size = self.mask_grid_scale();
        let half_size = cell_size / 2.0;
        let position = position - half_size;
        let mut rect = TerrainRect::from_local(position, cell_size);
        rect.bounds.position += half_size;
        rect
    }

    /// Return the value of the layer mask at the given mask pixel position.
    pub fn get_layer_mask(&self, position: Vector2<i32>, layer: usize) -> Option<u8> {
        let chunk_pos = self.chunk_containing_mask_pos(position);
        let chunk = self.find_chunk(chunk_pos)?;
        let origin = self.chunk_mask_pos_origin(chunk_pos);
        let pos = (position - origin).map(|x| x as usize);
        let index = pos.y * self.mask_size.x as usize + pos.x;
        let texture_data = chunk.layer_masks[layer].data_ref();
        let mask_data = texture_data.data();
        Some(mask_data[index])
    }

    /// Return the value of the height map at the given height pixel position.
    pub fn get_height(&self, position: Vector2<i32>) -> Option<f32> {
        let chunk_pos = self.chunk_containing_height_pos(position);
        let origin = self.chunk_height_pos_origin(chunk_pos);
        let pos = (position - origin).map(|x| x as usize);
        let end = self.height_map_size.map(|x| (x - 1) as usize);
        if let h @ Some(_) = self.get_height_in_chunk(chunk_pos, pos) {
            return h;
        }
        if pos.x == 0 {
            if let h @ Some(_) = self.get_height_in_chunk(
                Vector2::new(chunk_pos.x - 1, chunk_pos.y),
                Vector2::new(pos.x + end.x, pos.y),
            ) {
                return h;
            }
        }
        if pos.y == 0 {
            if let h @ Some(_) = self.get_height_in_chunk(
                Vector2::new(chunk_pos.x, chunk_pos.y - 1),
                Vector2::new(pos.x, pos.y + end.y),
            ) {
                return h;
            }
        }
        if pos.x == 0 && pos.y == 0 {
            if let h @ Some(_) = self.get_height_in_chunk(
                Vector2::new(chunk_pos.x - 1, chunk_pos.y - 1),
                Vector2::new(pos.x + end.x, pos.y + end.y),
            ) {
                return h;
            }
        }
        None
    }

    fn get_height_in_chunk(
        &self,
        chunk_pos: Vector2<i32>,
        pixel_pos: Vector2<usize>,
    ) -> Option<f32> {
        let index = pixel_pos.y * self.height_map_size.x as usize + pixel_pos.x;
        let chunk = self.find_chunk(chunk_pos)?;
        let texture_data = chunk.heightmap.as_ref().unwrap().data_ref();
        let height_map = texture_data.data_of_type::<f32>().unwrap();
        Some(height_map[index])
    }

    /// Return an interpolation of that the value should be for the given brush target
    /// at the given local 2D position.
    /// For height target, it returns the height.
    /// For mask targets, it returns 0.0 for transparent and 1.0 for opaque.
    pub fn interpolate_value(&self, position: Vector2<f32>, target: BrushTarget) -> f32 {
        let grid_square = match target {
            BrushTarget::HeightMap => self.get_height_grid_square(position),
            BrushTarget::LayerMask { .. } => self.get_mask_grid_square(position),
        };
        let p = grid_square.grid_position;
        let b = grid_square.bounds;
        let x0 = b.position.x;
        let y0 = b.position.y;
        let x1 = b.position.x + b.size.x;
        let y1 = b.position.y + b.size.y;
        let dx0 = position.x - x0;
        let dx1 = x1 - position.x;
        let dy0 = position.y - y0;
        let dy1 = y1 - position.y;
        let p00 = p;
        let p01 = Vector2::new(p.x, p.y + 1);
        let p10 = Vector2::new(p.x + 1, p.y);
        let p11 = Vector2::new(p.x + 1, p.y + 1);
        let (f00, f01, f10, f11) = match target {
            BrushTarget::HeightMap => (
                self.get_height(p00).unwrap_or(0.0),
                self.get_height(p01).unwrap_or(0.0),
                self.get_height(p10).unwrap_or(0.0),
                self.get_height(p11).unwrap_or(0.0),
            ),
            BrushTarget::LayerMask { layer } => (
                self.get_layer_mask(p00, layer).unwrap_or(0) as f32 / 255.0,
                self.get_layer_mask(p01, layer).unwrap_or(0) as f32 / 255.0,
                self.get_layer_mask(p10, layer).unwrap_or(0) as f32 / 255.0,
                self.get_layer_mask(p11, layer).unwrap_or(0) as f32 / 255.0,
            ),
        };
        let value = f00 * dx1 * dy1 + f10 * dx0 * dy1 + f01 * dx1 * dy0 + f11 * dx0 * dy0;
        value / (b.size.x * b.size.y)
    }

    /// Convert height pixel position into local 2D position.
    pub fn height_pos_to_local(&self, position: Vector2<i32>) -> Vector2<f32> {
        let pos = position.map(|x| x as f32);
        let chunk_size = self.height_map_size.map(|x| (x - 1) as f32);
        let physical_size = &self.chunk_size;
        Vector2::new(
            pos.x / chunk_size.x * physical_size.x,
            pos.y / chunk_size.y * physical_size.y,
        )
    }

    /// Convert mask pixel position into local 2D position.
    pub fn mask_pos_to_local(&self, position: Vector2<i32>) -> Vector2<f32> {
        let pos = position.map(|x| x as f32 + 0.5);
        let chunk_size = self.mask_size.map(|x| x as f32);
        let physical_size = &self.chunk_size;
        Vector2::new(
            pos.x / chunk_size.x * physical_size.x,
            pos.y / chunk_size.y * physical_size.y,
        )
    }

    /// Determines the chunk containing the given height pixel coordinate.
    /// Be aware that the edges of chunks overlap because the vertices along each edge of a chunk
    /// have the same height as the corresponding vertices of the next chunk in that direction.
    /// Due to this, if `position.x` is on the x-axis origin of the chunk returned by this method,
    /// then the position is also contained in the chunk at x - 1.
    /// Similarly, if `position.y` is on the y-axis origin, then the position is also in the y - 1 chunk.
    /// If position is on the origin in both the x and y axes, then the position is actually contained
    /// in 4 chunks.
    pub fn chunk_containing_height_pos(&self, position: Vector2<i32>) -> Vector2<i32> {
        // Subtract 1 from x and y to exclude the overlapping pixel along both axes from the chunk size.
        let chunk_size = self.height_map_size.map(|x| x - 1);
        pixel_position_to_grid_position(position, chunk_size)
    }

    /// Determines the position of the (0,0) coordinate of the given chunk
    /// as measured in height pixel coordinates.
    pub fn chunk_height_pos_origin(&self, chunk_grid_position: Vector2<i32>) -> Vector2<i32> {
        let chunk_size = *self.height_map_size;
        // Subtract 1 from x and y to exclude the overlapping pixel along both axes from the chunk size.
        let x = chunk_grid_position.x * (chunk_size.x as i32 - 1);
        let y = chunk_grid_position.y * (chunk_size.y as i32 - 1);
        Vector2::new(x, y)
    }

    /// Determines the chunk containing the given mask pixel coordinate.
    /// This method makes no guarantee that there is actually a chunk at the returned coordinates.
    /// It returns the grid_position that the chunk would have if it existed.
    pub fn chunk_containing_mask_pos(&self, position: Vector2<i32>) -> Vector2<i32> {
        pixel_position_to_grid_position(position, *self.mask_size)
    }

    /// Determines the position of the (0,0) coordinate of the given chunk
    /// as measured in mask pixel coordinates.
    pub fn chunk_mask_pos_origin(&self, chunk_grid_position: Vector2<i32>) -> Vector2<i32> {
        let chunk_size = *self.mask_size;
        let x = chunk_grid_position.x * chunk_size.x as i32;
        let y = chunk_grid_position.y * chunk_size.y as i32;
        Vector2::new(x, y)
    }

    /// Applies the given function to the value at the given position in mask pixel coordinates.
    /// This method calls the given function with the mask value of that pixel.
    /// If no chunk contains the given position, then the function is not called.
    pub fn update_mask_pixel<F>(&mut self, position: Vector2<i32>, layer: usize, func: F)
    where
        F: FnOnce(u8) -> u8,
    {
        let chunk_pos = self.chunk_containing_mask_pos(position);
        let origin = self.chunk_mask_pos_origin(chunk_pos);
        let pos = position - origin;
        let index = (pos.y * self.mask_size.x as i32 + pos.x) as usize;
        let Some(chunk) = self.find_chunk_mut(chunk_pos) else {
            return;
        };
        let mut texture_data = chunk.layer_masks[layer].data_ref();
        let mut texture_modifier = texture_data.modify();
        let mask = texture_modifier.data_mut_of_type::<u8>().unwrap();
        let value = &mut mask[index];
        *value = func(*value);
    }

    /// Applies the given function to the value at the given position in height pixel coordinates.
    /// This method calls the given function with the height value of that pixel.
    /// The returned value is written to every chunk that contains that pixel, replacing the current value.
    /// Most pixels are contained in only one chunk, but some pixels are contained in anywhere from zero to four chunks,
    /// due to chunks overlapping at the edges and corners.
    /// If no chunk contains the given position, then the function is not called.
    pub fn update_height_pixel<F, G>(
        &mut self,
        position: Vector2<i32>,
        mut pixel_func: F,
        mut chunk_func: G,
    ) where
        F: FnMut(f32) -> f32,
        G: FnMut(&Chunk),
    {
        let chunk_pos = self.chunk_containing_height_pos(position);
        let origin = self.chunk_height_pos_origin(chunk_pos);
        let pos = (position - origin).map(|x| x as usize);
        let mut result: Option<f32> = None;
        let end = self.height_map_size.map(|x| (x - 1) as usize);
        self.update_pixel_in_chunk(
            chunk_pos,
            pos,
            &mut result,
            &mut pixel_func,
            &mut chunk_func,
        );
        if pos.x == 0 {
            self.update_pixel_in_chunk(
                Vector2::new(chunk_pos.x - 1, chunk_pos.y),
                Vector2::new(pos.x + end.x, pos.y),
                &mut result,
                &mut pixel_func,
                &mut chunk_func,
            );
        }
        if pos.y == 0 {
            self.update_pixel_in_chunk(
                Vector2::new(chunk_pos.x, chunk_pos.y - 1),
                Vector2::new(pos.x, pos.y + end.y),
                &mut result,
                &mut pixel_func,
                &mut chunk_func,
            );
        }
        if pos.x == 0 && pos.y == 0 {
            self.update_pixel_in_chunk(
                Vector2::new(chunk_pos.x - 1, chunk_pos.y - 1),
                Vector2::new(pos.x + end.x, pos.y + end.y),
                &mut result,
                &mut pixel_func,
                &mut chunk_func,
            );
        }
    }

    fn update_pixel_in_chunk<F, G>(
        &mut self,
        chunk_pos: Vector2<i32>,
        pixel_pos: Vector2<usize>,
        result: &mut Option<f32>,
        pixel_func: F,
        chunk_func: G,
    ) where
        F: FnOnce(f32) -> f32,
        G: FnOnce(&Chunk),
    {
        let index = pixel_pos.y * self.height_map_size.x as usize + pixel_pos.x;
        let Some(chunk) = self.find_chunk_mut(chunk_pos) else {
            return;
        };
        chunk_func(chunk);
        let mut texture_data = chunk.heightmap.as_ref().unwrap().data_ref();
        let mut texture_modifier = texture_data.modify();
        let height_map = texture_modifier.data_mut_of_type::<f32>().unwrap();
        let value = &mut height_map[index];
        if let Some(new_value) = result {
            *value = *new_value;
        } else {
            *value = pixel_func(*value);
            *result = Some(*value);
        }
    }

    /// Applies the given function to each pixel of the height map.
    pub fn for_each_height_map_pixel<F>(&mut self, mut func: F)
    where
        F: FnMut(&mut f32, Vector2<f32>),
    {
        for chunk in self.chunks.iter_mut() {
            let mut texture_data = chunk.heightmap.as_ref().unwrap().data_ref();
            let mut texture_modifier = texture_data.modify();
            let height_map = texture_modifier.data_mut_of_type::<f32>().unwrap();

            for iy in 0..chunk.height_map_size.y {
                let kz = iy as f32 / (chunk.height_map_size.y - 1) as f32;
                for ix in 0..chunk.height_map_size.x {
                    let kx = ix as f32 / (chunk.height_map_size.x - 1) as f32;

                    let pixel_position = chunk.local_position()
                        + Vector2::new(kx * chunk.physical_size.x, kz * chunk.physical_size.y);

                    let index = (iy * chunk.height_map_size.x + ix) as usize;

                    func(&mut height_map[index], pixel_position)
                }
            }

            drop(texture_modifier);
            drop(texture_data);

            *chunk.quad_tree.lock() =
                make_quad_tree(&chunk.heightmap, chunk.height_map_size, chunk.block_size);
        }

        self.bounding_box_dirty.set(true);
    }

    /// Casts a ray and looks for intersections with the terrain. This method collects all results in
    /// given array with optional sorting by the time-of-impact.
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
                let texture = chunk.heightmap.as_ref().unwrap().data_ref();
                let height_map = texture.data_of_type::<f32>().unwrap();

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
                                    height_map[i0],
                                    pixel_position.y, // Remember Z -> Y mapping!
                                );
                                let v1 = Vector3::new(v0.x, height_map[i1], v0.z + cell_length);
                                let v2 = Vector3::new(v1.x + cell_width, height_map[i2], v1.z);
                                let v3 = Vector3::new(v0.x + cell_width, height_map[i3], v0.z);

                                for vertices in &[[v0, v1, v2], [v2, v3, v0]] {
                                    if let Some((toi, intersection)) =
                                        local_ray.triangle_intersection(vertices)
                                    {
                                        let normal = (vertices[2] - vertices[0])
                                            .cross(&(vertices[1] - vertices[0]))
                                            .try_normalize(f32::EPSILON)
                                            .unwrap_or_else(Vector3::y);

                                        let result = TerrainRayCastResult {
                                            position: self
                                                .global_transform()
                                                .transform_point(&Point3::from(intersection))
                                                .coords,
                                            height: intersection.y,
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
    pub fn add_layer(&mut self, layer: Layer, masks: Vec<TextureResource>) {
        self.insert_layer(layer, masks, self.layers.len())
    }

    /// Removes a layer at the given index together with its respective blending masks from each chunk.
    pub fn remove_layer(&mut self, layer_index: usize) -> (Layer, Vec<TextureResource>) {
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
    pub fn pop_layer(&mut self) -> Option<(Layer, Vec<TextureResource>)> {
        if self.layers.is_empty() {
            None
        } else {
            Some(self.remove_layer(self.layers.len() - 1))
        }
    }

    /// Inserts the layer at the given index together with its blending masks for each chunk.
    pub fn insert_layer(&mut self, layer: Layer, mut masks: Vec<TextureResource>, index: usize) {
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
                let new_mask_texture = TextureResource::from_bytes(
                    TextureKind::Rectangle {
                        width: new_size.x,
                        height: new_size.y,
                    },
                    data.pixel_kind(),
                    new_mask,
                    ResourceKind::Embedded,
                )
                .unwrap();

                drop(data);
                *mask = new_mask_texture;
            }
        }

        self.mask_size.set_value_and_mark_modified(new_size);
    }

    fn resize_height_maps(&mut self, mut new_size: Vector2<u32>) {
        // Height maps should be a 1 + a multiple of 2 and they should be at least
        // 3x3, since a 1x1 height map would be just a single vertex with no faces.
        new_size = new_size.sup(&Vector2::repeat(3));

        for chunk in self.chunks.iter_mut() {
            let texture = chunk.heightmap.as_ref().unwrap().data_ref();
            let mut heightmap = texture.data_of_type::<f32>().unwrap().to_vec();

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

            drop(texture);

            chunk.height_map_size = new_size;
            chunk.heightmap = Some(make_height_map_texture(resampled_heightmap, new_size));
            chunk.update_quad_tree();
        }

        self.height_map_size.set_value_and_mark_modified(new_size);
        self.bounding_box_dirty.set(true);
    }

    /// Returns data for rendering (vertex and index buffers).
    pub fn geometry(&self) -> &TerrainGeometry {
        &self.geometry
    }
    /// Create an object that specifies which TextureResources are being used by this terrain
    /// to hold the data for the given BrushTarget.
    pub fn texture_data(&self, target: BrushTarget) -> TerrainTextureData {
        let chunk_size = match target {
            BrushTarget::HeightMap => self.height_map_size(),
            BrushTarget::LayerMask { .. } => self.mask_size(),
        };
        let kind = match target {
            BrushTarget::HeightMap => TerrainTextureKind::Height,
            BrushTarget::LayerMask { .. } => TerrainTextureKind::Mask,
        };
        let resources: FxHashMap<Vector2<i32>, TextureResource> = match target {
            BrushTarget::HeightMap => self
                .chunks_ref()
                .iter()
                .map(|c| (c.grid_position(), c.heightmap().clone()))
                .collect(),
            BrushTarget::LayerMask { layer } => self
                .chunks_ref()
                .iter()
                .map(|c| (c.grid_position(), c.layer_masks[layer].clone()))
                .collect(),
        };
        TerrainTextureData {
            chunk_size,
            kind,
            resources,
        }
    }
    /// Modify the given BrushStroke so that it is using the given Brush and it is modifying this terrain.
    /// The BrushStroke will now hold references to the textures of this terrain for the target of the given brush,
    /// and so the stroke should not be used with other terrains until the stroke is finished.
    /// - `brush`: The Brush containing the brush shape and painting operation to perform.
    /// - `stroke`: The BrushStroke object to be reset to start a new stroke.
    fn start_stroke(&self, brush: Brush, stroke: &mut BrushStroke) {
        let target = brush.target;
        stroke.start_stroke(brush, self.self_handle, self.texture_data(target))
    }
    /// Modify the given BrushStroke to include a stamp of its brush at the given position.
    /// The location of the stamp relative to the textures is determined based on the global position
    /// of the terrain and the size of each terrain pixel.
    /// - `position`: The position of the brush in world coordinates.
    /// - `value`: The value of the brush stroke, whose meaning depends on the brush operation.
    /// For flatten brush operations, this is the target value to flatten toward.
    /// - `stroke`: The BrushStroke object to be modified.
    fn stamp(&self, position: Vector3<f32>, value: f32, stroke: &mut BrushStroke) {
        let Some(position) = self.project(position) else {
            return;
        };
        let position = match stroke.brush().target {
            BrushTarget::HeightMap => self.local_to_height_pixel(position),
            BrushTarget::LayerMask { .. } => self.local_to_mask_pixel(position),
        };
        let scale = match stroke.brush().target {
            BrushTarget::HeightMap => self.height_grid_scale(),
            BrushTarget::LayerMask { .. } => self.mask_grid_scale(),
        };
        stroke.stamp(position, scale, value);
    }
    /// Modify the given BrushStroke to include a stamp of its brush at the given position.
    /// The location of the stamp relative to the textures is determined based on the global position
    /// of the terrain and the size of each terrain pixel.
    /// - `start`: The start of the smear in world coordinates.
    /// - `end`: The end of the smear in world coordinates.
    /// - `value`: The value of the brush stroke, whose meaning depends on the brush operation.
    /// For flatten brush operations, this is the target value to flatten toward.
    /// - `stroke`: The BrushStroke object to be modified.
    fn smear(&self, start: Vector3<f32>, end: Vector3<f32>, value: f32, stroke: &mut BrushStroke) {
        let Some(start) = self.project(start) else {
            return;
        };
        let Some(end) = self.project(end) else {
            return;
        };
        let start = match stroke.brush().target {
            BrushTarget::HeightMap => self.local_to_height_pixel(start),
            BrushTarget::LayerMask { .. } => self.local_to_mask_pixel(start),
        };
        let end = match stroke.brush().target {
            BrushTarget::HeightMap => self.local_to_height_pixel(end),
            BrushTarget::LayerMask { .. } => self.local_to_mask_pixel(end),
        };
        let scale = match stroke.brush().target {
            BrushTarget::HeightMap => self.height_grid_scale(),
            BrushTarget::LayerMask { .. } => self.mask_grid_scale(),
        };
        stroke.smear(start, end, scale, value);
    }
}

impl NodeTrait for Terrain {
    crate::impl_query_component!();

    /// Returns pre-cached bounding axis-aligned bounding box of the terrain. Keep in mind that
    /// if you're modified terrain, bounding box will be recalculated and it is not fast.
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        if self.bounding_box_dirty.get() {
            let mut max_height = -f32::MAX;
            let mut min_height = f32::MAX;
            for chunk in self.chunks.iter() {
                let texture = chunk.heightmap.as_ref().unwrap().data_ref();
                let height_map = texture.data_of_type::<f32>().unwrap();
                for &height in height_map {
                    if height > max_height {
                        max_height = height;
                    }
                    if height < min_height {
                        min_height = height;
                    }
                }
            }

            let bounding_box = AxisAlignedBoundingBox::from_min_max(
                Vector3::new(
                    self.chunk_size.x * self.width_chunks.start as f32,
                    min_height,
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

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.global_visibility()
            || !self.is_globally_enabled()
            || (self.frustum_culling()
                && !ctx
                    .frustum
                    .map_or(true, |f| f.is_intersects_aabb(&self.world_bounding_box())))
        {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) && !self.cast_shadows() {
            return RdcControlFlow::Continue;
        }

        for c in self.chunks.iter() {
            c.update();
        }

        for (layer_index, layer) in self.layers().iter().enumerate() {
            for chunk in self.chunks_ref().iter() {
                // Generate a list of distances for each LOD that the terrain can render.
                // The first element of the list is the furthest distance, where the lowest LOD is used.
                // The formula used to produce this list has been chosen arbitrarily based on what seems to produce
                // the best results in the render.
                let quad_tree = chunk.quad_tree.lock();
                let levels = (0..quad_tree.max_level)
                    .map(|n| {
                        ctx.z_far
                            * ((quad_tree.max_level - n) as f32 / quad_tree.max_level as f32)
                                .powf(3.0)
                    })
                    .collect::<Vec<_>>();

                let chunk_transform =
                    self.global_transform() * Matrix4::new_translation(&chunk.position());

                // Use the `levels` list and the camera position to generate a list of all the positions
                // and scales where instances of the terrain geometry should appear in the render.
                // The instances will be scaled based on the LOD that is needed at the instance's distance
                // according to the `levels` list.
                let mut selection = Vec::new();
                quad_tree.select(
                    &chunk_transform,
                    self.height_map_size(),
                    self.chunk_size(),
                    ctx.frustum,
                    *ctx.observer_position,
                    &levels,
                    &mut selection,
                );

                let mut material = layer.material.deep_copy().data_ref().clone();

                Log::verify_message(
                    material.set_property(
                        &ImmutableString::new(&layer.mask_property_name),
                        PropertyValue::Sampler {
                            value: Some(chunk.layer_masks[layer_index].clone()),
                            fallback: Default::default(),
                        },
                    ),
                    "Unable to set mask texture for terrain material.",
                );

                Log::verify_message(
                    material.set_property(
                        &ImmutableString::new(&layer.height_map_property_name),
                        PropertyValue::Sampler {
                            value: chunk.heightmap.clone(),
                            fallback: Default::default(),
                        },
                    ),
                    "Unable to set height map texture for terrain material.",
                );

                for node in selection {
                    let kx = node.position.x as f32 / self.height_map_size.x as f32;
                    let kz = node.position.y as f32 / self.height_map_size.y as f32;

                    let kw = node.size.x as f32 / self.height_map_size.x as f32;
                    let kh = node.size.y as f32 / self.height_map_size.y as f32;

                    Log::verify_message(
                        material.set_property(
                            &ImmutableString::new(&layer.node_uv_offsets_property_name),
                            PropertyValue::Vector4(Vector4::new(kx, kz, kw, kh)),
                        ),
                        "Unable to set node uv offsets for terrain material.",
                    );

                    let material = MaterialResource::new_ok(Default::default(), material.clone());

                    let node_transform = chunk_transform
                        * Matrix4::new_translation(&Vector3::new(
                            kx * self.chunk_size.x,
                            0.0,
                            kz * self.chunk_size.y,
                        ))
                        * Matrix4::new_nonuniform_scaling(&Vector3::new(
                            kw * self.chunk_size.x,
                            1.0,
                            kh * self.chunk_size.y,
                        ));

                    if node.is_draw_full() {
                        ctx.storage.push(
                            &self.geometry.data,
                            &material,
                            RenderPath::Deferred,
                            self.decal_layer_index(),
                            layer_index as u64,
                            SurfaceInstanceData {
                                world_transform: node_transform,
                                bone_matrices: Default::default(),
                                depth_offset: self.depth_offset_factor(),
                                blend_shapes_weights: Default::default(),
                                element_range: ElementRange::Full,
                                persistent_identifier: PersistentIdentifier::new_combined(
                                    &self.geometry.data,
                                    self.self_handle,
                                    node.persistent_index,
                                ),
                                node_handle: self.self_handle,
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
                                        world_transform: node_transform,
                                        bone_matrices: Default::default(),
                                        depth_offset: self.depth_offset_factor(),
                                        blend_shapes_weights: Default::default(),
                                        element_range: self.geometry.quadrants[i],
                                        persistent_identifier: PersistentIdentifier::new_combined(
                                            &self.geometry.data,
                                            self.self_handle,
                                            node.persistent_index,
                                        ),
                                        node_handle: self.self_handle,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        RdcControlFlow::Continue
    }

    fn debug_draw(&self, ctx: &mut SceneDrawingContext) {
        for chunk in self.chunks.iter() {
            chunk.debug_draw(&self.global_transform(), ctx)
        }
    }
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

fn create_layer_mask(width: u32, height: u32, value: u8) -> TextureResource {
    let mask = TextureResource::from_bytes(
        TextureKind::Rectangle { width, height },
        TexturePixelKind::R8,
        vec![value; (width * height) as usize],
        ResourceKind::Embedded,
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
            height_map_size: Vector2::new(257, 257),
            block_size: Vector2::new(33, 33),
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
                    quad_tree: Mutex::new(QuadTree::new(
                        &heightmap,
                        self.height_map_size,
                        self.block_size,
                        0,
                    )),
                    height_map_size: self.height_map_size,
                    heightmap: Some(make_height_map_texture(heightmap, self.height_map_size)),
                    height_map_modifications_count: 0,
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
