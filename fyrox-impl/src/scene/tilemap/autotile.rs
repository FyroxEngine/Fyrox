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

use std::fmt::Debug;

use super::*;
use fxhash::FxHashMap;
use fyrox_autotile::{
    AutoPatternConstraint, AutoPatternValueMap, AutoTerrainPatternMap, AutoTileContext, AutoTiler,
    HashConstraintMap, HashWfcConstraint, OffsetPosition, PatternSource, TileConstraint,
    Vector2Offset, WfcConstrain, WfcFailure, WfcPropagator,
};
use fyrox_core::log::Log;

use crate::core::rand::Rng;
pub use fyrox_autotile::PatternBits;

impl From<NineI8> for PatternBits {
    fn from(value: NineI8) -> Self {
        Self(value.0)
    }
}

/// A value that specifies the terrain of a tile map cell for auto-tiling.
/// This value comes from the center of a nine-slice tile set property,
/// so it can constrain which tiles are permitted in a particular cell.
pub type TileTerrainId = i8;

/// Autotiler for the tiles of a [`TileSet`] that uses the nine-slice
/// values of one of the tile set's properties as the auto-tiler's pattern.
#[derive(Debug, Default, Clone)]
pub struct TileSetAutoTiler(AutoTiler<Vector2<i32>, PatternBits>);
/// This is a specialization of [`AutoTileContext`] for use with a [`TileSet`].
/// The pattern of each tile is taken from the values of a nine-slice property,
/// and [`TileDefinitionHandle`] is used to represent individual tiles.
///
/// See [`TileSetAutoTileContext::fill_pattern_map`] for details on how to fill a
/// `TileSetAutoTileContext` with data from a tile set in preparation for auto-tiling.
#[derive(Default, Clone)]
pub struct TileSetAutoTileContext(
    AutoTileContext<TileTerrainId, PatternBits, TileDefinitionHandle>,
);

impl Debug for TileSetAutoTileContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A map from `Vector2<i32>` positions to auto-tile cell constriants.
/// Each cell contraint is either an [`TileTerrainId`] which represents the center of the
/// permitted patterns for this cell, or else it is a [`PatternBits`] which
/// represents the only permitted pattern for this cell and thereby constrains
/// the neighboring cells.
pub type TileSetConstraintMap = HashConstraintMap<Vector2<i32>, TileTerrainId, PatternBits>;
/// A terrain pattern map for tile map auto-tiler, using the fields of a
/// nine-slice tile set property for the terrain types and the patterns.
pub type TileSetTerrainPatternMap = AutoTerrainPatternMap<TileTerrainId, PatternBits>;
/// A mapping from auto-tiler patterns to tile handles. Multiple tiles may share the
/// same pattern, and each tile is given a frequency value that controls the probability
/// of randomly selecting that tile when its pattern is chosen.
pub type TileSetAutoValueMap = AutoPatternValueMap<PatternBits, TileDefinitionHandle>;
/// A pair of a [`TileSetConstraintMap`] and a [`TileSetTerrainPatternMap`].
/// - The constraint map tells the auto-tiler which cells have undecided patterns,
///   which cells have fixed patterns, and the terrain type of the undecided cells.
/// - The terrain map tells the auto-tiler which patterns are available for each
///   terrain type, and the order in which the patterns should be tried.
pub type TileSetAutoTileConstraint<'a, 'b> =
    AutoPatternConstraint<'a, 'b, Vector2<i32>, TileTerrainId, PatternBits>;

/// A hash table based wave function collapse constraint that maps [`PatternBits`]
/// objects to sets of [`TileDefinitionHandle`]. Each handle refers to a tile,
/// and each tile is assigned a pattern and a frequency. The sum of the frequencies
/// of all a pattern's tiles becomes the frequency of the pattern, which is then
/// normalized into a probability between 0.0 and 1.0 for use as a constraint
/// for a [`TileSetWfcPropagator`] which will perform the wave function collapse.
#[derive(Default)]
pub struct TileSetWfcConstraint(HashWfcConstraint<PatternBits, TileDefinitionHandle>);

