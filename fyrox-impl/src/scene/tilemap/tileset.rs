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

//! Tile set is a special storage for tile descriptions. It is a sort of database, that contains
//! descriptions (definitions) for tiles. Tiles are organized into pages. Each page has particular
//! (x,y) coordinates and each tile within the page has its own (x,y) coordinates, so finding a tile
//! requires two pairs of coordinates (x,y):(x,y), which is called a [`TileDefinitionHandle`].
//!
//! A [`TileMap`] stores a `TileDefinitionHandle` for each cell, and it uses those handles to index
//! into a tile set to know how each cell should be rendered.
//!
//! See [`TileSet`] docs for more info and usage examples.

use crate::{
    asset::{
        io::ResourceIo,
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
        manager::ResourceManager,
        state::LoadError,
        Resource, ResourceData, ResourceDataRef,
    },
    core::{
        algebra::Vector2, color::Color, io::FileError, log::Log, reflect::prelude::*,
        type_traits::prelude::*, visitor::prelude::*, ImmutableString,
    },
    fxhash::{FxHashMap, FxHashSet},
    material::{MaterialResource, MaterialResourceExtension},
    resource::texture::TextureResource,
};
use std::{
    collections::hash_map::{self, Entry, Keys},
    error::Error,
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Arc,
};

use super::*;
use fyrox_core::{swap_hash_map_entries, swap_hash_map_entry};
pub use property::*;

const DEFAULT_TILE_SIZE: Vector2<u32> = Vector2::new(16, 16);
const DEFAULT_ANIMATION_FRAME_RATE: f32 = 12.0;

/// The color that is used to represent a tile where the property value matches the value that is
/// currently be drawn. This is only used when the the property value does not have a specially assigned color.
pub const ELEMENT_MATCH_HIGHLIGHT_COLOR: Color = Color::from_rgba(255, 255, 0, 200);

/// An error that may occur during tile set resource loading.
#[derive(Debug)]
pub enum TileSetResourceError {
    /// An i/o error has occurred.
    Io(FileError),

    /// An error that may occur due to version incompatibilities.
    Visit(VisitError),
}

impl Display for TileSetResourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(v) => {
                write!(f, "A file load error has occurred {v:?}")
            }
            Self::Visit(v) => {
                write!(
                    f,
                    "An error that may occur due to version incompatibilities. {v:?}"
                )
            }
        }
    }
}

impl From<FileError> for TileSetResourceError {
    fn from(e: FileError) -> Self {
        Self::Io(e)
    }
}

impl From<VisitError> for TileSetResourceError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

/// Definition of a tile.
#[derive(Clone, Default, PartialEq, Debug, Reflect, Visit)]
#[visit(optional)]
pub struct TileDefinition {
    /// The tile's material resource and the rect within that material that should be rendered as the tile.
    pub material_bounds: TileMaterialBounds,
    /// Miscellaneous data that is stored with the tile definition.
    pub data: TileData,
}

impl TileDefinition {
    fn from_material_bounds(material_bounds: TileMaterialBounds) -> Self {
        Self {
            material_bounds,
            data: TileData::default(),
        }
    }
}

impl OrthoTransform for TileDefinition {
    fn x_flipped(mut self) -> Self {
        self.material_bounds = self.material_bounds.x_flipped();
        self.data = self.data.x_flipped();
        self
    }

    fn rotated(mut self, amount: i8) -> Self {
        self.material_bounds = self.material_bounds.rotated(amount);
        self.data = self.data.rotated(amount);
        self
    }
}

/// A tile's material resource and the rect within that material that should be rendered as the tile.
#[derive(Clone, PartialEq, Debug, Reflect, Visit)]
pub struct TileMaterialBounds {
    /// Material of the tile.
    pub material: MaterialResource,
    /// A rectangle that defines pixel coordinates range for the tile within the material
    pub bounds: TileBounds,
}

impl Default for TileMaterialBounds {
    fn default() -> Self {
        Self {
            material: Resource::new_ok(
                Uuid::new_v4(),
                ResourceKind::Embedded,
                Material::standard_tile(),
            ),
            bounds: Default::default(),
        }
    }
}

impl OrthoTransform for TileMaterialBounds {
    fn x_flipped(mut self) -> Self {
        self.bounds = self.bounds.x_flipped();
        self
    }

    fn rotated(mut self, amount: i8) -> Self {
        self.bounds = self.bounds.rotated(amount);
        self
    }
}

/// Miscellaneous data that is stored with the tile definition.
#[derive(Clone, Default, PartialEq, Debug, Reflect, Visit)]
#[visit(optional)]
pub struct TileData {
    /// Colliders of the tile.
    pub colliders: FxHashMap<Uuid, TileCollider>,
    /// Color of the tile.
    pub color: Color,
    /// A custom set of properties. Properties could be used to assign additional information for
    /// tiles, such surface type (for example, lava, ice, dirt, etc.), physics properties and so
    /// on.
    pub properties: FxHashMap<Uuid, TileSetPropertyValue>,
}

impl OrthoTransform for TileData {
    fn x_flipped(mut self) -> Self {
        for (_, value) in self.properties.iter_mut() {
            *value = value.clone().x_flipped();
        }
        for (_, value) in self.colliders.iter_mut() {
            *value = value.clone().x_flipped();
        }
        self
    }

    fn rotated(mut self, amount: i8) -> Self {
        for (_, value) in self.properties.iter_mut() {
            *value = value.clone().rotated(amount);
        }
        for (_, value) in self.colliders.iter_mut() {
            *value = value.clone().rotated(amount);
        }
        self
    }
}

/// Specify the pixel coordinates of each corner of a tile within some material.
/// This is used in place of a standard `Rect` because tiles can be flipped and rotated
/// and otherwise transformed with respect the their source material, so for example the
/// left-top corner of the tile may be to the right of the right-top corner in the material.
#[derive(Clone, Default, PartialEq, Debug, Reflect, Visit)]
pub struct TileBounds {
    /// The position of the tile's left-top corner within its source material.
    pub left_top_corner: Vector2<u32>,
    /// The position of the tile's right-top corner within its source material.
    pub right_top_corner: Vector2<u32>,
    /// The position of the tile's left-bottom corner within its source material.
    pub right_bottom_corner: Vector2<u32>,
    /// The position of the tile's right-bottom corner within its source material.
    pub left_bottom_corner: Vector2<u32>,
}

fn pixel_coords_to_uv(position: Vector2<u32>, total_size: Vector2<u32>) -> Vector2<f32> {
    Vector2::new(
        position.x as f32 / total_size.x as f32,
        position.y as f32 / total_size.y as f32,
    )
}

impl TileBounds {
    /// Translates the pixel position of the corner of the tile into UV coordinates between 0 and 1.
    pub fn left_top_uv(&self, texture_size: Vector2<u32>) -> Vector2<f32> {
        pixel_coords_to_uv(self.left_top_corner, texture_size)
    }
    /// Translates the pixel position of the corner of the tile into UV coordinates between 0 and 1.
    pub fn right_top_uv(&self, texture_size: Vector2<u32>) -> Vector2<f32> {
        pixel_coords_to_uv(self.right_top_corner, texture_size)
    }
    /// Translates the pixel position of the corner of the tile into UV coordinates between 0 and 1.
    pub fn left_bottom_uv(&self, texture_size: Vector2<u32>) -> Vector2<f32> {
        pixel_coords_to_uv(self.left_bottom_corner, texture_size)
    }
    /// Translates the pixel position of the corner of the tile into UV coordinates between 0 and 1.
    pub fn right_bottom_uv(&self, texture_size: Vector2<u32>) -> Vector2<f32> {
        pixel_coords_to_uv(self.right_bottom_corner, texture_size)
    }
    /// Get a corner position based on an index from 0 to 3, in this order:
    /// left_bottom, right_bottom, right_top, left_top
    pub fn get(&self, index: usize) -> Vector2<u32> {
        match index {
            0 => self.left_bottom_corner,
            1 => self.right_bottom_corner,
            2 => self.right_top_corner,
            3 => self.left_top_corner,
            _ => panic!(),
        }
    }
    /// Modify a corner position based on an index from 0 to 3.
    /// left_bottom, right_bottom, right_top, left_top
    pub fn get_mut(&mut self, index: usize) -> &mut Vector2<u32> {
        match index {
            0 => &mut self.left_bottom_corner,
            1 => &mut self.right_bottom_corner,
            2 => &mut self.right_top_corner,
            3 => &mut self.left_top_corner,
            _ => panic!(),
        }
    }
}

impl OrthoTransform for TileBounds {
    fn x_flipped(mut self) -> Self {
        std::mem::swap(&mut self.left_top_corner, &mut self.right_top_corner);
        std::mem::swap(&mut self.left_bottom_corner, &mut self.right_bottom_corner);
        self
    }

    fn rotated(mut self, amount: i8) -> Self {
        let old = self.clone();
        let amount = amount.rem_euclid(4) as usize;
        for i in 0..4 {
            *self.get_mut((i + amount).rem_euclid(4)) = old.get(i);
        }
        self
    }
}

/// A tile set can contain multiple pages of tiles, and each page may have its own
/// independent source of tile data.
#[derive(Clone, Default, Debug, Visit, Reflect)]
pub struct TileSetPage {
    /// The tile that represents this page in the editor
    pub icon: TileDefinitionHandle,
    /// The source of the page's data.
    pub source: TileSetPageSource,
}

