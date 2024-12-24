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
//! descriptions (definitions) for tiles. See [`TileSet`] docs for more info and usage examples.

use crate::material::MaterialResourceExtension;
use crate::resource::texture::TextureResource;
use crate::{
    asset::{
        io::ResourceIo,
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
        manager::ResourceManager,
        state::LoadError,
        untyped::UntypedResource,
        Resource, ResourceData, ResourceDataRef,
    },
    core::{
        algebra::Vector2, color::Color, io::FileLoadError, reflect::prelude::*,
        type_traits::prelude::*, visitor::prelude::*, ImmutableString,
    },
    material::MaterialResource,
};
use fxhash::{FxHashMap, FxHashSet};
use fyrox_core::log::Log;
use std::collections::hash_map::{Entry, Keys};
use std::ops::{Deref, DerefMut};
use std::{
    any::Any,
    error::Error,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
    sync::Arc,
};

use super::*;
pub use property::*;

const DEFAULT_TILE_SIZE: Vector2<u32> = Vector2::new(16, 16);

/// The color that is used to represent a tile where the property value matches the value that is
/// currently be drawn. This is only used when the the property value does not have a specially assigned color.
pub const ELEMENT_MATCH_HIGHLIGHT_COLOR: Color = Color::from_rgba(255, 255, 0, 200);

