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

use std::{fmt::Debug, ops::Deref};

use super::*;

/// The contraints that control how the autotiler searches for tiles at each position.
pub trait AutoConstrain {
    /// The type of a tile's position.
    type Position: OffsetPosition;
    /// The type of tile terrains. Each terrain represents a set of possible patterns.
    type Terrain: TileTerrain;
    /// An iterator for all the positions that the autotiler should consider when
    /// choosing tiles. The autotiler will select tiles for position in the order
    /// they are delivered by this iterator.
    fn all_positions(&self) -> impl Iterator<Item = &Self::Position>;
    /// The contraint for the given position. This allows the autotiler to examine
    /// adjacent cells when deciding if a particular pattern is a valid choice
    /// for the cell under consideration.
    fn constraint_at(
        &self,
        position: &Self::Position,
    ) -> TileConstraint<Self::Terrain, <Self::Terrain as TileTerrain>::Pattern>;
    /// True if the given patterns may be placed adjacent to each other with the
    /// given offset. For example, if `offset` is up, then return true if
    /// `to` may be legally placed above `from`.
    fn is_legal(
        &self,
        from: &<Self::Terrain as TileTerrain>::Pattern,
        offset: &<Self::Position as OffsetPosition>::Offset,
        to: &<Self::Terrain as TileTerrain>::Pattern,
    ) -> bool;
    /// True if the given patterns may be placed adjacent to each other with the
    /// given offset. For example, if `offset` is up, then return true if
    /// `to` may be legally placed above `from`.
    fn is_legal_diagonal(
        &self,
        from: &<Self::Terrain as TileTerrain>::Pattern,
        diagonal: &<Self::Position as OffsetPosition>::Diagonal,
        to: &<Self::Terrain as TileTerrain>::Pattern,
    ) -> bool;
}

/// For autotiling, tiles are grouped into patterns, and patterns are grouped
/// into terrains. Each cell that is to be autotiled is given a terrain, and
/// the autotiler must choose a pattern from within that terrain.
pub trait TileTerrain {
    /// The type of the patterns within this terrain.
    type Pattern;
    /// Iterate through the patterns within the terrain in the order of priority.
    /// The autotiler will proceed through this iterator until it finds a pattern
    /// that is legal according to the constraints, then ignore any remaining patterns.
    fn all_patterns(&self) -> impl Iterator<Item = &Self::Pattern>;
}

/// Trait for objects that represent a particular autotiling problem to be solved,
/// with a set of positions that need tiles and a terrain for each position to control
/// which tiles are valid.
pub trait TerrainSource {
    /// The type of positions that need tiles.
    type Position;
    /// The type of the terrains that control which patterns are allowed at each position.
    type Terrain;
    /// Iterate over the positions that need tiles and the terrains at each position.
    fn iter(&self) -> impl Iterator<Item = NeededTerrain<Self::Position, Self::Terrain>> + '_;
    /// True if the given position needs to be tiled.
    fn contains_position(&self, position: &Self::Position) -> bool;
}

/// This represents a cell that needs to be autotiled, including its position,
/// its terrain, and the rules for what to do with the surrounding tiles.
pub struct NeededTerrain<Pos, Ter> {
    /// The position of the cell to be autotiled.
    pub position: Pos,
    /// The terrain which specifies the valid patterns for the cell.
    pub terrain: Ter,
    /// The rules for whether surrounding tiles may be changed while
    /// autitiling this cell.
    pub fill: ConstraintFillRules,
}

