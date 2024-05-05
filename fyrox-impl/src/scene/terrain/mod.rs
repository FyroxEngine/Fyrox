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

mod geometry;
mod quadtree;

/// Current implementation version marker.
pub const VERSION: u8 = 1;

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

fn make_quad_tree(
    texture: &Option<TextureResource>,
    height_map_size: Vector2<u32>,
    block_size: Vector2<u32>,
) -> QuadTree {
    let texture = texture.as_ref().unwrap().data_ref();
    let height_map = texture.data_of_type::<f32>().unwrap();
    QuadTree::new(height_map, height_map_size, block_size)
}

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

fn make_height_map_texture(height_map: Vec<f32>, size: Vector2<u32>) -> TextureResource {
    make_height_map_texture_internal(height_map, size).unwrap()
}

/// Chunk is smaller block of a terrain. Terrain can have as many chunks as you need, which always arranged in a
/// grid. You can add chunks from any side of a terrain. Chunks could be considered as a "sub-terrain", which could
/// use its own set of materials for layers. This could be useful for different biomes, to prevent high amount of
/// layers which could harm the performance.
#[derive(Debug, Reflect, PartialEq)]
pub struct Chunk {
    #[reflect(hidden)]
    quad_tree: QuadTree,
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
}

uuid_provider!(Chunk = "ae996754-69c1-49ba-9c17-a7bd4be072a9");

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
            quad_tree: make_quad_tree(&self.heightmap, self.height_map_size, self.block_size),
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
                self.position.visit("Position", &mut region)?;
                self.physical_size.visit("PhysicalSize", &mut region)?;
                self.height_map_size.visit("HeightMapSize", &mut region)?;
                self.layer_masks.visit("LayerMasks", &mut region)?;
                self.grid_position.visit("GridPosition", &mut region)?;
                let _ = self.block_size.visit("BlockSize", &mut region);
            }
            _ => (),
        }

        self.quad_tree = make_quad_tree(&self.heightmap, self.height_map_size, self.block_size);

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
    pub fn heightmap(&self) -> &TextureResource {
        self.heightmap.as_ref().unwrap()
    }

    /// Sets new height map to the chunk.
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
                                return std::mem::replace(&mut self.heightmap, Some(texture));
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
        let transform = *transform * Matrix4::new_translation(&self.position);

        self.quad_tree
            .debug_draw(&transform, self.height_map_size, self.physical_size, ctx)
    }

    fn set_block_size(&mut self, block_size: Vector2<u32>) {
        self.block_size = block_size;
        self.quad_tree = make_quad_tree(&self.heightmap, self.height_map_size, block_size);
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
/// Terrain has a single method for "painting" - [`Terrain::draw`], it accepts a brush with specific parameters,
/// which can either alternate height map or a layer mask. See method's documentation for more info.
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
#[derive(Debug, Reflect, Clone)]
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

    #[reflect(min_value = 8.0, step = 1.0, setter = "set_block_size")]
    block_size: InheritableVariable<Vector2<u32>>,

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
            block_size: Vector2::new(32, 32).into(),
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

    /// Sets new chunk size of the terrain (in meters). All chunks in the terrain will be repositioned according
    /// to their positions on the grid.
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

        self.bounding_box_dirty.set(true);

        old
    }

    /// Returns height map dimensions along each axis.
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

    /// Sets the new block size. Block size defines "granularity" of the terrain; the minimal terrain patch that
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

    /// Returns current block size of the terrain.
    pub fn block_size(&self) -> Vector2<u32> {
        *self.block_size
    }

    /// Returns the total amount of pixels along each axis of the layer blending mask.
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
                        heightmap: Some(make_height_map_texture(heightmap, self.height_map_size())),
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

            chunk.quad_tree =
                make_quad_tree(&chunk.heightmap, chunk.height_map_size, chunk.block_size);
        }

        self.bounding_box_dirty.set(true);
    }

    /// Multi-functional drawing method. It uses given brush to modify terrain, see [`Brush`] docs for
    /// more info.
    pub fn draw(&mut self, brush: &Brush) {
        let center = project(self.global_transform(), brush.center).unwrap();

        match brush.mode {
            BrushMode::ModifyHeightMap { amount } => {
                self.for_each_height_map_pixel(|pixel, pixel_position| {
                    let k = match brush.shape {
                        BrushShape::Circle { radius } => {
                            1.0 - ((center - pixel_position).norm() / radius).powf(2.0)
                        }
                        BrushShape::Rectangle { .. } => 1.0,
                    };

                    if brush.shape.contains(center, pixel_position) {
                        *pixel += k * amount;
                    }
                });
            }
            BrushMode::DrawOnMask { layer, alpha } => {
                if layer >= self.layers.len() {
                    return;
                }

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
            BrushMode::FlattenHeightMap { height } => {
                self.for_each_height_map_pixel(|pixel, pixel_position| {
                    if brush.shape.contains(center, pixel_position) {
                        *pixel = height;
                    }
                });
            }
        }
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
        new_size = new_size.sup(&Vector2::repeat(2));

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

                let chunk_transform =
                    self.global_transform() * Matrix4::new_translation(&chunk.position);

                let mut selection = Vec::new();
                chunk.quad_tree.select(
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

uuid_provider!(BrushShape = "a4dbfba0-077c-4658-9972-38384a8432f9");

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
    ///
    FlattenHeightMap {
        /// Fixed height value for flattening.
        height: f32,
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

uuid_provider!(BrushMode = "48ad4cac-05f3-485a-b2a3-66812713841f");

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
                    heightmap: Some(make_height_map_texture(heightmap, self.height_map_size)),
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