impl TileSetPage {
    /// True if this page is a material page.
    pub fn is_material(&self) -> bool {
        matches!(self.source, TileSetPageSource::Atlas(_))
    }
    /// True if this page is a freeform tile page.
    pub fn is_freeform(&self) -> bool {
        matches!(self.source, TileSetPageSource::Freeform(_))
    }
    /// True if this page contains tile transform groups.
    pub fn is_transform_set(&self) -> bool {
        matches!(self.source, TileSetPageSource::Transform(_))
    }
    /// True if this page contains tile transform groups.
    pub fn is_animation(&self) -> bool {
        matches!(self.source, TileSetPageSource::Animation(_))
    }
    /// The type of this page.
    pub fn page_type(&self) -> PageType {
        self.source.page_type()
    }
    /// True if a tile exists at the given position in this page.
    pub fn has_tile_at(&self, position: Vector2<i32>) -> bool {
        match &self.source {
            TileSetPageSource::Atlas(mat) => mat.tiles.contains_key(&position),
            TileSetPageSource::Freeform(map) => map.contains_key(&position),
            TileSetPageSource::Transform(tiles) => tiles.contains_key(&position),
            TileSetPageSource::Animation(tiles) => tiles.contains_key(&position),
        }
    }
    /// Generate a list of all tile positions in this page.
    pub fn keys(&self) -> Vec<Vector2<i32>> {
        match &self.source {
            TileSetPageSource::Atlas(mat) => mat.tiles.keys().copied().collect(),
            TileSetPageSource::Freeform(map) => map.keys().copied().collect(),
            TileSetPageSource::Transform(tiles) => tiles.keys().copied().collect(),
            TileSetPageSource::Animation(tiles) => tiles.keys().copied().collect(),
        }
    }
    /// The rect that contains all the tiles of this page.
    pub fn get_bounds(&self) -> OptionTileRect {
        let mut result = OptionTileRect::default();
        match &self.source {
            TileSetPageSource::Atlas(mat) => {
                for pos in mat.tiles.keys() {
                    result.push(*pos);
                }
            }
            TileSetPageSource::Freeform(map) => {
                for pos in map.keys() {
                    result.push(*pos);
                }
            }
            TileSetPageSource::Transform(tiles) => {
                for pos in tiles.keys() {
                    result.push(*pos);
                }
            }
            TileSetPageSource::Animation(tiles) => {
                for pos in tiles.keys() {
                    result.push(*pos);
                }
            }
        }
        result
    }
    /// Change this tile set according to the specifications in `update`, and change `update` so
    /// that it would refers the change to this tile set, if it were applied again at the same position.
    pub fn swap_tile(&mut self, position: Vector2<i32>, update: &mut TileDataUpdate) {
        match &mut self.source {
            TileSetPageSource::Atlas(map0) => swap_material_tile(map0, position, update),
            TileSetPageSource::Freeform(map0) => swap_freeform_tile(map0, position, update),
            TileSetPageSource::Transform(map0) => swap_transform_tile(map0, position, update),
            TileSetPageSource::Animation(map0) => swap_animation_tile(map0, position, update),
        }
    }

    /// Take all the values for the property with the given id, remove them from the page, and put them into the given hash map.
    /// At the same time, take all the values from the given hash map and put them into the page.
    pub fn swap_all_values_for_property(
        &mut self,
        page: Vector2<i32>,
        property_id: Uuid,
        values: &mut FxHashMap<TileDefinitionHandle, TileSetPropertyValue>,
    ) {
        match &mut self.source {
            TileSetPageSource::Atlas(map0) => {
                let tiles = &mut map0.tiles;
                for (tile, data) in tiles.iter_mut() {
                    let Some(handle) = TileDefinitionHandle::try_new(page, *tile) else {
                        continue;
                    };
                    swap_hash_map_entries(data.properties.entry(property_id), values.entry(handle));
                }
            }
            TileSetPageSource::Freeform(tiles) => {
                for (tile, tile_def) in tiles.iter_mut() {
                    let Some(handle) = TileDefinitionHandle::try_new(page, *tile) else {
                        continue;
                    };
                    swap_hash_map_entries(
                        tile_def.data.properties.entry(property_id),
                        values.entry(handle),
                    );
                }
            }
            TileSetPageSource::Transform(_) => (),
            TileSetPageSource::Animation(_) => (),
        }
    }
    /// Take all the colliders for the given collider id, remove them from the page, and put them into the given hash map.
    /// At the same time, take all the colliders from the given hash map and put them into the page.
    pub fn swap_all_values_for_collider(
        &mut self,
        page: Vector2<i32>,
        collider_id: Uuid,
        values: &mut FxHashMap<TileDefinitionHandle, TileCollider>,
    ) {
        match &mut self.source {
            TileSetPageSource::Atlas(map0) => {
                let tiles = &mut map0.tiles;
                for (tile, data) in tiles.iter_mut() {
                    let Some(handle) = TileDefinitionHandle::try_new(page, *tile) else {
                        continue;
                    };
                    swap_hash_map_entries(data.colliders.entry(collider_id), values.entry(handle));
                }
            }
            TileSetPageSource::Freeform(tiles) => {
                for (tile, tile_def) in tiles.iter_mut() {
                    let Some(handle) = TileDefinitionHandle::try_new(page, *tile) else {
                        continue;
                    };
                    swap_hash_map_entries(
                        tile_def.data.colliders.entry(collider_id),
                        values.entry(handle),
                    );
                }
            }
            _ => panic!(),
        }
    }
}

fn swap_material_tile(
    map0: &mut TileMaterial,
    position: Vector2<i32>,
    update: &mut TileDataUpdate,
) {
    let e0 = map0.tiles.entry(position);
    match (e0, update) {
        (Entry::Occupied(d0), d1 @ TileDataUpdate::Erase) => {
            *d1 = TileDataUpdate::MaterialTile(d0.remove())
        }
        (Entry::Vacant(d0), d1 @ TileDataUpdate::MaterialTile(_)) => {
            d0.insert(d1.take_data());
        }
        (Entry::Vacant(_), TileDataUpdate::Erase) => (),
        (Entry::Occupied(mut d0), d1) => d1.swap_with_data(d0.get_mut()),
        (Entry::Vacant(d0), d1) => {
            let mut data = TileData::default();
            d1.swap_with_data(&mut data);
            let _ = d0.insert(data);
            *d1 = TileDataUpdate::Erase;
        }
    }
}
fn swap_freeform_tile(
    map0: &mut TileGridMap<TileDefinition>,
    position: Vector2<i32>,
    update: &mut TileDataUpdate,
) {
    let e0 = map0.entry(position);
    match (e0, update) {
        (Entry::Occupied(mut d0), TileDataUpdate::Material(d1)) => {
            std::mem::swap(&mut d0.get_mut().material_bounds, d1)
        }
        (Entry::Vacant(d0), d1 @ TileDataUpdate::Material(_)) => {
            let TileDataUpdate::Material(material_bounds) = std::mem::take(d1) else {
                unreachable!();
            };
            let def = TileDefinition::from_material_bounds(material_bounds);
            let _ = d0.insert(def);
        }
        (Entry::Occupied(mut d0), TileDataUpdate::FreeformTile(d1)) => {
            std::mem::swap(d0.get_mut(), d1);
        }
        (Entry::Occupied(d0), d1 @ TileDataUpdate::Erase) => {
            *d1 = TileDataUpdate::FreeformTile(d0.remove())
        }
        (Entry::Vacant(d0), d1 @ TileDataUpdate::FreeformTile(_)) => {
            d0.insert(d1.take_definition());
        }
        (Entry::Vacant(_), TileDataUpdate::Erase) => (),
        (Entry::Occupied(mut d0), d1) => d1.swap_with_data(&mut d0.get_mut().data),
        (Entry::Vacant(_), d1) => {
            // We cannot create a freeform tile without a material, so destroy the unusable data in d1.
            *d1 = TileDataUpdate::Erase;
        }
    }
}

fn swap_transform_tile(
    map0: &mut TransformSetTiles,
    position: Vector2<i32>,
    update: &mut TileDataUpdate,
) {
    let e0 = map0.entry(position);
    let TileDataUpdate::TransformSet(handle) = update else {
        panic!()
    };
    swap_hash_map_entry(e0, handle);
}

fn swap_animation_tile(
    map0: &mut AnimationTiles,
    position: Vector2<i32>,
    update: &mut TileDataUpdate,
) {
    let e0 = map0.entry(position);
    let TileDataUpdate::TransformSet(handle) = update else {
        panic!()
    };
    swap_hash_map_entry(e0, handle);
}

/// A tile set contains three forms of tile, depending on the type of page.
/// This enum can represent a tile in any of those three forms.
#[derive(Debug, Clone)]
pub enum AbstractTile {
    /// A material tile contains data but no material information, because
    /// the material information comes from an atlas that spans the entire page.
    Atlas(TileData),
    /// A freeform tile contains a complete definition, including material,
    /// the UV bounds of the tile within that material, and tile data.
    /// Freeform tiles are the most flexible kind of tile.
    Freeform(TileDefinition),
    /// A transform tile contains no data, but it has a handle that refers to
    /// some tile somewhere else in the set. A transform page contains
    /// transform tiles in groups of 8 and specifies how its tiles are to be
    /// rotated and flipped.
    Transform(TileDefinitionHandle),
}

/// This is where tile set pages store their tile data.
#[derive(Clone, PartialEq, Debug, Visit, Reflect)]
pub enum TileSetPageSource {
    /// A page that gets its data from a material resource and arranges its tiles according to their positions in the material.
    /// All tiles in an atlas page share the same material and their UV data is automatically calculated based on the position
    /// of the tile on the page, with the tile at (0,-1) being the top-left corner of the material, and negative-y going toward
    /// the bottom of the material.
    Atlas(TileMaterial),
    /// A page that contains arbitrary tile definitions. These tiles are free to specify any material and any UV values for each
    /// tile, and tiles can be positioned anywhere on the page.
    Freeform(TileGridMap<TileDefinition>),
    /// A page that contains no tile definitions, but contains handles likes a brush.
    /// Handles into a transform set page can be used to connect a tile to a transformed version of that tile.
    /// Tiles are arranged in groups of 8, one for each of four 90-degree rotations, and four horizontally flipped 90-degree rotations.
    /// No two transform tiles may share the same handle, because that would
    /// cause the transformations to be ambiguous.
    Transform(TransformSetTiles),
    /// A page that contains no tile data, but contains handles referencing tiles
    /// on other pages and specifies how tiles animate over time.
    /// Animations proceed from left-to-right, with increasing x-coordinate,
    /// along continuous rows of tiles, until an empty cell is found, and then
    /// the animation returns to the start of the sequence and repeats.
    Animation(AnimationTiles),
}

impl Default for TileSetPageSource {
    fn default() -> Self {
        Self::Atlas(TileMaterial::default())
    }
}

