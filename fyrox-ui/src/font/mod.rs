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

//! A font resource allows [`FormattedText`](crate::formatted_text::FormattedText)
//! to render text as a series of glyphs taken from a font file such as a ttf file
//! or an otf file.

#![allow(clippy::unnecessary_to_owned)] // false-positive

use crate::core::{
    algebra::Vector2, rectpack::RectPacker, reflect::prelude::*, uuid::Uuid, uuid_provider,
    visitor::prelude::*, TypeUuidProvider,
};
use crate::font::loader::FontImportOptions;
use fxhash::FxHashMap;
use fyrox_core::math::Rect;
use fyrox_core::{err, uuid};
use fyrox_resource::manager::ResourceManager;
use fyrox_resource::state::LoadError;
use fyrox_resource::untyped::ResourceKind;
use fyrox_resource::{
    embedded_data_source, io::ResourceIo, manager::BuiltInResource, untyped::UntypedResource,
    Resource, ResourceData,
};
use lazy_static::lazy_static;
use std::{
    error::Error,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
    path::Path,
};

pub mod loader;

/// Arbitrarily chosen limit to the number of levels of recursion,
/// we will search through fallbacks. In most cases, a limit of 1 should
/// be sufficient, and if we get to 10, that most likely indicates
/// a cycle in the fallback fonts.
const MAX_FALLBACK_DEPTH: usize = 10;

enum FontError {
    FallbackNotLoaded,
    GlyphTooLarge,
}

/// The geometric data specifying where to find a glyph on a font atlas
/// texture for rendering text.
#[derive(Debug, Clone)]
pub struct FontGlyph {
    /// The vertical position of the glyph relative to other glyphs on the line, measured in font pixels.
    /// This would be 0 for a glyph with its bottom directly on the baseline, but may be
    /// negative if the glyph extends below the baseline.
    pub bitmap_top: f32,
    /// The horizontal position of the glyph relative to other glyphs on the line, measured in font pixels.
    /// This would usually be 0, but a negative value would allow a glyph to extend into the space usually reserved
    /// for the previous glyph on the line.
    pub bitmap_left: f32,
    /// The width of the glyph as measured in pixels. This may be more or less than `advance` depending on how much
    /// this glyph crowds into the space of the glyphs to the left and right of it on the line.
    pub bitmap_width: f32,
    /// The height of the glyph as measured in pixels.
    pub bitmap_height: f32,
    /// The horizontal distance between the start of this glyph and the start of the next glyph, measured in pixels.
    pub advance: f32,
    /// The position of the texture data of this glyph on the atlas page.
    /// Each corner of the quad containing the glyph is given a UV coordinate.
    pub tex_coords: [Vector2<f32>; 4],
    /// The index of the atlas page.
    pub page_index: usize,
    /// The position of the glyph measured to sub-pixel precision.
    /// It is like `bitmap_top`, `bitmap_left`, `bitmap_width`, and `bitmap_height`,
    /// except that those measure the whole pixels that the font needs for rendering while
    /// this rect contains the exact outline of the glyph, which is potentially smaller.
    pub bounds: Rect<f32>,
}

