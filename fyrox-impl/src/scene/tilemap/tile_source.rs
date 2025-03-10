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

//! The `tile_source` module contains structs that represent arrangements of tiles
//! to be used by tile map drawing tools such as rectangular fills and flood fills.
//! Tile sources can be randomized and they can repeat to create varied effects
//! while editing tile maps.

use fyrox_core::swap_hash_map_entry;

use crate::{
    core::{algebra::Vector2, reflect::prelude::*, visitor::prelude::*},
    fxhash::FxHashMap,
    rand::{seq::IteratorRandom, thread_rng},
};
use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    ops::{Deref, DerefMut},
    str::FromStr,
};

use super::*;

/// The type of coordinates stored in a a [TileDefinitionHandle].
pub type PalettePosition = Vector2<i16>;

#[inline]
fn try_position(source: Vector2<i32>) -> Option<PalettePosition> {
    Some(PalettePosition::new(
        source.x.try_into().ok()?,
        source.y.try_into().ok()?,
    ))
}

#[inline]
fn position_to_vector(source: PalettePosition) -> Vector2<i32> {
    source.map(|x| x as i32)
}

/// A 2D grid that contains tile data.
#[derive(Default, Debug, Clone, PartialEq, Reflect)]
pub struct TileGridMap<V: Debug>(FxHashMap<Vector2<i32>, V>);

impl<V: Visit + Default + Debug> Visit for TileGridMap<V> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl<V: Debug> Deref for TileGridMap<V> {
    type Target = FxHashMap<Vector2<i32>, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V: Debug> DerefMut for TileGridMap<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Position of a tile definition within some tile set
#[derive(Eq, PartialEq, Clone, Copy, Default, Hash, Reflect, Visit, TypeUuidProvider)]
#[type_uuid(id = "3eb69303-d361-482d-8094-44b9f9c323ca")]
#[repr(C)]
pub struct TileDefinitionHandle {
    /// Position of the tile's page
    pub page: PalettePosition,
    /// Position of the tile definition within the page
    pub tile: PalettePosition,
}

unsafe impl bytemuck::Zeroable for TileDefinitionHandle {}

unsafe impl bytemuck::Pod for TileDefinitionHandle {}

impl PartialOrd for TileDefinitionHandle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TileDefinitionHandle {
    fn cmp(&self, other: &Self) -> Ordering {
        self.page
            .y
            .cmp(&other.page.y)
            .reverse()
            .then(self.page.x.cmp(&other.page.x))
            .then(self.tile.y.cmp(&other.tile.y).reverse())
            .then(self.tile.x.cmp(&other.tile.x))
    }
}

impl Display for TileDefinitionHandle {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            f.write_str("Empty")
        } else {
            write!(
                f,
                "({},{}):({},{})",
                self.page.x, self.page.y, self.tile.x, self.tile.y
            )
        }
    }
}

impl Debug for TileDefinitionHandle {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TileDefinitionHandle({},{};{},{})",
            self.page.x, self.page.y, self.tile.x, self.tile.y
        )
    }
}

impl FromStr for TileDefinitionHandle {
    type Err = TileDefinitionHandleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(TileDefinitionHandleParseError)
    }
}

/// An syntax error in parsing a TileDefinitionHandle from a string.
#[derive(Debug)]
pub struct TileDefinitionHandleParseError;

impl Display for TileDefinitionHandleParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tile definition handle parse failure")
    }
}

impl Error for TileDefinitionHandleParseError {}

