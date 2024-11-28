// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Tile map is a 2D "image", made out of a small blocks called tiles. Tile maps used in 2D games to
//! build game worlds quickly and easily. See [`TileMap`] docs for more info and usage examples.

pub mod brush;
mod resource_grid;
mod tile_rect;
mod tile_source;
pub mod tileset;
mod transform;
mod update;

use brush::*;
use fyrox_graph::constructor::ConstructorProvider;
use resource_grid::*;
pub use tile_rect::*;
pub use tile_source::*;
use tileset::*;
pub use transform::*;
pub use update::*;

use crate::{
    asset::untyped::ResourceKind,
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    graph::BaseSceneGraph,
    material::{shader::ShaderResource, Material, MaterialResource, STANDARD_2D},
    renderer::{self, bundle::RenderContext},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::{
            buffer::{
                VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage,
                VertexTrait,
            },
            RenderPath,
        },
        node::{Node, NodeTrait, RdcControlFlow},
        Scene,
    },
};
use bytemuck::{Pod, Zeroable};
use std::{
    collections::hash_map::Entry,
    error::Error,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use super::{dim2::rectangle::RectangleVertex, node::constructor::NodeConstructor};

use crate::lazy_static::*;

lazy_static! {
    /// The default material for tiles that have no material set.
    pub static ref DEFAULT_TILE_MATERIAL: MaterialResource = MaterialResource::new_ok(
        ResourceKind::External("__DefaultTileMaterial".into()),
        Material::standard_tile()
    );
}

/// Swaps the content of a hash map entry with the content of an `Option`.
pub fn swap_hash_map_entry<K, V>(entry: Entry<K, V>, value: &mut Option<V>) {
    match (entry, value) {
        (Entry::Occupied(entry), p @ None) => *p = Some(entry.remove()),
        (Entry::Occupied(mut entry), Some(p)) => std::mem::swap(entry.get_mut(), p),
        (Entry::Vacant(_), None) => (),
        (Entry::Vacant(entry), p @ Some(_)) => drop(entry.insert(p.take().unwrap())),
    }
}

/// Swaps the content of two hash map entries.
pub fn swap_hash_map_entries<K0, K1, V>(entry0: Entry<K0, V>, entry1: Entry<K1, V>) {
    match (entry0, entry1) {
        (Entry::Occupied(e0), Entry::Vacant(e1)) => drop(e1.insert(e0.remove())),
        (Entry::Occupied(mut e0), Entry::Occupied(mut e1)) => {
            std::mem::swap(e0.get_mut(), e1.get_mut())
        }
        (Entry::Vacant(_), Entry::Vacant(_)) => (),
        (Entry::Vacant(e0), Entry::Occupied(e1)) => drop(e0.insert(e1.remove())),
    }
}

/// A record of the number of changes that have happened since the most recent save.
/// It is potentially negative, which represents undo changes to reach a state
/// from before the most recent save.
#[derive(Default, Debug, Copy, Clone)]
pub struct ChangeCount(bool);

impl ChangeCount {
    /// True if there are changes.
    #[inline]
    pub fn needs_save(&self) -> bool {
        self.0
    }
    /// Reset the number of changes to zero.
    #[inline]
    pub fn reset(&mut self) {
        self.0 = false;
    }
    /// Increase or decrease the number of changes, including the possibility of creating a negative number of changes.
    #[inline]
    pub fn increment(&mut self) {
        self.0 = true;
    }
}

/// A vertex for tiles.
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct TileVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates measured in pixels.
    pub tex_coord: Vector2<u32>,
    /// Diffuse color.
    pub color: Color,
}

impl VertexTrait for TileVertex {
    fn layout() -> &'static [VertexAttributeDescriptor] {
        &[
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Position,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 0,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::TexCoord0,
                data_type: VertexAttributeDataType::U32,
                size: 2,
                divisor: 0,
                shader_location: 1,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Color,
                data_type: VertexAttributeDataType::U8,
                size: 4,
                divisor: 0,
                shader_location: 2,
                normalized: true,
            },
        ]
    }
}