/// An error that may occur during tile set resource loading.
#[derive(Debug)]
pub enum TileSetResourceError {
    /// An i/o error has occurred.
    Io(FileLoadError),

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

impl From<FileLoadError> for TileSetResourceError {
    fn from(e: FileLoadError) -> Self {
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
            material: Resource::new_ok(ResourceKind::Embedded, Material::standard_tile()),
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
        matches!(self.source, TileSetPageSource::Material(_))
    }
    /// True if this page is a freeform tile page.
    pub fn is_freeform(&self) -> bool {
        matches!(self.source, TileSetPageSource::Freeform(_))
    }
    /// True if this page contains tile transform groups.
    pub fn is_transform_set(&self) -> bool {
        matches!(self.source, TileSetPageSource::TransformSet(_))
    }
    /// True if a tile exists at the given position in this page.
    pub fn has_tile_at(&self, position: Vector2<i32>) -> bool {
        match &self.source {
            TileSetPageSource::Material(mat) => mat.tiles.contains_key(&position),
            TileSetPageSource::Freeform(map) => map.contains_key(&position),
            TileSetPageSource::TransformSet(tiles) => tiles.contains_key(&position),
        }
    }
    /// Generate a list of all tile positions in this page.
    pub fn keys(&self) -> Vec<Vector2<i32>> {
        match &self.source {
            TileSetPageSource::Material(mat) => mat.tiles.keys().copied().collect(),
            TileSetPageSource::Freeform(map) => map.keys().copied().collect(),
            TileSetPageSource::TransformSet(tiles) => tiles.keys().copied().collect(),
        }
    }
    /// The rect that contains all the tiles of this page.
    pub fn get_bounds(&self) -> OptionTileRect {
        let mut result = OptionTileRect::default();
        match &self.source {
            TileSetPageSource::Material(mat) => {
                for pos in mat.tiles.keys() {
                    result.push(*pos);
                }
            }
            TileSetPageSource::Freeform(map) => {
                for pos in map.keys() {
                    result.push(*pos);
                }
            }
            TileSetPageSource::TransformSet(tiles) => {
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
            TileSetPageSource::Material(map0) => swap_material_tile(map0, position, update),
            TileSetPageSource::Freeform(map0) => swap_freeform_tile(map0, position, update),
            TileSetPageSource::TransformSet(map0) => swap_transform_tile(map0, position, update),
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
            TileSetPageSource::Material(map0) => {
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
            _ => panic!(),
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
            TileSetPageSource::Material(map0) => {
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

/// A tile set contains three forms of tile, depending on the type of page.
/// This enum can represent a tile in any of those three forms.
#[derive(Debug, Clone)]
pub enum AbstractTile {
    /// A material tile contains data but no material information.
    /// The reason for this counter-intuitive naming is that a material tile
    /// comes from a material page, which is a page that has a single material atlas
    /// shared across all of its tiles.
    Material(TileData),
    /// A freeform tile contains a complete definition, including material,
    /// the UV bounds of the tile within that material, and tile data.
    /// Freeform tiles are the most flexible kind of tile.
    Freeform(TileDefinition),
    /// A transform tile contains no data, but it has a handle that refers to
    /// some tile somewhere else in the set. A transform page contains
    /// transform tiles in groups of 8 and specifies how its tiles are to be
    /// rotated and flipped.
    /// No two transform tiles may share the same handle, because that would
    /// cause the transformations to be ambiguous.
    Transform(TileDefinitionHandle),
}

/// This is where tile set pages store their tile data.
#[derive(Clone, PartialEq, Debug, Visit, Reflect)]
pub enum TileSetPageSource {
    /// A page that gets its data from a material resource and arranges its tiles according to their positions in the material.
    /// All tiles in a material page share the same material and their UV data is automatically calculated based on the position
    /// of the tile on the page, with the tile at (0,-1) being the top-left corner of the material, and negative-y going toward
    /// the bottom of the material.
    Material(TileMaterial),
    /// A page that contains arbitrary tile definitions. These tiles are free to specify any material and any UV values for each
    /// tile, and tiles can be positioned anywhere on the page.
    Freeform(TileGridMap<TileDefinition>),
    /// A page that contains no tile definitions, but contains handles likes a brush.
    /// Handles into a transform set page can be used to connect a tile to a transformed version of that tile.
    /// Tiles are arranged in groups of 8, one for each of four 90-degree rotations, and four horizontally flipped 90-degree rotations.
    /// No two transform tiles may share the same handle, because that would
    /// cause the transformations to be ambiguous.
    TransformSet(TransformSetTiles),
}

impl Default for TileSetPageSource {
    fn default() -> Self {
        Self::Material(TileMaterial::default())
    }
}

impl TileSetPageSource {
    /// Create a new default material page.
    pub fn new_material() -> Self {
        Self::Material(TileMaterial::default())
    }
    /// Create a new default freeform page.
    pub fn new_free() -> Self {
        Self::Freeform(TileGridMap::default())
    }
    /// Create a new default transform page.
    pub fn new_transform() -> Self {
        Self::TransformSet(TransformSetTiles::default())
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
        Some(AbstractTile::Material(self.tiles.get(&position)?.clone()))
    }
    fn set_abstract_tile(
        &mut self,
        position: Vector2<i32>,
        tile: Option<AbstractTile>,
    ) -> Option<AbstractTile> {
        if let Some(tile) = tile {
            let AbstractTile::Material(data) = tile else {
                panic!();
            };
            self.tiles
                .insert(position, data)
                .map(AbstractTile::Material)
        } else {
            self.tiles.remove(&position).map(AbstractTile::Material)
        }
    }
    fn get_tile_data(&self, position: Vector2<i32>) -> Option<&TileData> {
        self.tiles.get(&position)
    }
    fn get_tile_data_mut(&mut self, position: Vector2<i32>) -> Option<&mut TileData> {
        self.tiles.get_mut(&position)
    }
}

/// Iterates through the handles of a single page within a tile set.
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
    type Item = TileDefinitionHandle;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.keys {
            PaletteIterator::Empty => None,
            PaletteIterator::Material(iter) => {
                iter.find_map(|t| TileDefinitionHandle::try_new(self.page, *t))
            }
            PaletteIterator::Freeform(iter) => {
                iter.find_map(|t| TileDefinitionHandle::try_new(self.page, *t))
            }
            PaletteIterator::TransformSet(iter) => {
                iter.find_map(|t| TileDefinitionHandle::try_new(self.page, *t))
            }
            PaletteIterator::Pages(iter) => {
                iter.find_map(|t| TileDefinitionHandle::try_new(self.page, *t))
            }
        }
    }
}

/// A wrapper for a [`TileSet`] resource reference that allows access to the data without panicking
/// even if the resource is not loaded. A tile set that is not loaded acts like an empty tile set.
pub struct TileSetRef<'a>(ResourceDataRef<'a, TileSet>);

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
    pub fn as_loaded_ref(&self) -> Option<&TileSet> {
        self.0.as_loaded_ref()
    }
    /// Slice containing the properties of this tile set, or an empty slice if the tile set is not loaded.
    pub fn properties(&self) -> &[TileSetPropertyLayer] {
        self.as_loaded_ref()
            .map(|t| t.properties.deref())
            .unwrap_or_default()
    }
    /// Slice containing the colliders of this tile set, or an empty slice if the tile set is not loaded.
    pub fn colliders(&self) -> &[TileSetColliderLayer] {
        self.as_loaded_ref()
            .map(|t| t.colliders.deref())
            .unwrap_or_default()
    }
    /// The color of the collider layer with the given uuid.
    pub fn collider_color(&self, uuid: Uuid) -> Option<Color> {
        self.as_loaded_ref()
            .map(|t| t.collider_color(uuid))
            .unwrap_or_default()
    }
    /// The collider of the given tile.
    pub fn tile_collider(&self, handle: TileDefinitionHandle, uuid: Uuid) -> &TileCollider {
        self.as_loaded_ref()
            .map(|t| t.tile_collider(handle, uuid))
            .unwrap_or_default()
    }
    /// The color of the given tile.
    pub fn tile_color(&self, handle: TileDefinitionHandle) -> Option<Color> {
        self.as_loaded_ref()
            .map(|t| t.tile_color(handle))
            .unwrap_or_default()
    }
    /// The data of the given tile.
    pub fn tile_data(&self, handle: TileDefinitionHandle) -> Option<&TileData> {
        self.as_loaded_ref()
            .map(|t| t.tile_data(handle))
            .unwrap_or_default()
    }
    /// The material and bounds of the given tile, if it stores its own material and bounds because it is a freeform tile.
    pub fn tile_bounds(&self, handle: TileDefinitionHandle) -> Option<&TileMaterialBounds> {
        self.as_loaded_ref()
            .map(|t| t.tile_bounds(handle))
            .unwrap_or_default()
    }
    /// The redirect target of the given tile. When a tile set tile does not contain its own data, but instead
    /// it points toward a tile elsewhere in the set, this method returns the TileDefinitionHandle of that other tile.
    pub fn tile_redirect(&self, handle: TileDefinitionHandle) -> Option<TileDefinitionHandle> {
        self.as_loaded_ref()
            .map(|t| t.tile_redirect(handle))
            .unwrap_or_default()
    }
    /// Generate a list of all tile positions in the given page.
    pub fn keys_on_page(&self, page: Vector2<i32>) -> Vec<Vector2<i32>> {
        self.as_loaded_ref()
            .map(|t| t.keys_on_page(page))
            .unwrap_or_default()
    }
    /// Generate a list of all page positions.
    pub fn page_keys(&self) -> Vec<Vector2<i32>> {
        self.as_loaded_ref()
            .map(|t| t.page_keys())
            .unwrap_or_default()
    }
    /// True if there is a tile at the given position on the given page.
    pub fn has_tile_at(&self, page: Vector2<i32>, tile: Vector2<i32>) -> bool {
        self.as_loaded_ref()
            .map(|t| t.has_tile_at(page, tile))
            .unwrap_or_default()
    }
    /// The handle of the icon that represents the given page.
    pub fn page_icon(&self, page: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.as_loaded_ref()
            .map(|t| t.page_icon(page))
            .unwrap_or_default()
    }
    /// Get the UUID of the property with the given name, if that property exists.
    pub fn property_name_to_uuid(&self, name: ImmutableString) -> Option<Uuid> {
        self.as_loaded_ref()
            .map(|t| t.property_name_to_uuid(name))
            .unwrap_or_default()
    }
    /// Get the UUID of the collider with the given name, if that collider exists.
    pub fn collider_name_to_uuid(&self, name: ImmutableString) -> Option<Uuid> {
        self.as_loaded_ref()
            .map(|t| t.collider_name_to_uuid(name))
            .unwrap_or_default()
    }
    /// Find a property layer by its UUID.
    pub fn find_property(&self, uuid: Uuid) -> Option<&TileSetPropertyLayer> {
        self.as_loaded_ref()
            .map(|t| t.find_property(uuid))
            .unwrap_or_default()
    }
    /// Find a collider layer by its UUID.
    pub fn find_collider(&self, uuid: Uuid) -> Option<&TileSetColliderLayer> {
        self.as_loaded_ref()
            .map(|t| t.find_collider(uuid))
            .unwrap_or_default()
    }
    /// Find every transform set tile handle in this set. A transform set tile reference is a tile
    /// with data that includes `transform_tile` with some value.
    /// The given function will be called as `func(source_tile, transform_tile)` where
    /// `source_tile` is the handle of the tile containing the data.
    pub fn rebuild_transform_sets(&mut self) {
        if let Some(t) = self.0.as_loaded_mut() {
            t.rebuild_transform_sets()
        }
    }
    /// Find a texture from some material page to serve as a preview for the tile set.
    pub fn preview_texture(&self) -> Option<TextureResource> {
        self.as_loaded_ref()
            .map(|t| t.preview_texture())
            .unwrap_or_default()
    }
    /// Get the page at the given position.
    pub fn get_page(&self, position: Vector2<i32>) -> Option<&TileSetPage> {
        self.as_loaded_ref()
            .map(|t| t.get_page(position))
            .unwrap_or_default()
    }
    /// Get the page at the given position.
    pub fn get_page_mut(&mut self, position: Vector2<i32>) -> Option<&mut TileSetPage> {
        self.0
            .as_loaded_mut()
            .map(|t| t.get_page_mut(position))
            .unwrap_or_default()
    }
    /// Returns true if the given handle points to a tile definition.
    pub fn is_valid_tile(&self, handle: TileDefinitionHandle) -> bool {
        self.as_loaded_ref()
            .map(|t| t.is_valid_tile(handle))
            .unwrap_or_default()
    }
    /// The tile at the given page and tile coordinates.
    pub fn get_abstract_tile(
        &self,
        page: Vector2<i32>,
        tile: Vector2<i32>,
    ) -> Option<AbstractTile> {
        self.as_loaded_ref()
            .map(|t| t.get_abstract_tile(page, tile))
            .unwrap_or_default()
    }
    /// The render data for the tile at the given handle after applying the transform.
    pub fn get_transformed_render_data(
        &self,
        trans: OrthoTransformation,
        handle: TileDefinitionHandle,
    ) -> Option<TileRenderData> {
        self.as_loaded_ref()
            .map(|t| t.get_transformed_render_data(trans, handle))
            .unwrap_or_else(|| Some(TileRenderData::missing_data()))
    }
    /// Return the `TileRenderData` needed to render the tile at the given handle.
    /// The handle is redirected if it refers to a reference to another tile.
    /// If a reference is redirected and the resulting handle does not point to a tile definition,
    /// then `TileRenderData::missing_tile()` is returned to that an error tile will be rendered.
    /// If the given handle does not point to a reference and it does not point to a tile definition,
    /// then None is returned since nothing should be rendered.
    pub fn get_tile_render_data(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<TileRenderData> {
        self.as_loaded_ref()
            .map(|t| t.get_tile_render_data(stage, handle))
            .unwrap_or_else(|| Some(TileRenderData::missing_data()))
    }
    /// The tile collider with the given UUID for the tile at the given handle.
    pub fn get_tile_collider(
        &self,
        handle: TileDefinitionHandle,
        uuid: Uuid,
    ) -> Option<&TileCollider> {
        self.as_loaded_ref()
            .map(|t| t.get_tile_collider(handle, uuid))
            .unwrap_or_default()
    }
    /// Loop through the tiles of the given page and find the render data for each tile,
    /// then passes it to the given function.
    pub fn palette_render_loop<F>(&self, stage: TilePaletteStage, page: Vector2<i32>, func: F)
    where
        F: FnMut(Vector2<i32>, TileRenderData),
    {
        if let Some(tile_set) = self.as_loaded_ref() {
            tile_set.palette_render_loop(stage, page, func);
        }
    }
    /// Loop through the tiles of the given page and find each of the tile colliders on each tile,
    /// then pass the collider to the given function along with the collider's UUID and color.
    pub fn tile_collider_loop<F>(&self, page: Vector2<i32>, func: F)
    where
        F: FnMut(Vector2<i32>, Uuid, Color, &TileCollider),
    {
        if let Some(tile_set) = self.as_loaded_ref() {
            tile_set.tile_collider_loop(page, func);
        }
    }

    /// Some tiles in a tile set are references to tiles elsewhere in the tile set.
    /// In particular, the tiles of a transform set page all contain references to other pages,
    /// and the page tiles are also references. If this method is given the handle of one of these
    /// reference tiles, then it returns the handle of the referenced tile.
    /// If the given handle does not refer to a reference, then it is returned.
    /// If the given handle points to a non-existent page, then None is returned.
    pub fn redirect_handle(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        self.as_loaded_ref()
            .map(|t| t.redirect_handle(stage, handle))
            .unwrap_or_default()
    }
    /// If the given handle refers to a transform set page, find the tile on that page and return the handle of wherever
    /// the tile comes from originally. Return None if the page is not a transform set or there is no tile at that position.
    pub fn get_transform_tile_source(
        &self,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        self.as_loaded_ref()
            .map(|t| t.get_transform_tile_source(handle))
            .unwrap_or_default()
    }
    /// Returns a clone of the full definition of the tile at the given handle, if possible.
    /// Use [TileSet::get_tile_data] if a clone is not needed.
    pub fn get_definition(&self, handle: TileDefinitionHandle) -> Option<TileDefinition> {
        self.as_loaded_ref()
            .map(|t| t.get_definition(handle))
            .unwrap_or_default()
    }
    /// Return a copy of the definition of the tile at the given handle with the given transformation applied.
    pub fn get_transformed_definition(
        &self,
        trans: OrthoTransformation,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinition> {
        self.as_loaded_ref()
            .map(|t| t.get_transformed_definition(trans, handle))
            .unwrap_or_default()
    }
    /// Get the tile definition at the given position.
    pub fn get_tile_bounds(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<TileMaterialBounds> {
        self.as_loaded_ref()
            .map(|t| t.get_tile_bounds(stage, handle))
            .unwrap_or_default()
    }
    /// The value of the property with the given UUID for the given tile.
    pub fn property_value(
        &self,
        handle: TileDefinitionHandle,
        property_id: Uuid,
    ) -> Option<TileSetPropertyValue> {
        self.as_loaded_ref()
            .map(|t| t.property_value(handle, property_id))
            .unwrap_or_default()
    }
    /// Get the tile data at the given position.
    pub fn get_tile_data(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<&TileData> {
        self.as_loaded_ref()
            .map(|t| t.get_tile_data(stage, handle))
            .unwrap_or_default()
    }
    /// Get the tile data at the given position.
    pub fn get_tile_data_mut(&mut self, handle: TileDefinitionHandle) -> Option<&mut TileData> {
        self.0
            .as_loaded_mut()
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
        self.as_loaded_ref()
            .map(|t| t.get_transformed_version(transform, handle))
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
        if let Some(tile_set) = self.as_loaded_ref() {
            tile_set.get_tiles(stage, page, iter, tiles);
        }
    }
    /// The bounding rect of the pages.
    pub fn pages_bounds(&self) -> OptionTileRect {
        self.as_loaded_ref()
            .map(|t| t.pages_bounds())
            .unwrap_or_default()
    }
    /// The bounding rect of the tiles of the given page.
    pub fn tiles_bounds(&self, stage: TilePaletteStage, page: Vector2<i32>) -> OptionTileRect {
        self.as_loaded_ref()
            .map(|t| t.tiles_bounds(stage, page))
            .unwrap_or_default()
    }

    /// Tries to find a tile definition with the given position.
    pub fn find_tile_at_position(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        position: Vector2<i32>,
    ) -> Option<TileDefinitionHandle> {
        self.as_loaded_ref()
            .map(|t| t.find_tile_at_position(stage, page, position))
            .unwrap_or_default()
    }

    /// Returns true if the tile set is unoccupied at the given position.
    pub fn is_free_at(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        position: Vector2<i32>,
    ) -> bool {
        self.as_loaded_ref()
            .map(|t| t.is_free_at(stage, page, position))
            .unwrap_or(true)
    }
}

/// Tile set is a special storage for tile descriptions. It is a sort of database, that contains
/// descriptions (definitions) for tiles. Such approach allows you to change appearance of all tiles
/// of particular kind at once.
#[derive(Clone, Default, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "7b7e057b-a41e-4150-ab3b-0ae99f4024f0")]
pub struct TileSet {
    /// A mapping from transformable tiles to the corresponding cells on transform set pages.
    #[visit(skip)]
    pub transform_map: FxHashMap<TileDefinitionHandle, TileDefinitionHandle>,
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
    pub change_count: ChangeCount,
}

impl TileSet {
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
            TileSetPageSource::Material(m) => m.get(&handle.tile()),
            TileSetPageSource::Freeform(m) => m.get(&handle.tile()).map(|def| &def.data),
            TileSetPageSource::TransformSet(_) => None,
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
        if let TileSetPageSource::TransformSet(m) = page_source {
            m.get(&handle.tile()).copied()
        } else {
            None
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
            TileSetPageSource::Material(m) => m.contains_key(&tile),
            TileSetPageSource::Freeform(m) => m.contains_key(&tile),
            TileSetPageSource::TransformSet(m) => m.contains_key(&tile),
        }
    }
    /// The handle of the icon that represents the given page.
    pub fn page_icon(&self, page: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.pages.get(&page).map(|p| p.icon)
    }
    /// Get the UUID of the property with the given name, if that property exists.
    pub fn property_name_to_uuid(&self, name: ImmutableString) -> Option<Uuid> {
        self.properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.uuid)
    }
    /// Get the UUID of the collider with the given name, if that collider exists.
    pub fn collider_name_to_uuid(&self, name: ImmutableString) -> Option<Uuid> {
        self.colliders
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.uuid)
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
            let TileSetPageSource::TransformSet(tiles) = &page.source else {
                continue;
            };
            self.transform_map.extend(
                tiles
                    .iter()
                    .filter_map(|(&k, &v)| Some((v, TileDefinitionHandle::try_new(position, k)?))),
            );
        }
    }
    /// Find a texture from some material page to serve as a preview for the tile set.
    pub fn preview_texture(&self) -> Option<TextureResource> {
        self.pages.values().find_map(|p| match &p.source {
            TileSetPageSource::Material(mat) => {
                mat.material.state().data()?.texture("diffuseTexture")
            }
            _ => None,
        })
    }
    /// Update the tile set using data stored in the given `TileSetUpdate`
    /// and modify the `TileSetUpdate` to become the reverse of the changes by storing
    /// the data that was removed from this TileSet.
    ///
    /// [`rebuild_transform_sets`](Self::rebuild_transform_sets) is automatically called if a transform set page is
    /// modified.
    ///
    /// Wherever there is incompatibility between the tile set and the given update,
    /// the tile set should gracefully ignore that part of the update, log the error,
    /// and set the update to [`TileDataUpdate::DoNothing`], because that is the correct
    /// reversal of nothing being done.
    pub fn swap(&mut self, update: &mut TileSetUpdate) {
        let mut transform_changes = false;
        for (handle, tile_update) in update.iter_mut() {
            let Some(page) = self.pages.get_mut(&handle.page()) else {
                Log::err("Tile set update page missing.");
                continue;
            };
            if page.is_transform_set() {
                transform_changes = true;
            }
            page.swap_tile(handle.tile(), tile_update);
        }
        if transform_changes {
            self.rebuild_transform_sets();
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
            TileSetPageSource::Material(mat) => mat.contains_key(&handle.tile()),
            TileSetPageSource::Freeform(map) => map.contains_key(&handle.tile()),
            TileSetPageSource::TransformSet(_) => false,
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
            TileSetPageSource::Material(tile_material) => tile_material.get_abstract_tile(tile),
            TileSetPageSource::Freeform(tile_grid_map) => {
                Some(AbstractTile::Freeform(tile_grid_map.get(&tile)?.clone()))
            }
            TileSetPageSource::TransformSet(tiles) => {
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
            (Source::Material(d0), value) => d0.set_abstract_tile(tile, value),
            (Source::Freeform(d0), Some(Tile::Freeform(d1))) => {
                Some(Tile::Freeform(d0.insert(tile, d1)?))
            }
            (Source::Freeform(d0), None) => Some(Tile::Freeform(d0.remove(&tile)?)),
            (Source::TransformSet(d0), Some(Tile::Transform(d1))) => {
                let handle = TileDefinitionHandle::try_new(page, tile).unwrap();
                let result = d0.insert(tile, d1);
                if let Some(source_handle) = result {
                    let _ = self.transform_map.remove(&source_handle);
                }
                let _ = self.transform_map.insert(d1, handle);
                result.map(Tile::Transform)
            }
            (Source::TransformSet(d0), None) => {
                let result = d0.remove(&tile);
                if let Some(source_handle) = result {
                    let _ = self.transform_map.remove(&source_handle);
                }
                result.map(Tile::Transform)
            }
            _ => panic!(),
        }
    }
    /// The render data for the tile at the given handle after applying the transform.
    pub fn get_transformed_render_data(
        &self,
        trans: OrthoTransformation,
        handle: TileDefinitionHandle,
    ) -> Option<TileRenderData> {
        if let Some(handle) = self.get_transformed_version(trans, handle) {
            self.get_tile_render_data(TilePaletteStage::Tiles, handle)
        } else {
            Some(
                self.get_tile_render_data(TilePaletteStage::Tiles, handle)?
                    .transformed(trans),
            )
        }
    }
    /// Return the `TileRenderData` needed to render the tile at the given handle.
    /// The handle is redirected if it refers to a reference to another tile.
    /// If a reference is redirected and the resulting handle does not point to a tile definition,
    /// then `TileRenderData::missing_tile()` is returned to that an error tile will be rendered.
    /// If the given handle does not point to a reference and it does not point to a tile definition,
    /// then None is returned since nothing should be rendered.
    pub fn get_tile_render_data(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<TileRenderData> {
        if self.is_free_at(stage, handle.page(), handle.tile()) {
            return None;
        }
        self.inner_get_render_data(stage, handle)
            .or_else(|| Some(TileRenderData::missing_data()))
    }
    fn inner_get_render_data(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<TileRenderData> {
        if self.is_valid_tile(self.redirect_handle(stage, handle)?) {
            Some(TileRenderData {
                material_bounds: Some(self.get_tile_bounds(stage, handle)?),
                color: self.get_tile_data(stage, handle)?.color,
            })
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
        if self.is_free_at(TilePaletteStage::Tiles, handle.page(), handle.tile()) {
            return None;
        }
        let handle = self.redirect_handle(TilePaletteStage::Tiles, handle)?;
        let data = self.get_tile_data(TilePaletteStage::Tiles, handle)?;
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
                    TileSetPageSource::Material(mat) => PaletteIterator::Material(mat.tiles.keys()),
                    TileSetPageSource::Freeform(map) => PaletteIterator::Freeform(map.keys()),
                    TileSetPageSource::TransformSet(tiles) => {
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
        for handle in self.palette_iterator(stage, page) {
            if let Some(data) = self.get_tile_render_data(stage, handle) {
                func(handle.tile(), data);
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
            for handle in self.palette_iterator(TilePaletteStage::Tiles, page) {
                if let Some(tile_collider) = self.get_tile_collider(handle, layer.uuid) {
                    if !tile_collider.is_none() {
                        func(handle.tile(), layer.uuid, layer.color, tile_collider);
                    }
                }
            }
        }
    }

    /// Some tiles in a tile set are references to tiles elsewhere in the tile set.
    /// In particular, the tiles of a transform set page all contain references to other pages,
    /// and the page tiles are also references. If this method is given the handle of one of these
    /// reference tiles, then it returns the handle of the referenced tile.
    /// If the given handle does not refer to a reference, then the given handle is returned.
    /// If the given handle points to a non-existent page, then None is returned.
    pub fn redirect_handle(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        match stage {
            TilePaletteStage::Tiles => match &self.pages.get(&handle.page())?.source {
                TileSetPageSource::TransformSet(tiles) => tiles.get_at(handle.tile()),
                _ => Some(handle),
            },
            TilePaletteStage::Pages => self
                .pages
                .get(&handle.tile())
                .and_then(|p| self.redirect_handle(TilePaletteStage::Tiles, p.icon)),
        }
    }
    /// If the given handle refers to a transform set page, find the tile on that page and return the handle of wherever
    /// the tile comes from originally. Return None if the page is not a transform set or there is no tile at that position.
    pub fn get_transform_tile_source(
        &self,
        handle: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        match &self.pages.get(&handle.page())?.source {
            TileSetPageSource::TransformSet(tiles) => tiles.get_at(handle.tile()),
            _ => None,
        }
    }
    /// Returns a clone of the full definition of the tile at the given handle, if possible.
    /// Use [TileSet::get_tile_data] if a clone is not needed.
    pub fn get_definition(&self, handle: TileDefinitionHandle) -> Option<TileDefinition> {
        Some(TileDefinition {
            material_bounds: self.get_tile_bounds(TilePaletteStage::Tiles, handle)?,
            data: self
                .get_tile_data(TilePaletteStage::Tiles, handle)
                .cloned()?,
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
    pub fn get_tile_bounds(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<TileMaterialBounds> {
        let handle = self.redirect_handle(stage, handle)?;
        match &self.pages.get(&handle.page())?.source {
            TileSetPageSource::Material(mat) => mat.get_tile_bounds(handle.tile()),
            TileSetPageSource::Freeform(map) => {
                Some(map.get(&handle.tile())?.material_bounds.clone())
            }
            TileSetPageSource::TransformSet(_) => None,
        }
    }
    /// The value of the property with the given UUID for the given tile.
    pub fn property_value(
        &self,
        handle: TileDefinitionHandle,
        property_id: Uuid,
    ) -> Option<TileSetPropertyValue> {
        self.get_tile_data(TilePaletteStage::Tiles, handle)
            .and_then(|d| {
                d.properties.get(&property_id).cloned().or_else(|| {
                    self.find_property(property_id)
                        .map(|p| p.prop_type.default_value())
                })
            })
    }
    /// Get the tile definition at the given position.
    pub fn get_tile_data(
        &self,
        stage: TilePaletteStage,
        handle: TileDefinitionHandle,
    ) -> Option<&TileData> {
        let handle = self.redirect_handle(stage, handle)?;
        match &self.pages.get(&handle.page())?.source {
            TileSetPageSource::Material(mat) => mat.get_tile_data(handle.tile()),
            TileSetPageSource::Freeform(map) => Some(&map.get(&handle.tile())?.data),
            TileSetPageSource::TransformSet(_) => None,
        }
    }
    /// Get the tile definition at the given position.
    pub fn get_tile_data_mut(&mut self, handle: TileDefinitionHandle) -> Option<&mut TileData> {
        match &mut self.pages.get_mut(&handle.page())?.source {
            TileSetPageSource::Material(mat) => mat.get_tile_data_mut(handle.tile()),
            TileSetPageSource::Freeform(map) => Some(&mut map.get_mut(&handle.tile())?.data),
            TileSetPageSource::TransformSet(_) => None,
        }
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
            TileSetPageSource::TransformSet(TransformSetTiles(tiles)) => Some(tiles),
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
            if let Some(tile) = TileDefinitionHandle::try_new(page, pos)
                .and_then(|h| self.redirect_handle(stage, h))
            {
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

    /// Tries to find a tile definition with the given position.
    pub fn find_tile_at_position(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        position: Vector2<i32>,
    ) -> Option<TileDefinitionHandle> {
        let handle = TileDefinitionHandle::try_new(page, position)?;
        self.redirect_handle(stage, handle)
    }

    /// Returns true if the tile set is unoccupied at the given position.
    pub fn is_free_at(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        position: Vector2<i32>,
    ) -> bool {
        match stage {
            TilePaletteStage::Pages => self.get_page(position).is_none(),
            TilePaletteStage::Tiles => {
                self.get_page(page).is_some() && !self.has_tile_at(page, position)
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
            if self.is_free_at(stage, page, pos) {
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
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

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
            Ok(LoaderPayload::new(tile_set))
        })
    }
}
