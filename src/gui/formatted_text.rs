use rg3d_core::{
    color::Color,
    math::vec2::Vec2,
    math::Rect
};
use std::{
    rc::Rc,
    cell::RefCell
};
use crate::{
    resource::ttf::Font,
    gui::{HorizontalAlignment, VerticalAlignment}
};

#[derive(Debug)]
pub struct TextGlyph {
    bounds: Rect<f32>,
    tex_coords: [Vec2; 4],
    color: Color,
}

impl TextGlyph {
    pub fn get_bounds(&self) -> Rect<f32> {
        self.bounds
    }

    pub fn get_tex_coords(&self) -> &[Vec2; 4] {
        &self.tex_coords
    }

    pub fn get_color(&self) -> Color {
        self.color
    }
}

#[derive(Copy, Clone, Debug)]
struct TextLine {
    begin: usize,
    end: usize,
    width: f32,
    x_offset: f32,
}

impl TextLine {
    fn new() -> TextLine {
        TextLine {
            begin: 0,
            end: 0,
            width: 0.0,
            x_offset: 0.0,
        }
    }
}

pub struct FormattedText {
    font: Rc<RefCell<Font>>,
    /// Text in UTF32 format.
    text: Vec<u32>,
    /// Temporary buffer used to split text on lines. We need it to reduce memory allocations
    /// when we changing text too frequently, here we sacrifice some memory in order to get
    /// more performance.
    lines: Vec<TextLine>,
    /// Final glyphs for draw buffer.
    glyphs: Vec<TextGlyph>,
}

impl FormattedText {
    fn new(font: Rc<RefCell<Font>>) -> FormattedText {
        FormattedText {
            text: Vec::new(),
            font,
            glyphs: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn get_glyphs(&self) -> &[TextGlyph] {
        &self.glyphs
    }

    pub fn get_font(&self) -> Rc<RefCell<Font>> {
        self.font.clone()
    }

    fn build(&mut self, text: &str, size: Vec2, color: Color, vertical_alignment: VerticalAlignment,
             horizontal_alignment: HorizontalAlignment) {
        // Convert text to UTF32.
        self.text.clear();
        for code in text.chars().map(|c| c as u32) {
            self.text.push(code);
        }

        let font = self.font.borrow();

        // Split on lines.
        let mut total_height = 0.0;
        let mut current_line = TextLine::new();
        self.lines.clear();
        for (i, code) in self.text.iter().enumerate() {
            let advance =
                match font.get_glyph(*code) {
                    Some(glyph) => glyph.get_advance(),
                    None => font.get_height()
                };
            let is_new_line = *code == u32::from(b'\n') || *code == u32::from(b'\r');
            let new_width = current_line.width + advance;
            if new_width > size.x || is_new_line {
                self.lines.push(current_line);
                current_line.begin = if is_new_line { i + 1 } else { i };
                current_line.end = current_line.begin + 1;
                current_line.width = advance;
                total_height += font.get_ascender();
            } else {
                current_line.width = new_width;
                current_line.end += 1;
            }
        }
        // Commit rest of text.
        if current_line.begin != current_line.end {
            current_line.end = self.text.len();
            self.lines.push(current_line);
            total_height += font.get_ascender();
        }

        // Align lines according to desired alignment.
        for line in self.lines.iter_mut() {
            match horizontal_alignment {
                HorizontalAlignment::Left => line.x_offset = 0.0,
                HorizontalAlignment::Center => line.x_offset = 0.5 * (size.x - line.width),
                HorizontalAlignment::Right => line.x_offset = size.x - line.width,
                HorizontalAlignment::Stretch => line.x_offset = 0.0
            }
        }

        // Generate glyphs for each text line.
        self.glyphs.clear();

        let cursor_y_start = match vertical_alignment {
            VerticalAlignment::Top => 0.0,
            VerticalAlignment::Center => (size.y - total_height) * 0.5,
            VerticalAlignment::Bottom => size.y - total_height,
            VerticalAlignment::Stretch => 0.0
        };

        let mut cursor = Vec2::make(size.x, cursor_y_start);
        for line in self.lines.iter() {
            cursor.x = line.x_offset;

            for code_index in line.begin..line.end {
                let code = self.text[code_index];

                match font.get_glyph(code) {
                    Some(glyph) => {
                        // Insert glyph
                        if glyph.has_outline() {
                            let rect = Rect {
                                x: cursor.x + glyph.get_bitmap_left(),
                                y: cursor.y + font.get_ascender() - glyph.get_bitmap_top() - glyph.get_bitmap_height(),
                                w: glyph.get_bitmap_width(),
                                h: glyph.get_bitmap_height(),
                            };
                            let text_glyph = TextGlyph {
                                bounds: rect,
                                tex_coords: *glyph.get_tex_coords(),
                                color,
                            };
                            self.glyphs.push(text_glyph);
                        }
                        cursor.x += glyph.get_advance();
                    }
                    None => {
                        // Insert invalid symbol
                        let rect = Rect {
                            x: cursor.x,
                            y: cursor.y + font.get_ascender(),
                            w: font.get_height(),
                            h: font.get_height(),
                        };
                        self.glyphs.push(TextGlyph {
                            bounds: rect,
                            tex_coords: [Vec2::zero(); 4],
                            color,
                        });
                        cursor.x += rect.w;
                    }
                }
            }

            cursor.y += font.get_ascender();
        }
    }
}

pub struct FormattedTextBuilder<'a> {
    color: Color,
    size: Vec2,
    text: Option<&'a str>,
    formatted_text: FormattedText,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
}

impl<'a> FormattedTextBuilder<'a> {
    /// Creates new formatted text builder with default parameters.
    pub fn new(font: Rc<RefCell<Font>>) -> FormattedTextBuilder<'a> {
        FormattedTextBuilder {
            text: None,
            formatted_text: FormattedText::new(font),
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            color: Color::white(),
            size: Vec2::make(128.0, 128.0),
        }
    }

    /// Creates new formatted text builder that will reuse existing
    /// buffers from existing formatted text. This is very useful to
    /// reduce memory allocations.
    pub fn reuse(formatted_text: FormattedText) -> FormattedTextBuilder<'a> {
        FormattedTextBuilder {
            text: None,
            formatted_text: FormattedText {
                // Take buffers out and reuse them so no need to allocate new
                // buffers every time when need to change a text.
                text: formatted_text.text,
                lines: formatted_text.lines,
                glyphs: formatted_text.glyphs,
                font: formatted_text.font,
            },
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            color: Color::white(),
            size: Vec2::make(128.0, 128.0),
        }
    }

    pub fn with_vertical_alignment(mut self, vertical_alignment: VerticalAlignment) -> Self {
        self.vertical_alignment = vertical_alignment;
        self
    }

    pub fn with_horizontal_alignment(mut self, horizontal_alignment: HorizontalAlignment) -> Self {
        self.horizontal_alignment = horizontal_alignment;
        self
    }

    pub fn with_text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }

    pub fn with_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn build(mut self) -> FormattedText {
        if let Some(text) = self.text {
            self.formatted_text.build(
                text,
                self.size,
                self.color,
                self.vertical_alignment,
                self.horizontal_alignment,
            );
        }

        self.formatted_text
    }
}