/// Trait for objects that represent a constraining environment for the autotiler.
/// Given any position, a `PatternSource` object can supply a [`TileConstraint`] for that position.
/// While a [`TerrainSource`] object alone may be enough to tell the autotiler what positions it needs
/// to fill with tiles, a `PatternSource` can provide information about the neighboring tiles, and thus
/// give the autotiler a complete picture of the environment it is tiling.
pub trait PatternSource {
    /// The type of positions that need tiles.
    type Position;
    /// The type of the terrains that control which patterns are allowed at each position.
    type Terrain;
    /// The type of the patterns that the autotiler must choose for each position.
    type Pattern;
    /// The contraint for the cell at the given position.
    fn get(&self, position: &Self::Position) -> TileConstraint<Self::Terrain, Self::Pattern>;
    /// The terrain of the pattern at the given position.
    fn get_terrain(&self, position: &Self::Position) -> Option<Self::Terrain>;
}

/// When a cell is autotiled it may be desired that adjacent cells should be modified
/// to fit with the new terrain of this cell. This object specifies whether that should
/// be done. Cells that are added this way keep their current terrain, but their pattern
/// may change.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConstraintFillRules {
    /// True if adjacent cells should be automatically added to the autotiling constraint
    /// so that they may be modified.
    pub include_adjacent: bool,
    /// True if diagonal cells should be automatically added to the autotiling constraint
    /// so that they may be modified.
    pub include_diagonal: bool,
}

/// A pattern source that is stored in a hash map.
#[derive(Clone)]
pub struct HashConstraintMap<Pos, Ter, Pat> {
    constraints: FxHashMap<Pos, TileConstraint<Ter, Pat>>,
    terrain_cells: Vec<Pos>,
}

impl<Ter: Debug, Pat: Debug> Debug for HashConstraintMap<Vector2<i32>, Ter, Pat> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HashConstraintMap:\n")?;
        for (p, v) in self.constraints.iter() {
            writeln!(f, " ({:2},{:2})->{v:?}", p.x, p.y)?;
        }
        write!(f, "terrain_cells: {:?}", self.terrain_cells)
    }
}

impl<Pos, Ter, Pat> HashConstraintMap<Pos, Ter, Pat>
where
    Pos: Hash + Eq + OffsetPosition,
    Ter: Clone,
    Pat: Clone,
{
    /// Replace the content of this map using the given `PatternSource` and `TerrainSource`.
    /// The `TerrainSource` specifies certain cells that need to be given tiles by specifying
    /// the required terrain for those tiles. The terrains and patterns for the surrounding
    /// cells are taken from the `PatternSource`.
    ///
    /// The immediately adjacent cells are added to the front of the list of cells to be
    /// assigned tiles and their terrain is taken from the `PatternSource`. The cells that
    /// are one further away are make immutable by constraining them to be exactly their
    /// current pattern from `PatternSource`.
    pub fn fill_from<T, P>(&mut self, terrains: &T, patterns: &P)
    where
        T: TerrainSource<Position = Pos, Terrain = Ter>,
        P: PatternSource<Position = Pos, Terrain = Ter, Pattern = Pat>,
    {
        self.clear();
        for ter in terrains.iter() {
            if ter.fill.include_diagonal {
                for offset in Pos::all_diagonals() {
                    let p = ter.position.clone() + offset;
                    if !self.terrain_cells.contains(&p) && !terrains.contains_position(&p) {
                        if let Some(terrain) = patterns.get_terrain(&p) {
                            self.insert(p.clone(), TileConstraint::Terrain(terrain));
                        }
                    }
                }
            }
            if ter.fill.include_adjacent {
                for offset in Pos::all_offsets() {
                    let p = ter.position.clone() + offset;
                    if !self.terrain_cells.contains(&p) && !terrains.contains_position(&p) {
                        if let Some(terrain) = patterns.get_terrain(&p) {
                            self.insert(p.clone(), TileConstraint::Terrain(terrain));
                        }
                    }
                }
            }
        }
        for ter in terrains.iter() {
            self.insert(ter.position, TileConstraint::Terrain(ter.terrain));
        }
        for pos in self.terrain_cells.clone().iter() {
            for offset in Pos::all_offsets() {
                let p = pos.clone() + offset;
                if !self.terrain_cells.contains(&p) {
                    self.insert(p.clone(), patterns.get(&p));
                }
            }
        }
    }
}

