#![allow(clippy::unnecessary_to_owned)] // false-positive

use crate::core::{
    algebra::Vector2, rectpack::RectPacker, reflect::prelude::*, uuid::Uuid, uuid_provider,
    visitor::prelude::*, TypeUuidProvider,
};
use fxhash::FxHashMap;
use fyrox_resource::untyped::UntypedResource;
use fyrox_resource::{io::ResourceIo, Resource, ResourceData};
use lazy_static::lazy_static;
use std::fmt::Formatter;
use std::{
    any::Any,
    error::Error,
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::Deref,
    path::Path,
};

pub mod loader;

#[derive(Debug)]
pub struct FontGlyph {
    pub top: f32,
    pub left: f32,
    pub advance: f32,
    pub tex_coords: [Vector2<f32>; 4],
    pub bitmap_width: usize,
    pub bitmap_height: usize,
    pub page_index: usize,
}

/// Page is a storage for rasterized glyphs.
pub struct Page {
    pub pixels: Vec<u8>,
    pub texture: Option<UntypedResource>,
    pub rect_packer: RectPacker<usize>,
    pub modified: bool,
}

impl Debug for Page {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Page")
            .field("Pixels", &self.pixels)
            .field("Texture", &self.texture)
            .field("Modified", &self.modified)
            .finish()
    }
}

/// Atlas is a storage for glyphs of a particular size, each atlas could have any number of pages to
/// store the rasterized glyphs.
#[derive(Default, Debug)]
pub struct Atlas {
    pub glyphs: Vec<FontGlyph>,
    pub char_map: FxHashMap<char, usize>,
    pub pages: Vec<Page>,
}