/// Page is a storage for rasterized glyphs.
#[derive(Clone)]
pub struct Page {
    /// The texture data for rendering some glyphs.
    /// When new glyphs are required, this data may be modified if space
    /// is available.
    pub pixels: Vec<u8>,
    /// An embedded texture containing a copy of `pixels`.
    /// This is used by the GPU to render the glyph, and it must be updated
    /// whenever `pixels` changes.
    pub texture: Option<UntypedResource>,
    /// A structure that contains the occupied rectangles within the page texture,
    /// allowing new glyphs to be added to the texture by finding unoccupied space,
    /// until the page is full.
    pub rect_packer: RectPacker<usize>,
    /// True if one or more glyphs have been added to the page, but
    /// `texture` has not yet been updated by copying the content of `pixels`.
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
#[derive(Default, Clone, Debug)]
pub struct Atlas {
    /// The geometric data used for rendering glyphs from the pages of this atlas,
    /// such as the size of each glyph, the index of its page, and the UVs of the corners
    /// of its quad.
    pub glyphs: Vec<FontGlyph>,
    /// A map to look up the index of the glyph for a particular char in the `glyphs` array.
    pub char_map: FxHashMap<char, usize>,
    /// The list of pages the contain the glyphs of this atlas. Each page is a texture
    /// that can be used to render glyphs based on the UV data stored in `glyphs`.
    pub pages: Vec<Page>,
}

impl Atlas {
    fn render_glyph(
        &mut self,
        font: &'_ fontdue::Font,
        unicode: char,
        char_index: u16,
        height: FontHeight,
        page_size: usize,
    ) -> Result<usize, FontError> {
        let border = 2;
        let (metrics, glyph_raster) = font.rasterize_indexed(char_index, height.0);

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

        // No space for the character in any of the existing pages, so create a new page.
        if placement_info.is_none() {
            let mut page = Page {
                pixels: vec![0; page_size * page_size],
                texture: None,
                rect_packer: RectPacker::new(page_size, page_size),
                modified: true,
            };

            let page_index = self.pages.len();

            if let Some(bounds) = page
                .rect_packer
                .find_free(metrics.width + border, metrics.height + border)
            {
                placement_info = Some((page_index, bounds));

                self.pages.push(page);
            }
        }

        let Some((page_index, placement_rect)) = placement_info else {
            err!(
                "Font error: The atlas page size is too small for a requested glyph at font size {}.\
            Glyph width: {}, height: {}, atlas page size: {}",
                height.0,
                metrics.width + border,
                metrics.height + border,
                page_size,
            );
            return Err(FontError::GlyphTooLarge);
        };
        let page = &mut self.pages[page_index];
        let glyph_index = self.glyphs.len();

        // Raise a flag to notify users that the content of the page has changed, and
        // it should be re-uploaded to GPU (if needed).
        page.modified = true;

        let mut glyph = FontGlyph {
            bitmap_left: metrics.xmin as f32,
            bitmap_top: metrics.ymin as f32,
            advance: metrics.advance_width,
            tex_coords: Default::default(),
            bitmap_width: metrics.width as f32,
            bitmap_height: metrics.height as f32,
            bounds: Rect::new(
                metrics.bounds.xmin,
                metrics.bounds.ymin,
                metrics.bounds.width,
                metrics.bounds.height,
            ),
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
                page.pixels[row * page_size + col] = glyph_raster[src_row * bw + src_col];
            }
        }

        self.glyphs.push(glyph);

        // Map the new glyph to its Unicode position.
        self.char_map.insert(unicode, glyph_index);

        Ok(glyph_index)
    }
    fn glyph(
        &mut self,
        font: &fontdue::Font,
        unicode: char,
        height: FontHeight,
        page_size: usize,
        fallbacks: &[Option<FontResource>],
    ) -> Option<&FontGlyph> {
        match self.char_map.get(&unicode) {
            Some(glyph_index) => self.glyphs.get(*glyph_index),
            None => {
                // Char might be missing because it wasn't requested earlier. Try to find
                // it in the inner font and render/pack it.
                let glyph_index = if let Some(char_index) = font.chars().get(&unicode) {
                    self.render_glyph(font, unicode, char_index.get(), height, page_size)
                        .ok()
                } else {
                    // Otherwise, search the fallback fonts for a glyph to add to the atlas.
                    match self.fallback_glyph(
                        MAX_FALLBACK_DEPTH,
                        fallbacks,
                        unicode,
                        height,
                        page_size,
                    ) {
                        Ok(Some(glyph_index)) => Some(glyph_index),
                        Ok(None) | Err(FontError::GlyphTooLarge) => {
                            // We have failed to find the character in the inner font and the fallbacks.
                            // Every font's default character is supposed to be at index 0, so add that to the atlas
                            // in the place of the character.
                            self.render_glyph(font, unicode, 0, height, page_size).ok()
                        }
                        Err(FontError::FallbackNotLoaded) => {
                            // If a fallback is not loaded successfully, do not write anything to the
                            // atlas and hope that the fallbacks will be ready next time.
                            None
                        }
                    }
                };
                glyph_index.and_then(|i| self.glyphs.get(i))
            }
        }
    }
    /// Attempt to render and return the index of the given char using the fallback fonts.
    /// Return the index if the glyph was found and rendered using a fallback font.
    /// Return None if the glyph was not found in any fallback font.
    fn fallback_glyph(
        &mut self,
        depth: usize,
        fonts: &[Option<FontResource>],
        unicode: char,
        height: FontHeight,
        page_size: usize,
    ) -> Result<Option<usize>, FontError> {
        let Some(depth) = depth.checked_sub(1) else {
            return Ok(None);
        };
        for font in fonts.iter().flatten() {
            if !font.is_ok() {
                return Err(FontError::FallbackNotLoaded);
            }
            let font = font.data_ref();
            let inner = font
                .inner
                .as_ref()
                .expect("Fallback font reader must be initialized!");
            if let Some(char_index) = inner.chars().get(&unicode) {
                return self
                    .render_glyph(inner, unicode, char_index.get(), height, page_size)
                    .map(Some);
            } else if let Some(glyph_index) =
                self.fallback_glyph(depth, &font.fallbacks, unicode, height, page_size)?
            {
                return Ok(Some(glyph_index));
            }
        }
        Ok(None)
    }
}

