use crate::math::vec2::Vec2;
use crate::math::Rect;
use crate::gui::Thickness;
use crate::resource::ttf::Font;
use std::os::raw::c_void;

#[derive(Copy, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn white() -> Color {
        Color { r: 255, g: 255, b: 255, a: 255 }
    }

    pub fn black() -> Color {
        Color { r: 0, g: 0, b: 0, a: 255 }
    }
}

#[repr(C)]
pub struct Vertex {
    pos: Vec2,
    tex_coord: Vec2,
    color: Color,
}

impl Vertex {
    fn new(pos: Vec2, tex_coord: Vec2, color: Color) -> Vertex {
        Vertex {
            pos,
            tex_coord,
            color,
        }
    }
}

pub enum CommandKind {
    Geometry,
    Clip,
}

pub struct Command {
    kind: CommandKind,
    texture: u32,
    index_offset: usize,
    triangle_count: usize,
    nesting: u8,
}

impl Command {
    #[inline]
    pub fn get_kind(&self) -> &CommandKind {
        &self.kind
    }

    #[inline]
    pub fn get_texture(&self) -> u32 {
        self.texture
    }

    #[inline]
    pub fn get_index_offset(&self) -> usize {
        self.index_offset
    }

    #[inline]
    pub fn get_triangle_count(&self) -> usize {
        self.triangle_count
    }

    #[inline]
    pub fn get_nesting(&self) -> u8 {
        self.nesting
    }
}

pub struct DrawingContext {
    vertex_buffer: Vec<Vertex>,
    index_buffer: Vec<i32>,
    command_buffer: Vec<Command>,
    clip_cmd_stack: Vec<i32>,
    opacity_stack: Vec<f32>,
    triangles_to_commit: usize,
    current_nesting: u8,
}

struct TextElement {
    bounds: Rect<f32>,
    tex_coords: [Vec2; 4],
    color: Color,
}

pub struct FormattedText {
    texture: u32,
    elements: Vec<TextElement>
}

impl FormattedText {
    pub fn new() -> FormattedText {
        FormattedText {
            texture: 0,
            elements: Vec::new(),
        }
    }

    pub fn set_text(&mut self, text: &str, font: &Font, pos: &Vec2, color: Color) {
        self.elements.clear();
        self.texture = font.get_texture_id();
        let mut cursor = *pos;
        for code in text.chars() {
            match font.get_glyph(code) {
                Some(glyph) => {
                    if glyph.has_outline() {
                        let rect = Rect {
                            x: cursor.x + glyph.get_bitmap_left(),
                            y: cursor.y + font.get_ascender() - glyph.get_bitmap_top() - glyph.get_bitmap_height(),
                            w: glyph.get_bitmap_width(),
                            h: glyph.get_bitmap_height(),
                        };
                        self.elements.push(TextElement {
                            bounds: rect,
                            tex_coords: glyph.get_tex_coords().clone(),
                            color
                        });
                    }
                    cursor.x += glyph.get_advance();
                }
                None => {
                    let rect = Rect {
                        x: cursor.x,
                        y: cursor.y + font.get_ascender(),
                        w: font.get_height(),
                        h: font.get_height(),
                    };
                    self.elements.push(TextElement {
                        bounds: rect,
                        tex_coords: [Vec2::new(); 4],
                        color
                    });
                    cursor.x += rect.w;
                }
            }
        }
    }
}

fn get_line_thickness_vector(a: &Vec2, b: &Vec2, thickness: f32) -> Vec2 {
    if let Some(dir) = (*b - *a).normalized() {
        dir.perpendicular().scale(thickness * 0.5)
    } else {
        Vec2::new()
    }
}

