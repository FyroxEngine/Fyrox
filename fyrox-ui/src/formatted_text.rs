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

use crate::{
    brush::Brush,
    core::{
        algebra::Vector2, color::Color, math::Rect, reflect::prelude::*, uuid_provider,
        variable::InheritableVariable, visitor::prelude::*,
    },
    font::{Font, FontGlyph, FontHeight, FontResource},
    style::StyledProperty,
    HorizontalAlignment, VerticalAlignment,
};
use fyrox_core::log::Log;
use fyrox_resource::state::{LoadError, ResourceState};
pub use run::*;
use std::{
    ops::{Range, RangeBounds},
    path::PathBuf,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};
use textwrapper::*;

mod run;
mod textwrapper;

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
    pub source_char_index: usize,
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
    fn new() -> Self {
        Self {
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

impl GlyphMetrics<'_> {
    fn ascender(&self) -> f32 {
        self.font.ascender(self.size)
    }

    fn descender(&self) -> f32 {
        self.font.descender(self.size)
    }

    fn newline_advance(&self) -> f32 {
        self.size / 2.0
    }

    fn horizontal_kerning(&self, left: char, right: char) -> Option<f32> {
        self.font.horizontal_kerning(self.size, left, right)
    }

    fn advance(&mut self, c: char) -> f32 {
        match c {
            '\n' => self.newline_advance(),
            _ => self.font.glyph_advance(c, self.size),
        }
    }

    fn glyph(&mut self, c: char, super_sampling_scale: f32) -> Option<&FontGlyph> {
        self.font.glyph(c, self.size * super_sampling_scale)
    }
}

fn build_glyph(
    metrics: &mut GlyphMetrics,
    mut x: f32,
    mut y: f32,
    source_char_index: usize,
    character: char,
    prev_character: Option<char>,
    super_sampling_scale: f32,
) -> (TextGlyph, f32) {
    let ascender = metrics.ascender();
    let font_size = metrics.size;

    x = x.floor();
    y = y.floor();

    // Request larger glyph with super sampling scaling.
    match metrics.glyph(character, super_sampling_scale) {
        Some(glyph) => {
            // Discard super sampling scaling in the produced glyphs, because we're interested only
            // in larger texture size, not the "physical" size.
            let k = 1.0 / super_sampling_scale;
            // Insert glyph
            let rect = Rect::new(
                x + glyph.bitmap_left * k,
                y + ascender.floor() - glyph.bitmap_top * k - (glyph.bitmap_height * k),
                glyph.bitmap_width * k,
                glyph.bitmap_height * k,
            );
            let text_glyph = TextGlyph {
                bounds: rect,
                tex_coords: glyph.tex_coords,
                atlas_page_index: glyph.page_index,
                source_char_index,
            };
            let advance = glyph.advance
                + prev_character
                    .and_then(|prev| metrics.horizontal_kerning(prev, character))
                    .unwrap_or_default();
            (text_glyph, advance * k)
        }
        None => {
            // Insert invalid symbol
            let rect = Rect::new(x, y + ascender, font_size, font_size);
            let text_glyph = TextGlyph {
                bounds: rect,
                tex_coords: [Vector2::default(); 4],
                atlas_page_index: 0,
                source_char_index,
            };
            (text_glyph, rect.w())
        }
    }
}

struct WrapSink<'a> {
    lines: &'a mut Vec<TextLine>,
    normal_width: f32,
    first_width: f32,
}

impl LineSink for WrapSink<'_> {
    fn push_line(&mut self, range: Range<usize>, width: f32) {
        let mut line = TextLine::new();
        line.begin = range.start;
        line.end = range.end;
        line.width = width;
        self.lines.push(line);
    }

    fn max_width(&self) -> f32 {
        if self.lines.is_empty() {
            self.first_width
        } else {
            self.normal_width
        }
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
    #[reflect(hidden)]
    brush: InheritableVariable<Brush>,
    #[visit(skip)]
    #[reflect(hidden)]
    constraint: Vector2<f32>,
    wrap: InheritableVariable<WrapMode>,
    mask_char: InheritableVariable<Option<char>>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) super_sampling_scale: f32,
    #[visit(rename = "Height")]
    font_size: InheritableVariable<StyledProperty<f32>>,
    pub shadow: InheritableVariable<bool>,
    pub shadow_brush: InheritableVariable<Brush>,
    pub shadow_dilation: InheritableVariable<f32>,
    pub shadow_offset: InheritableVariable<Vector2<f32>>,
    #[visit(optional)]
    pub runs: RunSet,
    /// The indent amount of the first line of the text.
    /// A negative indent will cause every line except the first to indent.
    #[visit(optional)]
    pub line_indent: InheritableVariable<f32>,
    /// The space between lines.
    #[visit(optional)]
    pub line_space: InheritableVariable<f32>,
}

