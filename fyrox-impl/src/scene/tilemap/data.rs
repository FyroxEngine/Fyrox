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

use std::{collections::hash_map, path::Path};

use crate::asset::{Resource, ResourceData};
use fxhash::FxHashMap;
use fyrox_core::visitor::BinaryBlob;

use crate::core::{
    algebra::Vector2, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
};

use super::*;

const CHUNK_WIDTH: usize = 16;
const CHUNK_HEIGHT: usize = 16;
const WIDTH_BITS: i32 = (CHUNK_WIDTH - 1) as i32;
const HEIGHT_BITS: i32 = (CHUNK_HEIGHT - 1) as i32;

/// Resource for storing the tile handles of a tile map.
pub type TileMapDataResource = Resource<TileMapData>;

/// Given a tile position, calculate the position of the chunk containing that tile
/// and the position of the tile within that chunk, and return them as a pair:
/// (chunk position, tile position within chunk)
fn tile_position_to_chunk_position(position: Vector2<i32>) -> (Vector2<i32>, Vector2<i32>) {
    let x = position.x;
    let y = position.y;
    let x_chunk = x & !WIDTH_BITS;
    let y_chunk = y & !HEIGHT_BITS;
    (
        Vector2::new(x_chunk, y_chunk),
        Vector2::new(x - x_chunk, y - y_chunk),
    )
}

#[derive(Clone, Debug, Reflect)]
struct Chunk([TileDefinitionHandle; CHUNK_WIDTH * CHUNK_HEIGHT]);

impl Visit for Chunk {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.is_reading() {
            let mut data = Vec::default();
            BinaryBlob { vec: &mut data }.visit(name, visitor)?;
            if data.len() != CHUNK_WIDTH * CHUNK_HEIGHT {
                return Err(VisitError::User(
                    "Wrong number of handles in a chunk".into(),
                ));
            }
            self.0
                .clone_from_slice(&data[0..CHUNK_WIDTH * CHUNK_HEIGHT]);
            Ok(())
        } else {
            BinaryBlob {
                vec: &mut self.0.to_vec(),
            }
            .visit(name, visitor)
        }
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self([TileDefinitionHandle::EMPTY; CHUNK_WIDTH * CHUNK_HEIGHT])
    }
}

impl std::ops::Index<Vector2<i32>> for Chunk {
    type Output = TileDefinitionHandle;

    fn index(&self, index: Vector2<i32>) -> &Self::Output {
        let x: usize = index.x.try_into().unwrap();
        let y: usize = index.y.try_into().unwrap();
        &self.0[x + y * CHUNK_WIDTH]
    }
}

impl std::ops::IndexMut<Vector2<i32>> for Chunk {
    fn index_mut(&mut self, index: Vector2<i32>) -> &mut Self::Output {
        let x: usize = index.x.try_into().unwrap();
        let y: usize = index.y.try_into().unwrap();
        &mut self.0[x + y * CHUNK_WIDTH]
    }
}

impl Chunk {
    fn iter(&self, offset: Vector2<i32>) -> ChunkIterator {
        ChunkIterator {
            position: Vector2::new(0, 0),
            chunk: self,
            offset,
        }
    }
    fn is_empty(&self) -> bool {
        self.0.iter().all(|h| *h == TileDefinitionHandle::EMPTY)
    }
}

struct ChunkIterator<'a> {
    position: Vector2<i32>,
    offset: Vector2<i32>,
    chunk: &'a Chunk,
}

impl Iterator for ChunkIterator<'_> {
    type Item = (Vector2<i32>, TileDefinitionHandle);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.position.y >= CHUNK_HEIGHT as i32 {
                return None;
            }
            let result_position = self.position;
            let result = self.chunk[result_position];
            if self.position.x < (CHUNK_WIDTH - 1) as i32 {
                self.position.x += 1;
            } else {
                self.position.x = 0;
                self.position.y += 1;
            }
            if !result.is_empty() {
                return Some((result_position + self.offset, result));
            }
        }
    }
}

/// Iterator over the tiles of a [`TileMapData`] in the form of (position, handle).
pub struct TileMapDataIterator<'a, P: 'a> {
    predicate: P,
    map_iter: hash_map::Iter<'a, Vector2<i32>, Chunk>,
    chunk_iter: Option<ChunkIterator<'a>>,
}