impl<Pos, Ter, Pat> HashConstraintMap<Pos, Ter, Pat>
where
    Pos: Hash + Eq + Clone,
{
    /// Makes this map empty, so all cells have [`TileConstraint::None`].
    pub fn clear(&mut self) {
        self.constraints.clear();
        self.terrain_cells.clear();
    }
    /// Add the given constraint to this map.
    pub fn insert(&mut self, pos: Pos, constraint: TileConstraint<Ter, Pat>) {
        if constraint.is_terrain() {
            self.terrain_cells.push(pos.clone());
        }
        _ = self.constraints.insert(pos, constraint);
    }
    /// Get the constraint for the given position.
    pub fn get(&self, pos: &Pos) -> &TileConstraint<Ter, Pat> {
        self.constraints.get(pos).unwrap_or_default()
    }
    /// Iterate over all the terrain cells that are to be tiled in the order
    /// that they should be tiled.
    pub fn all_positions(&self) -> impl Iterator<Item = &Pos> {
        self.terrain_cells.iter()
    }
}

impl<Pos, Ter, Pat> Default for HashConstraintMap<Pos, Ter, Pat> {
    fn default() -> Self {
        Self {
            constraints: FxHashMap::default(),
            terrain_cells: Vec::default(),
        }
    }
}

impl<Pos, Ter, Pat> Deref for HashConstraintMap<Pos, Ter, Pat> {
    type Target = FxHashMap<Pos, TileConstraint<Ter, Pat>>;

    fn deref(&self) -> &Self::Target {
        &self.constraints
    }
}

/// The ways in which a cell's choice of tile can be constrained.
#[derive(Debug, Clone)]
pub enum TileConstraint<T, P> {
    /// No constraint. This means that the cell is outside of the area of consideration
    /// for the autotiler, such as beyond the edge of the world. Cells with this constraint
    /// put no limits on what may be adjacent in any direction.
    None,
    /// A terrain is a set of possible patterns. Any pattern within the terrain may be chosen
    /// to be the pattern for this cell.
    Terrain(T),
    /// A pattern constraint means that the pattern for this cell has already been chosen and
    /// may not change.
    Pattern(P),
}

impl<T, P> Default for TileConstraint<T, P> {
    fn default() -> Self {
        Self::None
    }
}

impl<T, P> Default for &TileConstraint<T, P> {
    fn default() -> Self {
        &TileConstraint::None
    }
}

impl<T, P> TileConstraint<T, P> {
    /// True if this is the None constraint that does not restrict which pattern
    /// may be chosen.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
    /// True if this constraint is not None, meaning it is either a pattern or a terrain.
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
    /// True if this constraint is a terrain, meaning there are zero or more permitted patterns.
    pub fn is_terrain(&self) -> bool {
        matches!(self, Self::Terrain(_))
    }
    /// Tue if this contraint is a pattern, meaning that only that exact pattern is permitted.
    pub fn is_pattern(&self) -> bool {
        matches!(self, Self::Pattern(_))
    }
    /// An iterator over all the patterns explicitly permitted by this contraint,
    /// or an empty iterator if the constraint is None.
    /// The None constraint permits any pattern, but it is not possible to create
    /// an interator over all patterns here, so `all_patterns` should usually only
    /// be called if [`is_some`](Self::is_some) returns true.
    pub fn all_patterns(&self) -> TileConstraint<impl Iterator<Item = &P>, &P>
    where
        T: TileTerrain<Pattern = P>,
    {
        match self {
            Self::None => TileConstraint::None,
            Self::Terrain(t) => TileConstraint::Terrain(t.all_patterns()),
            Self::Pattern(p) => TileConstraint::Pattern(p),
        }
    }
}

