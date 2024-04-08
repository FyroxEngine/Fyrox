use crate::{
    brush::Brush,
    core::{algebra::Vector2, color::Color, math::Rect, reflect::prelude::*, visitor::prelude::*},
    font::{Font, FontGlyph, FontResource},
    HorizontalAlignment, VerticalAlignment,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use std::ops::Range;
use strum_macros::{AsRefStr, EnumString, VariantNames};

mod textwrapper;
use textwrapper::*;

/// Defines a position in the text. It is just a coordinates of a character in text.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Visit, Reflect)]
pub struct Position {
    /// Line index.
    pub line: usize,

    /// Offset from the beginning of the line.
    pub offset: usize,
}

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

    pub fn y_distance(&self, y: f32) -> f32 {
        (self.y_offset + self.height / 2.0 - y).abs()
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

struct GlyphMetrics<'a> {
    font: &'a mut Font,
    size: f32,
}

impl<'a> GlyphMetrics<'a> {
    fn ascender(&self) -> f32 {
        self.font.ascender(self.size)
    }
    fn descender(&self) -> f32 {
        self.font.descender(self.size)
    }
    fn newline_advance(&self) -> f32 {
        self.size / 2.0
    }
    fn advance(&mut self, c: char) -> f32 {
        match c {
            '\n' => self.newline_advance(),
            _ => self.font.glyph_advance(c, self.size),
        }
    }
    fn glyph(&mut self, c: char) -> Option<&FontGlyph> {
        self.font.glyph(c, self.size)
    }
}

fn build_glyph(metrics: &mut GlyphMetrics, x: f32, y: f32, character: char) -> (TextGlyph, f32) {
    let ascender = metrics.ascender();
    let font_size = metrics.size;
    match metrics.glyph(character) {
        Some(glyph) => {
            // Insert glyph
            let rect = Rect::new(
                x + glyph.left.floor(),
                y + ascender.floor() - glyph.top.floor() - glyph.bitmap_height as f32,
                glyph.bitmap_width as f32,
                glyph.bitmap_height as f32,
            );
            let text_glyph = TextGlyph {
                bounds: rect,
                tex_coords: glyph.tex_coords,
                atlas_page_index: glyph.page_index,
            };
            (text_glyph, glyph.advance)
        }
        None => {
            // Insert invalid symbol
            let rect = Rect::new(x, y + ascender, font_size, font_size);
            let text_glyph = TextGlyph {
                bounds: rect,
                tex_coords: [Vector2::default(); 4],
                atlas_page_index: 0,
            };
            (text_glyph, rect.w())
        }
    }
}

struct WrapSink<'a> {
    lines: &'a mut Vec<TextLine>,
    max_width: f32,
}

impl<'a> LineSink for WrapSink<'a> {
    fn push_line(&mut self, range: Range<usize>, width: f32) {
        let mut line = TextLine::new();
        line.begin = range.start;
        line.end = range.end;
        line.width = width;
        self.lines.push(line);
    }

    fn max_width(&self) -> f32 {
        self.max_width
    }
}

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

impl FormattedText {
    pub fn nearest_valid_position(&self, start: Position) -> Position {
        if self.lines.is_empty() {
            return Position::default();
        }
        let mut pos = start;
        pos.line = usize::min(pos.line, self.lines.len() - 1);
        pos.offset = usize::min(pos.offset, self.lines[pos.line].len());
        pos
    }
    pub fn get_relative_position_x(&self, start: Position, offset: isize) -> Position {
        if self.lines.is_empty() {
            return Position::default();
        }
        let mut pos = self.nearest_valid_position(start);
        let distance = offset.abs();
        for _ in 0..distance {
            if offset < 0 {
                if pos.offset > 0 {
                    pos.offset -= 1
                } else if pos.line > 0 {
                    pos.line -= 1;
                    pos.offset = self.lines[pos.line].len().saturating_sub(1);
                } else {
                    pos.offset = 0;
                    break;
                }
            } else {
                let line = &self.lines[pos.line];
                if pos.offset + 1 < line.len() {
                    pos.offset += 1;
                } else if pos.line + 1 < self.lines.len() {
                    pos.line += 1;
                    pos.offset = 0;
                } else {
                    pos.offset = line.len();
                    break;
                }
            }
        }
        pos
    }