/// A font resource and the associated data required for rendering glyphs from the font.
#[derive(Default, Clone, Debug, Reflect, Visit)]
pub struct Font {
    /// The source font data, such as might come from a ttf file.
    #[reflect(hidden)]
    #[visit(skip)]
    pub inner: Option<fontdue::Font>,
    /// The atlases containing textures that allow the GPU to render glyphs from this font,
    /// or from its fallbacks. Each font size gets its own atlas.
    #[reflect(hidden)]
    #[visit(skip)]
    pub atlases: FxHashMap<FontHeight, Atlas>,
    /// The size of each atlas page in font pixels. Each page is a square measuring `page_size` x `page_size`.
    #[reflect(hidden)]
    #[visit(skip)]
    pub page_size: usize,
    /// A font representing the bold version of this font.
    #[visit(skip)]
    pub bold: Option<FontResource>,
    /// A font representing the italic version of this font.
    #[visit(skip)]
    pub italic: Option<FontResource>,
    /// A font representing the bold italic version of this font.
    #[visit(skip)]
    pub bold_italic: Option<FontResource>,
    /// Fallback fonts are used for rendering special characters that do not have glyphs in this
    /// font.
    #[visit(skip)]
    pub fallbacks: Vec<Option<FontResource>>,
}

uuid_provider!(Font = "692fec79-103a-483c-bb0b-9fc3a349cb48");

impl ResourceData for Font {
    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        false
    }

    fn try_clone_box(&self) -> Option<Box<dyn ResourceData>> {
        Some(Box::new(self.clone()))
    }
}

/// The size the text that a font is supposed to render.
/// Each distinct size gets its own atlas page, and `FontHeight`
/// allows a f32 to be hashed to look up the page.
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

/// A resource that allows a font to be loaded.
pub type FontResource = Resource<Font>;