/// Each brush and tile set has two palette areas: the pages and the tiles within each page.
/// These two areas are called stages, and each of the two stages needs to be handled separately.
/// Giving a particular `TilePaletteStage` to a tile map palette will control which kind of
/// tiles it will display.
#[derive(Clone, Copy, Default, Debug, Visit, Reflect, PartialEq)]
pub enum TilePaletteStage {
    /// The page tile stage. These tiles allow the user to select which page they want to use.
    #[default]
    Pages,
    /// The stage for tiles within a page.
    Tiles,
}

/// Tile is a base block of a tile map. It has a position and a handle of tile definition, stored
/// in the respective tile set.
#[derive(Clone, Reflect, Default, Debug, PartialEq, Visit, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "e429ca1b-a311-46c3-b580-d5a2f49db7e2")]
pub struct Tile {
    /// Position of the tile (in grid coordinates).
    pub position: Vector2<i32>,
    /// A handle of the tile definition.
    pub definition_handle: TileDefinitionHandle,
}

/// Adapt an iterator over positions into an iterator over `(Vector2<i32>, TileHandleDefinition)`.
#[derive(Debug, Clone)]
pub struct TileIter<I> {
    source: TileResource,
    stage: TilePaletteStage,
    page: Vector2<i32>,
    positions: I,
}

impl<I: Iterator<Item = Vector2<i32>>> Iterator for TileIter<I> {
    type Item = (Vector2<i32>, TileDefinitionHandle);