impl Atlas {
    fn glyph(
        &mut self,
        font: &fontdue::Font,
        unicode: char,
        height: FontHeight,
        page_size: usize,
    ) -> Option<&FontGlyph> {
        let border = 2;

        match self.char_map.get(&unicode) {
            Some(glyph_index) => {
                return self.glyphs.get(*glyph_index);
            }
            None => {
                // Char might be missing, because it wasn't requested earlier. Try to find
                // it in the inner font and render/pack it.

                if let Some(char_index) = font.chars().get(&unicode) {
                    let (metrics, glyph_raster) =
                        font.rasterize_indexed(char_index.get(), height.0);

                    // Find a page, that is capable to fit the new character or create a new
                    // page and put the character there.
                    let mut placement_info =
                        self.pages
                            .iter_mut()
                            .enumerate()
                            .find_map(|(page_index, page)| {
                                page.rect_packer
                                    .find_free(metrics.width + border, metrics.height + border)
                                    .map(|bounds| (page_index, bounds))
                            });

                    // No space for the character in any of the existing pages, create a new page.
                    if placement_info.is_none() {
                        let mut page = Page {
                            pixels: vec![0; page_size * page_size],
                            texture: None,
                            rect_packer: RectPacker::new(page_size, page_size),
                            modified: true,
                        };

                        let page_index = self.pages.len();

                        match page
                            .rect_packer
                            .find_free(metrics.width + border, metrics.height + border)
                        {
                            Some(bounds) => {
                                placement_info = Some((page_index, bounds));

                                self.pages.push(page);
                            }
                            None => {
                                // No free space in the given page size (requested glyph is too big).
                                return None;
                            }
                        }
                    }

                    let (page_index, placement_rect) = placement_info?;
                    let page = &mut self.pages[page_index];
                    let glyph_index = self.glyphs.len();

                    // Raise a flag to notify users that the content of the page has changed, and
                    // it should be re-uploaded to GPU (if needed).
                    page.modified = true;

                    let mut glyph = FontGlyph {
                        left: metrics.xmin as f32,
                        top: metrics.ymin as f32,
                        advance: metrics.advance_width,
                        tex_coords: Default::default(),
                        bitmap_width: metrics.width,
                        bitmap_height: metrics.height,
                        page_index,
                    };

                    let k = 1.0 / page_size as f32;

                    let bw = placement_rect.w().saturating_sub(border);
                    let bh = placement_rect.h().saturating_sub(border);
                    let bx = placement_rect.x() + border / 2;
                    let by = placement_rect.y() + border / 2;

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

                    // Copy glyph pixels to the atlas pixels
                    for (src_row, row) in (by..row_end).enumerate() {
                        for (src_col, col) in (bx..col_end).enumerate() {
                            page.pixels[row * page_size + col] =
                                glyph_raster[src_row * bw + src_col];
                        }
                    }

                    self.glyphs.push(glyph);

                    // Map the new glyph to its unicode position.
                    self.char_map.insert(unicode, glyph_index);

                    self.glyphs.get(glyph_index)
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Default, Debug, Reflect, Visit)]
#[reflect(hide_all)]
pub struct Font {
    #[visit(skip)]
    pub inner: Option<fontdue::Font>,
    #[visit(skip)]
    pub atlases: FxHashMap<FontHeight, Atlas>,
    #[visit(skip)]
    pub page_size: usize,
}

uuid_provider!(Font = "692fec79-103a-483c-bb0b-9fc3a349cb48");

impl ResourceData for Font {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        false
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct FontHeight(pub f32);

impl From<f32> for FontHeight {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl PartialEq for FontHeight {
    fn eq(&self, other: &Self) -> bool {
        fyrox_core::value_as_u8_slice(&self.0) == fyrox_core::value_as_u8_slice(&other.0)
    }
}

impl Eq for FontHeight {}

impl Hash for FontHeight {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Don't care about "genius" Rust decision to make f32 non-hashable. If a user is dumb enough
        // to put NaN or any other special value as a glyph height, then it is their choice.
        fyrox_core::hash_as_bytes(&self.0, state)
    }
}

pub type FontResource = Resource<Font>;

lazy_static! {
    pub static ref BUILT_IN_FONT: FontResource = FontResource::new_ok(
        "__BUILT_IN_FONT__".into(),
        Font::from_memory(include_bytes!("./built_in_font.ttf").to_vec(), 1024).unwrap(),
    );
}

impl Font {
    pub fn from_memory(
        data: impl Deref<Target = [u8]>,
        page_size: usize,
    ) -> Result<Self, &'static str> {
        let fontdue_font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())?;
        Ok(Font {
            inner: Some(fontdue_font),
            atlases: Default::default(),
            page_size,
        })
    }

    pub async fn from_file<P: AsRef<Path>>(
        path: P,
        page_size: usize,
        io: &dyn ResourceIo,
    ) -> Result<Self, &'static str> {
        if let Ok(file_content) = io.load_file(path.as_ref()).await {
            Self::from_memory(file_content, page_size)
        } else {
            Err("Unable to read file")
        }
    }

    /// Tries to get a glyph at the given unicode position of the given height. If there's no rendered
    /// glyph, this method tries to render the glyph and put into a suitable atlas (see [`Atlas`] docs
    /// for more info). If the given unicode position has no representation in the font, [`None`] will
    /// be returned. If the requested size of the glyph is too big to fit into the page size of the
    /// font, [`None`] will be returned. Keep in mind, that this method is free to create as many atlases
    /// with any number of pages in them. Each atlas corresponds to a particular glyph size, each glyph
    /// in the atlas could be rendered at any page in the atlas.
    #[inline]
    pub fn glyph(&mut self, unicode: char, height: f32) -> Option<&FontGlyph> {
        self.atlases
            .entry(FontHeight(height))
            .or_insert_with(|| Atlas {
                glyphs: Default::default(),
                char_map: Default::default(),
                pages: Default::default(),
            })
            .glyph(
                self.inner
                    .as_ref()
                    .expect("Font reader must be initialized!"),
                unicode,
                FontHeight(height),
                self.page_size,
            )
    }

    #[inline]
    pub fn ascender(&self, height: f32) -> f32 {
        self.inner
            .as_ref()
            .unwrap()
            .horizontal_line_metrics(height)
            .map(|m| m.ascent)
            .unwrap_or_default()
    }

    #[inline]
    pub fn descender(&self, height: f32) -> f32 {
        self.inner
            .as_ref()
            .unwrap()
            .horizontal_line_metrics(height)
            .map(|m| m.descent)
            .unwrap_or_default()
    }

    #[inline]
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    #[inline]
    pub fn glyph_advance(&mut self, unicode: char, height: f32) -> f32 {
        self.glyph(unicode, height)
            .map_or(height, |glyph| glyph.advance)
    }
}

/// Font builder allows you to load fonts in declarative manner.
pub struct FontBuilder {
    page_size: usize,
}

impl FontBuilder {
    /// Creates a default FontBuilder.
    pub fn new() -> Self {
        Self { page_size: 1024 }
    }

    /// Creates a new font from the data at the specified path.
    pub async fn build_from_file(
        self,
        path: impl AsRef<Path>,
        io: &dyn ResourceIo,
    ) -> Result<Font, &'static str> {
        Font::from_file(path, self.page_size, io).await
    }

    /// Creates a new font from bytes in memory.
    pub fn build_from_memory(self, data: impl Deref<Target = [u8]>) -> Result<Font, &'static str> {
        Font::from_memory(data, self.page_size)
    }
}