impl<P: FnMut(Vector2<i32>) -> bool> Iterator for TileMapDataIterator<'_, P> {
    type Item = (Vector2<i32>, TileDefinitionHandle);
    fn next(&mut self) -> Option<Self::Item> {
        let chunk_iter = match &mut self.chunk_iter {
            Some(iter) => iter,
            None => self.next_chunk()?,
        };
        if let Some(result) = chunk_iter.next() {
            Some(result)
        } else {
            self.next_chunk()?.next()
        }
    }
}

impl<'a, P: FnMut(Vector2<i32>) -> bool> TileMapDataIterator<'a, P> {
    fn next_chunk(&mut self) -> Option<&mut ChunkIterator<'a>> {
        loop {
            let (pos, chunk) = self.map_iter.next()?;
            if (self.predicate)(*pos) {
                return Some(self.chunk_iter.insert(chunk.iter(*pos)));
            }
        }
    }
}

/// Asset containing the tile handles of a tile map.
#[derive(Clone, Default, Debug, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "a8e4b6b4-c1bd-4ed9-a753-0d5a3dfe1729")]
pub struct TileMapData {
    content: FxHashMap<Vector2<i32>, Chunk>,
}

impl Visit for TileMapData {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if !visitor.is_reading() {
            self.shrink_to_fit();
        }
        self.content.visit(name, visitor)
    }
}

impl ResourceData for TileMapData {
    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut visitor = Visitor::new();
        self.visit("TileMapData", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        false
    }
}

impl TileSource for TileMapData {
    fn brush(&self) -> Option<&TileMapBrushResource> {
        None
    }
    fn transformation(&self) -> OrthoTransformation {
        OrthoTransformation::default()
    }

    fn get_at(&self, position: Vector2<i32>) -> Option<StampElement> {
        self.get(position).map(|h| h.into())
    }
}

impl BoundedTileSource for TileMapData {
    fn bounding_rect(&self) -> OptionTileRect {
        let mut rect = OptionTileRect::default();
        for (pos, _) in self.iter() {
            rect.push(pos);
        }
        rect
    }
}