impl DrawingContext {
    pub fn new() -> DrawingContext {
        DrawingContext {
            vertex_buffer: Vec::new(),
            index_buffer: Vec::new(),
            command_buffer: Vec::new(),
            clip_cmd_stack: Vec::new(),
            opacity_stack: Vec::new(),
            triangles_to_commit: 0,
            current_nesting: 0,
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.vertex_buffer.clear();
        self.index_buffer.clear();
        self.command_buffer.clear();
        self.clip_cmd_stack.clear();
        self.opacity_stack.clear();
        self.triangles_to_commit = 0;
        self.current_nesting = 0;
    }


    #[inline]
    pub fn get_command_buffer(&self) -> &[Command] {
        self.command_buffer.as_slice()
    }

    #[inline]
    pub fn get_vertices(&self) -> &[Vertex] {
        self.vertex_buffer.as_slice()
    }

    #[inline]
    pub fn get_indices(&self) -> &[i32] {
        self.index_buffer.as_slice()
    }

    #[inline]
    pub fn get_vertices_ptr(&self) -> *const c_void {
        self.vertex_buffer.as_ptr() as *const c_void
    }

    #[inline]
    pub fn get_indices_ptr(&self) -> *const c_void {
        self.index_buffer.as_ptr() as *const c_void
    }

    #[inline]
    pub fn get_vertices_bytes(&self) -> isize {
        (self.vertex_buffer.len() * std::mem::size_of::<Vertex>()) as isize
    }

    #[inline]
    pub fn get_vertex_size(&self) -> i32 {
        std::mem::size_of::<Vertex>() as i32
    }

    #[inline]
    pub fn get_index_size(&self) -> i32 {
        std::mem::size_of::<i32>() as i32
    }

    #[inline]
    pub fn get_indices_bytes(&self) -> isize {
        (self.index_buffer.len() * std::mem::size_of::<i32>()) as isize
    }

    #[inline]
    fn push_vertex(&mut self, pos: Vec2, tex_coord: Vec2, color: Color) {
        self.vertex_buffer.push(Vertex::new(pos, tex_coord, color));
    }

    #[inline]
    fn push_triangle(&mut self, a: i32, b: i32, c: i32) {
        self.index_buffer.push(a);
        self.index_buffer.push(b);
        self.index_buffer.push(c);
        self.triangles_to_commit += 1;
    }

    #[inline]
    fn get_index_origin(&self) -> i32 {
        if self.index_buffer.len() > 0 {
            self.index_buffer.last().unwrap() + 1
        } else {
            0
        }
    }

    pub fn push_line(&mut self, a: &Vec2, b: &Vec2, thickness: f32, color: Color) {
        let perp = get_line_thickness_vector(a, b, thickness);
        self.push_vertex(*a - perp, Vec2::make(0.0, 0.0), color);
        self.push_vertex(*b - perp, Vec2::make(1.0, 0.0), color);
        self.push_vertex(*a + perp, Vec2::make(1.0, 1.0), color);
        self.push_vertex(*b + perp, Vec2::make(0.0, 1.0), color);

        let index = self.get_index_origin();
        self.push_triangle(index, index + 1, index + 2);
        self.push_triangle(index, index + 2, index + 3);
    }

    pub fn push_rect(&mut self, rect: &Rect<f32>, thickness: f32, color: Color) {
        let offset = thickness * 0.5;

        let left_top = Vec2::make(rect.x + offset, rect.y + thickness);
        let right_top = Vec2::make(rect.x + rect.w - offset, rect.y + thickness);
        let right_bottom = Vec2::make(rect.x + rect.w - offset, rect.y + rect.h - thickness);
        let left_bottom = Vec2::make(rect.x + offset, rect.y + rect.h - thickness);
        let left_top_off = Vec2::make(rect.x, rect.y + offset);
        let right_top_off = Vec2::make(rect.x + rect.w, rect.y + offset);
        let right_bottom_off = Vec2::make(rect.x + rect.w, rect.y + rect.h - offset);
        let left_bottom_off = Vec2::make(rect.x, rect.y + rect.h - offset);

        // Horizontal lines
        self.push_line(&left_top_off, &right_top_off, thickness, color);
        self.push_line(&right_bottom_off, &left_bottom_off, thickness, color);

        // Vertical lines
        self.push_line(&right_top, &right_bottom, thickness, color);
        self.push_line(&left_bottom, &left_top, thickness, color);
    }

    pub fn push_rect_vary(&mut self, rect: &Rect<f32>, thickness: Thickness, color: Color) {
        let left_top = Vec2::make(rect.x + thickness.left * 0.5, rect.y + thickness.top);
        let right_top = Vec2::make(rect.x + rect.w - thickness.right * 0.5, rect.y + thickness.top);
        let right_bottom = Vec2::make(rect.x + rect.w - thickness.right * 0.5, rect.y + rect.h - thickness.bottom);
        let left_bottom = Vec2::make(rect.x + thickness.left * 0.5, rect.y + rect.h - thickness.bottom);
        let left_top_off = Vec2::make(rect.x, rect.y + thickness.top * 0.5);
        let right_top_off = Vec2::make(rect.x + rect.w, rect.y + thickness.top * 0.5);
        let right_bottom_off = Vec2::make(rect.x + rect.w, rect.y + rect.h - thickness.bottom * 0.5);
        let left_bottom_off = Vec2::make(rect.x, rect.y + rect.h - thickness.bottom * 0.5);

        // Horizontal lines
        self.push_line(&left_top_off, &right_top_off, thickness.top, color);
        self.push_line(&right_bottom_off, &left_bottom_off, thickness.bottom, color);

        // Vertical lines
        self.push_line(&right_top, &right_bottom, thickness.right, color);
        self.push_line(&left_bottom, &left_top, thickness.left, color);
    }

    pub fn push_rect_filled(&mut self, rect: &Rect<f32>, tex_coords: Option<&[Vec2; 4]>, color: Color) {
        self.push_vertex(Vec2::make(rect.x, rect.y), tex_coords.map_or(Vec2::make(0.0, 0.0), |t| t[0]), color);
        self.push_vertex(Vec2::make(rect.x + rect.w, rect.y), tex_coords.map_or(Vec2::make(1.0, 0.0), |t| t[1]), color);
        self.push_vertex(Vec2::make(rect.x + rect.w, rect.y + rect.h), tex_coords.map_or(Vec2::make(1.0, 1.0), |t| t[2]), color);
        self.push_vertex(Vec2::make(rect.x, rect.y + rect.h), tex_coords.map_or(Vec2::make(0.0, 1.0), |t| t[3]), color);

        let index = self.get_index_origin();
        self.push_triangle(index, index + 1, index + 2);
        self.push_triangle(index, index + 2, index + 3);
    }

    pub fn commit(&mut self, kind: CommandKind, texture: u32) {
        if self.triangles_to_commit > 0 {
            self.command_buffer.push(Command {
                kind,
                texture,
                nesting: self.current_nesting,
                index_offset: if self.index_buffer.len() > 0 {
                    self.index_buffer.len() - self.triangles_to_commit * 3
                } else {
                    0
                },
                triangle_count: self.triangles_to_commit,
            });
            self.triangles_to_commit = 0;
        }
    }

    pub fn draw_text(&mut self, formatted_text: &FormattedText) {
        for element in formatted_text.elements.iter() {
            self.push_rect_filled(&element.bounds, Some(&element.tex_coords), element.color);
        }
        self.commit(CommandKind::Geometry, formatted_text.texture);
    }
}