impl Deref for TileSetWfcConstraint {
    type Target = HashWfcConstraint<PatternBits, TileDefinitionHandle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TileSetWfcConstraint {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TileSetWfcConstraint {
    /// Clear the data from this constraint and replace it with new data
    /// based upon the given tile set and property UUIDs. All of the tiles
    /// of the tile set are searched and each one's pattern property is checked
    /// for a center value that matches one of the keys of `terrain_freq`.
    /// When such a tile is found, it is inserted into the map using the pattern property
    /// value as the pattern and the frequency property value as the frequency.
    /// - `tile_set`: The tile set to iterate through.
    /// - `pattern_property`: The UUID of a nine-slice property in `tile_set` that will
    ///   be used for the pattern of each tile. The center value of the nine-slice is
    ///   the terrain, unless the center value is 0, in which case the tile is ignored.
    /// - `frequency_property`: The UUID of a float property in `tile_set` that will be
    ///   used for the frequency of each tile. If None, then every tile has frequency 1.0.
    /// - `terrain_freq`: A hash map of the terrains that will be used in wave function collapse.
    ///   Tiles whose center value are not keys in this hash map will be ignored.
    ///   Tiles whose center value are keys in this hash map will have their frequency
    ///   multiplied by the corresponding value in the hash map to calculate the final
    ///   frequency of the tile. This allows terrains to have their frequency weighted,
    ///   and allows unwanted terrains to be excluded.
    ///
    /// *Note:* Terrain 0 is treated specially. It has one pattern, the all-zeros default
    /// [`PatternBits`], and it corresponds to the empty tile. Tiles whose pattern value
    /// have 0 in the center are presumed to be not intended to be part of the wave
    /// function collapse, since 0 is the default value. The frequency of choosing empty
    /// tiles in wave function collapse can be controlled by setting `terrain_freq[0]` to
    /// some value.
    pub fn fill_pattern_map(
        &mut self,
        tile_set: &TileSet,
        pattern_property: TileSetPropertyNine,
        frequency_property: Option<TileSetPropertyF32>,
        terrain_freq: &FxHashMap<TileTerrainId, f32>,
    ) -> Result<(), FillPatternMapError> {
        self.clear();
        if let Some(&frequency) = terrain_freq.get(&0) {
            self.add(
                PatternBits::default(),
                frequency,
                TileDefinitionHandle::EMPTY,
            );
        }
        if tile_set
            .find_property(*pattern_property.property_uuid())
            .is_none()
        {
            return Err(FillPatternMapError::PatternInvalidId);
        }
        if let Some(id) = frequency_property {
            if tile_set.find_property(*id.property_uuid()).is_none() {
                return Err(FillPatternMapError::FrequencyInvalidId);
            }
        }
        for handle in tile_set.all_tiles() {
            let frequency = if let Some(id) = frequency_property {
                id.get_from_tile_set(tile_set, handle)
                    .map_err(|_| FillPatternMapError::FrequencyWrongType)?
            } else {
                1.0
            };
            if frequency <= 0.0 {
                continue;
            }
            let pattern: NineI8 = pattern_property
                .get_from_tile_set(tile_set, handle)
                .map_err(|_| FillPatternMapError::PatternWrongType)?;
            let pattern: PatternBits = pattern.into();
            let center = pattern.center();
            if center != 0 {
                if let Some(terrain_frequency) = terrain_freq.get(&center) {
                    self.add(pattern, frequency * terrain_frequency, handle);
                }
            }
        }
        self.finalize_with_terrain_normalization(PatternBits::center);
        Ok(())
    }
}

/// An error that might occur while filling a pattern map from a tile set
/// using [`TileSetAutoTileContext::fill_pattern_map`].
#[derive(Debug, PartialEq, Eq)]
pub enum FillPatternMapError {
    /// The UUID for the frequency property was not found in the tile set.
    FrequencyInvalidId,
    /// The frequency property was not f32.
    FrequencyWrongType,
    /// The UUID for the terrain property was not found in the tile set.
    PatternInvalidId,
    /// The terrain property was not a nine-slice.
    PatternWrongType,
}

impl Error for FillPatternMapError {}

impl Display for FillPatternMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FillPatternMapError::FrequencyInvalidId => write!(
                f,
                "The property UUID for the frequency does not match any property in the tile set."
            ),
            FillPatternMapError::FrequencyWrongType => {
                write!(f, "The frequency property should be an f32.")
            }
            FillPatternMapError::PatternInvalidId => write!(
                f,
                "The property UUID for the pattern does not match any property in the tile set."
            ),
            FillPatternMapError::PatternWrongType => {
                write!(f, "The pattern property should be a nine-slice property.")
            }
        }
    }
}