impl FormattedText {
    pub fn font_at(&self, index: usize) -> FontResource {
        self.runs.font_at(index).unwrap_or_else(|| self.get_font())
    }
    pub fn font_size_at(&self, index: usize) -> f32 {
        self.runs
            .font_size_at(index)
            .unwrap_or_else(|| self.font_size().property)
    }
    pub fn brush_at(&self, index: usize) -> Brush {
        self.runs.brush_at(index).unwrap_or_else(|| self.brush())
    }
    pub fn shadow_at(&self, index: usize) -> bool {
        self.runs.shadow_at(index).unwrap_or(*self.shadow)
    }
    pub fn shadow_brush_at(&self, index: usize) -> Brush {
        self.runs
            .shadow_brush_at(index)
            .unwrap_or_else(|| self.shadow_brush.clone_inner())
    }
    pub fn shadow_dilation_at(&self, index: usize) -> f32 {
        self.runs
            .shadow_dilation_at(index)
            .unwrap_or(*self.shadow_dilation)
    }
    pub fn shadow_offset_at(&self, index: usize) -> Vector2<f32> {
        self.runs
            .shadow_offset_at(index)
            .unwrap_or(*self.shadow_offset)
    }
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
        match self.lines.iter().enumerate().next_back() {
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
        if self.font.state().data().is_none() {
            return Default::default();
        }
        let position = self.nearest_valid_position(position);
        let line = &self.lines[position.line];
        let caret_pos = Vector2::new(line.x_offset, line.y_offset);
        let range = line.begin..line.begin + position.offset;
        caret_pos + Vector2::new(self.get_range_width(range), 0.0)
    }

