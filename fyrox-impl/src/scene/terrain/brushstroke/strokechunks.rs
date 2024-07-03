//! This module manages the record of which pixels have been recently edited by a brushstroke.
//! It stores the modified chunks and the pixels within each chunk since the last time
//! the changes were written to the terrain's textures.
use super::{ChunkData, StrokeData, TerrainTextureKind};
use crate::core::algebra::Vector2;
use crate::fxhash::{FxHashMap, FxHashSet};
use crate::resource::texture::TextureResource;
use crate::scene::terrain::pixel_position_to_grid_position;

/// The list of modified pixels in each chunk.
#[derive(Debug, Default)]
pub struct StrokeChunks {
    /// The size of each chunk as measured by distance from one chunk origin to the next.
    /// This does not include the overlap pixel around the edges of height textures,
    /// because that overlap does not contribute to the distance between the origins of the textuers.
    chunk_size: Vector2<u32>,
    kind: TerrainTextureKind,
    /// The position of each written pixel within each chunk.
    written_pixels: FxHashMap<Vector2<i32>, FxHashSet<Vector2<u32>>>,
    /// The number of pixels written to this object.
    count: usize,
    /// Pixel hash sets that are allocated but not currently in use
    unused_chunks: Vec<FxHashSet<Vector2<u32>>>,
}

