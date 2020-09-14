use crate::{
    core::{math::vec2::Vec2, rectpack::RectPacker},
    draw::SharedTexture,
};
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    fs::File,
    io::Read,
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
    pub width: f32,
    pub height: f32,
    pub advance: f32,
    pub tex_coords: [Vec2; 4],
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
    atlas_size: i32,
    pub texture: Option<SharedTexture>,
}

#[derive(Debug, Clone)]
pub struct SharedFont(pub Arc<Mutex<Font>>);

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
        &[0x0020..0x00FF] // Basic Latin + Latin Supplement
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
                        left: metrics.bounds.xmin,
                        top: metrics.bounds.ymin,
                        width: metrics.bounds.xmax - metrics.bounds.xmin,
                        height: metrics.bounds.ymax - metrics.bounds.ymin,
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

    pub fn from_file<P: AsRef<Path>>(
        path: P,
        height: f32,
        char_set: &[Range<u32>],
    ) -> Result<Self, &'static str> {
        if let Ok(ref mut file) = File::open(path) {
            let mut file_content: Vec<u8> =
                Vec::with_capacity(file.metadata().unwrap().len() as usize);
            file.read_to_end(&mut file_content).unwrap();

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
    pub fn atlas_size(&self) -> i32 {
        self.atlas_size
    }

    #[inline]
    pub fn glyph_advance(&self, c: u32) -> f32 {
        self.glyph(c).map_or(self.height(), |glyph| glyph.advance)
    }

    #[inline]
    fn compute_atlas_size(&self, border: i32) -> i32 {
        let mut area = 0.0;
        for glyph in self.glyphs.iter() {
            area += (glyph.bitmap_width + border as usize) as f32
                * (glyph.bitmap_height + border as usize) as f32;
        }
        (1.15 * area.sqrt()) as i32
    }

    fn pack(&mut self) {
        let border = 2;
        self.atlas_size = self.compute_atlas_size(border);
        self.atlas = vec![0; (self.atlas_size * self.atlas_size) as usize];
        let k = 1.0 / self.atlas_size as f32;
        let mut rect_packer = RectPacker::new(self.atlas_size, self.atlas_size);
        for glyph in self.glyphs.iter_mut() {
            if let Some(bounds) = rect_packer.find_free(
                glyph.bitmap_width as i32 + border,
                glyph.bitmap_height as i32 + border,
            ) {
                let bw = bounds.w - border;
                let bh = bounds.h - border;
                let bx = bounds.x + border / 2;
                let by = bounds.y + border / 2;

                let tw = bw as f32 * k;
                let th = bh as f32 * k;
                let tx = bx as f32 * k;
                let ty = by as f32 * k;

                glyph.tex_coords[0] = Vec2 { x: tx, y: ty };
                glyph.tex_coords[1] = Vec2 { x: tx + tw, y: ty };
                glyph.tex_coords[2] = Vec2 {
                    x: tx + tw,
                    y: ty + th,
                };
                glyph.tex_coords[3] = Vec2 { x: tx, y: ty + th };

                let row_end = by + bh;
                let col_end = bx + bw;

                // Copy glyph pixels to atlas pixels
                let mut src_row = 0;
                for row in by..row_end {
                    let mut src_col = 0;
                    for col in bx..col_end {
                        self.atlas[(row * self.atlas_size + col) as usize] =
                            glyph.pixels[(src_row * bw + src_col) as usize];
                        src_col += 1;
                    }
                    src_row += 1;
                }
            } else {
                println!("Insufficient atlas size!");
            }
        }
    }
}