impl<T, P> Iterator for TileConstraint<T, P>
where
    T: Iterator<Item = P>,
    P: Clone,
{
    type Item = P;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::None => None,
            Self::Terrain(t) => t.next(),
            Self::Pattern(p) => {
                let p = p.clone();
                *self = Self::None;
                Some(p)
            }
        }
    }
}

/// The object responsible for autotiling and owning a hash map
/// were the result is stored.
#[derive(Debug, Default, Clone)]
pub struct AutoTiler<Pos, Pat> {
    patterns: FxHashMap<Pos, Pat>,
}

impl<Pos, Pat> std::ops::Deref for AutoTiler<Pos, Pat> {
    type Target = FxHashMap<Pos, Pat>;

    fn deref(&self) -> &Self::Target {
        &self.patterns
    }
}

impl<Pos, Pat> std::ops::DerefMut for AutoTiler<Pos, Pat> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.patterns
    }
}

impl<Pos: OffsetPosition, Pat: Clone + Debug> AutoTiler<Pos, Pat> {
    /// Fill the autotile map with patterns according to the given constraint.
    /// The hash map is not automatically cleared by this method, so it may be
    /// called multiple times with different constraints to build up a single
    /// autotile solution.
    pub fn autotile<C, T>(&mut self, constraint: &C)
    where
        C: AutoConstrain<Position = Pos, Terrain = T>,
        T: TileTerrain<Pattern = Pat> + Debug,
        Pos: Debug,
        <Pos as OffsetPosition>::Offset: Debug,
    {
        for pos in constraint.all_positions() {
            if !self.patterns.contains_key(pos) {
                if let Some(pat) = self.find_pattern(pos, constraint) {
                    _ = self.patterns.insert(pos.clone(), pat);
                }
            }
        }
    }
    fn find_pattern<C, T>(&self, position: &Pos, constraint: &C) -> Option<Pat>
    where
        C: AutoConstrain<Position = Pos, Terrain = T>,
        T: TileTerrain<Pattern = Pat> + Debug,
        Pos: Debug,
        <Pos as OffsetPosition>::Offset: Debug,
    {
        for pat in constraint.constraint_at(position).all_patterns() {
            if self.is_pattern_legal(position, pat, constraint) {
                return Some(pat.clone());
            }
        }
        None
    }
    fn is_pattern_legal<C, T>(&self, position: &Pos, pattern: &Pat, constraint: &C) -> bool
    where
        C: AutoConstrain<Position = Pos, Terrain = T>,
        T: TileTerrain<Pattern = Pat>,
        <Pos as OffsetPosition>::Offset: Debug,
    {
        for diagonal in Pos::all_diagonals() {
            let p = position.clone() + diagonal.clone();
            if let Some(pat) = self.patterns.get(&p) {
                if !constraint.is_legal_diagonal(pattern, &diagonal, pat) {
                    return false;
                }
            } else {
                let cell_constraint = constraint.constraint_at(&p);
                if cell_constraint.is_some()
                    && !cell_constraint
                        .all_patterns()
                        .any(|pat| constraint.is_legal_diagonal(pattern, &diagonal, pat))
                {
                    return false;
                }
            }
        }
        for offset in Pos::all_offsets() {
            let p = position.clone() + offset.clone();
            if let Some(pat) = self.patterns.get(&p) {
                if !constraint.is_legal(pattern, &offset, pat) {
                    return false;
                }
            } else {
                let cell_constraint = constraint.constraint_at(&p);
                if cell_constraint.is_some()
                    && !cell_constraint
                        .all_patterns()
                        .any(|pat| constraint.is_legal(pattern, &offset, pat))
                {
                    return false;
                }
            }
        }
        true
    }
}

