use crate::core::algebra::Vector2;
use crate::core::io;
use crate::{core::rectpack::RectPacker, draw::SharedTexture};
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    ops::{Deref, Range},
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Copy, Clone, Debug)]
struct Point {
    x: f32,
    y: f32,
    flags: u8,
}

#[derive(Debug)]
struct Polygon {
    points: Vec<Point>,
}

#[derive(Debug)]
pub struct FontGlyph {
    pub top: f32,
    pub left: f32,
    pub advance: f32,
    pub tex_coords: [Vector2<f32>; 4],
    pub bitmap_width: usize,
    pub bitmap_height: usize,
    pub pixels: Vec<u8>,
}

pub struct Font {
    height: f32,
    glyphs: Vec<FontGlyph>,
    ascender: f32,
    descender: f32,
    char_map: HashMap<u32, usize>,
    atlas: Vec<u8>,
    atlas_size: usize,
    pub texture: Option<SharedTexture>,
}

#[derive(Debug, Clone)]
pub struct SharedFont(pub Arc<Mutex<Font>>);

impl SharedFont {
    pub fn new(font: Font) -> Self {
        Self(Arc::new(Mutex::new(font)))
    }
}

impl From<Arc<Mutex<Font>>> for SharedFont {
    fn from(arc: Arc<Mutex<Font>>) -> Self {
        SharedFont(arc)
    }
}

impl PartialEq for SharedFont {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.deref(), other.0.deref())
    }
}

impl Debug for Font {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Font")
    }
}