impl Deref for TileSetAutoTileContext {
    type Target = AutoTileContext<TileTerrainId, PatternBits, TileDefinitionHandle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TileSetAutoTileContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TileSetAutoTileContext {
    /// Prepare for auto-tiling by extracting tile patterns from the given tile set.
    /// Each tile has an associated pattern through the value of the property
    /// with the UUID given by `pattern_property`. Each pattern contains nine i8 values
    /// arranged in a 3x3 grid. The outer eight values are used to determine which tiles
    /// are permitted to be adjacent to which other tiles, while the central i8 controls
    /// which cells may have this tile. The central value of 0 is reserved for cells
    /// that should be empty, and any tiles which have a central value of 0 are ignored
    /// by the auto-tiler.
    ///
    /// When more than one tile has the same pattern, the auto-tiler must choose between
    /// the tiles randomly. The `frequency_property` is used to weigh the probabilities
    /// of choosing each tile, so tiles with higher values are more likely to be chosen.
    /// If no frequency property is given, then all tiles are equally likely.
    pub fn fill_pattern_map(
        &mut self,
        tile_set: &TileSet,
        pattern_property: TileSetPropertyNine,
        frequency_property: Option<TileSetPropertyF32>,
    ) -> Result<(), FillPatternMapError> {
        self.clear();
        self.add(0, PatternBits::default(), 1.0, TileDefinitionHandle::EMPTY);
        if tile_set
            .find_property(*pattern_property.property_uuid())
            .is_none()
        {
            return Err(FillPatternMapError::PatternInvalidId);
        }
        if let Some(id) = frequency_property {
            if tile_set.find_property(*id.property_uuid()).is_none() {
                return Err(FillPatternMapError::FrequencyInvalidId);
            }
        }
        for handle in tile_set.all_tiles() {
            let frequency = if let Some(id) = frequency_property {
                id.get_from_tile_set(tile_set, handle)
                    .map_err(|_| FillPatternMapError::FrequencyWrongType)?
            } else {
                1.0
            };
            if frequency <= 0.0 {
                continue;
            }
            let pattern: NineI8 = pattern_property
                .get_from_tile_set(tile_set, handle)
                .map_err(|_| FillPatternMapError::PatternWrongType)?;
            let pattern: PatternBits = pattern.into();
            let center = pattern.center();
            if center != 0 {
                self.add(center, pattern, frequency, handle);
            }
        }
        self.sort();
        Ok(())
    }
}

impl Deref for TileSetAutoTiler {
    type Target = AutoTiler<Vector2<i32>, PatternBits>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TileSetAutoTiler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TileSetAutoTiler {
    /// Modify the given tile map update based on the result of the
    /// auto-tiler.
    pub fn apply_autotile_to_update<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        value_map: &TileSetAutoValueMap,
        update: &mut MacroTilesUpdate,
    ) {
        for (pos, pat) in self.iter() {
            let Some(handle) = value_map
                .get(pat)
                .and_then(|ps| ps.get_random(rng))
                .cloned()
            else {
                continue;
            };
            let source = update.get(pos).cloned().flatten().and_then(|el| el.source);
            let handle = if handle.is_empty() {
                None
            } else {
                Some(StampElement { handle, source })
            };
            _ = update.insert(*pos, handle);
        }
    }
}

/// An object that wraps a tile map, a tile set, an update, and the UUID of the property
/// that is used to store the pattern of each tile, and then provides access to the pattern
/// at any cell. It looks for the cell's handle in the update, then in the tile map, and
/// then uses the tile set to find the pattern.
pub struct TileSetPatternSource<'a, 'b, 'c> {
    /// The tile map to get tile handles from.
    pub tile_map: &'a TileMap,
    /// The tile set to get property values from.
    pub tile_set: &'b TileSet,
    /// The current set of modifications to the tile map.
    /// When a cell has been modified, the updated handle will be used
    /// instead of the handle stored in `tile_map`.
    pub update: &'c MacroTilesUpdate,
    /// The UUID of the property to use as the tile pattern.
    pub property_id: TileSetPropertyNine,
}

impl TileSetPatternSource<'_, '_, '_> {
    fn pattern_at(&self, position: &Vector2<i32>) -> Result<PatternBits, TilePropertyError> {
        let Some(element) = self
            .update
            .get(position)
            .cloned()
            .unwrap_or_else(|| self.tile_map.tile_handle(*position).map(|h| h.into()))
        else {
            return Ok(PatternBits::default());
        };
        self.property_id
            .get_from_tile_set(self.tile_set, element.handle)
            .map(|v| v.into())
    }
}