lazy_static! {
    /// Fyrox's default build-in font for rendering bold italic text when no other font is specified.
    pub static ref BOLD_ITALIC: BuiltInResource<Font> = BuiltInResource::new(
        "__BOLD_ITALIC__",
        embedded_data_source!("./bold_italic.ttf"),
        |data| {
            FontResource::new_ok(
                uuid!("f5b02124-9601-452a-9368-3fa2a9703ecd"),
                ResourceKind::External,
                Font::from_memory(data.to_vec(), 1024, FontStyles::default(), Vec::default()).unwrap(),
            )
        }
    );
    /// Fyrox's default build-in font for rendering italic text when no other font is specified.
    pub static ref BUILT_IN_ITALIC: BuiltInResource<Font> = BuiltInResource::new(
        "__BUILT_IN_ITALIC__",
        embedded_data_source!("./built_in_italic.ttf"),
        |data| {
            let bold = Some(BOLD_ITALIC.resource());
            let styles = FontStyles{bold, ..FontStyles::default()};
            FontResource::new_ok(
                uuid!("1cd79487-6c76-4370-91c2-e6e1e728950a"),
                ResourceKind::External,
                Font::from_memory(data.to_vec(), 1024, styles, Vec::default()).unwrap(),
            )
        }
    );
    /// Fyrox's default build-in font for rendering bold text when no other font is specified.
    pub static ref BUILT_IN_BOLD: BuiltInResource<Font> = BuiltInResource::new(
        "__BUILT_IN_BOLD__",
        embedded_data_source!("./built_in_bold.ttf"),
        |data| {
            let italic = Some(BOLD_ITALIC.resource());
            let styles = FontStyles{italic, ..FontStyles::default()};
            FontResource::new_ok(
                uuid!("8a471243-2466-4241-a4cb-c341ce8e844a"),
                ResourceKind::External,
                Font::from_memory(data.to_vec(), 1024, styles, Vec::default()).unwrap(),
            )
        }
    );
    /// Fyrox's default build-in font for rendering text when no other font is specified.
    pub static ref BUILT_IN_FONT: BuiltInResource<Font> = BuiltInResource::new(
        "__BUILT_IN_FONT__",
        embedded_data_source!("./built_in_font.ttf"),
        |data| {
            let styles = FontStyles{
                bold: Some(BUILT_IN_BOLD.resource()),
                italic: Some(BUILT_IN_ITALIC.resource()),
                bold_italic: Some(BOLD_ITALIC.resource()),
            };
            FontResource::new_ok(
                uuid!("77260e8e-f6fa-429c-8009-13dda2673925"),
                ResourceKind::External,
                Font::from_memory(data.to_vec(), 1024, styles, Vec::default()).unwrap(),
            )
        }
    );
}

/// Wait for all subfonts of this font to be completely loaded, including recursively searching
/// through the fallback fonts, and the fallbacks of the fallbacks, to ensure that this font is
/// fully ready to be used. In the event that any of the fonts failed to load, or a cycle is found
/// in the fallbacks, then an error is returned.
pub async fn wait_for_subfonts(font: FontResource) -> Result<FontResource, LoadError> {
    let mut stack = Vec::new();
    let font = font.await?;
    let bold = font.data_ref().bold.clone();
    if let Some(bold) = bold {
        wait_for_fallbacks(bold, &mut stack).await?;
        stack.clear();
    }
    let italic = font.data_ref().italic.clone();
    if let Some(italic) = italic {
        wait_for_fallbacks(italic, &mut stack).await?;
        stack.clear();
    }
    let bold_italic = font.data_ref().bold_italic.clone();
    if let Some(bold_italic) = bold_italic {
        wait_for_fallbacks(bold_italic, &mut stack).await?;
        stack.clear();
    }
    wait_for_fallbacks(font, &mut stack).await
}

fn write_font_names<W: std::fmt::Write>(fonts: &[FontResource], out: &mut W) -> std::fmt::Result {
    fn write_name<W: std::fmt::Write>(font: &FontResource, out: &mut W) -> std::fmt::Result {
        if font.is_ok() {
            out.write_str(font.data_ref().name().unwrap_or("unnamed"))
        } else {
            out.write_str("unnamed")
        }
    }
    if let Some((first, rest)) = fonts.split_first() {
        write_name(first, out)?;
        for font in rest {
            out.write_str(" > ")?;
            write_name(font, out)?;
        }
    }
    Ok(())
}

/// Recursively wait for all fallback fonts of the given font to be loaded,
/// so that this font is fully ready to be rendered, and use the given stack
/// to prevent infinite recursion due to a fallback cycle. An error is returned
/// if a cycle is detected.
async fn wait_for_fallbacks(
    font: FontResource,
    stack: &mut Vec<FontResource>,
) -> Result<FontResource, LoadError> {
    if stack.contains(&font) {
        let mut err = "Cyclic fallback fonts at: ".to_string();
        write_font_names(stack, &mut err).unwrap();
        return Err(LoadError::new(err));
    }
    stack.push(font.clone());
    let font = font.await?;
    let fallbacks = font
        .data_ref()
        .fallbacks
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    for fallback in fallbacks {
        Box::pin(wait_for_fallbacks(fallback, stack)).await?;
    }
    _ = stack.pop();
    Ok(font)
}