/// This contains the background information required for autotiling
/// independent of the particular cells to be filled, so it can be used
/// for multiple autotiling tasks. It contains a hash map from terrain
/// values to lists of patterns, representing both the patterns contained
/// within each terrain and the order in which the patterns should be examined.
/// It also contains and a hash map from patterns to a [`ProbabilitySet`] of tiles,
/// representing how a tile should be randomly selected once a pattern is chosen.
#[derive(Clone)]
pub struct AutoTileContext<Ter, Pat, Tile> {
    pattern_list_pool: Vec<Vec<Pat>>,
    probability_set_pool: Vec<ProbabilitySet<Tile>>,
    /// Hash map from terrain values to the list of pattern values that each terrain represents.
    /// The pattern values should be sorted into the order in which the autotiler will try to insert
    /// the patterns into the cells.
    pub patterns: AutoTerrainPatternMap<Ter, Pat>,
    /// Hash map from pattern values to [`ProbabilitySet`] of tile values, giving each tile the probability
    /// it should have when randomly selecting a tile for a given pattern.
    pub values: AutoPatternValueMap<Pat, Tile>,
}

impl<Ter: Debug, Pat: Debug, Tile: Debug> Debug for AutoTileContext<Ter, Pat, Tile> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AutoTileContext\n")?;
        f.write_str("patterns:\n")?;
        for (p, v) in self.patterns.iter() {
            writeln!(f, "{p:?} -> {v:?}")?;
        }
        f.write_str("values:\n")?;
        for (p, v) in self.values.iter() {
            writeln!(f, "{p:?} -> {v:?}")?;
        }
        Ok(())
    }
}

impl<Ter, Pat, Tile> Default for AutoTileContext<Ter, Pat, Tile> {
    fn default() -> Self {
        Self {
            pattern_list_pool: Default::default(),
            probability_set_pool: Default::default(),
            patterns: Default::default(),
            values: Default::default(),
        }
    }
}

impl<Ter: Eq + Hash, Pat: Eq + Hash + Clone, Tile> AutoTileContext<Ter, Pat, Tile> {
    /// True if this context contains no patterns.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
    /// Make the context empty in preparation for building a new context.
    pub fn clear(&mut self) {
        for (_, mut list) in self.patterns.drain() {
            list.clear();
            self.pattern_list_pool.push(list);
        }
        for (_, mut set) in self.values.drain() {
            set.clear();
            self.probability_set_pool.push(set);
        }
    }
    /// Add a particular tile to the context and specify that tile's terrain, pattern,
    /// and frequency. The higher the frequency, the more likely this tile will be chosen
    /// if it shares the same pattern as another tile.
    pub fn add(&mut self, key: Ter, pattern: Pat, frequency: f32, value: Tile) {
        self.patterns
            .entry(key)
            .or_insert_with(|| self.pattern_list_pool.pop().unwrap_or_default())
            .push(pattern.clone());
        self.values
            .entry(pattern)
            .or_insert_with(|| self.probability_set_pool.pop().unwrap_or_default())
            .add(frequency, value);
    }
    /// Sort the pattern values according to their natural order.
    pub fn sort(&mut self)
    where
        Pat: Ord,
    {
        for pattern_list in self.patterns.values_mut() {
            pattern_list.sort();
            pattern_list.dedup();
        }
    }
    /// Get a random tile that has the given pattern, if possible.
    pub fn get_random_value<R: Rng + ?Sized>(&self, rng: &mut R, pattern: &Pat) -> Option<&Tile> {
        self.values.get(pattern).and_then(|vs| vs.get_random(rng))
    }
}

/// A hash map from terrain values to lists of patterns.
/// Each pattern has a terrain, and this map is used to list all of
/// the patterns for a given terrain id value in the order of priority.
/// If multiple patterns in the list may be chosen for some cell,
/// the pattern nearest the front of the list should be chosen.
///
/// * Ter: The type of the terrain ids that are to be associated with lists of patterns.
/// * Pat: The type of a pattern.
///
/// See [`AutoTileContext`] for a way to construct an `AutoTerrainPatternMap`.
/// Once an `AutoTerrainPatternMap` has been constructed, it may be
/// used as part of an [`AutoPatternConstraint`].
pub type AutoTerrainPatternMap<Ter, Pat> = FxHashMap<Ter, Vec<Pat>>;