    pub fn get_relative_position_y(&self, start: Position, offset: isize) -> Position {
        let mut pos = self.nearest_valid_position(start);
        pos.line = pos.line.saturating_add_signed(offset);
        self.nearest_valid_position(pos)
    }

    pub fn get_line_range(&self, line: usize) -> Range<Position> {
        let length = self.lines.get(line).map(TextLine::len).unwrap_or(0);
        Range {
            start: Position { line, offset: 0 },
            end: Position {
                line,
                offset: length,
            },
        }
    }

    pub fn iter_line_ranges_within(
        &self,
        range: Range<Position>,
    ) -> impl Iterator<Item = Range<Position>> + '_ {
        (range.start.line..=range.end.line).map(move |i| {
            let r = self.get_line_range(i);
            Range {
                start: Position::max(range.start, r.start),
                end: Position::min(range.end, r.end),
            }
        })
    }

    pub fn end_position(&self) -> Position {
        match self.lines.iter().enumerate().last() {
            Some((i, line)) => Position {
                line: i,
                offset: line.len(),
            },
            None => Position::default(),
        }
    }

    fn position_to_char_index_internal(&self, position: Position, clamp: bool) -> Option<usize> {
        self.lines.get(position.line).map(|line| {
            line.begin
                + position.offset.min(if clamp {
                    line.len().saturating_sub(1)
                } else {
                    line.len()
                })
        })
    }
    pub fn position_range_to_char_index_range(&self, range: Range<Position>) -> Range<usize> {
        let start = self
            .position_to_char_index_unclamped(range.start)
            .unwrap_or(0);
        let end = self
            .position_to_char_index_unclamped(range.end)
            .unwrap_or(self.text.len());
        start..end
    }
    /// Maps input [`Position`] to a linear position in character array.
    /// The index returned is the index of the character after the position, which may be
    /// out-of-bounds if thee position is at the end of the text.
    /// You should check the index before trying to use it to fetch data from inner array of characters.
    pub fn position_to_char_index_unclamped(&self, position: Position) -> Option<usize> {
        self.position_to_char_index_internal(position, false)
    }

    /// Maps input [`Position`] to a linear position in character array.
    /// The index returned is usually the index of the character after the position,
    /// but if the position is at the end of a line then return the index of the character _before_ the position.
    /// In other words, the last two positions of each line are mapped to the same character index.
    /// Output index will always be valid for fetching, if the method returned `Some(index)`.
    /// The index however cannot be used for text insertion, because it cannot point to a "place after last char".
    pub fn position_to_char_index_clamped(&self, position: Position) -> Option<usize> {
        self.position_to_char_index_internal(position, true)
    }

    /// Maps linear character index (as in string) to its actual location in the text.
    pub fn char_index_to_position(&self, i: usize) -> Option<Position> {
        self.lines
            .iter()
            .enumerate()
            .find_map(|(line_index, line)| {
                if (line.begin..line.end).contains(&i) {
                    Some(Position {
                        line: line_index,
                        offset: i - line.begin,
                    })
                } else {
                    None
                }
            })
            .or(Some(self.end_position()))
    }

    pub fn position_to_local(&self, position: Position) -> Vector2<f32> {
        let mut state = self.font.state();
        let Some(font) = state.data() else {
            return Default::default();
        };
        let mut metrics = GlyphMetrics {
            font,
            size: *self.font_size,
        };
        let mut caret_pos = Vector2::default();
        let position = self.nearest_valid_position(position);

        let line = self.lines[position.line];
        let raw_text = self.get_raw_text();
        caret_pos += Vector2::new(line.x_offset, line.y_offset);
        for (offset, char_index) in (line.begin..line.end).enumerate() {
            if offset >= position.offset {
                break;
            }
            if let Some(advance) = raw_text.get(char_index).map(|c| metrics.advance(*c)) {
                caret_pos.x += advance;
            } else {
                caret_pos.x += metrics.size;
            }
        }
        caret_pos
    }

    pub fn local_to_position(&self, point: Vector2<f32>) -> Position {
        let font = self.get_font();
        let mut state = font.state();
        let Some(font) = state.data() else {
            return Position::default();
        };
        let mut metrics = GlyphMetrics {
            font,
            size: self.font_size(),
        };
        let y = point.y;

        let Some(line_index) = self
            .lines
            .iter()
            .enumerate()
            .map(|(i, a)| (i, a.y_distance(y)))
            .min_by(|a, b| f32::total_cmp(&a.1, &b.1))
            .map(|(i, _)| i)
        else {
            return Position::default();
        };
        let line = self.lines[line_index];
        let x = point.x - line.x_offset;
        let mut glyph_x: f32 = 0.0;
        let mut min_dist: f32 = x.abs();
        let mut min_index: usize = 0;
        let raw_text = self.get_raw_text();
        for (offset, char_index) in (line.begin..line.end).enumerate() {
            if let Some(advance) = raw_text.get(char_index).map(|c| metrics.advance(*c)) {
                glyph_x += advance;
            } else {
                glyph_x += self.font_size();
            }
            let dist = (x - glyph_x).abs();
            if dist < min_dist {
                min_dist = dist;
                min_index = offset + 1;
            }
        }
        Position {
            line: line_index,
            offset: min_index,
        }
    }

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

    pub fn text_range(&self, range: Range<usize>) -> String {
        self.text[range].iter().collect()
    }

    pub fn get_range_width<T: IntoIterator<Item = usize>>(&self, range: T) -> f32 {
        let mut width = 0.0;
        if let Some(font) = self.font.state().data() {
            let mut metrics = GlyphMetrics {
                font,
                size: self.font_size(),
            };
            for index in range {
                // We can't trust the range values, check to prevent panic.
                if let Some(glyph) = self.text.get(index) {
                    width += metrics.advance(*glyph);
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
        let mut metrics = GlyphMetrics {
            font,
            size: self.font_size(),
        };
        let line_height: f32 = metrics.ascender();

        self.lines.clear();
        let sink = WrapSink {
            lines: &mut self.lines,
            max_width: self.constraint.x,
        };
        if let Some(mask) = *self.mask_char {
            let advance = metrics.advance(mask);
            match *self.wrap {
                WrapMode::NoWrap => wrap_mask(NoWrap::new(sink), self.text.len(), mask, advance),
                WrapMode::Letter => wrap_mask(
                    LetterWrap::new(sink),
                    self.text.len(),
                    mask,
                    *self.font_size,
                ),
                WrapMode::Word => wrap_mask(WordWrap::new(sink), self.text.len(), mask, advance),
            }
        } else {
            match *self.wrap {
                WrapMode::NoWrap => wrap(NoWrap::new(sink), &mut metrics, self.text.as_slice()),
                WrapMode::Letter => wrap(LetterWrap::new(sink), &mut metrics, self.text.as_slice()),
                WrapMode::Word => wrap(WordWrap::new(sink), &mut metrics, self.text.as_slice()),
            }
        }

        let total_height = line_height * self.lines.len() as f32;
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

        let mut y: f32 = cursor_y_start;
        for line in self.lines.iter_mut() {
            let mut x = line.x_offset;
            if let Some(mask) = *self.mask_char {
                for c in std::iter::repeat::<char>(mask).take(line.len()) {
                    let (glyph, advance) = build_glyph(&mut metrics, x, y, c);
                    self.glyphs.push(glyph);
                    x += advance;
                }
            } else {
                for c in self.text.iter().take(line.end).skip(line.begin).cloned() {
                    match c {
                        '\n' => {
                            x += metrics.newline_advance();
                        }
                        _ => {
                            let (glyph, advance) = build_glyph(&mut metrics, x, y, c);
                            self.glyphs.push(glyph);
                            x += advance;
                        }
                    }
                }
            }
            line.height = line_height;
            line.y_offset = y;
            y += line_height;
        }

        let size_x = self
            .lines
            .iter()
            .map(|line| line.width)
            .max_by(f32::total_cmp)
            .unwrap_or_default();
        // Minus here is because descender has negative value.
        let size_y = total_height - metrics.descender();
        Vector2::new(size_x, size_y)
    }
}

fn wrap<W: TextWrapper>(mut wrapper: W, metrics: &mut GlyphMetrics, text: &[char]) {
    for &character in text.iter() {
        let advance = metrics.advance(character);
        wrapper.push(character, advance);
    }
    wrapper.finish();
}

fn wrap_mask<W: TextWrapper>(mut wrapper: W, length: usize, mask_char: char, advance: f32) {
    for _ in 0..length {
        wrapper.push(mask_char, advance);
    }
    wrapper.finish();
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