#[derive(Default, Debug, Clone)]
pub struct FontStyles {
    pub bold: Option<FontResource>,
    pub italic: Option<FontResource>,
    pub bold_italic: Option<FontResource>,
}

impl Font {
    /// The name of the font, if available.
    pub fn name(&self) -> Option<&str> {
        self.inner.as_ref().and_then(|f| f.name())
    }

    /// Create a font from an u8 array of font data such as one might get from a font file.
    pub fn from_memory(
        data: impl Deref<Target = [u8]>,
        page_size: usize,
        styles: FontStyles,
        fallbacks: Vec<Option<FontResource>>,
    ) -> Result<Self, &'static str> {
        let fontdue_font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())?;
        Ok(Font {
            inner: Some(fontdue_font),
            atlases: Default::default(),
            page_size,
            bold: styles.bold,
            italic: styles.italic,
            bold_italic: styles.bold_italic,
            fallbacks,
        })
    }

    /// Asynchronously read font data from the file at the given path to construct a font.
    pub async fn from_file<P: AsRef<Path>>(
        path: P,
        options: FontImportOptions,
        io: &dyn ResourceIo,
        resource_manager: &ResourceManager,
    ) -> Result<Self, LoadError> {
        if let Ok(file_content) = io.load_file(path.as_ref()).await {
            let page_size = options.page_size;
            let mut bold = options.bold;
            let mut italic = options.italic;
            let mut bold_italic = options.bold_italic;
            if let Some(bold) = &mut bold {
                resource_manager.request_resource(bold);
            }
            if let Some(italic) = &mut italic {
                resource_manager.request_resource(italic);
            }
            if let Some(bold_italic) = &mut bold_italic {
                resource_manager.request_resource(bold_italic);
            }
            let mut fallbacks = options.fallbacks;
            for font in fallbacks.iter_mut().flatten() {
                resource_manager.request_resource(font);
            }
            let styles = FontStyles {
                bold,
                italic,
                bold_italic,
            };
            Self::from_memory(file_content, page_size, styles, fallbacks).map_err(LoadError::new)
        } else {
            Err(LoadError::new("Unable to read file"))
        }
    }

    /// Tries to get a glyph at the given Unicode position of the given height. If there's no rendered
    /// glyph, this method tries to render the glyph and put into a suitable atlas (see [`Atlas`] docs
    /// for more info). If the given Unicode position has no representation in the font, [`None`] will
    /// be returned. If the requested size of the glyph is too big to fit into the page size of the
    /// font, [`None`] will be returned. Keep in mind that this method is free to create as many atlases
    /// with any number of pages in them. Each atlas corresponds to a particular glyph size, each glyph
    /// in the atlas could be rendered at any page in the atlas.
    #[inline]
    pub fn glyph(&mut self, unicode: char, height: f32) -> Option<&FontGlyph> {
        if !height.is_finite() || height <= f32::EPSILON {
            return None;
        }
        let height = FontHeight(height);
        let inner = self
            .inner
            .as_ref()
            .expect("Font reader must be initialized!");
        self.atlases.entry(height).or_default().glyph(
            inner,
            unicode,
            height,
            self.page_size,
            &self.fallbacks,
        )
    }

    /// The highest point of any glyph of this font above the baseline, usually positive.
    #[inline]
    pub fn ascender(&self, height: f32) -> f32 {
        self.inner
            .as_ref()
            .unwrap()
            .horizontal_line_metrics(height)
            .map(|m| m.ascent)
            .unwrap_or_default()
    }

    /// The lowest point of any glyph of this font below the baseline, usually negative.
    #[inline]
    pub fn descender(&self, height: f32) -> f32 {
        self.inner
            .as_ref()
            .unwrap()
            .horizontal_line_metrics(height)
            .map(|m| m.descent)
            .unwrap_or_default()
    }

    /// The horizontal scaled kerning value for the given two characters, if available,
    /// scaled to the given text height. The kerning value is usually negative, and it is added
    /// to the advance of the left glyph to bring the right glyph closer when appropriate for some
    /// pairs of glyphs.
    #[inline]
    pub fn horizontal_kerning(&self, height: f32, left: char, right: char) -> Option<f32> {
        self.inner
            .as_ref()
            .unwrap()
            .horizontal_kern(left, right, height)
    }

    /// The size of each atlas page in font pixels. Each page is a square measuring `page_size` x `page_size`.
    #[inline]
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    /// The horizontal distance between the start of the glyph for the given character and the start of the next
    /// glyph, when rendered for the given text height.
    #[inline]
    pub fn glyph_advance(&mut self, unicode: char, height: f32) -> f32 {
        self.glyph(unicode, height)
            .map_or(height, |glyph| glyph.advance)
    }
}