    fn next(&mut self) -> Option<Self::Item> {
        self.positions.find_map(|p| {
            let h = self.source.get_tile_handle(self.stage, self.page, p)?;
            Some((p, h))
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Visit, Reflect)]
/// Abstract source of tiles, which can either be a tile set or a brush.
pub enum TileResource {
    /// A tile resource containing no tiles.
    #[default]
    Empty,
    /// Getting tiles from a tile set
    TileSet(TileSetResource),
    /// Getting tiles from a brush
    Brush(TileMapBrushResource),
}

impl TileResource {
    #[inline]
    pub fn page_icon(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        match self {
            TileResource::Empty => None,
            TileResource::TileSet(r) => r.state().data()?.page_icon(position),
            TileResource::Brush(r) => r.state().data()?.page_icon(position),
        }
    }
    /// Returns true if this resource is a tile set.
    #[inline]
    pub fn is_tile_set(&self) -> bool {
        matches!(self, TileResource::TileSet(_))
    }
    /// Returns true if this resource is a brush.
    #[inline]
    pub fn is_brush(&self) -> bool {
        matches!(self, TileResource::Brush(_))
    }
    /// Return the path of the resource.
    pub fn path(&self) -> Option<PathBuf> {
        match self {
            TileResource::Empty => None,
            TileResource::TileSet(r) => r.kind().into_path(),
            TileResource::Brush(r) => r.kind().into_path(),
        }
    }
    /// True if the resource is external and its `change_count` is not zero.
    pub fn needs_save(&self) -> bool {
        match self {
            TileResource::Empty => false,
            TileResource::TileSet(r) => {
                r.header().kind.is_external() && r.data_ref().change_count.needs_save()
            }
            TileResource::Brush(r) => {
                r.header().kind.is_external() && r.data_ref().change_count.needs_save()
            }
        }
    }
    /// Attempt to save the resource to its file, if it has one and if `change_count` not zero.
    /// Otherwise do nothing and return Ok to indicate success.
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        match self {
            TileResource::Empty => Ok(()),
            TileResource::TileSet(r) => {
                if r.header().kind.is_external() && r.data_ref().change_count.needs_save() {
                    let result = r.save_back();
                    if result.is_ok() {
                        r.data_ref().change_count.reset();
                    }
                    result
                } else {
                    Ok(())
                }
            }
            TileResource::Brush(r) => {
                if r.header().kind.is_external() && r.data_ref().change_count.needs_save() {
                    let result = r.save_back();
                    if result.is_ok() {
                        r.data_ref().change_count.reset();
                    }
                    result
                } else {
                    Ok(())
                }
            }
        }
    }
    /// Returns the tile set associated with this resource.
    /// If the resource is a tile set, the return that tile set.
    /// If the resource is a brush, then return the tile set used by that brush.
    pub fn get_tile_set(&self) -> Option<TileSetResource> {
        match self {
            TileResource::Empty => None,
            TileResource::TileSet(r) => Some(r.clone()),
            TileResource::Brush(r) => r.state().data()?.tile_set.clone(),
        }
    }
    /// Build a list of the positions of all tiles on the given page.
    pub fn get_all_tile_positions(&self, page: Vector2<i32>) -> Vec<Vector2<i32>> {
        match self {
            TileResource::Empty => Vec::new(),
            TileResource::TileSet(r) => r
                .state()
                .data()
                .map(|r| r.keys_on_page(page))
                .unwrap_or_default(),
            TileResource::Brush(r) => r
                .state()
                .data()
                .and_then(|r| {
                    r.pages
                        .get(&page)
                        .map(|p| p.tiles.keys().copied().collect())
                })
                .unwrap_or_default(),
        }
    }
    /// Build a list of the posiitons of all pages.
    pub fn get_all_page_positions(&self) -> Vec<Vector2<i32>> {
        match self {
            TileResource::Empty => Vec::new(),
            TileResource::TileSet(r) => r.state().data().map(|r| r.page_keys()).unwrap_or_default(),
            TileResource::Brush(r) => r
                .state()
                .data()
                .map(|r| r.pages.keys().copied().collect())
                .unwrap_or_default(),
        }
    }
    /// True if there is a page at the given position.
    pub fn has_page_at(&self, position: Vector2<i32>) -> bool {
        match self {
            TileResource::Empty => false,
            TileResource::TileSet(r) => r
                .state()
                .data()
                .map(|r| r.pages.contains_key(&position))
                .unwrap_or(false),
            TileResource::Brush(r) => r
                .state()
                .data()
                .map(|r| r.pages.contains_key(&position))
                .unwrap_or(false),
        }
    }
    /// True if there is a material page at the given coordinates.
    pub fn is_material_page(&self, position: Vector2<i32>) -> bool {
        match self {
            TileResource::TileSet(r) => r
                .state()
                .data()
                .and_then(|r| r.pages.get(&position))
                .map(|p| matches!(p.source, TileSetPageSource::Material(_)))
                .unwrap_or(false),
            _ => false,
        }
    }
    /// True if there is a free tile page at the given coordinates.
    pub fn is_free_page(&self, position: Vector2<i32>) -> bool {
        match self {
            TileResource::TileSet(r) => r
                .state()
                .data()
                .and_then(|r| r.pages.get(&position))
                .map(|p| matches!(p.source, TileSetPageSource::Freeform(_)))
                .unwrap_or(false),
            _ => false,
        }
    }
    /// True if there is a transform page at the given coordinates.
    pub fn is_transform_page(&self, position: Vector2<i32>) -> bool {
        match self {
            TileResource::TileSet(r) => r
                .state()
                .data()
                .and_then(|r| r.pages.get(&position))
                .map(|p| matches!(p.source, TileSetPageSource::TransformSet(_)))
                .unwrap_or(false),
            _ => false,
        }
    }
    /// True if there is a brush page at the given coordinates.
    pub fn is_brush_page(&self, position: Vector2<i32>) -> bool {
        match self {
            TileResource::Brush(r) => r
                .state()
                .data()
                .map(|r| r.pages.contains_key(&position))
                .unwrap_or(false),
            _ => false,
        }
    }
    /// Return true if there is a tile at the given position on the page at the given position.
    pub fn has_tile_at(&self, page: Vector2<i32>, tile: Vector2<i32>) -> bool {
        match self {
            TileResource::Empty => false,
            TileResource::TileSet(r) => r
                .state()
                .data()
                .map(|r| r.has_tile_at(page, tile))
                .unwrap_or(false),
            TileResource::Brush(r) => r
                .state()
                .data()
                .map(|r| r.has_tile_at(page, tile))
                .unwrap_or(false),
        }
    }
    /// Returns the TileDefinitionHandle that points to the data in the tile set that represents this tile.
    /// Even if this resource is actually a brush, the handle returned still refers to some page and position
    /// in the brush's tile set.
    pub fn get_tile_handle(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        position: Vector2<i32>,
    ) -> Option<TileDefinitionHandle> {
        match self {
            TileResource::Empty => None,
            TileResource::TileSet(r) => r
                .state()
                .data()?
                .find_tile_at_position(stage, page, position),
            TileResource::Brush(r) => r
                .state()
                .data()?
                .find_tile_at_position(stage, page, position),
        }
    }
    /// Returns an iterator over `(Vector2<i32>, TileDefinitionHandle)` where the first
    /// member of the pair is the position of the tile on the page as provided by `positions`
    /// and the second member is the handle that would be returned from [`get_tile_handle`](Self::get_tile_handle).
    pub fn get_tile_iter<I: Iterator<Item = Vector2<i32>>>(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        positions: I,
    ) -> TileIter<I> {
        TileIter {
            source: self.clone(),
            stage,
            page,
            positions,
        }
    }
    /// Construct a Tiles object holding the tile definition handles for the tiles
    /// at the given positions on the given page.
    pub fn get_tiles<I: Iterator<Item = Vector2<i32>>>(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        iter: I,
        tiles: &mut Tiles,
    ) {
        match self {
            TileResource::Empty => (),
            TileResource::TileSet(res) => {
                if let Some(tile_set) = res.state().data() {
                    tile_set.get_tiles(stage, page, iter, tiles);
                }
            }
            TileResource::Brush(res) => {
                if let Some(brush) = res.state().data() {
                    brush.get_tiles(stage, page, iter, tiles);
                }
            }
        }
    }
    /// Repeatedly call the given function with each tile for the given stage and page.
    /// The function is given the position of the tile within the palette and the
    /// data for rendering the tile.
    pub fn tile_render_loop<F>(&self, stage: TilePaletteStage, page: Vector2<i32>, func: F)
    where
        F: FnMut(Vector2<i32>, TileRenderData),
    {
        match self {
            TileResource::Empty => (),
            TileResource::TileSet(res) => {
                if let Some(data) = res.state().data() {
                    data.palette_render_loop(stage, page, func)
                }
            }
            TileResource::Brush(res) => {
                if let Some(data) = res.state().data() {
                    data.palette_render_loop(stage, page, func)
                }
            }
        };
    }
    /// Returns the rectangle within a material that a tile should show
    /// at the given stage and handle.
    pub fn get_tile_bounds(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<TileMaterialBounds> {
        match self {
            TileResource::Empty => None,
            TileResource::TileSet(res) => res
                .state()
                .data()
                .map(|d| d.get_tile_bounds(stage, handle))
                .unwrap_or_default(),
            TileResource::Brush(res) => res
                .state()
                .data()
                .map(|d| d.get_tile_bounds(stage, handle))
                .unwrap_or_default(),
        }
    }
    /// Returns a reference to the data stored with a tile at the given stage and handle.
    pub fn get_tile_data<F, V>(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
        func: F,
    ) -> Option<V>
    where
        F: FnOnce(&TileData) -> V,
    {
        match self {
            TileResource::Empty => None,
            TileResource::TileSet(res) => Some(func(res.data_ref().get_tile_data(stage, handle)?)),
            TileResource::Brush(res) => res.data_ref().get_tile_data(stage, handle, func),
        }
    }
    /// The bounds of the tiles on the given page.
    pub fn tiles_bounds(&self, stage: TilePaletteStage, page: Vector2<i32>) -> OptionTileRect {
        match self {
            TileResource::Empty => OptionTileRect::default(),
            TileResource::TileSet(res) => res.data_ref().tiles_bounds(stage, page),
            TileResource::Brush(res) => res.data_ref().tiles_bounds(stage, page),
        }
    }
    /// Fills the tile resource at the given point using the given tile source. This method
    /// extends the resource when trying to fill at a point that lies outside the bounding rectangle.
    /// Keep in mind, that flood fill is only possible either on free cells or on cells with the same
    /// tile kind.
    pub fn flood_fill<S: TileSource>(
        &self,
        page: Vector2<i32>,
        position: Vector2<i32>,
        brush: &S,
        tiles: &mut TransTilesUpdate,
    ) {
        match self {
            TileResource::Empty => (),
            TileResource::TileSet(_) => (),
            TileResource::Brush(res) => {
                let data = res.data_ref();
                let Some(source) = data.pages.get(&page) else {
                    return;
                };
                tiles.flood_fill(&source.tiles, position, brush);
            }
        }
    }
}

/// The specification for how to render a tile.
#[derive(Clone, Default, Debug)]
pub struct TileRenderData {
    /// The material to use to render this tile.
    pub material_bounds: Option<TileMaterialBounds>,
    /// The color to use to render the tile
    pub color: Color,
}

impl TileRenderData {
    /// Returns TileRenderData to represent an error due to render data being unavailable.
    pub fn missing_data() -> TileRenderData {
        Self {
            material_bounds: None,
            color: Color::HOT_PINK,
        }
    }
}

impl OrthoTransform for TileRenderData {
    fn x_flipped(mut self) -> Self {
        self.material_bounds = self.material_bounds.map(|b| b.x_flipped());
        self
    }

    fn rotated(mut self, amount: i8) -> Self {
        self.material_bounds = self.material_bounds.map(|b| b.rotated(amount));
        self
    }
}

/// Tile map is a 2D "image", made out of a small blocks called tiles. Tile maps used in 2D games to
/// build game worlds quickly and easily.
///
/// ## Example
///
/// The following example creates a simple tile map with two tile types - grass and stone. It creates
/// stone foundation and lays grass on top of it.
///
/// ```rust
/// use fyrox_impl::{
///     asset::untyped::ResourceKind,
///     core::{algebra::Vector2, color::Color, math::Rect, pool::Handle},
///     material::{Material, MaterialResource},
///     scene::{
///         base::BaseBuilder,
///         graph::Graph,
///         node::Node,
///         tilemap::{
///             tileset::{TileCollider, TileDefinition, TileSet, TileSetResource},
///             Tile, TileMapBuilder, Tiles,
///         },
///     },
/// };
///
/// fn create_tile_map(graph: &mut Graph) -> Handle<Node> {
///     // Each tile could have its own material, for simplicity it is just a standard 2D material.
///     let material = MaterialResource::new_ok(ResourceKind::Embedded, Material::standard_2d());
///
///     // Create a tile set - it is a data source for the tile map. Tile map will reference the tiles
///     // stored in the tile set by handles. We'll create two tile types with different colors.
///     let mut tile_set = TileSet::default();
///     let stone_tile = tile_set.add_tile(TileDefinition {
///         material: material.clone(),
///         uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
///         collider: TileCollider::Rectangle,
///         color: Color::BROWN,
///         position: Default::default(),
///         properties: vec![],
///     });
///     let grass_tile = tile_set.add_tile(TileDefinition {
///         material,
///         uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
///         collider: TileCollider::Rectangle,
///         color: Color::GREEN,
///         position: Default::default(),
///         properties: vec![],
///     });
///     let tile_set = TileSetResource::new_ok(ResourceKind::Embedded, tile_set);
///
///     let mut tiles = Tiles::default();
///
///     // Create stone foundation.
///     for x in 0..10 {
///         for y in 0..2 {
///             tiles.insert(Tile {
///                 position: Vector2::new(x, y),
///                 definition_handle: stone_tile,
///             });
///         }
///     }
///
///     // Add grass on top of it.
///     for x in 0..10 {
///         tiles.insert(Tile {
///             position: Vector2::new(x, 2),
///             definition_handle: grass_tile,
///         });
///     }
///
///     // Finally create the tile map.
///     TileMapBuilder::new(BaseBuilder::new())
///         .with_tile_set(tile_set)
///         .with_tiles(tiles)
///         .build(graph)
/// }
/// ```
#[derive(Clone, Reflect, Debug, Visit, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "aa9a3385-a4af-4faf-a69a-8d3af1a3aa67")]
pub struct TileMap {
    base: Base,
    tile_set: InheritableVariable<Option<TileSetResource>>,
    /// Tile container of the tile map.
    #[reflect(hidden)]
    pub tiles: InheritableVariable<Tiles>,
    tile_scale: InheritableVariable<Vector2<f32>>,
    brushes: InheritableVariable<Vec<Option<TileMapBrushResource>>>,
    active_brush: InheritableVariable<Option<TileMapBrushResource>>,
}

impl TileSource for TileMap {
    fn transformation(&self) -> OrthoTransformation {
        OrthoTransformation::default()
    }
    fn get_at(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.tiles.get_at(position)
    }
}

impl TileMap {
    /// Returns a reference to the current tile set (if any).
    #[inline]
    pub fn tile_set(&self) -> Option<&TileSetResource> {
        self.tile_set.as_ref()
    }

    /// Sets new tile set.
    #[inline]
    pub fn set_tile_set(&mut self, tile_set: Option<TileSetResource>) {
        self.tile_set.set_value_and_mark_modified(tile_set);
    }

    /// Returns a reference to the tile container.
    #[inline]
    pub fn tiles(&self) -> &Tiles {
        &self.tiles
    }

    /// Iterate the tiles.
    pub fn iter(&self) -> impl Iterator<Item = Tile> + '_ {
        self.tiles.iter().map(|(p, h)| Tile {
            position: *p,
            definition_handle: *h,
        })
    }

    /// Sets new tiles.
    #[inline]
    pub fn set_tiles(&mut self, tiles: Tiles) {
        self.tiles.set_value_and_mark_modified(tiles);
    }

    /// Returns current tile scaling.
    #[inline]
    pub fn tile_scale(&self) -> Vector2<f32> {
        *self.tile_scale
    }

    /// Sets new tile scaling, which defines tile size.
    #[inline]
    pub fn set_tile_scale(&mut self, tile_scale: Vector2<f32>) {
        self.tile_scale.set_value_and_mark_modified(tile_scale);
    }

    /// Inserts a tile in the tile map. Returns previous tile, located at the same position as
    /// the new one (if any).
    #[inline]
    pub fn insert_tile(
        &mut self,
        position: Vector2<i32>,
        tile: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        self.tiles.insert(position, tile)
    }

    /// Removes a tile from the tile map.
    #[inline]
    pub fn remove_tile(&mut self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.tiles.remove(&position)
    }

    /// Returns active brush of the tile map.
    #[inline]
    pub fn active_brush(&self) -> Option<TileMapBrushResource> {
        (*self.active_brush).clone()
    }

    /// Sets new active brush of the tile map.
    #[inline]
    pub fn set_active_brush(&mut self, brush: Option<TileMapBrushResource>) {
        self.active_brush.set_value_and_mark_modified(brush);
    }

    /// Returns a reference to the set of brushes.
    #[inline]
    pub fn brushes(&self) -> &[Option<TileMapBrushResource>] {
        &self.brushes
    }

    /// Sets news brushes of the tile map. This set could be used to store the most used brushes.
    #[inline]
    pub fn set_brushes(&mut self, brushes: Vec<Option<TileMapBrushResource>>) {
        self.brushes.set_value_and_mark_modified(brushes);
    }

    /// Calculates bounding rectangle in grid coordinates.
    #[inline]
    pub fn bounding_rect(&self) -> OptionTileRect {
        self.tiles.bounding_rect()
    }

    /// Calculates grid-space position (tile coordinates) from world-space. Could be used to find
    /// tile coordinates from arbitrary point in world space. It is especially useful, if the tile
    /// map is rotated or shifted.
    #[inline]
    pub fn world_to_grid(&self, world_position: Vector3<f32>) -> Vector2<i32> {
        let inv_global_transform = self.global_transform().try_inverse().unwrap_or_default();
        let local_space_position = inv_global_transform.transform_point(&world_position.into());
        Vector2::new(
            local_space_position.x.round() as i32,
            local_space_position.y.round() as i32,
        )
    }

    /// Calculates world-space position from grid-space position (tile coordinates).
    #[inline]
    pub fn grid_to_world(&self, grid_position: Vector2<i32>) -> Vector3<f32> {
        let v3 = grid_position.cast::<f32>().to_homogeneous();
        self.global_transform().transform_point(&v3.into()).coords
    }

    fn push_color_tile(&self, position: Vector2<i32>, color: Color, ctx: &mut RenderContext) {
        let global_transform = self.global_transform();
        let position = position.cast::<f32>().to_homogeneous();
        let vertices = [
            RectangleVertex {
                position: global_transform
                    .transform_point(&(position + Vector3::new(0.0, 1.0, 0.0)).into())
                    .coords,
                tex_coord: Vector2::default(),
                color,
            },
            RectangleVertex {
                position: global_transform
                    .transform_point(&(position + Vector3::new(1.0, 1.0, 0.0)).into())
                    .coords,
                tex_coord: Vector2::default(),
                color,
            },
            RectangleVertex {
                position: global_transform
                    .transform_point(&(position + Vector3::new(1.00, 0.0, 0.0)).into())
                    .coords,
                tex_coord: Vector2::default(),
                color,
            },
            RectangleVertex {
                position: global_transform
                    .transform_point(&(position + Vector3::new(0.0, 0.0, 0.0)).into())
                    .coords,
                tex_coord: Vector2::default(),
                color,
            },
        ];

        let triangles = [TriangleDefinition([0, 1, 2]), TriangleDefinition([2, 3, 0])];

        let sort_index = ctx.calculate_sorting_index(self.global_position());

        ctx.storage.push_triangles(
            RectangleVertex::layout(),
            &STANDARD_2D.resource,
            RenderPath::Forward,
            sort_index,
            self.handle(),
            &mut move |mut vertex_buffer, mut triangle_buffer| {
                let start_vertex_index = vertex_buffer.vertex_count();

                vertex_buffer.push_vertices(&vertices).unwrap();

                triangle_buffer
                    .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
            },
        );
    }

    fn push_tile(
        &self,
        position: Vector2<i32>,
        material: &MaterialResource,
        bounds: &TileBounds,
        color: Color,
        ctx: &mut RenderContext,
    ) {
        let global_transform = self.global_transform();
        let position = position.cast::<f32>().to_homogeneous();
        let vertices = [
            TileVertex {
                position: global_transform
                    .transform_point(&(position + Vector3::new(0.0, 1.0, 0.0)).into())
                    .coords,
                tex_coord: bounds.right_top_corner,
                color,
            },
            TileVertex {
                position: global_transform
                    .transform_point(&(position + Vector3::new(1.0, 1.0, 0.0)).into())
                    .coords,
                tex_coord: bounds.left_top_corner,
                color,
            },
            TileVertex {
                position: global_transform
                    .transform_point(&(position + Vector3::new(1.00, 0.0, 0.0)).into())
                    .coords,
                tex_coord: bounds.left_bottom_corner,
                color,
            },
            TileVertex {
                position: global_transform
                    .transform_point(&(position + Vector3::new(0.0, 0.0, 0.0)).into())
                    .coords,
                tex_coord: bounds.right_bottom_corner,
                color,
            },
        ];

        let triangles = [TriangleDefinition([0, 1, 2]), TriangleDefinition([2, 3, 0])];

        let sort_index = ctx.calculate_sorting_index(self.global_position());

        ctx.storage.push_triangles(
            TileVertex::layout(),
            material,
            RenderPath::Forward,
            sort_index,
            self.handle(),
            &mut move |mut vertex_buffer, mut triangle_buffer| {
                let start_vertex_index = vertex_buffer.vertex_count();

                vertex_buffer.push_vertices(&vertices).unwrap();

                triangle_buffer
                    .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
            },
        );
    }
}

impl Default for TileMap {
    fn default() -> Self {
        Self {
            base: Default::default(),
            tile_set: Default::default(),
            tiles: Default::default(),
            tile_scale: Vector2::repeat(1.0).into(),
            brushes: Default::default(),
            active_brush: Default::default(),
        }
    }
}

impl Deref for TileMap {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for TileMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl ConstructorProvider<Node, Graph> for TileMap {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_variant("Tile Map", |_| {
                TileMapBuilder::new(BaseBuilder::new().with_name("Tile Map"))
                    .build_node()
                    .into()
            })
            .with_group("2D")
    }
}

impl NodeTrait for TileMap {
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        let Some(rect) = *self.bounding_rect() else {
            return AxisAlignedBoundingBox::default();
        };