impl TileSetPageSource {
    /// Create a new default material page.
    pub fn new_material() -> Self {
        Self::Atlas(TileMaterial::default())
    }
    /// Create a new default freeform page.
    pub fn new_free() -> Self {
        Self::Freeform(TileGridMap::default())
    }
    /// Create a new default transform page.
    pub fn new_transform() -> Self {
        Self::Transform(TransformSetTiles::default())
    }
    /// Create a new default transform page.
    pub fn new_animation() -> Self {
        Self::Animation(AnimationTiles::default())
    }
    /// The type of this page.
    pub fn page_type(&self) -> PageType {
        match self {
            TileSetPageSource::Atlas(_) => PageType::Atlas,
            TileSetPageSource::Freeform(_) => PageType::Freeform,
            TileSetPageSource::Transform(_) => PageType::Transform,
            TileSetPageSource::Animation(_) => PageType::Animation,
        }
    }
    /// True if this page contains some tile at the given position.
    pub fn contains_tile_at(&self, position: Vector2<i32>) -> bool {
        match self {
            TileSetPageSource::Atlas(map) => map.contains_key(&position),
            TileSetPageSource::Freeform(map) => map.contains_key(&position),
            TileSetPageSource::Transform(map) => map.contains_key(&position),
            TileSetPageSource::Animation(map) => map.contains_key(&position),
        }
    }
}

/// The tile data for transform set pages of a [`TileSet`].
/// Each transform set page contains a hash map of [`TileDefinitionHandle`]
/// that are divided into groups of 8, and each group is arranged into 2x4,
/// with two rows and four columns. The left 2x2 represent four 90-degree rotations
/// of a tile, so any tile within that 2x2 can be transformed into any other by rotation.
/// The right 2x2 represents the same rotated tile but flipped horizontally, which allows
/// for any combination of 90-degree rotations, horizontal flips, and vertical flips by
/// moving around the 2x4 grid.
#[derive(Default, Clone, PartialEq, Debug, Reflect, Visit)]
pub struct TransformSetTiles(
    #[reflect(hidden)]
    #[visit(optional)]
    pub Tiles,
);

impl Deref for TransformSetTiles {
    type Target = Tiles;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TransformSetTiles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A page that contains no tile data, but contains handles referencing tiles
/// on other pages and specifies how tiles animate over time.
/// Animations proceed from left-to-right, with increasing x-coordinate,
/// along continuous rows of tiles, until an empty cell is found, and then
/// the animation returns to the start of the sequence and repeats.
#[derive(Clone, PartialEq, Debug, Reflect, Visit)]
pub struct AnimationTiles {
    /// The speed of the animation in frames per second.
    pub frame_rate: f32,
    /// The tile animation sequences represented as a grid of [`TileDefinitionHandle`].
    #[reflect(hidden)]
    #[visit(optional)]
    pub tiles: Tiles,
}

impl Default for AnimationTiles {
    fn default() -> Self {
        Self {
            frame_rate: DEFAULT_ANIMATION_FRAME_RATE,
            tiles: Default::default(),
        }
    }
}

impl Deref for AnimationTiles {
    type Target = Tiles;

    fn deref(&self) -> &Self::Target {
        &self.tiles
    }
}

impl DerefMut for AnimationTiles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tiles
    }
}

/// A material resource plus the size of each tile, so that the tile set can
/// carve up the material into tiles.
#[derive(Clone, PartialEq, Debug, Visit, Reflect)]
pub struct TileMaterial {
    /// The source material.
    pub material: MaterialResource,
    /// The size of each tile in pixels.
    pub tile_size: Vector2<u32>,
    /// The tile data that goes along with each tile of the material.
    pub tiles: TileGridMap<TileData>,
}

impl Deref for TileMaterial {
    type Target = TileGridMap<TileData>;

    fn deref(&self) -> &Self::Target {
        &self.tiles
    }
}

impl DerefMut for TileMaterial {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tiles
    }
}

impl Default for TileMaterial {
    fn default() -> Self {
        Self {
            material: DEFAULT_TILE_MATERIAL.deep_copy_as_embedded(),
            tile_size: DEFAULT_TILE_SIZE,
            tiles: TileGridMap::default(),
        }
    }
}

impl TileMaterial {
    fn get_tile_bounds(&self, position: Vector2<i32>) -> Option<TileMaterialBounds> {
        let origin = Vector2::new(
            u32::try_from(position.x).ok()? * self.tile_size.x,
            u32::try_from(-1 - position.y).ok()? * self.tile_size.y,
        );
        Some(TileMaterialBounds {
            material: self.material.clone(),
            bounds: TileBounds {
                left_top_corner: origin,
                right_top_corner: origin + Vector2::new(self.tile_size.x, 0),
                left_bottom_corner: origin + Vector2::new(0, self.tile_size.y),
                right_bottom_corner: origin + self.tile_size,
            },
        })
    }
    fn get_abstract_tile(&self, position: Vector2<i32>) -> Option<AbstractTile> {
        Some(AbstractTile::Atlas(self.tiles.get(&position)?.clone()))
    }
    fn set_abstract_tile(
        &mut self,
        position: Vector2<i32>,
        tile: Option<AbstractTile>,
    ) -> Option<AbstractTile> {
        if let Some(tile) = tile {
            let AbstractTile::Atlas(data) = tile else {
                panic!();
            };
            self.tiles.insert(position, data).map(AbstractTile::Atlas)
        } else {
            self.tiles.remove(&position).map(AbstractTile::Atlas)
        }
    }
    fn get_tile_data(&self, position: Vector2<i32>) -> Option<&TileData> {
        self.tiles.get(&position)
    }
    fn get_tile_data_mut(&mut self, position: Vector2<i32>) -> Option<&mut TileData> {
        self.tiles.get_mut(&position)
    }
}

/// Iterates through the valid handles of a tile set.
#[derive(Default)]
pub struct TileSetHandleIterator<'a> {
    pages_iter: Option<hash_map::Iter<'a, Vector2<i32>, TileSetPage>>,
    page_position: Vector2<i32>,
    handles_iter: TileSetPageHandleIterator<'a>,
}

#[derive(Default)]
enum TileSetPageHandleIterator<'a> {
    #[default]
    Empty,
    Atlas(Keys<'a, Vector2<i32>, TileData>),
    Freeform(Keys<'a, Vector2<i32>, TileDefinition>),
}

impl<'a> TileSetHandleIterator<'a> {
    fn new(pages_iter: Option<hash_map::Iter<'a, Vector2<i32>, TileSetPage>>) -> Self {
        Self {
            pages_iter,
            page_position: Vector2::default(),
            handles_iter: TileSetPageHandleIterator::Empty,
        }
    }
}

impl Iterator for TileSetPageHandleIterator<'_> {
    type Item = Vector2<i32>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            TileSetPageHandleIterator::Empty => None,
            TileSetPageHandleIterator::Atlas(keys) => keys.next().cloned(),
            TileSetPageHandleIterator::Freeform(keys) => keys.next().cloned(),
        }
    }
}

impl Iterator for TileSetHandleIterator<'_> {
    type Item = TileDefinitionHandle;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(tile) = self.handles_iter.next() {
                return TileDefinitionHandle::try_new(self.page_position, tile);
            }
            let (&page_position, page) = self.pages_iter.as_mut()?.next()?;
            self.page_position = page_position;
            self.handles_iter = match &page.source {
                TileSetPageSource::Atlas(map) => TileSetPageHandleIterator::Atlas(map.keys()),
                TileSetPageSource::Freeform(map) => TileSetPageHandleIterator::Freeform(map.keys()),
                _ => TileSetPageHandleIterator::Empty,
            };
        }
    }
}

/// Iterates through the positions of the tiles of a single page within a tile set.
pub struct TileSetPaletteIterator<'a> {
    keys: PaletteIterator<'a>,
    page: Vector2<i32>,
}

enum PaletteIterator<'a> {
    Empty,
    Material(Keys<'a, Vector2<i32>, TileData>),
    Freeform(Keys<'a, Vector2<i32>, TileDefinition>),
    TransformSet(Keys<'a, Vector2<i32>, TileDefinitionHandle>),
    Pages(Keys<'a, Vector2<i32>, TileSetPage>),
}

impl Iterator for TileSetPaletteIterator<'_> {
    type Item = ResourceTilePosition;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.keys {
            PaletteIterator::Empty => None,
            PaletteIterator::Material(iter) => iter
                .next()
                .map(|t| ResourceTilePosition::Tile(self.page, *t)),
            PaletteIterator::Freeform(iter) => iter
                .next()
                .map(|t| ResourceTilePosition::Tile(self.page, *t)),
            PaletteIterator::TransformSet(iter) => iter
                .next()
                .map(|t| ResourceTilePosition::Tile(self.page, *t)),
            PaletteIterator::Pages(iter) => iter.next().copied().map(ResourceTilePosition::Page),
        }
    }
}

/// A wrapper for a [`TileSet`] resource reference that allows access to the data without panicking
/// even if the resource is not loaded. A tile set that is not loaded acts like an empty tile set.
pub struct TileSetRef<'a>(ResourceDataRef<'a, TileSet>);
/// Maybe a [`TileSet`], maybe not, depending on whether the resource was successfully loaded.
/// If it is not a `TileSet`, its methods pretend it is an empty `TileSet`.
pub struct OptionTileSet<'a>(pub Option<&'a mut TileSet>);

impl<'a> From<ResourceDataRef<'a, TileSet>> for TileSetRef<'a> {
    fn from(value: ResourceDataRef<'a, TileSet>) -> Self {
        Self(value)
    }
}

impl<'a> TileSetRef<'a> {
    /// Locks the given resource and constructs a TileSetRef using that resource, allowing
    /// the tile set to be used without danger of panicking due to failing to load.
    pub fn new(tile_set: &'a TileSetResource) -> Self {
        tile_set.data_ref().into()
    }
    /// Borrows the underlying TileSet, if it is actually loaded.
    #[inline]
    pub fn as_loaded(&mut self) -> OptionTileSet {
        OptionTileSet(self.0.as_loaded_mut())
    }
}