impl StrokeChunks {
    /// The number of modified pixels that this object is currently tracking
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }
    /// The kind of texture being edited
    #[inline]
    pub fn kind(&self) -> TerrainTextureKind {
        self.kind
    }
    /// Erase the currently stored pixel data and prepare for a new set of pixels.
    pub fn clear(&mut self) {
        self.count = 0;
        // Move and clear the no-longer needed chunk pixel sets into the unused list.
        for mut c in self.written_pixels.drain().map(|(_, v)| v) {
            c.clear();
            self.unused_chunks.push(c);
        }
    }
    /// Update the texture kind and the texture size.
    pub fn set_layout(&mut self, kind: TerrainTextureKind, size: Vector2<u32>) {
        self.kind = kind;
        // If the texture is a height texture, then its edges overlap with the neigboring chunks,
        // so the size we need is one less than the actual size in each dimension.
        self.chunk_size = match kind {
            TerrainTextureKind::Height => size.map(|x| x - 1),
            TerrainTextureKind::Mask => size,
        };
    }
    /// For every chunk that has been written in this object since the last clear, copy the texture data from the given textures
    /// and store it in the given list of texture data, if the list does not already contain data for those coordinates.
    ///
    /// The purpose of this is to save a backup copy of chunks that are modified by the current brushstroke so that an undo command
    /// can be created. This should be called immediately before [StrokeChunks::apply] so that the copied textures are unmodified.
    /// If saved chunk data already exists for some chunk, nothing is done since it is presumed that existing data is the original
    /// data that we are trying to preserve.
    ///
    /// - `textures`: The source of texture data.
    /// - `saved_chunk_data`: The list of chunk data that may be modified if it does not already contain a copy of chunk data for each
    /// written chunk coordinates.
    pub fn copy_texture_data(
        &self,
        textures: &FxHashMap<Vector2<i32>, TextureResource>,
        saved_chunk_data: &mut Vec<ChunkData>,
    ) {
        for (c, _) in self.written_pixels.iter() {
            if saved_chunk_data.iter().any(|x| x.grid_position == *c) {
                continue;
            }
            let Some(texture) = textures.get(c) else {
                continue;
            };
            saved_chunk_data.push(ChunkData::from_texture(*c, texture));
        }
    }
    /// Use the pixels stored in this object to modify the given textures with
    /// pixel data from the given StrokeData.
    /// Once the textures have been modified using this method [StrokeChunks::clear]
    /// should be called, since the data in this object has served its purpose.
    pub fn apply<V>(
        &self,
        stroke: &StrokeData<V>,
        textures: &FxHashMap<Vector2<i32>, TextureResource>,
    ) where
        V: Clone,
    {
        for (c, pxs) in self.written_pixels.iter() {
            let Some(texture) = textures.get(c) else {
                continue;
            };
            let mut texture_data = texture.data_ref();
            let mut modify = texture_data.modify();
            let Some(data) = modify.data_mut_of_type::<V>() else {
                continue;
            };
            let origin = self.chunk_to_origin(*c);
            let row_size = self.row_size();
            for p in pxs.iter() {
                let Some(value) = stroke.latest_pixel_value(origin + p.map(|x| x as i32)) else {
                    continue;
                };
                let index = p.x as usize + p.y as usize * row_size;
                data[index].clone_from(value);
            }
        }
    }
    /// Calculates which chunk contains the given pixel position.
    #[inline]
    pub fn pixel_position_to_grid_position(&self, position: Vector2<i32>) -> Vector2<i32> {
        pixel_position_to_grid_position(position, self.chunk_size)
    }
    /// Calculates the origin pixel position of the given chunk.
    pub fn chunk_to_origin(&self, grid_position: Vector2<i32>) -> Vector2<i32> {
        Vector2::new(
            grid_position.x * self.chunk_size.x as i32,
            grid_position.y * self.chunk_size.y as i32,
        )
    }
    /// The width of the texture in pixels.
    pub fn row_size(&self) -> usize {
        match self.kind {
            TerrainTextureKind::Height => (self.chunk_size.x + 1) as usize,
            TerrainTextureKind::Mask => self.chunk_size.x as usize,
        }
    }
    /// Calculate the index of a pixel at the given position within texture data,
    /// based on the row size. The given position is relative to the origin of the texture
    /// and must be within the bounds of the texture.
    pub fn pixel_index(&self, position: Vector2<i32>) -> usize {
        if position.x < 0
            || position.x >= self.chunk_size.x as i32
            || position.y < 0
            || position.y >= self.chunk_size.y as i32
        {
            panic!(
                "Invalid pixel position: ({}, {}) within ({}, {})",
                position.x, position.y, self.chunk_size.x, self.chunk_size.y
            );
        }
        let p = position.map(|x| x as usize);
        p.x + p.y * self.row_size()
    }
    /// Insert the the pixel at the given position into this data.
    /// This method determines which chunks have a pixel at that position
    /// and marks each of those chunks as being modified.
    pub fn write(&mut self, position: Vector2<i32>) {
        let grid_pos = self.pixel_position_to_grid_position(position);
        let origin = self.chunk_to_origin(grid_pos);
        let pos = (position - origin).map(|x| x as u32);
        self.count += 1;
        self.write_to_chunk(grid_pos, pos);
        if self.kind == TerrainTextureKind::Height {
            if pos.x == 0 {
                self.write_to_chunk(
                    Vector2::new(grid_pos.x - 1, grid_pos.y),
                    Vector2::new(self.chunk_size.x, pos.y),
                );
            }
            if pos.y == 0 {
                self.write_to_chunk(
                    Vector2::new(grid_pos.x, grid_pos.y - 1),
                    Vector2::new(pos.x, self.chunk_size.y),
                );
            }
            if pos.x == 0 && pos.y == 0 {
                self.write_to_chunk(
                    Vector2::new(grid_pos.x - 1, grid_pos.y - 1),
                    self.chunk_size,
                );
            }
        }
    }
    fn write_to_chunk(&mut self, grid_pos: Vector2<i32>, position: Vector2<u32>) {
        let mut unused = std::mem::take(&mut self.unused_chunks);
        self.written_pixels
            .entry(grid_pos)
            .or_insert_with(|| unused.pop().unwrap_or_default())
            .insert(position);
        self.unused_chunks = unused;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const RANDOM_POINTS: &[(i32, i32)] = &[
        (0, 0),
        (1, 1),
        (2, 2),
        (-1, -1),
        (20, -123),
        (-11, 22),
        (42, 285),
        (360, -180),
        (123, -456),
        (54, 32),
        (-2, -3),
    ];
    #[test]
    fn chunk_to_origin() {
        let mut chunks = StrokeChunks::default();
        chunks.set_layout(TerrainTextureKind::Height, Vector2::new(5, 5));
        assert_eq!(
            chunks.chunk_to_origin(Vector2::new(0, 0)),
            Vector2::new(0, 0)
        );
        assert_eq!(
            chunks.chunk_to_origin(Vector2::new(1, 0)),
            Vector2::new(4, 0)
        );
        assert_eq!(
            chunks.chunk_to_origin(Vector2::new(-2, -1)),
            Vector2::new(-8, -4)
        );
    }
    #[test]
    fn pixel_position_to_grid_position() {
        let mut chunks = StrokeChunks::default();
        chunks.set_layout(TerrainTextureKind::Height, Vector2::new(5, 5));
        assert_eq!(
            chunks.pixel_position_to_grid_position(Vector2::new(0, 0)),
            Vector2::new(0, 0)
        );
        assert_eq!(
            chunks.pixel_position_to_grid_position(Vector2::new(2, 3)),
            Vector2::new(0, 0)
        );
        assert_eq!(
            chunks.pixel_position_to_grid_position(Vector2::new(-1, -1)),
            Vector2::new(-1, -1)
        );
        assert_eq!(
            chunks.pixel_position_to_grid_position(Vector2::new(4, -4)),
            Vector2::new(1, -1)
        );
    }
    fn test_points_height(size: Vector2<u32>) {
        let mut chunks = StrokeChunks::default();
        chunks.set_layout(TerrainTextureKind::Height, size);
        for p in RANDOM_POINTS.iter() {
            let p = Vector2::new(p.0, p.1);
            let grid_pos = chunks.pixel_position_to_grid_position(p);
            let origin = chunks.chunk_to_origin(grid_pos);
            let pixel = p - origin;
            test_point(p, pixel, size.map(|x| x - 1));
        }
        let s = size.map(|x| x as i32);
        for x in -s.x..=s.x * 2 {
            for y in -s.y..=s.y * 2 {
                let p = Vector2::new(x, y);
                let grid_pos = chunks.pixel_position_to_grid_position(p);
                let origin = chunks.chunk_to_origin(grid_pos);
                let pixel = p - origin;
                test_point(p, pixel, size.map(|x| x - 1));
            }
        }
    }
    fn test_points_mask(size: Vector2<u32>) {
        let mut chunks = StrokeChunks::default();
        chunks.set_layout(TerrainTextureKind::Mask, size);
        for p in RANDOM_POINTS.iter() {
            let p = Vector2::new(p.0, p.1);
            let grid_pos = chunks.pixel_position_to_grid_position(p);
            let origin = chunks.chunk_to_origin(grid_pos);
            let pixel = p - origin;
            test_point(p, pixel, size);
        }
        let s = size.map(|x| x as i32);
        for x in -s.x..=s.x * 2 {
            for y in -s.y..=s.y * 2 {
                let p = Vector2::new(x, y);
                let grid_pos = chunks.pixel_position_to_grid_position(p);
                let origin = chunks.chunk_to_origin(grid_pos);
                let pixel = p - origin;
                test_point(p, pixel, size);
            }
        }
    }
    fn test_point(p: Vector2<i32>, pixel: Vector2<i32>, size: Vector2<u32>) {
        assert!(
            pixel.x >= 0,
            "({}, {}) -> ({}, {})",
            p.x,
            p.y,
            pixel.x,
            pixel.y
        );
        assert!(
            pixel.y >= 0,
            "({}, {}) -> ({}, {})",
            p.x,
            p.y,
            pixel.x,
            pixel.y
        );
        assert!(
            pixel.x < size.x as i32,
            "({}, {}) -> ({}, {})",
            p.x,
            p.y,
            pixel.x,
            pixel.y
        );
        assert!(
            pixel.y < size.y as i32,
            "({}, {}) -> ({}, {})",
            p.x,
            p.y,
            pixel.x,
            pixel.y
        );
    }
    #[test]
    fn random_points_5x5() {
        test_points_height(Vector2::new(5, 5));
        test_points_mask(Vector2::new(5, 5));
    }
    #[test]
    fn random_points_10x10() {
        test_points_height(Vector2::new(10, 10));
        test_points_mask(Vector2::new(10, 10));
    }
    #[test]
    fn random_points_257x257() {
        test_points_height(Vector2::new(257, 257));
        test_points_mask(Vector2::new(257, 257));
    }
}