        let min_pos = rect.position.cast::<f32>().to_homogeneous();
        let max_pos = (rect.position + rect.size).cast::<f32>().to_homogeneous();

        AxisAlignedBoundingBox::from_min_max(min_pos, max_pos)
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.should_be_rendered(ctx.frustum) {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) {
            return RdcControlFlow::Continue;
        }

        let Some(ref tile_set_resource) = *self.tile_set else {
            return RdcControlFlow::Continue;
        };

        if !tile_set_resource.is_ok() {
            return RdcControlFlow::Continue;
        }

        let tile_set = tile_set_resource.data_ref();

        for (position, definition_handle) in self.tiles.iter() {
            let Some(data) =
                tile_set.get_tile_render_data(TilePaletteStage::Tiles, *definition_handle)
            else {
                continue;
            };
            let tile_bounds = &data.material_bounds;
            let mat = tile_bounds.as_ref().map(|b| &b.material);
            let def_bounds = TileBounds::default();
            let bounds = tile_bounds
                .as_ref()
                .map(|b| &b.bounds)
                .unwrap_or(&def_bounds);
            let color = data.color;
            if let Some(material) = mat {
                self.push_tile(*position, material, bounds, color, ctx);
            } else {
                self.push_color_tile(*position, color, ctx);
            }
        }

