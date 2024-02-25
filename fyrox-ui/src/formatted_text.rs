use crate::{
    brush::Brush,
    core::{algebra::Vector2, color::Color, math::Rect, reflect::prelude::*, visitor::prelude::*},
    font::FontResource,
    HorizontalAlignment, VerticalAlignment,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use std::ops::Range;
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[derive(Debug, Clone, Default)]
pub struct TextGlyph {
    pub bounds: Rect<f32>,
    pub tex_coords: [Vector2<f32>; 4],
    pub atlas_page_index: usize,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct TextLine {
    /// Index of starting symbol in text array.
    pub begin: usize,
    /// Index of ending symbol in text array.
    pub end: usize,
    /// Total width of line.
    pub width: f32,
    /// Total height of line. Usually just ascender of a font.
    pub height: f32,
    /// Local horizontal position of line.
    pub x_offset: f32,
    /// Local vertical position of line.
    pub y_offset: f32,
}

impl TextLine {
    fn new() -> TextLine {
        TextLine {
            begin: 0,
            end: 0,
            width: 0.0,
            height: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.begin
    }

    pub fn is_empty(&self) -> bool {
        self.end == self.begin
    }
}

/// Wrapping mode for formatted text.
#[derive(
    Default,
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Hash,
    Debug,
    Eq,
    Visit,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
)]
pub enum WrapMode {
    /// No wrapping needed.
    #[default]
    NoWrap,

    /// Letter-based wrapping.
    Letter,

    /// Word-based wrapping.
    Word,
}

uuid_provider!(WrapMode = "f1290ceb-3fee-461f-a1e9-f9450bd06805");

#[derive(Default, Clone, Debug, Visit, Reflect)]
pub struct FormattedText {
    font: InheritableVariable<FontResource>,
    text: InheritableVariable<Vec<char>>,
    // Temporary buffer used to split text on lines. We need it to reduce memory allocations
    // when we changing text too frequently, here we sacrifice some memory in order to get
    // more performance.
    #[reflect(hidden)]
    #[visit(skip)]
    lines: Vec<TextLine>,
    // Final glyphs for draw buffer.
    #[visit(skip)]
    #[reflect(hidden)]
    glyphs: Vec<TextGlyph>,
    vertical_alignment: InheritableVariable<VerticalAlignment>,
    horizontal_alignment: InheritableVariable<HorizontalAlignment>,
    brush: InheritableVariable<Brush>,
    #[visit(skip)]
    #[reflect(hidden)]
    constraint: Vector2<f32>,
    wrap: InheritableVariable<WrapMode>,
    mask_char: InheritableVariable<Option<char>>,
    #[visit(rename = "Height")]
    font_size: InheritableVariable<f32>,
    pub shadow: InheritableVariable<bool>,
    pub shadow_brush: InheritableVariable<Brush>,
    pub shadow_dilation: InheritableVariable<f32>,
    pub shadow_offset: InheritableVariable<Vector2<f32>>,
}

#[derive(Copy, Clone, Debug)]
struct Word {
    width: f32,
    length: usize,
}

impl FormattedText {
    pub fn get_glyphs(&self) -> &[TextGlyph] {
        &self.glyphs
    }

    pub fn get_font(&self) -> FontResource {
        (*self.font).clone()
    }

    pub fn set_font(&mut self, font: FontResource) -> &mut Self {
        self.font.set_value_and_mark_modified(font);
        self
    }

    pub fn font_size(&self) -> f32 {
        *self.font_size
    }

    pub fn set_font_size(&mut self, font_size: f32) -> &mut Self {
        self.font_size.set_value_and_mark_modified(font_size);
        self
    }

    pub fn get_lines(&self) -> &[TextLine] {
        &self.lines
    }

    pub fn set_vertical_alignment(&mut self, vertical_alignment: VerticalAlignment) -> &mut Self {
        self.vertical_alignment
            .set_value_and_mark_modified(vertical_alignment);
        self
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        *self.vertical_alignment
    }

    pub fn set_horizontal_alignment(
        &mut self,
        horizontal_alignment: HorizontalAlignment,
    ) -> &mut Self {
        self.horizontal_alignment
            .set_value_and_mark_modified(horizontal_alignment);
        self
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        *self.horizontal_alignment
    }

    pub fn set_brush(&mut self, brush: Brush) -> &mut Self {
        self.brush.set_value_and_mark_modified(brush);
        self
    }

    pub fn brush(&self) -> Brush {
        (*self.brush).clone()
    }

    pub fn set_constraint(&mut self, constraint: Vector2<f32>) -> &mut Self {
        self.constraint = constraint;
        self
    }

    pub fn get_raw_text(&self) -> &[char] {
        &self.text
    }

    pub fn text(&self) -> String {
        self.text.iter().collect()
    }

    pub fn get_range_width<T: IntoIterator<Item = usize>>(&self, range: T) -> f32 {
        let mut width = 0.0;
        if let Some(font) = self.font.state().data() {
            for index in range {
                // We can't trust the range values, check to prevent panic.
                if let Some(glyph) = self.text.get(index) {
                    width += font.glyph_advance(*glyph, *self.font_size);
                }
            }
        }
        width
    }

    pub fn set_text<P: AsRef<str>>(&mut self, text: P) -> &mut Self {
        self.text
            .set_value_and_mark_modified(text.as_ref().chars().collect());
        self
    }

    pub fn set_wrap(&mut self, wrap: WrapMode) -> &mut Self {
        self.wrap.set_value_and_mark_modified(wrap);
        self
    }

    /// Sets whether the shadow enabled or not.
    pub fn set_shadow(&mut self, shadow: bool) -> &mut Self {
        self.shadow.set_value_and_mark_modified(shadow);
        self
    }

    /// Sets desired shadow brush. It will be used to render the shadow.
    pub fn set_shadow_brush(&mut self, brush: Brush) -> &mut Self {
        self.shadow_brush.set_value_and_mark_modified(brush);
        self
    }

    /// Sets desired shadow dilation in units. Keep in mind that the dilation is absolute,
    /// not percentage-based.
    pub fn set_shadow_dilation(&mut self, thickness: f32) -> &mut Self {
        self.shadow_dilation.set_value_and_mark_modified(thickness);
        self
    }

    /// Sets desired shadow offset in units.
    pub fn set_shadow_offset(&mut self, offset: Vector2<f32>) -> &mut Self {
        self.shadow_offset.set_value_and_mark_modified(offset);
        self
    }

    pub fn wrap_mode(&self) -> WrapMode {
        *self.wrap
    }

    pub fn insert_char(&mut self, code: char, index: usize) -> &mut Self {
        self.text.insert(index, code);
        self
    }

    pub fn insert_str(&mut self, str: &str, position: usize) -> &mut Self {
        for (i, code) in str.chars().enumerate() {
            self.text.insert(position + i, code);
        }

        self
    }

    pub fn remove_range(&mut self, range: Range<usize>) -> &mut Self {
        self.text.drain(range);
        self
    }

    pub fn remove_at(&mut self, index: usize) -> &mut Self {
        self.text.remove(index);
        self
    }

    pub fn build(&mut self) -> Vector2<f32> {
        let mut font_state = self.font.state();
        let Some(font) = font_state.data() else {
            return Default::default();
        };

        let masked_text;
        let text = if let Some(mask_char) = *self.mask_char {
            masked_text = (0..self.text.len()).map(|_| mask_char).collect();
            &masked_text
        } else {
            &*self.text
        };

        // Split on lines.
        let mut total_height = 0.0;
        let mut current_line = TextLine::new();
        let mut word: Option<Word> = None;
        self.lines.clear();
        for (i, &character) in text.iter().enumerate() {
            let advance = match font.glyph(character, *self.font_size) {
                Some(glyph) => glyph.advance,
                None => *self.font_size,
            };
            let is_new_line = character == '\n' || character == '\r';
            let new_width = current_line.width + advance;
            let is_white_space = character.is_whitespace();
            let word_ended = word.is_some() && is_white_space || i == self.text.len() - 1;

            if *self.wrap == WrapMode::Word && !is_white_space {
                match word.as_mut() {
                    Some(word) => {
                        word.width += advance;
                        word.length += 1;
                    }
                    None => {
                        word = Some(Word {
                            width: advance,
                            length: 1,
                        });
                    }
                };
            }

            if is_new_line {
                if let Some(word) = word.take() {
                    current_line.width += word.width;
                    current_line.end += word.length;
                }
                self.lines.push(current_line);
                current_line.begin = if is_new_line { i + 1 } else { i };
                current_line.end = current_line.begin;
                current_line.width = advance;
                total_height += font.ascender(*self.font_size);
            } else {
                match *self.wrap {
                    WrapMode::NoWrap => {
                        current_line.width = new_width;
                        current_line.end += 1;
                    }
                    WrapMode::Letter => {
                        if new_width > self.constraint.x {
                            self.lines.push(current_line);
                            current_line.begin = if is_new_line { i + 1 } else { i };
                            current_line.end = current_line.begin + 1;
                            current_line.width = advance;
                            total_height += font.ascender(*self.font_size);
                        } else {
                            current_line.width = new_width;
                            current_line.end += 1;
                        }
                    }
                    WrapMode::Word => {
                        if word_ended {
                            if let Some(word) = word.take() {
                                if word.width > self.constraint.x {
                                    // The word is longer than available constraints.
                                    // Push the word as a whole.
                                    current_line.width += word.width;
                                    current_line.end += word.length;
                                    self.lines.push(current_line);
                                    current_line.begin = current_line.end;
                                    current_line.width = 0.0;
                                    total_height += font.ascender(*self.font_size);
                                } else if current_line.width + word.width > self.constraint.x {
                                    // The word will exceed horizontal constraint, we have to
                                    // commit current line and move the word in the next line.
                                    self.lines.push(current_line);
                                    current_line.begin = i - word.length;
                                    current_line.end = i;
                                    current_line.width = word.width;
                                    total_height += font.ascender(*self.font_size);
                                } else {
                                    // The word does not exceed horizontal constraint, append it
                                    // to the line.
                                    current_line.width += word.width;
                                    current_line.end += word.length;
                                }
                            }
                        }

                        // White-space characters are not part of word so pass them through.
                        if is_white_space {
                            current_line.end += 1;
                            current_line.width += advance;
                        }
                    }
                }
            }
        }
        // Commit rest of text.
        if current_line.begin != current_line.end {
            for &character in text.iter().skip(current_line.end) {
                let advance = match font.glyph(character, *self.font_size) {
                    Some(glyph) => glyph.advance,
                    None => *self.font_size,
                };
                current_line.width += advance;
            }
            current_line.end = self.text.len();
            self.lines.push(current_line);
            total_height += font.ascender(*self.font_size);
        }

        // Align lines according to desired alignment.
        for line in self.lines.iter_mut() {
            match *self.horizontal_alignment {
                HorizontalAlignment::Left => line.x_offset = 0.0,
                HorizontalAlignment::Center => {
                    if self.constraint.x.is_infinite() {
                        line.x_offset = 0.0;
                    } else {
                        line.x_offset = 0.5 * (self.constraint.x - line.width).max(0.0);
                    }
                }
                HorizontalAlignment::Right => {
                    if self.constraint.x.is_infinite() {
                        line.x_offset = 0.0;
                    } else {
                        line.x_offset = (self.constraint.x - line.width).max(0.0)
                    }
                }
                HorizontalAlignment::Stretch => line.x_offset = 0.0,
            }
        }

        // Generate glyphs for each text line.
        self.glyphs.clear();

        let cursor_y_start = match *self.vertical_alignment {
            VerticalAlignment::Top => 0.0,
            VerticalAlignment::Center => {
                if self.constraint.y.is_infinite() {
                    0.0
                } else {
                    (self.constraint.y - total_height).max(0.0) * 0.5
                }
            }
            VerticalAlignment::Bottom => {
                if self.constraint.y.is_infinite() {
                    0.0
                } else {
                    (self.constraint.y - total_height).max(0.0)
                }
            }
            VerticalAlignment::Stretch => 0.0,
        };

        let cursor_x_start = if self.constraint.x.is_infinite() {
            0.0
        } else {
            self.constraint.x
        };

        let mut cursor = Vector2::new(cursor_x_start, cursor_y_start);
        for line in self.lines.iter_mut() {
            cursor.x = line.x_offset;

            let ascender = font.ascender(*self.font_size);
            for &character in text.iter().take(line.end).skip(line.begin) {
                match font.glyph(character, *self.font_size) {
                    Some(glyph) => {
                        // Insert glyph
                        let rect = Rect::new(
                            cursor.x + glyph.left.floor(),
                            cursor.y + ascender.floor()
                                - glyph.top.floor()
                                - glyph.bitmap_height as f32,
                            glyph.bitmap_width as f32,
                            glyph.bitmap_height as f32,
                        );
                        let text_glyph = TextGlyph {
                            bounds: rect,
                            tex_coords: glyph.tex_coords,
                            atlas_page_index: glyph.page_index,
                        };
                        self.glyphs.push(text_glyph);

                        cursor.x += glyph.advance;
                    }
                    None => {
                        // Insert invalid symbol
                        let rect = Rect::new(
                            cursor.x,
                            cursor.y + font.ascender(*self.font_size),
                            *self.font_size,
                            *self.font_size,
                        );
                        self.glyphs.push(TextGlyph {
                            bounds: rect,
                            tex_coords: [Vector2::default(); 4],
                            atlas_page_index: 0,
                        });
                        cursor.x += rect.w();
                    }
                }
            }
            line.height = font.ascender(*self.font_size);
            line.y_offset = cursor.y;
            cursor.y += font.ascender(*self.font_size);
        }

        // Minus here is because descender has negative value.
        let mut full_size = Vector2::new(0.0, total_height - font.descender(*self.font_size));
        for line in self.lines.iter() {
            full_size.x = line.width.max(full_size.x);
        }
        full_size
    }
}

pub struct FormattedTextBuilder {
    font: FontResource,
    brush: Brush,
    constraint: Vector2<f32>,
    text: String,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    wrap: WrapMode,
    mask_char: Option<char>,
    shadow: bool,
    shadow_brush: Brush,
    shadow_dilation: f32,
    shadow_offset: Vector2<f32>,
    font_size: f32,
}

impl FormattedTextBuilder {
    /// Creates new formatted text builder with default parameters.
    pub fn new(font: FontResource) -> FormattedTextBuilder {
        FormattedTextBuilder {
            font,
            text: "".to_owned(),
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            brush: Brush::Solid(Color::WHITE),
            constraint: Vector2::new(128.0, 128.0),
            wrap: WrapMode::NoWrap,
            mask_char: None,
            shadow: false,
            shadow_brush: Brush::Solid(Color::BLACK),
            shadow_dilation: 1.0,
            shadow_offset: Vector2::new(1.0, 1.0),
            font_size: 14.0,
        }
    }

    pub fn with_vertical_alignment(mut self, vertical_alignment: VerticalAlignment) -> Self {
        self.vertical_alignment = vertical_alignment;
        self
    }

    pub fn with_wrap(mut self, wrap: WrapMode) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn with_horizontal_alignment(mut self, horizontal_alignment: HorizontalAlignment) -> Self {
        self.horizontal_alignment = horizontal_alignment;
        self
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.text = text;
        self
    }

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn with_constraint(mut self, constraint: Vector2<f32>) -> Self {
        self.constraint = constraint;
        self
    }

    pub fn with_brush(mut self, brush: Brush) -> Self {
        self.brush = brush;
        self
    }

    pub fn with_mask_char(mut self, mask_char: Option<char>) -> Self {
        self.mask_char = mask_char;
        self
    }

    /// Whether the shadow enabled or not.
    pub fn with_shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    /// Sets desired shadow brush. It will be used to render the shadow.
    pub fn with_shadow_brush(mut self, brush: Brush) -> Self {
        self.shadow_brush = brush;
        self
    }

    /// Sets desired shadow dilation in units. Keep in mind that the dilation is absolute,
    /// not percentage-based.
    pub fn with_shadow_dilation(mut self, thickness: f32) -> Self {
        self.shadow_dilation = thickness;
        self
    }

    /// Sets desired shadow offset in units.
    pub fn with_shadow_offset(mut self, offset: Vector2<f32>) -> Self {
        self.shadow_offset = offset;
        self
    }

    pub fn build(self) -> FormattedText {
        FormattedText {
            text: self.text.chars().collect::<Vec<char>>().into(),
            lines: Vec::new(),
            glyphs: Vec::new(),
            vertical_alignment: self.vertical_alignment.into(),
            horizontal_alignment: self.horizontal_alignment.into(),
            brush: self.brush.into(),
            constraint: self.constraint,
            wrap: self.wrap.into(),
            mask_char: self.mask_char.into(),
            font_size: self.font_size.into(),
            shadow: self.shadow.into(),
            shadow_brush: self.shadow_brush.into(),
            font: self.font.into(),
            shadow_dilation: self.shadow_dilation.into(),
            shadow_offset: self.shadow_offset.into(),
        }
    }
}