impl Font {
    pub fn default_char_set() -> &'static [Range<u32>] {
        &[
            // Basic Latin + Latin Supplement
            0x0020..0x00FF,
        ]
    }

    pub fn korean_char_set() -> &'static [Range<u32>] {
        &[
            // Basic Latin + Latin Supplement
            0x0020..0x00FF,
            // Korean alphabets
            0x3131..0x3163,
            // Korean characters
            0xAC00..0xD7A3,
            // Invalid
            0xFFFD..0xFFFD,
        ]
    }

    pub fn chinese_full_char_set() -> &'static [Range<u32>] {
        &[
            // Basic Latin + Latin Supplement
            0x0020..0x00FF,
            // General Punctuation
            0x2000..0x206F,
            // CJK Symbols and Punctuations, Hiragana, Katakana
            0x3000..0x30FF,
            // Katakana Phonetic Extensions
            0x31F0..0x31FF,
            // Half-width characters
            0xFF00..0xFFEF,
            // Invalid
            0xFFFD..0xFFFD,
            // CJK Ideograms
            0x4e00..0x9FAF,
        ]
    }

    pub fn cyrillic_char_set() -> &'static [Range<u32>] {
        &[
            // Basic Latin + Latin Supplement
            0x0020..0x00FF,
            // Cyrillic + Cyrillic Supplement
            0x0400..0x052F,
            // Cyrillic Extended-A
            0x2DE0..0x2DFF,
            // Cyrillic Extended-B
            0xA640..0xA69F,
        ]
    }

    pub fn thai_char_set() -> &'static [Range<u32>] {
        &[
            // Basic Latin + Latin Supplement
            0x0020..0x00FF,
            // Punctuations
            0x2010..0x205E,
            // Thai
            0x0E00..0x0E7F,
        ]
    }

    pub fn vietnamese_char_set() -> &'static [Range<u32>] {
        &[
            // Basic Latin + Latin Supplement
            0x0020..0x00FF,
            // Vietnamese
            0x0102..0x0103,
            0x0110..0x0111,
            0x0128..0x0129,
            0x0168..0x0169,
            0x01A0..0x01A1,
            0x01AF..0x01B0,
            0x1EA0..0x1EF9,
        ]
    }

    pub fn from_memory(
        data: Vec<u8>,
        height: f32,
        char_set: &[Range<u32>],
    ) -> Result<Self, &'static str> {
        let fontdue_font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())?;
        let font_metrics = fontdue_font.horizontal_line_metrics(height).unwrap();

        let mut font = Font {
            height,
            glyphs: Vec::new(),
            ascender: font_metrics.ascent,
            descender: font_metrics.descent,
            char_map: HashMap::new(),
            atlas: Vec::new(),
            atlas_size: 0,
            texture: None,
        };

        let mut index = 0;
        for range in char_set {
            for unicode in range.start..range.end {
                if let Some(character) = std::char::from_u32(unicode) {
                    let (metrics, bitmap) = fontdue_font.rasterize(character, height);

                    font.glyphs.push(FontGlyph {
                        left: metrics.xmin as f32,
                        top: metrics.ymin as f32,
                        pixels: bitmap,
                        advance: metrics.advance_width,
                        tex_coords: Default::default(),
                        bitmap_width: metrics.width,
                        bitmap_height: metrics.height,
                    });

                    font.char_map.insert(unicode, index);
                    index += 1;
                }
            }
        }

        font.pack();

        Ok(font)
    }

    pub async fn from_file<P: AsRef<Path>>(
        path: P,
        height: f32,
        char_set: &[Range<u32>],
    ) -> Result<Self, &'static str> {
        if let Ok(file_content) = io::load_file(path).await {
            Self::from_memory(file_content, height, char_set)
        } else {
            Err("Unable to read file")
        }
    }

    #[inline]
    pub fn glyph(&self, unicode: u32) -> Option<&FontGlyph> {
        match self.char_map.get(&unicode) {
            Some(glyph_index) => self.glyphs.get(*glyph_index),
            None => None,
        }
    }

    #[inline]
    pub fn glyph_index(&self, unicode: u32) -> Option<usize> {
        self.char_map.get(&unicode).cloned()
    }

    #[inline]
    pub fn glyphs(&self) -> &[FontGlyph] {
        &self.glyphs
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.height
    }

    #[inline]
    pub fn ascender(&self) -> f32 {
        self.ascender
    }

    #[inline]
    pub fn descender(&self) -> f32 {
        self.descender
    }

    #[inline]
    pub fn atlas_pixels(&self) -> &[u8] {
        self.atlas.as_slice()
    }

    #[inline]
    pub fn atlas_size(&self) -> usize {
        self.atlas_size
    }

    #[inline]
    pub fn glyph_advance(&self, c: u32) -> f32 {
        self.glyph(c).map_or(self.height(), |glyph| glyph.advance)
    }

    #[inline]
    fn compute_atlas_size(&self, border: usize) -> usize {
        let mut area = 0.0;
        for glyph in self.glyphs.iter() {
            area += (glyph.bitmap_width + border) as f32 * (glyph.bitmap_height + border) as f32;
        }
        (1.3 * area.sqrt()) as usize
    }

    fn pack(&mut self) {
        let border = 2;
        self.atlas_size = self.compute_atlas_size(border);
        self.atlas = vec![0; (self.atlas_size * self.atlas_size) as usize];
        let k = 1.0 / self.atlas_size as f32;
        let mut rect_packer = RectPacker::new(self.atlas_size, self.atlas_size);
        for glyph in self.glyphs.iter_mut() {
            if let Some(bounds) =
                rect_packer.find_free(glyph.bitmap_width + border, glyph.bitmap_height + border)
            {
                let bw = (bounds.w() - border) as usize;
                let bh = (bounds.h() - border) as usize;
                let bx = (bounds.x() + border / 2) as usize;
                let by = (bounds.y() + border / 2) as usize;

                let tw = bw as f32 * k;
                let th = bh as f32 * k;
                let tx = bx as f32 * k;
                let ty = by as f32 * k;

                glyph.tex_coords[0] = Vector2::new(tx, ty);
                glyph.tex_coords[1] = Vector2::new(tx + tw, ty);
                glyph.tex_coords[2] = Vector2::new(tx + tw, ty + th);
                glyph.tex_coords[3] = Vector2::new(tx, ty + th);

                let row_end = by + bh;
                let col_end = bx + bw;

                // Copy glyph pixels to atlas pixels
                for (src_row, row) in (by..row_end).enumerate() {
                    for (src_col, col) in (bx..col_end).enumerate() {
                        self.atlas[row * self.atlas_size + col] =
                            glyph.pixels[src_row * bw + src_col];
                    }
                }
            } else {
                println!("Insufficient atlas size!");
            }
        }
    }
}