impl TileDefinitionHandle {
    /// Handle the represents the absence of a tile.
    pub const EMPTY: Self = Self::new(i16::MIN, i16::MIN, i16::MIN, i16::MIN);
    /// True if this handle represents there being no tile, [`EMPTY`](Self::EMPTY).
    pub fn is_empty(&self) -> bool {
        self == &Self::EMPTY
    }
    /// Attempt to construct a handle for the given page and tile positions.
    /// Handles use a pair of i16 vectors, so that the total is 64 bits.
    /// If the given vectors are outside of the range that can be represented as i16 coordinates,
    /// then None is returned.
    pub fn try_new(page: Vector2<i32>, tile: Vector2<i32>) -> Option<Self> {
        Some(Self {
            page: try_position(page)?,
            tile: try_position(tile)?,
        })
    }
    /// Construct a handle directly from coordinates. This is intended for cases
    /// where certain tile handles may need to be hard-coded as having special significance.
    pub const fn new(page_x: i16, page_y: i16, tile_x: i16, tile_y: i16) -> Self {
        Self {
            page: PalettePosition::new(page_x, page_y),
            tile: PalettePosition::new(tile_x, tile_y),
        }
    }
    /// Extracts the page coordinates and converts them to an i32 vector.
    pub fn page(&self) -> Vector2<i32> {
        position_to_vector(self.page)
    }
    /// Extracts the tile coordinates and converts them to an i32 vector.
    pub fn tile(&self) -> Vector2<i32> {
        position_to_vector(self.tile)
    }
    /// Convert a string into a tile definition handle by finding four numbers.
    /// The first two numbers are the page coodrinates. The second two numbers are the tile coordinates.
    /// None is returned if there are more than four numbers, fewer than four numbers, or any number produces an error in parsing.
    pub fn parse(s: &str) -> Option<Self> {
        if s.eq_ignore_ascii_case("Empty") {
            return Some(Self::EMPTY);
        }
        let mut iter = s
            .split(|c: char| c != '-' && !c.is_ascii_digit())
            .filter(|w| !w.is_empty());
        let a: i16 = iter.next()?.parse().ok()?;
        let b: i16 = iter.next()?.parse().ok()?;
        let c: i16 = iter.next()?.parse().ok()?;
        let d: i16 = iter.next()?.parse().ok()?;
        if iter.next().is_some() {
            None
        } else {
            Some(Self::new(a, b, c, d))
        }
    }
}

/// A region of tiles to be filled from some source of tiles.
#[derive(Debug, Default, Clone)]
pub struct TileRegion {
    /// The position to put the (0,0) tile of the tile source.
    /// If `origin` is not within `bounds` then the (0,0) tile will not actually be used.
    pub origin: Vector2<i32>,
    /// The area to fill.
    pub bounds: OptionTileRect,
}

impl TileRegion {
    /// Construct a region with its origin in one of the four corners of the given bounds.
    /// The corner of the origin is based on the given direction.
    pub fn from_bounds_and_direction(bounds: OptionTileRect, direction: Vector2<i32>) -> Self {
        let Some(bounds) = *bounds else {
            return Self::default();
        };
        let x0 = if direction.x <= 0 {
            bounds.left_bottom_corner().x
        } else {
            bounds.right_top_corner().x
        };
        let y0 = if direction.y <= 0 {
            bounds.left_bottom_corner().y
        } else {
            bounds.right_top_corner().y
        };
        Self {
            origin: Vector2::new(x0, y0),
            bounds: bounds.into(),
        }
    }
    /// Construct a region with `bounds` that contain `origin` and `end`.
    pub fn from_points(origin: Vector2<i32>, end: Vector2<i32>) -> Self {
        Self {
            origin,
            bounds: OptionTileRect::from_points(origin, end),
        }
    }
    /// Copy the region and replace its bound.
    pub fn with_bounds(mut self, bounds: OptionTileRect) -> Self {
        self.bounds = bounds;
        self
    }
    /// Reduce the size of `bounds` by deflating them by the given amounts.
    pub fn deflate(mut self, dw: i32, dh: i32) -> Self {
        self.bounds = self.bounds.deflate(dw, dh);
        self
    }
    /// Iterator over `(target, source)` pairs where `target` is the position to put the tile
    /// and `source` is the position to get the tile from within the tile source.
    /// Every position within `bounds` will appear once as the `target`.
    /// If `origin` is within `bounds`, then `(origin, (0,0))` will be produced by the iterator.
    pub fn iter(&self) -> impl Iterator<Item = (Vector2<i32>, Vector2<i32>)> + '_ {
        self.bounds.iter().map(|p| (p, p - self.origin))
    }
}

/// A trait for types that can produce a TileDefinitionHandle upon demand,
/// for use with drawing on tilemaps.
pub trait TileSource {
    /// The source where these tiles were originally taken from.
    fn brush(&self) -> Option<&TileMapBrushResource>;
    /// The transformation that should be applied to the tiles before they are written.
    fn transformation(&self) -> OrthoTransformation;
    /// Produce a tile definition handle for the given position. If an area of multiple
    /// tiles is being filled, then the given position represents where the tile
    /// will go within the area.
    fn get_at(&self, position: Vector2<i32>) -> Option<StampElement>;
}

/// A trait for types that can produce a TileDefinitionHandle upon demand,
/// for use with drawing on tilemaps.
pub trait BoundedTileSource: TileSource {
    /// Calculates bounding rectangle in grid coordinates.
    fn bounding_rect(&self) -> OptionTileRect;
}

/// A tile source that always produces the same tile.
#[derive(Clone, Debug)]
pub struct SingleTileSource(pub OrthoTransformation, pub StampElement);