/// A hash map from patterns to tiles. It is possible for multiple tiles to share
/// the same pattern, and in that case there is no way for the autotiler to deterministically
/// choose a tile. This map gives each pattern a [`ProbabilitySet`] that can be used
/// to randomly select a tile based upon the relative frequency of each tile.
///
/// * Pat: The type of a tile pattern.
/// * Tile: The type of a tile.
///
/// See [`AutoTileContext`] for a way to construct an `AutoPatternValueMap`.
pub type AutoPatternValueMap<Pat, Tile> = FxHashMap<Pat, ProbabilitySet<Tile>>;

/// A pair of an [`HashConstraintMap`] and an [`AutoTerrainPatternMap`].
/// - The constraint map tells the auto-tiler which cells have undecided patterns,
///   which cells have fixed patterns, the terrain type of the undecided cells, and
///   the order in which to fill them.
/// - The terrain map tells the auto-tiler which patterns are available for each
///   terrain type, and the order in which the patterns should be tried.
///
/// See [`AutoTileContext`] for a way to construct an [`AutoTerrainPatternMap`].
pub struct AutoPatternConstraint<'a, 'b, Pos, Ter, Pat> {
    /// The position constraints define the specific problem for the autotiler to solve
    /// by giving it the constraints for each cell, including the cells whose tiles are already
    /// determined and the cells that are yet to be determined.
    pub position_constraints: &'a HashConstraintMap<Pos, Ter, Pat>,
    /// The pattern contraints define the patterns that are available for each terrain.
    /// This will usually remain fixed across many calls to the autotiler, and so it may be reused
    /// so long as the set of tiles does not change.
    /// See [`AutoTileContext`] for a way to construct an [`AutoTerrainPatternMap`].
    pub pattern_constraints: &'b AutoTerrainPatternMap<Ter, Pat>,
}

impl<'b, Pos, Ter, Pat> AutoConstrain for AutoPatternConstraint<'_, 'b, Pos, Ter, Pat>
where
    Pos: OffsetPosition,
    Ter: Hash + Eq,
    Pat: TilePattern<Offset = Pos::Offset, Diagonal = Pos::Diagonal> + Clone,
    Pat: Debug,
    Pos::Offset: Debug,
{
    type Position = Pos;

    type Terrain = ListTerrain<'b, Pat>;

    fn all_positions(&self) -> impl Iterator<Item = &Self::Position> {
        self.position_constraints.all_positions()
    }

    fn constraint_at(&self, position: &Pos) -> TileConstraint<ListTerrain<'b, Pat>, Pat> {
        match self.position_constraints.get(position) {
            TileConstraint::Terrain(ter) => match self.pattern_constraints.get(ter) {
                Some(pat_list) => TileConstraint::Terrain(ListTerrain(pat_list)),
                None => TileConstraint::None,
            },
            TileConstraint::Pattern(pat) => TileConstraint::Pattern(pat.clone()),
            TileConstraint::None => TileConstraint::None,
        }
    }

    fn is_legal(&self, from: &Pat, offset: &Pos::Offset, to: &Pat) -> bool {
        from.is_legal(offset, to)
    }
    fn is_legal_diagonal(&self, from: &Pat, diagonal: &Pos::Diagonal, to: &Pat) -> bool {
        from.is_legal_diagonal(diagonal, to)
    }
}

/// An implementation of [`TileTerrain`] based upon a slice of patterns.
#[derive(Debug, Clone)]
pub struct ListTerrain<'a, P>(pub &'a [P]);

impl<P> TileTerrain for ListTerrain<'_, P> {
    type Pattern = P;

    fn all_patterns(&self) -> impl Iterator<Item = &P> {
        self.0.iter()
    }
}