impl PatternSource for TileSetPatternSource<'_, '_, '_> {
    type Position = Vector2<i32>;
    type Terrain = TileTerrainId;
    type Pattern = PatternBits;

    fn get_terrain(&self, position: &Vector2<i32>) -> Option<TileTerrainId> {
        match self.pattern_at(position) {
            Ok(pattern) => Some(pattern.center()),
            Err(err) => {
                Log::err(err.to_string());
                None
            }
        }
    }
    /// The constraint for the cell at the given position.
    fn get(&self, position: &Vector2<i32>) -> TileConstraint<TileTerrainId, PatternBits> {
        match self.pattern_at(position) {
            Ok(pattern) => TileConstraint::Pattern(pattern),
            Err(err) => {
                Log::err(err.to_string());
                TileConstraint::None
            }
        }
    }
}

/// Wave function collapse propagator for the tiles of a [`TileSet`] that uses
/// the nine-slice values of one of the tile set's properties as the wave function's
/// pattern.
#[derive(Debug, Default, Clone)]
pub struct TileSetWfcPropagator {
    propagator: WfcPropagator<Vector2<i32>, PatternBits>,
    edge_restrictions: FxHashMap<Vector2<i32>, PatternBits>,
}

impl Deref for TileSetWfcPropagator {
    type Target = WfcPropagator<Vector2<i32>, PatternBits>;

    fn deref(&self) -> &Self::Target {
        &self.propagator
    }
}

impl DerefMut for TileSetWfcPropagator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.propagator
    }
}

impl TileSetWfcPropagator {
    /// After all the wave cells have been added using [`WfcPropagator::add_cell`],
    /// this method may be used to automatically restrict the edges of the cells by
    /// using the given tile map to find the patterns of surrounding tiles and
    /// calling [`WfcPropagator::restrict_edge`] as appropriate.
    ///
    /// This forces the wave function collapse to fit smoothly with the existing
    /// tiles of the tile map, though such restrictions may make failure more likely.
    /// [`WfcFailure`] is returned if wave function collapse is made impossible by the
    /// restrictions.
    pub fn constrain_edges<Con>(
        &mut self,
        tile_set: &TileSet,
        pattern_property: TileSetPropertyNine,
        tile_map: &TileMap,
        update: &MacroTilesUpdate,
        constraint: &Con,
    ) -> Result<(), WfcFailure>
    where
        Con: WfcConstrain<Pattern = PatternBits, Offset = Vector2Offset>,
    {
        let tiles = tile_map.tiles();
        let tiles = tiles.map(|r| r.data_ref());
        let mut edge_restrictions = std::mem::take(&mut self.edge_restrictions);
        edge_restrictions.clear();
        for &p in self.positions() {
            for offset in Vector2::all_offsets() {
                let p = p + offset;
                if self.edge_restrictions.contains_key(&p) {
                    continue;
                }
                if self.contains_cell(&p) {
                    continue;
                }
                let handle = update.get(&p).map(|el| {
                    el.as_ref()
                        .map(|el| el.handle)
                        .unwrap_or(TileDefinitionHandle::EMPTY)
                });
                let handle = if let Some(handle) = handle {
                    handle
                } else if let Some(tiles) = tiles.as_ref() {
                    tiles.get(p).unwrap_or(TileDefinitionHandle::EMPTY)
                } else {
                    TileDefinitionHandle::EMPTY
                };
                let pattern = pattern_property
                    .get_from_tile_set(tile_set, handle)
                    .unwrap_or_default();
                edge_restrictions.insert(p, PatternBits(pattern.into()));
            }
        }
        for (p, pattern) in edge_restrictions.drain() {
            self.restrict_edge(&p, &pattern, constraint)?;
        }
        self.edge_restrictions = edge_restrictions;
        Ok(())
    }
    /// Modify the given tile map update based on the result of the
    /// autotiler.
    pub fn apply_autotile_to_update<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        value_map: &TileSetWfcConstraint,
        update: &mut MacroTilesUpdate,
    ) {
        for (pos, pat) in self.assigned_patterns() {
            let Some(&handle) = value_map.get_random(rng, pat) else {
                continue;
            };
            let source = update.get(pos).cloned().flatten().and_then(|el| el.source);
            let handle = if handle.is_empty() {
                None
            } else {
                Some(StampElement { handle, source })
            };
            _ = update.insert(*pos, handle);
        }
    }
    /// Modify the given tile map data based on the result of the
    /// autotiler.
    pub fn apply_autotile_to_data<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        value_map: &TileSetWfcConstraint,
        data: &mut TileMapData,
    ) {
        for (pos, pat) in self.assigned_patterns() {
            let Some(&handle) = value_map.get_random(rng, pat) else {
                continue;
            };
            data.set(*pos, handle);
        }
    }
}