impl<'a> Deref for OptionTileSet<'a> {
    type Target = Option<&'a mut TileSet>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> OptionTileSet<'a> {
    /// A reference to the underlying `TileSet` if it was successfully loaded.
    pub fn as_ref(&'a self) -> Option<&'a TileSet> {
        self.0.as_deref()
    }
    /// Iterate all valid tile handles.
    pub fn all_tiles(&self) -> TileSetHandleIterator {
        self.as_ref().map(|t| t.all_tiles()).unwrap_or_default()
    }
    /// The property value for the property of the given name for the tile at the given position in this tile map.
    /// If there is no tile data at the given handle or no property at the given UUID,
    /// then the default value for the property's value type is returned.
    pub fn tile_property_value<T>(
        &self,
        handle: TileDefinitionHandle,
        property_id: Uuid,
    ) -> Result<T, TilePropertyError>
    where
        T: TryFrom<TileSetPropertyValue, Error = TilePropertyError> + Default,
    {
        self.as_ref()
            .map(|t| t.tile_property_value(handle, property_id))
            .unwrap_or(Err(TilePropertyError::MissingTileSet))
    }
    /// The property value for the property of the given name for the tile at the given position in this tile map.
    /// If there is no tile data at the given handle, then the default value for the property's value type is returned.
    pub fn tile_property_value_by_name(
        &self,
        handle: TileDefinitionHandle,
        property_name: &ImmutableString,
    ) -> Result<TileSetPropertyValue, TilePropertyError> {
        self.as_ref()
            .map(|t| t.tile_property_value_by_name(handle, property_name))
            .unwrap_or(Err(TilePropertyError::MissingTileSet))
    }
    /// The property value for the property of the given UUID for the tile at the given position in this tile map.
    /// If there is no tile data at the given handle, then the default value for the property's value type is returned.
    pub fn tile_property_value_by_uuid_untyped(
        &self,
        handle: TileDefinitionHandle,
        property_id: Uuid,
    ) -> Result<TileSetPropertyValue, TilePropertyError> {
        self.as_ref()
            .map(|t| t.tile_property_value_by_uuid_untyped(handle, property_id))
            .unwrap_or(Err(TilePropertyError::MissingTileSet))
    }
    /// Slice containing the properties of this tile set, or an empty slice if the tile set is not loaded.
    pub fn properties(&self) -> &[TileSetPropertyLayer] {
        self.as_ref()
            .map(|t| t.properties.deref())
            .unwrap_or_default()
    }
    /// Slice containing the colliders of this tile set, or an empty slice if the tile set is not loaded.
    pub fn colliders(&self) -> &[TileSetColliderLayer] {
        self.as_ref()
            .map(|t| t.colliders.deref())
            .unwrap_or_default()
    }
    /// The color of the collider layer with the given uuid.
    pub fn collider_color(&self, uuid: Uuid) -> Option<Color> {
        self.as_ref()
            .map(|t| t.collider_color(uuid))
            .unwrap_or_default()
    }
    /// The collider of the given tile.
    pub fn tile_collider(&self, handle: TileDefinitionHandle, uuid: Uuid) -> &TileCollider {
        self.as_ref()
            .map(|t| t.tile_collider(handle, uuid))
            .unwrap_or_default()
    }
    /// The color of the given tile.
    pub fn tile_color(&self, handle: TileDefinitionHandle) -> Option<Color> {
        self.as_ref()
            .map(|t| t.tile_color(handle))
            .unwrap_or_default()
    }
    /// The data of the given tile.
    pub fn tile_data(&self, handle: TileDefinitionHandle) -> Option<&TileData> {
        self.as_ref()
            .map(|t| t.tile_data(handle))
            .unwrap_or_default()
    }
    /// The material and bounds of the given tile, if it stores its own material and bounds because it is a freeform tile.
    pub fn tile_bounds(&self, handle: TileDefinitionHandle) -> Option<&TileMaterialBounds> {
        self.as_ref()
            .map(|t| t.tile_bounds(handle))
            .unwrap_or_default()
    }
    /// The redirect target of the given tile. When a tile set tile does not contain its own data, but instead
    /// it points toward a tile elsewhere in the set, this method returns the TileDefinitionHandle of that other tile.
    pub fn tile_redirect(&self, handle: TileDefinitionHandle) -> Option<TileDefinitionHandle> {
        self.as_ref()
            .map(|t| t.tile_redirect(handle))
            .unwrap_or_default()
    }
    /// Generate a list of all tile positions in the given page.
    pub fn keys_on_page(&self, page: Vector2<i32>) -> Vec<Vector2<i32>> {
        self.as_ref()
            .map(|t| t.keys_on_page(page))
            .unwrap_or_default()
    }
    /// Generate a list of all page positions.
    pub fn page_keys(&self) -> Vec<Vector2<i32>> {
        self.as_ref().map(|t| t.page_keys()).unwrap_or_default()
    }
    /// True if there is a tile at the given position on the given page.
    pub fn has_tile_at(&self, page: Vector2<i32>, tile: Vector2<i32>) -> bool {
        self.as_ref()
            .map(|t| t.has_tile_at(page, tile))
            .unwrap_or_default()
    }
    /// The handle of the icon that represents the given page.
    pub fn page_icon(&self, page: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.as_ref().map(|t| t.page_icon(page)).unwrap_or_default()
    }
    /// Get the UUID of the property with the given name, if that property exists.
    pub fn property_name_to_uuid(&self, name: &ImmutableString) -> Option<Uuid> {
        self.as_ref()
            .map(|t| t.property_name_to_uuid(name))
            .unwrap_or_default()
    }
    /// Get the UUID of the collider with the given name, if that collider exists.
    pub fn collider_name_to_uuid(&self, name: &ImmutableString) -> Option<Uuid> {
        self.as_ref()
            .map(|t| t.collider_name_to_uuid(name))
            .unwrap_or_default()
    }
    /// Find a property layer by its UUID.
    pub fn find_property(&self, uuid: Uuid) -> Option<&TileSetPropertyLayer> {
        self.as_ref()
            .map(|t| t.find_property(uuid))
            .unwrap_or_default()
    }
    /// Find a collider layer by its UUID.
    pub fn find_collider(&self, uuid: Uuid) -> Option<&TileSetColliderLayer> {
        self.as_ref()
            .map(|t| t.find_collider(uuid))
            .unwrap_or_default()
    }
    /// Find every transform set tile handle in this set. A transform set tile reference is a tile
    /// with data that includes `transform_tile` with some value.
    /// The given function will be called as `func(source_tile, transform_tile)` where
    /// `source_tile` is the handle of the tile containing the data.
    pub fn rebuild_transform_sets(&mut self) {
        if let Some(t) = &mut self.0 {
            t.rebuild_transform_sets()
        }
    }
    /// Iterate through the tiles of every animation page and establish the connection between
    /// the tiles of other pages and their corresponding position in an animation page.
    /// This should happen after any animation page is changed and before it is next used.
    pub fn rebuild_animations(&mut self) {
        if let Some(t) = &mut self.0 {
            t.rebuild_animations()
        }
    }
    /// Find a texture from some material page to serve as a preview for the tile set.
    pub fn preview_texture(&self) -> Option<TextureResource> {
        self.as_ref()
            .map(|t| t.preview_texture())
            .unwrap_or_default()
    }
    /// Get the page at the given position.
    pub fn get_page(&self, position: Vector2<i32>) -> Option<&TileSetPage> {
        self.as_ref()
            .map(|t| t.get_page(position))
            .unwrap_or_default()
    }
    /// Get the page at the given position.
    pub fn get_page_mut(&mut self, position: Vector2<i32>) -> Option<&mut TileSetPage> {
        self.0
            .as_mut()
            .map(|t| t.get_page_mut(position))
            .unwrap_or_default()
    }
    /// Returns true if the given handle points to a tile definition.
    pub fn is_valid_tile(&self, handle: TileDefinitionHandle) -> bool {
        self.as_ref()
            .map(|t| t.is_valid_tile(handle))
            .unwrap_or_default()
    }
    /// The tile at the given page and tile coordinates.
    pub fn get_abstract_tile(
        &self,
        page: Vector2<i32>,
        tile: Vector2<i32>,
    ) -> Option<AbstractTile> {
        self.as_ref()
            .map(|t| t.get_abstract_tile(page, tile))
            .unwrap_or_default()
    }
    /// The render data for the tile at the given handle after applying the transform.
    pub fn get_transformed_render_data(
        &self,
        trans: OrthoTransformation,
        handle: TileDefinitionHandle,
    ) -> Option<TileRenderData> {
        self.as_ref()
            .map(|t| t.get_transformed_render_data(trans, handle))
            .unwrap_or_else(|| Some(TileRenderData::missing_data()))
    }
    /// Return the `TileRenderData` needed to render the tile at the given handle.
    /// The handle is redirected if it refers to a reference to another tile.
    /// If a reference is redirected and the resulting handle does not point to a tile definition,
    /// then `TileRenderData::missing_tile()` is returned to that an error tile will be rendered.
    /// If the given handle does not point to a reference and it does not point to a tile definition,
    /// then None is returned since nothing should be rendered.
    pub fn get_tile_render_data(&self, position: ResourceTilePosition) -> Option<TileRenderData> {
        self.as_ref()
            .map(|t| t.get_tile_render_data(position))
            .unwrap_or_else(|| Some(TileRenderData::missing_data()))
    }
    /// The tile collider with the given UUID for the tile at the given handle.
    pub fn get_tile_collider(
        &self,
        handle: TileDefinitionHandle,
        uuid: Uuid,
    ) -> Option<&TileCollider> {
        self.as_ref()
            .map(|t| t.get_tile_collider(handle, uuid))
            .unwrap_or_default()
    }
    /// Loop through the tiles of the given page and find the render data for each tile,
    /// then passes it to the given function.
    pub fn palette_render_loop<F>(&self, stage: TilePaletteStage, page: Vector2<i32>, func: F)
    where
        F: FnMut(Vector2<i32>, TileRenderData),
    {
        if let Some(tile_set) = &self.0 {
            tile_set.palette_render_loop(stage, page, func);
        }
    }
    /// Loop through the tiles of the given page and find each of the tile colliders on each tile,
    /// then pass the collider to the given function along with the collider's UUID and color.
    pub fn tile_collider_loop<F>(&self, page: Vector2<i32>, func: F)
    where
        F: FnMut(Vector2<i32>, Uuid, Color, &TileCollider),
    {
        if let Some(tile_set) = &self.0 {
            tile_set.tile_collider_loop(page, func);
        }
    }

    /// Some tiles in a tile set are references to tiles elsewhere in the tile set.
    /// In particular, the tiles of a transform set page all contain references to other pages,
    /// and the page tiles are also references. If this method is given the position of one of these
    /// reference tiles, then it returns the handle of the referenced tile.
    /// If the given position points to a tile without a redirect, then the tile's handle is returned.
    /// If the given position points to a non-existent page or a non-existent tile, then None is returned.
    pub fn redirect_handle(&self, position: ResourceTilePosition) -> Option<TileDefinitionHandle> {
        self.as_ref()
            .map(|t| t.redirect_handle(position))
            .unwrap_or_default()
    }
    /// If the given handle refers to a transform set page, find the tile on that page and return the handle of wherever
    /// the tile comes from originally. Return None if the page is not a transform set or there is no tile at that position.
    pub fn get_transform_tile_source(
        &self,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        self.as_ref()
            .map(|t| t.get_transform_tile_source(handle))
            .unwrap_or_default()
    }
    /// Returns a clone of the full definition of the tile at the given handle, if possible.
    /// Use [TileSet::get_tile_data] if a clone is not needed.
    pub fn get_definition(&self, handle: TileDefinitionHandle) -> Option<TileDefinition> {
        self.as_ref()
            .map(|t| t.get_definition(handle))
            .unwrap_or_default()
    }
    /// Return a copy of the definition of the tile at the given handle with the given transformation applied.
    pub fn get_transformed_definition(
        &self,
        trans: OrthoTransformation,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinition> {
        self.as_ref()
            .map(|t| t.get_transformed_definition(trans, handle))
            .unwrap_or_default()
    }
    /// Get the tile definition at the given position.
    pub fn get_tile_bounds(&self, position: ResourceTilePosition) -> Option<TileMaterialBounds> {
        self.as_ref()
            .map(|t| t.get_tile_bounds(position))
            .unwrap_or_default()
    }
    /// The value of the property with the given UUID for the given tile.
    pub fn property_value(
        &self,
        handle: TileDefinitionHandle,
        property_id: Uuid,
    ) -> Option<TileSetPropertyValue> {
        self.as_ref()
            .map(|t| t.property_value(handle, property_id))
            .unwrap_or_default()
    }
    /// Get the tile data at the given position.
    pub fn get_tile_data(&self, position: ResourceTilePosition) -> Option<&TileData> {
        self.as_ref()
            .map(|t| t.get_tile_data(position))
            .unwrap_or_default()
    }
    /// Get the tile data at the given position.
    pub fn get_tile_data_mut(&mut self, handle: TileDefinitionHandle) -> Option<&mut TileData> {
        self.0
            .as_mut()
            .map(|t| t.get_tile_data_mut(handle))
            .unwrap_or_default()
    }

    /// Finds the handle of the tile that represents a transformed version of the tile at the given handle, if such a tile exists.
    /// The given tile needs to have a `transform_tile` in its data, that handle needs to point to a transform set page,
    /// and that page needs to have a tile in the position corresponding to the desired transform relative to the `transform_tile` position.
    /// All 8 possible transforms are grouped together in 4x2 rectangles within each transform set page, and every transformation is possible
    /// so long as all 8 cells are filled with tiles. Otherwise, None is returned.
    pub fn get_transformed_version(
        &self,
        transform: OrthoTransformation,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        self.as_ref()
            .map(|t| t.get_transformed_version(transform, handle))
            .unwrap_or_default()
    }

    /// The handle of the tile in the animation sequence starting from the given tile handle
    /// at the given time, or none if the given handle is not part of any animation sequence.
    pub fn get_animated_version(
        &self,
        time: f32,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        self.as_ref()
            .map(|t| t.get_animated_version(time, handle))
            .unwrap_or_default()
    }

    /// Get the tile definition handles for all of the given coordinates on the given page.
    pub fn get_tiles<I: Iterator<Item = Vector2<i32>>>(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        iter: I,
        tiles: &mut Tiles,
    ) {
        if let Some(tile_set) = &self.0 {
            tile_set.get_tiles(stage, page, iter, tiles);
        }
    }
    /// The bounding rect of the pages.
    pub fn pages_bounds(&self) -> OptionTileRect {
        self.as_ref().map(|t| t.pages_bounds()).unwrap_or_default()
    }
    /// The bounding rect of the tiles of the given page.
    pub fn tiles_bounds(&self, stage: TilePaletteStage, page: Vector2<i32>) -> OptionTileRect {
        self.as_ref()
            .map(|t| t.tiles_bounds(stage, page))
            .unwrap_or_default()
    }

    /// Returns true if the tile set is unoccupied at the given position.
    pub fn is_free_at(&self, position: ResourceTilePosition) -> bool {
        self.as_ref()
            .map(|t| t.is_free_at(position))
            .unwrap_or(true)
    }
}

/// The index of an animation within the animation list, and an offset
/// to indicate where we should start playing within the animation.
#[derive(Debug, Default, Clone)]
struct AnimationRef {
    index: usize,
    offset: i32,
}

/// The position and length of an animation within some animation page.
#[derive(Debug, Default, Clone)]
struct Animation {
    start: TileDefinitionHandle,
    length: i32,
}

impl Animation {
    fn iter(&self) -> impl Iterator<Item = (TileDefinitionHandle, i32)> {
        let page = self.start.page();
        let tile = self.start.tile();
        (0..self.length).filter_map(move |i| {
            let x = tile.x.saturating_add(i);
            let handle = TileDefinitionHandle::try_new(page, Vector2::new(x, tile.y))?;
            Some((handle, i))
        })
    }
    fn page(&self) -> Vector2<i32> {
        self.start.page()
    }
    fn frame(&self, frame: i32) -> Vector2<i32> {
        let tile = self.start.tile();
        Vector2::new(tile.x + frame.rem_euclid(self.length), tile.y)
    }
}

/// A lookup table to locate animations within a tile set.
#[derive(Debug, Default, Clone)]
struct AnimationCache {
    handle_to_animation: FxHashMap<TileDefinitionHandle, AnimationRef>,
    animations: Vec<Animation>,
}

impl AnimationCache {
    fn clear(&mut self) {
        self.handle_to_animation.clear();
        self.animations.clear();
    }
    fn add_animation(&mut self, start: TileDefinitionHandle, length: i32) {
        self.animations.push(Animation { start, length });
    }
    fn get_animation_and_offset(&self, handle: TileDefinitionHandle) -> Option<(&Animation, i32)> {
        let AnimationRef { index, offset } = self.handle_to_animation.get(&handle)?;
        let animation = self.animations.get(*index)?;
        Some((animation, *offset))
    }
}

/// Tile set is a special storage for tile descriptions. It is a sort of database, that contains
/// descriptions (definitions) for tiles. Such approach allows you to change appearance of all tiles
/// of particular kind at once.
///
/// The tile data of the tile set is divided into pages, and pages come in three different types depending
/// on what kind of data is stored on the page. See [`TileSetPage`] for more information about the
/// page variants.
///
/// A tile set also contains extra layers of data that may be included with each tile:
/// Collider layers and property layers.
///
/// A *property layer* allows a particular value to be assigned to each tile.
/// The each layer has a name and a data type for the value, and it may optionally have
/// a list of pre-defined values and names for each of the pre-defined values.
/// This makes it easier to keep track of values which may have special meanings
/// when editing the tile set. See [`TileSetPropertyLayer`] for more information.
///
/// A *collider layer* allows a shape to be assigned to each tile for the purpose of
/// constructing a physics object for the tile map.
/// Each layer has a name and a color. The color is used to allow the user to quickly
/// identify which shapes correspond to which layers while in the tile set editor.
/// See [`TileSetColliderLayer`] for more information.
#[derive(Clone, Default, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "7b7e057b-a41e-4150-ab3b-0ae99f4024f0")]
pub struct TileSet {
    /// A mapping from animated tiles to the corresponding cells on animation pages.
    #[reflect(hidden)]
    #[visit(skip)]
    animation_map: AnimationCache,
    /// A mapping from transformable tiles to the corresponding cells on transform set pages.
    #[reflect(hidden)]
    #[visit(skip)]
    transform_map: FxHashMap<TileDefinitionHandle, TileDefinitionHandle>,
    /// The set of pages, organized by position.
    pub pages: FxHashMap<Vector2<i32>, TileSetPage>,
    /// Collider layers, in the order in which the layers should be presented in the editor.
    pub colliders: Vec<TileSetColliderLayer>,
    /// Property types in the order in which the layers should be presented in the editor.
    pub properties: Vec<TileSetPropertyLayer>,
    /// A count of changes since last save. New changes add +1. Reverting to previous
    /// states add -1. Reverting to a state before the last save can result in negative
    /// values. Saving is unnecessary whenever this value is 0.
    #[reflect(hidden)]
    #[visit(skip)]
    pub change_flag: ChangeFlag,
}

impl TileSet {
    /// The stamp element for the given position, if the tile in that cell is used
    /// to create a stamp. The [`StampElement::handle`] refers to the location of the tile within the
    /// tile set, while the [`StampElement::source`] refers to the location of the tile within
    /// the brush.
    pub fn stamp_element(&self, position: ResourceTilePosition) -> Option<StampElement> {
        self.redirect_handle(position).map(|handle| StampElement {
            handle,
            source: Some(position),
        })
    }

    /// Iterate all valid tile handles.
    pub fn all_tiles(&self) -> TileSetHandleIterator {
        TileSetHandleIterator::new(Some(self.pages.iter()))
    }
    /// The property value for the property of the given name for the tile at the given position in this tile map.
    /// If there is no tile data at the given handle or no property at the given UUID,
    /// then the default value for the property's value type is returned.
    pub fn tile_property_value<T>(
        &self,
        handle: TileDefinitionHandle,
        property_id: Uuid,
    ) -> Result<T, TilePropertyError>
    where
        T: TryFrom<TileSetPropertyValue, Error = TilePropertyError> + Default,
    {
        self.property_value(handle, property_id)
            .map(T::try_from)
            .unwrap_or_else(|| Ok(T::default()))
    }
    /// The property value for the property of the given name for the tile at the given position in this tile map.
    /// If there is no tile data at the given handle, then the default value for the property's value type is returned.
    pub fn tile_property_value_by_name(
        &self,
        handle: TileDefinitionHandle,
        property_name: &ImmutableString,
    ) -> Result<TileSetPropertyValue, TilePropertyError> {
        let property = self
            .find_property_by_name(property_name)
            .ok_or_else(|| TilePropertyError::UnrecognizedName(property_name.clone()))?;
        Ok(self
            .property_value(handle, property.uuid)
            .unwrap_or_else(|| property.prop_type.default_value()))
    }
    /// The property value for the property of the given UUID for the tile at the given position in this tile map.
    /// If there is no tile data at the given handle, then the default value for the property's value type is returned.
    pub fn tile_property_value_by_uuid_untyped(
        &self,
        handle: TileDefinitionHandle,
        property_id: Uuid,
    ) -> Result<TileSetPropertyValue, TilePropertyError> {
        if let Some(value) = self.property_value(handle, property_id) {
            Ok(value)
        } else {
            let property = self
                .find_property(property_id)
                .ok_or(TilePropertyError::UnrecognizedUuid(property_id))?;
            Ok(property.prop_type.default_value())
        }
    }
    /// The color of the collider layer with the given uuid.
    pub fn collider_color(&self, uuid: Uuid) -> Option<Color> {
        self.find_collider(uuid).map(|layer| layer.color)
    }
    /// The collider of the given tile.
    pub fn tile_collider(&self, handle: TileDefinitionHandle, uuid: Uuid) -> &TileCollider {
        let Some(data) = self.tile_data(handle) else {
            return &TileCollider::None;
        };
        data.colliders.get(&uuid).unwrap_or(&TileCollider::None)
    }
    /// The color of the given tile.
    pub fn tile_color(&self, handle: TileDefinitionHandle) -> Option<Color> {
        self.tile_data(handle).map(|d| d.color)
    }
    /// The data of the given tile.
    pub fn tile_data(&self, handle: TileDefinitionHandle) -> Option<&TileData> {
        let page_source = self.pages.get(&handle.page()).map(|p| &p.source)?;
        match page_source {
            TileSetPageSource::Atlas(m) => m.get(&handle.tile()),
            TileSetPageSource::Freeform(m) => m.get(&handle.tile()).map(|def| &def.data),
            TileSetPageSource::Transform(_) => None,
            TileSetPageSource::Animation(_) => None,
        }
    }
    /// The material and bounds of the given tile, if it stores its own material and bounds because it is a freeform tile.
    pub fn tile_bounds(&self, handle: TileDefinitionHandle) -> Option<&TileMaterialBounds> {
        let page_source = self.pages.get(&handle.page()).map(|p| &p.source)?;
        if let TileSetPageSource::Freeform(m) = page_source {
            m.get(&handle.tile()).map(|t| &t.material_bounds)
        } else {
            None
        }
    }
    /// The redirect target of the given tile. When a tile set tile does not contain its own data, but instead
    /// it points toward a tile elsewhere in the set, this method returns the TileDefinitionHandle of that other tile.
    pub fn tile_redirect(&self, handle: TileDefinitionHandle) -> Option<TileDefinitionHandle> {
        let page_source = self.pages.get(&handle.page()).map(|p| &p.source)?;
        match page_source {
            TileSetPageSource::Transform(m) => m.get(&handle.tile()).copied(),
            TileSetPageSource::Animation(m) => m.get(&handle.tile()).copied(),
            _ => None,
        }
    }
    /// Generate a list of all tile positions in the given page.
    pub fn keys_on_page(&self, page: Vector2<i32>) -> Vec<Vector2<i32>> {
        self.pages.get(&page).map(|p| p.keys()).unwrap_or_default()
    }
    /// Generate a list of all page positions.
    pub fn page_keys(&self) -> Vec<Vector2<i32>> {
        self.pages.keys().copied().collect()
    }
    /// True if there is a tile at the given position on the given page.
    pub fn has_tile_at(&self, page: Vector2<i32>, tile: Vector2<i32>) -> bool {
        let Some(page) = self.pages.get(&page).map(|p| &p.source) else {
            return false;
        };
        match page {
            TileSetPageSource::Atlas(m) => m.contains_key(&tile),
            TileSetPageSource::Freeform(m) => m.contains_key(&tile),
            TileSetPageSource::Transform(m) => m.contains_key(&tile),
            TileSetPageSource::Animation(m) => m.contains_key(&tile),
        }
    }
    /// The handle of the icon that represents the given page.
    pub fn page_icon(&self, page: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.pages.get(&page).map(|p| p.icon)
    }
    /// Get the UUID of the property with the given name, if that property exists.
    pub fn property_name_to_uuid(&self, name: &ImmutableString) -> Option<Uuid> {
        self.find_property_by_name(name).map(|p| p.uuid)
    }
    /// Get the UUID of the collider with the given name, if that collider exists.
    pub fn collider_name_to_uuid(&self, name: &ImmutableString) -> Option<Uuid> {
        self.colliders
            .iter()
            .find(|c| &c.name == name)
            .map(|c| c.uuid)
    }
    /// Find a property layer by its name.
    pub fn find_property_by_name(
        &self,
        property_name: &ImmutableString,
    ) -> Option<&TileSetPropertyLayer> {
        self.properties.iter().find(|p| &p.name == property_name)
    }
    /// Find a property layer by its UUID.
    pub fn find_property(&self, uuid: Uuid) -> Option<&TileSetPropertyLayer> {
        self.properties.iter().find(|p| p.uuid == uuid)
    }
    /// Find a property layer by its UUID.
    pub fn find_property_mut(&mut self, uuid: Uuid) -> Option<&mut TileSetPropertyLayer> {
        self.properties.iter_mut().find(|p| p.uuid == uuid)
    }
    /// Find a collider layer by its UUID.
    pub fn find_collider(&self, uuid: Uuid) -> Option<&TileSetColliderLayer> {
        self.colliders.iter().find(|p| p.uuid == uuid)
    }
    /// Find a collider layer by its UUID.
    pub fn find_collider_mut(&mut self, uuid: Uuid) -> Option<&mut TileSetColliderLayer> {
        self.colliders.iter_mut().find(|p| p.uuid == uuid)
    }
    /// Iterate through the tiles of every transform set page and establish the connection between
    /// the tiles of other pages and their corresponding position in a transform set page.
    /// This should happen after any transform set is changed and before it is next used.
    pub fn rebuild_transform_sets(&mut self) {
        self.transform_map.clear();
        for (&position, page) in self.pages.iter_mut() {
            let TileSetPageSource::Transform(tiles) = &page.source else {
                continue;
            };
            self.transform_map.extend(
                tiles
                    .iter()
                    .filter_map(|(&k, &v)| Some((v, TileDefinitionHandle::try_new(position, k)?))),
            );
        }
    }
    /// Iterate through the tiles of every animation page and establish the connection between
    /// the tiles of other pages and their corresponding position in an animation page.
    /// This should happen after any animation page is changed and before it is next used.
    ///
    /// If a tile appears in multiple animation pages, the pages with greater y-coordinate
    /// are prioritized, followed by prioritizing lower x-coordinate. If a tile appears
    /// more than once on the same page, the cell with greater y-coordinate is prioritized,
    /// followed by lower x-coordinate. In this way a unique animation is always chosen for
    /// every tile that appears on any animation page.
    pub fn rebuild_animations(&mut self) {
        self.animation_map.clear();
        for (&position, page) in self.pages.iter_mut() {
            let TileSetPageSource::Animation(tiles) = &page.source else {
                continue;
            };
            for &k in tiles.keys() {
                let left = Vector2::new(k.x - 1, k.y);
                if tiles.contains_key(&left) {
                    continue;
                }
                let mut right = Vector2::new(k.x + 1, k.y);
                while tiles.contains_key(&right) {
                    right.x += 1;
                }
                let Some(start) = TileDefinitionHandle::try_new(position, k) else {
                    continue;
                };
                let length = right.x - k.x;
                self.animation_map.add_animation(start, length);
            }
        }
        for (index, animation) in self.animation_map.animations.iter().enumerate() {
            let page = self.pages.get(&animation.page()).unwrap();
            let TileSetPageSource::Animation(tiles) = &page.source else {
                unreachable!();
            };
            for (handle, offset) in animation.iter() {
                let handle = *tiles.get(&handle.tile()).unwrap();
                let anim_ref = AnimationRef { index, offset };
                match self.animation_map.handle_to_animation.entry(handle) {
                    Entry::Occupied(mut entry) => {
                        let prev = entry.get();
                        if offset == 0 && prev.offset != 0 {
                            entry.insert(anim_ref);
                        } else if (offset == 0) == (prev.offset == 0) {
                            let new_start = animation.start;
                            let prev_start = self.animation_map.animations[prev.index].start;
                            if new_start < prev_start {
                                entry.insert(anim_ref);
                            }
                        }
                    }
                    Entry::Vacant(entry) => drop(entry.insert(anim_ref)),
                }
            }
        }
    }
    /// Find a texture from some material page to serve as a preview for the tile set.
    pub fn preview_texture(&self) -> Option<TextureResource> {
        self.pages
            .iter()
            .filter_map(|(&pos, p)| match &p.source {
                TileSetPageSource::Atlas(mat) => {
                    Some((pos, mat.material.state().data()?.texture("diffuseTexture")?))
                }
                _ => None,
            })
            .min_by(|(a, _), (b, _)| a.y.cmp(&b.y).reverse().then(a.x.cmp(&b.x)))
            .map(|(_, texture)| texture)
    }
    /// Update the tile set using data stored in the given `TileSetUpdate`
    /// and modify the `TileSetUpdate` to become the reverse of the changes by storing
    /// the data that was removed from this TileSet.
    ///
    /// [`rebuild_transform_sets`](Self::rebuild_transform_sets) is automatically called if a transform set page is
    /// modified.
    /// [`rebuild_animations`](Self::rebuild_animations) is automatically called if an animation page is
    /// modified.
    ///
    /// Wherever there is incompatibility between the tile set and the given update,
    /// the tile set should gracefully ignore that part of the update, log the error,
    /// and set the update to [`TileDataUpdate::DoNothing`], because that is the correct
    /// reversal of nothing being done.
    pub fn swap(&mut self, update: &mut TileSetUpdate) {
        let mut transform_changes = false;
        let mut animation_changes: bool = false;
        for (handle, tile_update) in update.iter_mut() {
            let Some(page) = self.pages.get_mut(&handle.page()) else {
                Log::err("Tile set update page missing.");
                continue;
            };
            if page.is_transform_set() {
                transform_changes = true;
            }
            if page.is_animation() {
                animation_changes = true;
            }
            page.swap_tile(handle.tile(), tile_update);
        }
        if transform_changes {
            self.rebuild_transform_sets();
        }
        if animation_changes {
            self.rebuild_animations();
        }
    }
    /// Get the page at the given position.
    pub fn get_page(&self, position: Vector2<i32>) -> Option<&TileSetPage> {
        self.pages.get(&position)
    }
    /// Get the page at the given position.
    pub fn get_page_mut(&mut self, position: Vector2<i32>) -> Option<&mut TileSetPage> {
        self.pages.get_mut(&position)
    }
    /// Insert the given page at the given position.
    pub fn insert_page(
        &mut self,
        position: Vector2<i32>,
        page: TileSetPage,
    ) -> Option<TileSetPage> {
        self.pages.insert(position, page)
    }
    /// Remove the page at the given position, if there is a page at that position.
    pub fn remove_page(&mut self, position: Vector2<i32>) -> Option<TileSetPage> {
        self.pages.remove(&position)
    }
    /// Returns true if the given handle points to a tile definition.
    pub fn is_valid_tile(&self, handle: TileDefinitionHandle) -> bool {
        let Some(source) = self.pages.get(&handle.page()).map(|p| &p.source) else {
            return false;
        };
        match source {
            TileSetPageSource::Atlas(mat) => mat.contains_key(&handle.tile()),
            TileSetPageSource::Freeform(map) => map.contains_key(&handle.tile()),
            TileSetPageSource::Transform(_) => false,
            TileSetPageSource::Animation(_) => false,
        }
    }
    /// The tile at the given page and tile coordinates.
    pub fn get_abstract_tile(
        &self,
        page: Vector2<i32>,
        tile: Vector2<i32>,
    ) -> Option<AbstractTile> {
        let source = self.pages.get(&page).map(|p| &p.source)?;
        match source {
            TileSetPageSource::Atlas(tile_material) => tile_material.get_abstract_tile(tile),
            TileSetPageSource::Freeform(tile_grid_map) => {
                Some(AbstractTile::Freeform(tile_grid_map.get(&tile)?.clone()))
            }
            TileSetPageSource::Transform(tiles) => {
                Some(AbstractTile::Transform(tiles.get(&tile).copied()?))
            }
            TileSetPageSource::Animation(tiles) => {
                Some(AbstractTile::Transform(tiles.get(&tile).copied()?))
            }
        }
    }
    /// Put the given tile into the cell at the given page and tile coordinates.
    pub fn set_abstract_tile(
        &mut self,
        page: Vector2<i32>,
        tile: Vector2<i32>,
        value: Option<AbstractTile>,
    ) -> Option<AbstractTile> {
        let Some(source) = self.pages.get_mut(&page).map(|p| &mut p.source) else {
            panic!();
        };
        use AbstractTile as Tile;
        use TileSetPageSource as Source;
        match (source, value) {
            (Source::Atlas(d0), value) => d0.set_abstract_tile(tile, value),
            (Source::Freeform(d0), Some(Tile::Freeform(d1))) => {
                Some(Tile::Freeform(d0.insert(tile, d1)?))
            }
            (Source::Freeform(d0), None) => Some(Tile::Freeform(d0.remove(&tile)?)),
            (Source::Transform(d0), Some(Tile::Transform(d1))) => {
                d0.insert(tile, d1).map(Tile::Transform)
            }
            (Source::Transform(d0), None) => d0.remove(&tile).map(Tile::Transform),
            (Source::Animation(d0), Some(Tile::Transform(d1))) => {
                d0.insert(tile, d1).map(Tile::Transform)
            }
            (Source::Animation(d0), None) => d0.remove(&tile).map(Tile::Transform),
            _ => panic!(),
        }
    }
    /// The render data for the tile at the given handle after applying the transform.
    pub fn get_transformed_render_data(
        &self,
        trans: OrthoTransformation,
        handle: TileDefinitionHandle,
    ) -> Option<TileRenderData> {
        if handle.is_empty() {
            Some(TileRenderData::empty())
        } else if let Some(handle) = self.get_transformed_version(trans, handle) {
            self.get_tile_render_data(handle.into())
        } else {
            Some(self.get_tile_render_data(handle.into())?.transformed(trans))
        }
    }
    /// Return the `TileRenderData` needed to render the tile at the given handle.
    /// The handle is redirected if it refers to a reference to another tile.
    /// If a reference is redirected and the resulting handle does not point to a tile definition,
    /// then `TileRenderData::missing_tile()` is returned to that an error tile will be rendered.
    /// If the given handle does not point to a reference and it does not point to a tile definition,
    /// then None is returned since nothing should be rendered.
    pub fn get_tile_render_data(&self, position: ResourceTilePosition) -> Option<TileRenderData> {
        if self.is_free_at(position) {
            return None;
        }
        self.inner_get_render_data(position)
            .or_else(|| Some(TileRenderData::missing_data()))
    }
    fn inner_get_render_data(&self, position: ResourceTilePosition) -> Option<TileRenderData> {
        let handle = self.redirect_handle(position)?;
        if handle.is_empty() {
            Some(TileRenderData::empty())
        } else if self.is_valid_tile(handle) {
            Some(TileRenderData::new(
                Some(self.get_tile_bounds(position)?),
                self.get_tile_data(position)?.color,
            ))
        } else {
            Some(TileRenderData::missing_data())
        }
    }
    /// The tile collider with the given UUID for the tile at the given handle.
    pub fn get_tile_collider(
        &self,
        handle: TileDefinitionHandle,
        uuid: Uuid,
    ) -> Option<&TileCollider> {
        if self.is_free_at(handle.into()) {
            return None;
        }
        let handle = self.redirect_handle(handle.into())?;
        let data = self.get_tile_data(handle.into())?;
        data.colliders.get(&uuid)
    }
    /// An iterator over the `TileDefinitionHandle` of each tile on the given page.
    pub fn palette_iterator(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
    ) -> TileSetPaletteIterator {
        TileSetPaletteIterator {
            page,
            keys: self.palette_keys(stage, page),
        }
    }
    fn palette_keys(&self, stage: TilePaletteStage, page: Vector2<i32>) -> PaletteIterator {
        match stage {
            TilePaletteStage::Pages => PaletteIterator::Pages(self.pages.keys()),
            TilePaletteStage::Tiles => {
                let Some(page) = self.pages.get(&page) else {
                    return PaletteIterator::Empty;
                };
                match &page.source {
                    TileSetPageSource::Atlas(mat) => PaletteIterator::Material(mat.tiles.keys()),
                    TileSetPageSource::Freeform(map) => PaletteIterator::Freeform(map.keys()),
                    TileSetPageSource::Transform(tiles) => {
                        PaletteIterator::TransformSet(tiles.keys())
                    }
                    TileSetPageSource::Animation(tiles) => {
                        PaletteIterator::TransformSet(tiles.keys())
                    }
                }
            }
        }
    }

    /// Loop through the tiles of the given page and find the render data for each tile,
    /// then passes it to the given function.
    pub fn palette_render_loop<F>(&self, stage: TilePaletteStage, page: Vector2<i32>, mut func: F)
    where
        F: FnMut(Vector2<i32>, TileRenderData),
    {
        for position in self.palette_iterator(stage, page) {
            if let Some(data) = self.get_tile_render_data(position) {
                func(position.stage_position(), data);
            }
        }
    }
    /// Loop through the tiles of the given page and find each of the tile colliders on each tile,
    /// then pass the collider to the given function along with the collider's UUID and color.
    pub fn tile_collider_loop<F>(&self, page: Vector2<i32>, mut func: F)
    where
        F: FnMut(Vector2<i32>, Uuid, Color, &TileCollider),
    {
        for layer in self.colliders.iter() {
            for position in self.palette_iterator(TilePaletteStage::Tiles, page) {
                if let Some(tile_collider) =
                    self.get_tile_collider(position.handle().unwrap(), layer.uuid)
                {
                    if !tile_collider.is_none() {
                        func(
                            position.stage_position(),
                            layer.uuid,
                            layer.color,
                            tile_collider,
                        );
                    }
                }
            }
        }
    }

    /// Some tiles in a tile set are references to tiles elsewhere in the tile set.
    /// In particular, the tiles of a transform set page all contain references to other pages,
    /// and the page tiles are also references. If this method is given the position of one of these
    /// reference tiles, then it returns the handle of the referenced tile.
    /// If the given position points to a tile without a redirect, then the tile's handle is returned.
    /// If the given position points to a non-existent page or a non-existent tile, then None is returned.
    pub fn redirect_handle(&self, position: ResourceTilePosition) -> Option<TileDefinitionHandle> {
        match position.stage() {
            TilePaletteStage::Tiles => match &self.pages.get(&position.page())?.source {
                TileSetPageSource::Transform(tiles) => {
                    tiles.get(&position.stage_position()).copied()
                }
                TileSetPageSource::Animation(tiles) => {
                    tiles.get(&position.stage_position()).copied()
                }
                page => {
                    if page.contains_tile_at(position.stage_position()) {
                        position.handle()
                    } else {
                        None
                    }
                }
            },
            TilePaletteStage::Pages => self
                .pages
                .get(&position.stage_position())
                .and_then(|p| self.redirect_handle(ResourceTilePosition::from(p.icon))),
        }
    }
    /// If the given handle refers to a transform set page, find the tile on that page and return the handle of wherever
    /// the tile comes from originally. Return None if the page is not a transform set or there is no tile at that position.
    pub fn get_transform_tile_source(
        &self,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        match &self.pages.get(&handle.page())?.source {
            TileSetPageSource::Transform(tiles) => tiles.get(&handle.tile()).copied(),
            _ => None,
        }
    }
    /// Returns a clone of the full definition of the tile at the given handle, if possible.
    /// Use [TileSet::get_tile_data] if a clone is not needed.
    pub fn get_definition(&self, handle: TileDefinitionHandle) -> Option<TileDefinition> {
        Some(TileDefinition {
            material_bounds: self.get_tile_bounds(handle.into())?,
            data: self.get_tile_data(handle.into()).cloned()?,
        })
    }
    /// Return a copy of the definition of the tile at the given handle with the given transformation applied.
    pub fn get_transformed_definition(
        &self,
        trans: OrthoTransformation,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinition> {
        if let Some(handle) = self.get_transformed_version(trans, handle) {
            self.get_definition(handle)
        } else {
            Some(self.get_definition(handle)?.transformed(trans))
        }
    }
    /// Get the tile definition at the given position.
    pub fn get_tile_bounds(&self, position: ResourceTilePosition) -> Option<TileMaterialBounds> {
        let handle = self.redirect_handle(position)?;
        match &self.pages.get(&handle.page())?.source {
            TileSetPageSource::Atlas(mat) => mat.get_tile_bounds(handle.tile()),
            TileSetPageSource::Freeform(map) => {
                Some(map.get(&handle.tile())?.material_bounds.clone())
            }
            TileSetPageSource::Transform(_) => None,
            TileSetPageSource::Animation(_) => None,
        }
    }
    /// The value of the property with the given UUID for the given tile.
    pub fn property_value(
        &self,
        handle: TileDefinitionHandle,
        property_id: Uuid,
    ) -> Option<TileSetPropertyValue> {
        self.get_tile_data(handle.into()).and_then(|d| {
            d.properties.get(&property_id).cloned().or_else(|| {
                self.find_property(property_id)
                    .map(|p| p.prop_type.default_value())
            })
        })
    }
    /// Get the tile definition at the given position.
    pub fn get_tile_data(&self, position: ResourceTilePosition) -> Option<&TileData> {
        let handle = self.redirect_handle(position)?;
        match &self.pages.get(&handle.page())?.source {
            TileSetPageSource::Atlas(mat) => mat.get_tile_data(handle.tile()),
            TileSetPageSource::Freeform(map) => Some(&map.get(&handle.tile())?.data),
            TileSetPageSource::Transform(_) => None,
            TileSetPageSource::Animation(_) => None,
        }
    }
    /// Get the tile definition at the given position.
    pub fn get_tile_data_mut(&mut self, handle: TileDefinitionHandle) -> Option<&mut TileData> {
        match &mut self.pages.get_mut(&handle.page())?.source {
            TileSetPageSource::Atlas(mat) => mat.get_tile_data_mut(handle.tile()),
            TileSetPageSource::Freeform(map) => Some(&mut map.get_mut(&handle.tile())?.data),
            TileSetPageSource::Transform(_) => None,
            TileSetPageSource::Animation(_) => None,
        }
    }

    /// The handle of the tile in the animation sequence starting from the given tile handle
    /// at the given time, or none if the given handle is not part of any animation sequence.
    pub fn get_animated_version(
        &self,
        time: f32,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        let (animation, offset) = self.animation_map.get_animation_and_offset(handle)?;
        let page = self.get_page(animation.page())?;
        let TileSetPageSource::Animation(AnimationTiles { frame_rate, tiles }) = &page.source
        else {
            return None;
        };
        let frame_rate = *frame_rate;
        let length = animation.length;
        let frame_position = (time * frame_rate).rem_euclid(length as f32);
        let frame_index = (frame_position.floor() as i32).clamp(0, length - 1);
        let frame_index = (frame_index + offset).rem_euclid(length);
        let frame = animation.frame(frame_index);
        tiles.get(&frame).copied()
    }

    /// Finds the handle of the tile that represents a transformed version of the tile at the given handle, if such a tile exists.
    /// The given tile needs to have a `transform_tile` in its data, that handle needs to point to a transform set page,
    /// and that page needs to have a tile in the position corresponding to the desired transform relative to the `transform_tile` position.
    /// All 8 possible transforms are grouped together in 4x2 rectangles within each transform set page, and every transformation is possible
    /// so long as all 8 cells are filled with tiles. Otherwise, None is returned.
    pub fn get_transformed_version(
        &self,
        transform: OrthoTransformation,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        if transform.is_identity() {
            return Some(handle);
        }
        let transform_tile = self.transform_map.get(&handle)?;
        let page = self.get_page(transform_tile.page())?;
        let tiles = match &page.source {
            TileSetPageSource::Transform(TransformSetTiles(tiles)) => Some(tiles),
            _ => None,
        }?;
        let cell = TransformSetCell::from_position(transform_tile.tile())
            .transformed(transform)
            .into_position();
        tiles.get(&cell).copied()
    }

    /// Get the tile definition handles for all of the given coordinates on the given page.
    pub fn get_tiles<I: Iterator<Item = Vector2<i32>>>(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        iter: I,
        tiles: &mut Tiles,
    ) {
        for pos in iter {
            if let Some(tile) = self.redirect_handle(ResourceTilePosition::new(stage, page, pos)) {
                tiles.insert(pos, tile);
            }
        }
    }
    /// The bounding rect of the pages.
    pub fn pages_bounds(&self) -> OptionTileRect {
        let mut result = OptionTileRect::default();
        for pos in self.pages.keys() {
            result.push(*pos);
        }
        result
    }
    /// The bounding rect of the tiles of the given page.
    pub fn tiles_bounds(&self, stage: TilePaletteStage, page: Vector2<i32>) -> OptionTileRect {
        match stage {
            TilePaletteStage::Tiles => {
                let Some(page) = self.pages.get(&page) else {
                    return OptionTileRect::default();
                };
                page.get_bounds()
            }
            TilePaletteStage::Pages => self.pages_bounds(),
        }
    }

    /// Load a tile set resource from the specific file path.
    pub async fn from_file(
        path: &Path,
        resource_manager: ResourceManager,
        io: &dyn ResourceIo,
    ) -> Result<Self, TileSetResourceError> {
        let bytes = io.load_file(path).await?;
        let mut visitor = Visitor::load_from_memory(&bytes)?;
        visitor.blackboard.register(Arc::new(resource_manager));
        let mut tile_set = TileSet::default();
        tile_set.visit("TileSet", &mut visitor)?;
        Ok(tile_set)
    }

    /// Returns true if the tile set is unoccupied at the given position.
    pub fn is_free_at(&self, position: ResourceTilePosition) -> bool {
        match position.stage() {
            TilePaletteStage::Pages => self.get_page(position.stage_position()).is_none(),
            TilePaletteStage::Tiles => {
                self.get_page(position.page()).is_some()
                    && !self.has_tile_at(position.page(), position.stage_position())
            }
        }
    }

    /// Tries to find free location at the given position. It uses brute-force searching algorithm
    /// and could be slow if called dozens of time per frame or on a large tile set.
    pub fn find_free_location(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        position: Vector2<i32>,
    ) -> Vector2<i32> {
        let mut visited = FxHashSet::default();
        let mut stack = vec![position];
        while let Some(pos) = stack.pop() {
            if visited.contains(&pos) {
                continue;
            }
            visited.insert(pos);
            if self.is_free_at(ResourceTilePosition::new(stage, page, pos)) {
                return pos;
            } else {
                stack.extend_from_slice(&[
                    Vector2::new(pos.x + 1, pos.y),
                    Vector2::new(pos.x + 1, pos.y + 1),
                    Vector2::new(pos.x, pos.y + 1),
                    Vector2::new(pos.x - 1, pos.y + 1),
                    Vector2::new(pos.x - 1, pos.y),
                    Vector2::new(pos.x - 1, pos.y - 1),
                    Vector2::new(pos.x, pos.y - 1),
                    Vector2::new(pos.x + 1, pos.y - 1),
                ]);
            }
        }
        Default::default()
    }
    /// Take all the values for the property with the given id, remove them from the page, and put them into the given hash map.
    /// At the same time, take all the values from the given hash map and put them into the page.
    pub fn swap_all_values_for_property(
        &mut self,
        property_id: Uuid,
        values: &mut FxHashMap<TileDefinitionHandle, TileSetPropertyValue>,
    ) {
        for (page_pos, page) in self.pages.iter_mut() {
            page.swap_all_values_for_property(*page_pos, property_id, values);
        }
    }
    /// Take all the colliders for the given collider id, remove them from the tile set, and put them into the given hash map.
    /// At the same time, take all the colliders from the given hash map and put them into the page.
    pub fn swap_all_values_for_collider(
        &mut self,
        collider_id: Uuid,
        values: &mut FxHashMap<TileDefinitionHandle, TileCollider>,
    ) {
        for (page_pos, page) in self.pages.iter_mut() {
            page.swap_all_values_for_collider(*page_pos, collider_id, values);
        }
    }
}

impl ResourceData for TileSet {
    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut visitor = Visitor::new();
        self.visit("TileSet", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

/// An alias for `Resource<TileSet>`.
pub type TileSetResource = Resource<TileSet>;

/// Standard tile set resource loader.
pub struct TileSetLoader {
    /// Resource manager of the engine.
    pub resource_manager: ResourceManager,
}

impl ResourceLoader for TileSetLoader {
    fn extensions(&self) -> &[&str] {
        &["tileset"]
    }

    fn data_type_uuid(&self) -> Uuid {
        <TileSet as TypeUuidProvider>::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        Box::pin(async move {
            let mut tile_set = TileSet::from_file(&path, resource_manager, io.as_ref())
                .await
                .map_err(LoadError::new)?;
            tile_set.rebuild_transform_sets();
            tile_set.rebuild_animations();
            Ok(LoaderPayload::new(tile_set))
        })
    }
}