    pub fn local_to_position(&self, point: Vector2<f32>) -> Position {
        if self.get_font().state().data().is_none() {
            return Position::default();
        }
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
        for (offset, char_index) in (line.begin..line.end).enumerate() {
            glyph_x += self.get_char_width(char_index).unwrap_or_default();
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

    pub fn get_glyph_draw_values(
        &self,
        layer: DrawValueLayer,
        glyph: &TextGlyph,
    ) -> GlyphDrawValues {
        let atlas_page_index = glyph.atlas_page_index;
        let i = glyph.source_char_index;
        let font = self.font_at(i);
        let height = FontHeight::from(self.font_size_at(i) * self.super_sampling_scale);
        match layer {
            DrawValueLayer::Main => GlyphDrawValues {
                atlas_page_index,
                font,
                brush: self.brush_at(i),
                height,
            },
            DrawValueLayer::Shadow => GlyphDrawValues {
                atlas_page_index,
                font,
                brush: self.shadow_brush_at(i),
                height,
            },
        }
    }

    pub fn get_font(&self) -> FontResource {
        (*self.font).clone()
    }

    pub fn set_font(&mut self, font: FontResource) -> &mut Self {
        self.font.set_value_and_mark_modified(font);
        self
    }

    pub fn font_size(&self) -> &StyledProperty<f32> {
        &self.font_size
    }

    pub fn super_sampled_font_size(&self) -> f32 {
        **self.font_size * self.super_sampling_scale
    }

    pub fn set_font_size(&mut self, font_size: StyledProperty<f32>) -> &mut Self {
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

    pub fn set_super_sampling_scale(&mut self, scale: f32) -> &mut Self {
        self.super_sampling_scale = scale;
        self
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

    /// The width of the character at the given index.
    pub fn get_char_width(&self, index: usize) -> Option<f32> {
        let glyph = self.text.get(index)?;
        Some(
            GlyphMetrics {
                font: &mut self.font_at(index).data_ref(),
                size: self.font_size_at(index),
            }
            .advance(*glyph),
        )
    }

    /// The width of the characters at the indices in the given iterator.
    /// This is equivalent to calling [`get_char_width`](Self::get_char_width) repeatedly and summing the results.
    pub fn get_range_width<T: IntoIterator<Item = usize>>(&self, range: T) -> f32 {
        let mut width = 0.0;
        for index in range {
            width += self.get_char_width(index).unwrap_or_default();
        }
        width
    }

    /// A rectangle relative to the top-left corner of the text that contains the given
    /// range of characters on the given line. None is returned if the `line` is out of
    /// bounds. The `range` is relative to the start of the line, so 0 is the first character
    /// of the line, not the first character of the text.
    ///
    /// This rect is appropriate for drawing a selection or highlight for the text,
    /// and the lower edge of the rectangle can be used to draw an underline.
    pub fn text_rect<R: RangeBounds<usize>>(&self, line: usize, range: R) -> Option<Rect<f32>> {
        let line = self.lines.get(line)?;
        let x = line.x_offset;
        let y = line.y_offset;
        let h = line.height;
        use std::ops::Bound;
        let start = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => line.len(),
        };
        let start = line.begin + start;
        let end = line.begin + end;
        let offset = self.get_range_width(line.begin..start);
        let w = self.get_range_width(start..end);
        Some(Rect::new(offset + x, y, w, h))
    }

    pub fn set_text<P: AsRef<str>>(&mut self, text: P) -> &mut Self {
        self.text
            .set_value_and_mark_modified(text.as_ref().chars().collect());
        self
    }

    pub fn set_chars(&mut self, text: Vec<char>) -> &mut Self {
        self.text.set_value_and_mark_modified(text);
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

    /// Runs can optionally modify various style settings for portions of the text.
    /// Later runs override earlier runs if their ranges overlap and the later run
    /// sets a property that conflicts with an earlier run.
    pub fn runs(&self) -> &RunSet {
        &self.runs
    }

    /// Modify runs of the text to set the style for portions of the text.
    /// Later runs potentially override earlier runs if the ranges of the runs overlap and the later run
    /// sets a property that conflicts with an earlier run.
    pub fn runs_mut(&mut self) -> &mut RunSet {
        &mut self.runs
    }

    /// Replace runs of the text to set the style for portions of the text.
    /// Later runs potentially override earlier runs if the ranges of the runs overlap and the later run
    /// sets a property that conflicts with an earlier run.
    pub fn set_runs(&mut self, runs: RunSet) -> &mut Self {
        self.runs = runs;
        self
    }

    /// The amount of indent of the first line, horizontally separating it
    /// from the start of the remaining lines.
    /// If the indent is negative, then the first line will not be indented
    /// while all the other lines will be indented. By default this is 0.0.
    pub fn set_line_indent(&mut self, indent: f32) -> &mut Self {
        self.line_indent.set_value_and_mark_modified(indent);
        self
    }

    /// The amount of indent of the first line, horizontally separating it
    /// from the start of the remaining lines.
    /// If the indent is negative, then the first line will not be indented
    /// while all the other lines will be indented. By default this is 0.0.
    pub fn line_indent(&mut self) -> f32 {
        *self.line_indent
    }

    /// The space separating each line from the line above and below.
    /// By default this is 0.0.
    pub fn set_line_space(&mut self, space: f32) -> &mut Self {
        self.line_space.set_value_and_mark_modified(space);
        self
    }

    /// The space separating each line from the line above and below.
    /// By default this is 0.0.
    pub fn line_space(&self) -> f32 {
        *self.line_space
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

    /// Returns once all fonts used by this FormattedText are finished loading.
    pub async fn wait_for_fonts(&mut self) -> Result<(), LoadError> {
        (*self.font).clone().await?;
        for run in self.runs.iter() {
            if let Some(font) = run.font() {
                font.clone().await?;
            }
        }
        Ok(())
    }

    /// Returns true if all fonts used by this resource are Ok.
    /// This `FormattedText` will not build successfully unless this returns true.
    pub fn are_fonts_loaded(&self) -> bool {
        if !self.font.is_ok() {
            return false;
        }
        for run in self.runs.iter() {
            if let Some(font) = run.font() {
                if !font.is_ok() {
                    return false;
                }
            }
        }
        true
    }

    pub fn are_fonts_loading(&self) -> bool {
        if !self.font.is_loading() {
            return true;
        }
        for run in self.runs.iter() {
            if let Some(font) = run.font() {
                if !font.is_loading() {
                    return true;
                }
            }
        }
        false
    }

    pub fn font_load_error_list(&self) -> Vec<(PathBuf, LoadError)> {
        let mut list = vec![];
        if let ResourceState::LoadError { path, error } = &self.font.header().state {
            list.push((path.clone(), error.clone()));
        }
        for run in self.runs.iter() {
            if let Some(font) = run.font() {
                if let ResourceState::LoadError { path, error } = &font.header().state {
                    list.push((path.clone(), error.clone()));
                }
            }
        }
        list
    }

    pub fn font_loading_summary(&self) -> String {
        use std::fmt::Write;
        let mut result = String::default();
        write!(result, "Primary font: {}", self.font.header().state).unwrap();
        for run in self.runs.iter() {
            if let Some(font) = run.font() {
                write!(result, "\nRun {:?}: {}", run.range, font.header().state).unwrap();
            }
        }
        result
    }

    pub fn build(&mut self) -> Vector2<f32> {
        let mut lines = std::mem::take(&mut self.lines);
        lines.clear();
        // Fail early if any font is not available.
        if !self.are_fonts_loaded() {
            Log::err(format!(
                "Text failed to build due to unloaded fonts. {:?}.\n{}",
                self.text(),
                self.font_loading_summary(),
            ));
            return Vector2::default();
        }
        let first_indent = self.line_indent.max(0.0);
        let normal_indent = -self.line_indent.min(0.0);
        let sink = WrapSink {
            lines: &mut lines,
            normal_width: self.constraint.x - normal_indent,
            first_width: self.constraint.x - first_indent,
        };
        if let Some(mask) = *self.mask_char {
            let advance = GlyphMetrics {
                font: &mut self.font.data_ref(),
                size: **self.font_size,
            }
            .advance(mask);
            match *self.wrap {
                WrapMode::NoWrap => wrap_mask(NoWrap::new(sink), self.text.len(), mask, advance),
                WrapMode::Letter => wrap_mask(
                    LetterWrap::new(sink),
                    self.text.len(),
                    mask,
                    **self.font_size,
                ),
                WrapMode::Word => wrap_mask(WordWrap::new(sink), self.text.len(), mask, advance),
            }
        } else {
            let source = self.text.iter().enumerate().map(|(i, c)| {
                let a = GlyphMetrics {
                    font: &mut self.font_at(i).data_ref(),
                    size: self.font_size_at(i),
                }
                .advance(*c);
                (*c, a)
            });
            match *self.wrap {
                WrapMode::NoWrap => wrap(NoWrap::new(sink), source),
                WrapMode::Letter => wrap(LetterWrap::new(sink), source),
                WrapMode::Word => wrap(WordWrap::new(sink), source),
            }
        }

        let mut total_height = 0.0;
        // Align lines according to desired alignment.
        for (i, line) in lines.iter_mut().enumerate() {
            let indent = if i == 0 { first_indent } else { normal_indent };
            match *self.horizontal_alignment {
                HorizontalAlignment::Left => line.x_offset = indent,
                HorizontalAlignment::Center => {
                    if self.constraint.x.is_infinite() {
                        line.x_offset = indent;
                    } else {
                        line.x_offset = 0.5 * (self.constraint.x - line.width).max(0.0);
                    }
                }
                HorizontalAlignment::Right => {
                    if self.constraint.x.is_infinite() {
                        line.x_offset = indent;
                    } else {
                        line.x_offset = (self.constraint.x - line.width - indent).max(0.0)
                    }
                }
                HorizontalAlignment::Stretch => line.x_offset = indent,
            }
        }
        // Calculate line height
        for line in lines.iter_mut() {
            if self.mask_char.is_some() || self.runs.is_empty() {
                line.height = GlyphMetrics {
                    font: &mut self.font.data_ref(),
                    size: **self.font_size,
                }
                .ascender();
            } else {
                for i in line.begin..line.end {
                    let h = GlyphMetrics {
                        font: &mut self.font_at(i).data_ref(),
                        size: self.font_size_at(i),
                    }
                    .ascender();
                    line.height = line.height.max(h);
                }
            }
            total_height += line.height + self.line_space();
        }
        total_height -= self.line_space();

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
        let mut y: f32 = cursor_y_start.floor();
        for line in lines.iter_mut() {
            let mut x = line.x_offset.floor();
            if let Some(mask) = *self.mask_char {
                let mut prev = None;
                let mut metrics = GlyphMetrics {
                    font: &mut self.font.data_ref(),
                    size: **self.font_size,
                };
                for c in std::iter::repeat_n(mask, line.len()) {
                    let (glyph, advance) =
                        build_glyph(&mut metrics, x, y, 0, c, prev, self.super_sampling_scale);
                    self.glyphs.push(glyph);
                    x += advance;
                    prev = Some(c);
                }
            } else {
                let mut prev = None;
                for (i, &c) in self.text.iter().enumerate().take(line.end).skip(line.begin) {
                    let font = self.font_at(i);
                    let font = &mut font.data_ref();
                    let mut metrics = GlyphMetrics {
                        font,
                        size: self.font_size_at(i),
                    };
                    match c {
                        '\n' => {
                            x += metrics.newline_advance();
                        }
                        _ => {
                            let y1 = y + line.height - metrics.ascender();
                            let scale = self.super_sampling_scale;
                            let (glyph, advance) =
                                build_glyph(&mut metrics, x, y1, i, c, prev, scale);
                            self.glyphs.push(glyph);
                            x += advance;
                        }
                    }
                    prev = Some(c);
                }
            }
            line.y_offset = y;
            y += line.height + self.line_space();
        }

        let size_x = if self.constraint.x.is_finite() {
            self.constraint.x
        } else {
            lines
                .iter()
                .map(|line| line.width)
                .max_by(f32::total_cmp)
                .unwrap_or_default()
        };
        let size_y = if self.constraint.y.is_finite() {
            self.constraint.y
        } else {
            let descender = if self.mask_char.is_some() || self.runs.is_empty() {
                GlyphMetrics {
                    font: &mut self.font.data_ref(),
                    size: **self.font_size,
                }
                .descender()
            } else if let Some(line) = self.lines.last() {
                (line.begin..line.end)
                    .map(|i| {
                        GlyphMetrics {
                            font: &mut self.font_at(i).data_ref(),
                            size: self.font_size_at(i),
                        }
                        .descender()
                    })
                    .min_by(f32::total_cmp)
                    .unwrap_or_default()
            } else {
                0.0
            };
            // Minus here is because descender has negative value.
            total_height - descender
        };
        self.lines = lines;
        Vector2::new(size_x, size_y)
    }
}

fn wrap<W, I>(mut wrapper: W, source: I)
where
    W: TextWrapper,
    I: Iterator<Item = (char, f32)>,
{
    for (character, advance) in source {
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
    text: Vec<char>,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    wrap: WrapMode,
    mask_char: Option<char>,
    shadow: bool,
    shadow_brush: Brush,
    shadow_dilation: f32,
    shadow_offset: Vector2<f32>,
    font_size: StyledProperty<f32>,
    super_sampling_scaling: f32,
    runs: Vec<Run>,
    /// The amount of indentation on the first line of the text.
    line_indent: f32,
    /// The space between lines.
    line_space: f32,
}

impl FormattedTextBuilder {
    /// Creates new formatted text builder with default parameters.
    pub fn new(font: FontResource) -> Self {
        Self {
            font,
            text: Vec::default(),
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
            font_size: 14.0f32.into(),
            super_sampling_scaling: 1.0,
            runs: Vec::default(),
            line_indent: 0.0,
            line_space: 0.0,
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
        self.text = text.chars().collect();
        self
    }

    pub fn with_chars(mut self, text: Vec<char>) -> Self {
        self.text = text;
        self
    }

    pub fn with_font_size(mut self, font_size: StyledProperty<f32>) -> Self {
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

    /// Sets desired super sampling scaling.
    pub fn with_super_sampling_scaling(mut self, scaling: f32) -> Self {
        self.super_sampling_scaling = scaling;
        self
    }

    /// Adds the given run to the text to set the style for a portion of the text.
    /// Later runs potentially overriding earlier runs if the ranges of the runs overlap and the later run
    /// sets a property that conflicts with an earlier run.
    pub fn with_run(mut self, run: Run) -> Self {
        self.runs.push(run);
        self
    }

    /// Adds multiple runs to the text to set the style of portions of the text.
    /// Later runs potentially overriding earlier runs if the ranges of the runs overlap and the later run
    /// sets a property that conflicts with an earlier run.
    pub fn with_runs<I: IntoIterator<Item = Run>>(mut self, runs: I) -> Self {
        self.runs.extend(runs);
        self
    }

    /// The amount of indent of the first line, horizontally separating it
    /// from the start of the remaining lines.
    /// If the indent is negative, then the first line will not be indented
    /// while all the other lines will be indented. By default this is 0.0.
    pub fn with_line_indent(mut self, indent: f32) -> Self {
        self.line_indent = indent;
        self
    }

    /// The space separating each line from the line above and below.
    /// By default this is 0.0.
    pub fn with_line_space(mut self, space: f32) -> Self {
        self.line_space = space;
        self
    }

    pub fn build(self) -> FormattedText {
        FormattedText {
            text: self.text.into(),
            lines: Vec::new(),
            glyphs: Vec::new(),
            vertical_alignment: self.vertical_alignment.into(),
            horizontal_alignment: self.horizontal_alignment.into(),
            brush: self.brush.into(),
            constraint: self.constraint,
            wrap: self.wrap.into(),
            mask_char: self.mask_char.into(),
            super_sampling_scale: self.super_sampling_scaling,
            font_size: self.font_size.into(),
            shadow: self.shadow.into(),
            shadow_brush: self.shadow_brush.into(),
            font: self.font.into(),
            shadow_dilation: self.shadow_dilation.into(),
            shadow_offset: self.shadow_offset.into(),
            runs: self.runs.into(),
            line_indent: self.line_indent.into(),
            line_space: self.line_space.into(),
        }
    }
}