impl TileMapData {
    /// Iterate over all pairs of (position, handle) in this data.
    pub fn iter(&self) -> impl Iterator<Item = (Vector2<i32>, TileDefinitionHandle)> + '_ {
        let map_iter = self.content.iter();
        TileMapDataIterator {
            predicate: |_| true,
            map_iter,
            chunk_iter: None,
        }
    }
    /// Iterate over all pairs of (position, handle) in this data.
    pub fn bounded_iter(
        &self,
        bounds: OptionTileRect,
    ) -> impl Iterator<Item = (Vector2<i32>, TileDefinitionHandle)> + '_ {
        let map_iter = self.content.iter();
        TileMapDataIterator {
            predicate: move |pos: Vector2<i32>| {
                bounds.intersects(TileRect::new(
                    pos.x,
                    pos.y,
                    CHUNK_WIDTH as i32,
                    CHUNK_HEIGHT as i32,
                ))
            },
            map_iter,
            chunk_iter: None,
        }
    }
    /// Apply the updates specified in the given `TileUpdate` and modify it so that it
    /// contains the tiles require to undo the change. Calling `swap_tiles` twice with the same
    /// `TileUpdate` object will do the changes and then undo them, leaving the tiles unchanged in the end.
    pub fn swap_tiles(&mut self, tiles: &mut TilesUpdate) {
        for (p, h) in tiles.iter_mut() {
            *h = self.replace(*p, *h);
        }
    }
    /// Get the handle for the tile at the given position, if one exists.
    pub fn get(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        let (chunk, pos) = tile_position_to_chunk_position(position);
        let chunk = self.content.get(&chunk)?;
        let handle = chunk[pos];
        if handle.is_empty() {
            None
        } else {
            Some(handle)
        }
    }
    /// Replace the handle at the given position with the given handle and return the original
    /// handle at that position.
    pub fn replace(
        &mut self,
        position: Vector2<i32>,
        value: Option<TileDefinitionHandle>,
    ) -> Option<TileDefinitionHandle> {
        let (chunk, pos) = tile_position_to_chunk_position(position);
        if let Some(chunk) = self.content.get_mut(&chunk) {
            let handle = &mut chunk[pos];
            let result = *handle;
            *handle = value.unwrap_or(TileDefinitionHandle::EMPTY);
            if result.is_empty() {
                None
            } else {
                Some(result)
            }
        } else if let Some(value) = value {
            let chunk = self.content.entry(chunk).or_default();
            chunk[pos] = value;
            None
        } else {
            None
        }
    }
    /// Set a new handle for the tile at the given position.
    pub fn set(&mut self, position: Vector2<i32>, value: TileDefinitionHandle) {
        let (chunk, pos) = tile_position_to_chunk_position(position);
        let chunk = self.content.entry(chunk).or_default();
        chunk[pos] = value;
    }
    /// Remove the tile at the given position.
    pub fn remove(&mut self, position: Vector2<i32>) {
        let (chunk, pos) = tile_position_to_chunk_position(position);
        if let Some(chunk) = self.content.get_mut(&chunk) {
            chunk[pos] = TileDefinitionHandle::EMPTY;
        }
    }
    /// Remove all empty chunks.
    pub fn shrink_to_fit(&mut self) {
        self.content.retain(|_, v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    fn v(x: i32, y: i32) -> Vector2<i32> {
        Vector2::new(x, y)
    }

    fn h(a: i16, b: i16, c: i16, d: i16) -> TileDefinitionHandle {
        TileDefinitionHandle::new(a, b, c, d)
    }

    fn v_ord(a: &Vector2<i32>, b: &Vector2<i32>) -> Ordering {
        a.y.cmp(&b.y).reverse().then(a.x.cmp(&b.x))
    }

    #[test]
    fn position_to_chunk() {
        assert_eq!(
            tile_position_to_chunk_position(v(16, 16)),
            (v(16, 16), v(0, 0))
        );
        assert_eq!(tile_position_to_chunk_position(v(0, 0)), (v(0, 0), v(0, 0)));
        assert_eq!(
            tile_position_to_chunk_position(v(-5, 5)),
            (v(-16, 0), v(11, 5))
        );
        assert_eq!(
            tile_position_to_chunk_position(v(-16, 5)),
            (v(-16, 0), v(0, 5))
        );
        assert_eq!(
            tile_position_to_chunk_position(v(-17, 5)),
            (v(-32, 0), v(15, 5))
        );
    }
    #[test]
    fn create_chunks() {
        let mut data = TileMapData::default();
        let coords = vec![
            (v(0, 0), h(1, 2, 3, 4)),
            (v(-1, -2), h(1, 2, 3, 0)),
            (v(16, 16), h(1, 2, 3, 5)),
            (v(-1, -1), h(1, 2, 3, 6)),
            (v(-17, 0), h(1, 2, 3, 7)),
        ];
        for (pos, handle) in coords.iter() {
            data.set(*pos, *handle);
        }
        let mut coords = coords
            .into_iter()
            .map(|(p, _)| tile_position_to_chunk_position(p).0)
            .collect::<Vec<_>>();
        coords.sort_by(v_ord);
        coords.dedup();
        let mut result = data.content.keys().copied().collect::<Vec<_>>();
        result.sort_by(v_ord);
        assert_eq!(result, coords);
    }
    #[test]
    fn iter_full_chunk() {
        let mut data = TileMapData::default();
        let mut required = FxHashSet::default();
        let mut extra = Vec::default();
        for x in 0..CHUNK_WIDTH as i32 {
            for y in 0..CHUNK_HEIGHT as i32 {
                data.set(v(x, y), h(0, 0, 0, 0));
                required.insert(v(x, y));
            }
        }
        for (result, _) in data.iter() {
            if !required.remove(&result) {
                extra.push(result);
            }
        }
        let required = required.into_iter().collect::<Vec<_>>();
        assert_eq!((required, extra), (vec![], vec![]));
    }
    #[test]
    fn iter() {
        let mut data = TileMapData::default();
        let mut coords = vec![
            (v(0, 0), h(1, 2, 3, 4)),
            (v(-1, -2), h(1, 2, 3, 0)),
            (v(16, 16), h(1, 2, 3, 5)),
            (v(-1, -1), h(1, 2, 3, 6)),
            (v(-17, 0), h(1, 2, 3, 7)),
        ];
        for (pos, handle) in coords.iter() {
            data.set(*pos, *handle);
        }
        let mut result = data.iter().collect::<Vec<_>>();
        result.sort_by(|(a, _), (b, _)| v_ord(a, b));
        coords.sort_by(|(a, _), (b, _)| v_ord(a, b));
        assert_eq!(result, coords);
    }
}