impl TileSource for SingleTileSource {
    fn brush(&self) -> Option<&TileMapBrushResource> {
        None
    }
    fn transformation(&self) -> OrthoTransformation {
        self.0
    }
    fn get_at(&self, _position: Vector2<i32>) -> Option<StampElement> {
        Some(self.1.clone())
    }
}

/// A tile source that produces a random tile from the included set of tiles.
pub struct RandomTileSource<'a>(pub &'a Stamp);

impl TileSource for RandomTileSource<'_> {
    fn brush(&self) -> Option<&TileMapBrushResource> {
        self.0.brush()
    }
    fn transformation(&self) -> OrthoTransformation {
        self.0.transformation()
    }
    fn get_at(&self, _position: Vector2<i32>) -> Option<StampElement> {
        self.0.values().choose(&mut thread_rng()).cloned()
    }
}

/// A tile source that produces a random tile from the included set of tiles.
pub struct PartialRandomTileSource<'a>(pub &'a Stamp, pub OptionTileRect);

impl TileSource for PartialRandomTileSource<'_> {
    fn brush(&self) -> Option<&TileMapBrushResource> {
        self.0.brush()
    }
    fn transformation(&self) -> OrthoTransformation {
        self.0.transformation()
    }
    fn get_at(&self, _position: Vector2<i32>) -> Option<StampElement> {
        let pos = self.1.iter().choose(&mut thread_rng())?;
        self.0.get_at(pos)
    }
}

/// A tile source that adapts another source so that it infinitely repeats the tiles
/// within the given rect.
pub struct RepeatTileSource<'a, S> {
    /// The tiles to repeat
    pub source: &'a S,
    /// The region within the stamp to repeat
    pub region: TileRegion,
}

impl<S: TileSource> TileSource for RepeatTileSource<'_, S> {
    fn brush(&self) -> Option<&TileMapBrushResource> {
        self.source.brush()
    }
    fn transformation(&self) -> OrthoTransformation {
        self.source.transformation()
    }
    fn get_at(&self, position: Vector2<i32>) -> Option<StampElement> {
        let rect = (*self.region.bounds)?;
        let rect_pos = rect.position;
        let size = rect.size;
        let pos = position + self.region.origin - rect_pos;
        let x = pos.x.rem_euclid(size.x);
        let y = pos.y.rem_euclid(size.y);
        self.source.get_at(Vector2::new(x, y) + rect_pos)
    }
}

/// A set of tiles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tiles(TileGridMap<TileDefinitionHandle>);

/// A set of tiles and a transformation, which represents the tiles that the user has selected
/// to draw with.
#[derive(Clone, Debug, Default, Visit)]
pub struct Stamp {
    transform: OrthoTransformation,
    #[visit(skip)]
    elements: OrthoTransformMap<StampElement>,
    #[visit(skip)]
    source: Option<TileBook>,
}

/// Each cell of a stamp must have a tile handle and it may optionally have
/// the handle of a brush cell where the tile was taken from.
#[derive(Clone, Debug)]
pub struct StampElement {
    /// The stamp cell's tile handle
    pub handle: TileDefinitionHandle,
    /// The brush cell that this stamp element came from.
    pub source: Option<ResourceTilePosition>,
}

impl From<TileDefinitionHandle> for StampElement {
    fn from(handle: TileDefinitionHandle) -> Self {
        Self {
            handle,
            source: None,
        }
    }
}

impl TileSource for Tiles {
    fn brush(&self) -> Option<&TileMapBrushResource> {
        None
    }
    fn transformation(&self) -> OrthoTransformation {
        OrthoTransformation::default()
    }
    fn get_at(&self, position: Vector2<i32>) -> Option<StampElement> {
        self.get(&position).copied().map(|h| h.into())
    }
}

impl Visit for Tiles {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Deref for Tiles {
    type Target = TileGridMap<TileDefinitionHandle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tiles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TileSource for Stamp {
    fn brush(&self) -> Option<&TileMapBrushResource> {
        self.source.as_ref().and_then(|s| s.brush_ref())
    }
    fn transformation(&self) -> OrthoTransformation {
        self.transform
    }
    fn get_at(&self, position: Vector2<i32>) -> Option<StampElement> {
        self.elements.get(position).cloned()
    }
}

impl Stamp {
    /// The resource where this stamp was taken from.
    pub fn source(&self) -> Option<&TileBook> {
        self.source.as_ref()
    }
    /// Iterate over the tile handles of the stamp.
    pub fn tile_iter(&self) -> impl Iterator<Item = TileDefinitionHandle> + '_ {
        self.elements.values().map(|s| s.handle)
    }
    /// Create a repeating tile source from this stamp to repeat from `start` to `end.`
    pub fn repeat(&self, start: Vector2<i32>, end: Vector2<i32>) -> RepeatTileSource<Stamp> {
        let bounds = self.bounding_rect();
        RepeatTileSource {
            source: self,
            region: TileRegion::from_bounds_and_direction(bounds, start - end),
        }
    }