/// Font builder allows you to load fonts in a declarative manner.
pub struct FontBuilder {
    /// The size of each atlas page in font pixels. Each page is a square measuring `page_size` x `page_size`.
    page_size: usize,
    bold: Option<FontResource>,
    italic: Option<FontResource>,
    bold_italic: Option<FontResource>,
    fallbacks: Vec<Option<FontResource>>,
}

impl FontBuilder {
    /// Creates a default FontBuilder.
    pub fn new() -> Self {
        Self {
            page_size: 1024,
            bold: None,
            italic: None,
            bold_italic: None,
            fallbacks: Vec::default(),
        }
    }

    /// The size of each atlas page in font pixels. Each page is a square measuring `page_size` x `page_size`.
    pub fn with_page_size(mut self, size: usize) -> Self {
        self.page_size = size;
        self
    }

    /// The bold version of this font.
    pub fn with_bold(mut self, font: FontResource) -> Self {
        self.bold = Some(font);
        self
    }

    /// The italic version of this font.
    pub fn with_italic(mut self, font: FontResource) -> Self {
        self.italic = Some(font);
        self
    }

    pub fn with_bold_italic(mut self, font: FontResource) -> Self {
        self.bold_italic = Some(font);
        self
    }

    /// A fallback font to supply glyphs for special characters that are not represented in
    /// this font.
    pub fn with_fallback(mut self, font: FontResource) -> Self {
        self.fallbacks.push(Some(font));
        self
    }

    /// A list of fallback fonts to supply glyphs for special characters that are not represented
    /// in this font, replacing any existing fallbacks for this font.
    pub fn with_fallbacks(mut self, fallbacks: Vec<Option<FontResource>>) -> Self {
        self.fallbacks = fallbacks;
        self
    }

    /// Build the options object for this font.
    fn into_options(self) -> FontImportOptions {
        FontImportOptions {
            page_size: self.page_size,
            bold: self.bold,
            italic: self.italic,
            bold_italic: self.bold_italic,
            fallbacks: self.fallbacks,
        }
    }

    /// Creates a new font from the data at the specified path.
    pub async fn build_from_file(
        self,
        path: impl AsRef<Path>,
        io: &dyn ResourceIo,
        resource_manager: &ResourceManager,
    ) -> Result<Font, LoadError> {
        Font::from_file(path, self.into_options(), io, resource_manager).await
    }

    /// Creates a new font from bytes in memory.
    pub fn build_from_memory(
        mut self,
        data: impl Deref<Target = [u8]>,
        resource_manager: &ResourceManager,
    ) -> Result<Font, &'static str> {
        for font in self.fallbacks.iter_mut().flatten() {
            resource_manager.request_resource(font);
        }
        let styles = FontStyles {
            bold: self.bold,
            italic: self.italic,
            bold_italic: self.bold_italic,
        };
        Font::from_memory(data, self.page_size, styles, self.fallbacks)
    }
}