        RdcControlFlow::Continue
    }

    fn validate(&self, _scene: &Scene) -> Result<(), String> {
        if self.tile_set.is_none() {
            Err(
                "Tile set resource is not set. Tile map will not be rendered correctly!"
                    .to_string(),
            )
        } else {
            Ok(())
        }
    }
}

/// Tile map builder allows you to create [`TileMap`] scene nodes.
pub struct TileMapBuilder {
    base_builder: BaseBuilder,
    tile_set: Option<TileSetResource>,
    tiles: Tiles,
    tile_scale: Vector2<f32>,
    brushes: Vec<Option<TileMapBrushResource>>,
}

impl TileMapBuilder {
    /// Creates new tile map builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            tile_set: None,
            tiles: Default::default(),
            tile_scale: Vector2::repeat(1.0),
            brushes: Default::default(),
        }
    }

    /// Sets the desired tile set.
    pub fn with_tile_set(mut self, tile_set: TileSetResource) -> Self {
        self.tile_set = Some(tile_set);
        self
    }

    /// Sets the actual tiles of the tile map.
    pub fn with_tiles(mut self, tiles: Tiles) -> Self {
        self.tiles = tiles;
        self
    }

    /// Sets the actual tile scaling.
    pub fn with_tile_scale(mut self, tile_scale: Vector2<f32>) -> Self {
        self.tile_scale = tile_scale;
        self
    }

    /// Sets brushes of the tile map.
    pub fn with_brushes(mut self, brushes: Vec<Option<TileMapBrushResource>>) -> Self {
        self.brushes = brushes;
        self
    }

    /// Builds tile map scene node, but not adds it to a scene graph.
    pub fn build_node(self) -> Node {
        Node::new(TileMap {
            base: self.base_builder.build_base(),
            tile_set: self.tile_set.into(),
            tiles: self.tiles.into(),
            tile_scale: self.tile_scale.into(),
            brushes: self.brushes.into(),
            active_brush: Default::default(),
        })
    }

    /// Finishes tile map building and adds it to the specified scene graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