    /// Create a repeating tile source from the stamp with no specified direction for the repeat.
    pub fn repeat_anywhere(&self) -> RepeatTileSource<Stamp> {
        let bounds = self.bounding_rect();
        RepeatTileSource {
            source: self,
            region: TileRegion::from_bounds_and_direction(bounds, Vector2::new(0, 0)),
        }
    }

    /// True if this stamp contains no tiles.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
    /// Turn this stamp into an empty stamp.
    pub fn clear(&mut self) {
        self.transform = OrthoTransformation::identity();
        self.elements.clear();
    }
    /// Clear this stamp and fill it with the given tiles.
    /// The tiles are moved so that their center is (0,0).
    /// The transform is set to identity.
    pub fn build<I: Iterator<Item = (Vector2<i32>, StampElement)> + Clone>(
        &mut self,
        book: Option<TileBook>,
        source: I,
    ) {
        self.source = book;
        self.clear();
        let mut rect = OptionTileRect::default();
        for (p, _) in source.clone() {
            rect.push(p);
        }
        let Some(rect) = *rect else {
            return;
        };
        let center = rect.center();
        for (p, e) in source {
            _ = self.insert(p - center, e);
        }
    }
    /// Rotate the stamp by the given number of 90-degree turns.
    pub fn rotate(&mut self, amount: i8) {
        self.transform = self.transform.rotated(amount);
        self.elements = std::mem::take(&mut self.elements).rotated(amount);
    }
    /// Flip along the x axis.
    pub fn x_flip(&mut self) {
        self.transform = self.transform.x_flipped();
        self.elements = std::mem::take(&mut self.elements).x_flipped();
    }
    /// Flip along the y axis.
    pub fn y_flip(&mut self) {
        self.transform = self.transform.y_flipped();
        self.elements = std::mem::take(&mut self.elements).y_flipped();
    }
    /// Rotate the stamp by the given number of 90-degree turns.
    pub fn transform(&mut self, amount: OrthoTransformation) {
        self.transform = self.transform.transformed(amount);
        self.elements = std::mem::take(&mut self.elements).transformed(amount);
    }
}

impl Deref for Stamp {
    type Target = OrthoTransformMap<StampElement>;
    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}

impl DerefMut for Stamp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elements
    }
}

impl Tiles {
    /// Construct a new tile set from the given hash map.
    pub fn new(source: TileGridMap<TileDefinitionHandle>) -> Self {
        Self(source)
    }
    /// Find the first empty cell in the negative-x direction and the first empty
    /// cell in the positive-x direction.
    pub fn find_continuous_horizontal_span(&self, position: Vector2<i32>) -> (i32, i32) {
        let y = position.y;
        let mut min = position.x;
        while self.contains_key(&Vector2::new(min, y)) {
            min -= 1;
        }
        let mut max = position.x;
        while self.contains_key(&Vector2::new(max, y)) {
            max += 1;
        }
        (min, max)
    }
    /// Apply the updates specified in the given `TileUpdate` and modify it so that it
    /// contains the tiles require to undo the change. Calling `swap_tiles` twice with the same
    /// `TileUpdate` object will do the changes and then undo them, leaving the tiles unchanged in the end.
    pub fn swap_tiles(&mut self, updates: &mut TilesUpdate) {
        for (k, v) in updates.iter_mut() {
            swap_hash_map_entry(self.entry(*k), v);
        }
    }
    /// Calculates bounding rectangle in grid coordinates.
    #[inline]
    pub fn bounding_rect(&self) -> OptionTileRect {
        let mut result = OptionTileRect::default();
        for position in self.keys() {
            result.push(*position);
        }
        result
    }

    /// Clears the tile container.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checking that TileDefinitionHandle is using the expected data layout as
    /// required for its unsafe `bytemuck::Pod` implementation.
    #[test]
    fn size_of_handle() {
        assert_eq!(std::mem::size_of::<TileDefinitionHandle>(), 8);
    }

    #[test]
    fn zero_handle() {
        assert_eq!(
            TileDefinitionHandle::zeroed(),
            TileDefinitionHandle::default()
        );
    }
}
