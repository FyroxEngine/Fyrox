use crate::{
    Thickness,
    formatted_text::FormattedText,
    core::{
        color::Color,
        math::{
            vec2::Vec2,
            Rect,
        },
    },
    core::math::TriangleDefinition,
    brush::Brush,
    ttf::Font
};
use std::{
    any::Any,
    sync::{Arc, Mutex}
};

#[repr(C)]
pub struct Vertex {
    pos: Vec2,
    tex_coord: Vec2,
}

impl Vertex {
    fn new(pos: Vec2, tex_coord: Vec2) -> Vertex {
        Vertex {
            pos,
            tex_coord,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum CommandKind {
    Geometry,
    Clip,
}

pub type Texture = dyn Any + Sync + Send;

#[derive(Clone)]
pub enum CommandTexture {
    None,
    Texture(Arc<Texture>),
    Font(Arc<Mutex<Font>>),
}

#[derive(Clone)]
pub struct Command {
    min: Vec2,
    max: Vec2,
    kind: CommandKind,
    brush: Brush,
    texture: CommandTexture,
    start_triangle: usize,
    triangle_count: usize,
    nesting: u8,
}

impl Command {
    #[inline]
    pub fn get_kind(&self) -> &CommandKind {
        &self.kind
    }

    #[inline]
    pub fn brush(&self) -> &Brush {
        &self.brush
    }

    #[inline]
    pub fn texture(&self) -> &CommandTexture {
        &self.texture
    }

    #[inline]
    pub fn min(&self) -> Vec2 {
        self.min
    }

    #[inline]
    pub fn max(&self) -> Vec2 {
        self.max
    }

    #[inline]
    pub fn get_start_triangle(&self) -> usize {
        self.start_triangle
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
    triangle_buffer: Vec<TriangleDefinition>,
    command_buffer: Vec<Command>,
    clip_cmd_stack: Vec<usize>,
    opacity_stack: Vec<f32>,
    triangles_to_commit: usize,
    current_nesting: u8,
}

fn get_line_thickness_vector(a: Vec2, b: Vec2, thickness: f32) -> Vec2 {
    if let Some(dir) = (b - a).normalized() {
        dir.perpendicular().scale(thickness * 0.5)
    } else {
        Vec2::ZERO
    }
}

impl Default for DrawingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawingContext {
    pub fn new() -> DrawingContext {
        DrawingContext {
            vertex_buffer: Vec::new(),
            triangle_buffer: Vec::new(),
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
        self.triangle_buffer.clear();
        self.command_buffer.clear();
        self.clip_cmd_stack.clear();
        self.opacity_stack.clear();
        self.triangles_to_commit = 0;
        self.current_nesting = 0;
    }

    #[inline]
    pub fn get_vertices(&self) -> &[Vertex] {
        self.vertex_buffer.as_slice()
    }

    #[inline]
    pub fn get_triangles(&self) -> &[TriangleDefinition] {
        self.triangle_buffer.as_slice()
    }

    #[inline]
    fn push_vertex(&mut self, pos: Vec2, tex_coord: Vec2) {
        self.vertex_buffer.push(Vertex::new(pos, tex_coord));
    }

    #[inline]
    pub fn set_nesting(&mut self, nesting: u8) {
        self.current_nesting = nesting;
    }

    #[inline]
    fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.triangle_buffer.push(TriangleDefinition { indices: [a, b, c] });
        self.triangles_to_commit += 1;
    }

    #[inline]
    fn index_origin(&self) -> u32 {
        match self.triangle_buffer.last() {
            Some(last) => last.indices.last().unwrap() + 1,
            None => 0
        }
    }

    #[inline]
    pub fn get_commands(&self) -> &Vec<Command> {
        &self.command_buffer
    }

    pub fn is_command_contains_point(&self, command: &Command, pos: Vec2) -> bool {
        let last = command.start_triangle + command.triangle_count;

        // Check each triangle from command for intersection with mouse pointer
        for j in command.start_triangle..last {
            let triangle = if let Some(triangle) = self.triangle_buffer.get(j) {
                triangle
            } else {
                return false;
            };

            let va = match self.vertex_buffer.get(triangle.indices[0] as usize) {
                Some(v) => v,
                None => return false
            };
            let vb = match self.vertex_buffer.get(triangle.indices[1] as usize) {
                Some(v) => v,
                None => return false
            };
            let vc = match self.vertex_buffer.get(triangle.indices[2] as usize) {
                Some(v) => v,
                None => return false
            };

            // Check if point is in triangle.
            let v0 = vc.pos - va.pos;
            let v1 = vb.pos - va.pos;
            let v2 = pos - va.pos;

            let dot00 = v0.dot(v0);
            let dot01 = v0.dot(v1);
            let dot02 = v0.dot(v2);
            let dot11 = v1.dot(v1);
            let dot12 = v1.dot(v2);

            let denom = dot00 * dot11 - dot01 * dot01;

            if denom <= std::f32::EPSILON {
                // We don't want floating-point exceptions
                return false;
            }

            let inv_denom = 1.0 / denom;
            let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
            let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

            if (u >= 0.0) && (v >= 0.0) && (u + v < 1.0) {
                return true;
            }
        }

        false
    }

    pub fn push_line(&mut self, a: Vec2, b: Vec2, thickness: f32) {
        let perp = get_line_thickness_vector(a, b, thickness);
        self.push_vertex(a - perp, Vec2::new(0.0, 0.0));
        self.push_vertex(b - perp, Vec2::new(1.0, 0.0));
        self.push_vertex(a + perp, Vec2::new(1.0, 1.0));
        self.push_vertex(b + perp, Vec2::new(0.0, 1.0));

        let index = self.index_origin();
        self.push_triangle(index, index + 1, index + 2);
        self.push_triangle(index + 2, index + 1, index + 3);
    }

    pub fn push_rect(&mut self, rect: &Rect<f32>, thickness: f32) {
        let offset = thickness * 0.5;

        let left_top = Vec2::new(rect.x + offset, rect.y + thickness);
        let right_top = Vec2::new(rect.x + rect.w - offset, rect.y + thickness);
        let right_bottom = Vec2::new(rect.x + rect.w - offset, rect.y + rect.h - thickness);
        let left_bottom = Vec2::new(rect.x + offset, rect.y + rect.h - thickness);
        let left_top_off = Vec2::new(rect.x, rect.y + offset);
        let right_top_off = Vec2::new(rect.x + rect.w, rect.y + offset);
        let right_bottom_off = Vec2::new(rect.x + rect.w, rect.y + rect.h - offset);
        let left_bottom_off = Vec2::new(rect.x, rect.y + rect.h - offset);

        // Horizontal lines
        self.push_line(left_top_off, right_top_off, thickness);
        self.push_line(right_bottom_off, left_bottom_off, thickness);

        // Vertical line
        self.push_line(right_top, right_bottom, thickness);
        self.push_line(left_bottom, left_top, thickness);
    }

    pub fn push_rect_vary(&mut self, rect: &Rect<f32>, thickness: Thickness) {
        let left_top = Vec2::new(rect.x + thickness.left * 0.5, rect.y + thickness.top);
        let right_top = Vec2::new(rect.x + rect.w - thickness.right * 0.5, rect.y + thickness.top);
        let right_bottom = Vec2::new(rect.x + rect.w - thickness.right * 0.5, rect.y + rect.h - thickness.bottom);
        let left_bottom = Vec2::new(rect.x + thickness.left * 0.5, rect.y + rect.h - thickness.bottom);
        let left_top_off = Vec2::new(rect.x, rect.y + thickness.top * 0.5);
        let right_top_off = Vec2::new(rect.x + rect.w, rect.y + thickness.top * 0.5);
        let right_bottom_off = Vec2::new(rect.x + rect.w, rect.y + rect.h - thickness.bottom * 0.5);
        let left_bottom_off = Vec2::new(rect.x, rect.y + rect.h - thickness.bottom * 0.5);

        // Horizontal lines
        self.push_line(left_top_off, right_top_off, thickness.top);
        self.push_line(right_bottom_off, left_bottom_off, thickness.bottom);

        // Vertical lines
        self.push_line(right_top, right_bottom, thickness.right);
        self.push_line(left_bottom, left_top, thickness.left);
    }

    pub fn push_rect_filled(&mut self, rect: &Rect<f32>, tex_coords: Option<&[Vec2; 4]>) {
        self.push_vertex(Vec2::new(rect.x, rect.y), tex_coords.map_or(Vec2::new(0.0, 0.0), |t| t[0]));
        self.push_vertex(Vec2::new(rect.x + rect.w, rect.y), tex_coords.map_or(Vec2::new(1.0, 0.0), |t| t[1]));
        self.push_vertex(Vec2::new(rect.x + rect.w, rect.y + rect.h), tex_coords.map_or(Vec2::new(1.0, 1.0), |t| t[2]));
        self.push_vertex(Vec2::new(rect.x, rect.y + rect.h), tex_coords.map_or(Vec2::new(0.0, 1.0), |t| t[3]));

        let index = self.index_origin();
        self.push_triangle(index, index + 1, index + 2);
        self.push_triangle(index, index + 2, index + 3);
    }

    pub fn push_circle(&mut self, origin: Vec2, radius: f32, segments: usize) {
        if segments >= 3 {
            self.push_vertex(origin, Vec2::ZERO);

            let two_pi = 2.0 * std::f32::consts::PI;
            let delta_angle = two_pi / (segments as f32);
            let mut angle = 0.0;
            while angle < two_pi {
                let x = origin.x + radius * angle.cos();
                let y = origin.y + radius * angle.sin();
                self.push_vertex(Vec2::new(x, y), Vec2::ZERO);
                angle += delta_angle;
            }

            let index = self.index_origin();
            for segment in 0..(segments - 1) {
                self.push_triangle(index, (segment - 1) as u32, segment as u32);
            }
        }
    }

    pub fn commit(&mut self, kind: CommandKind, brush: Brush, texture: CommandTexture) {
        if self.triangles_to_commit > 0 {
            let start_triangle = if !self.triangle_buffer.is_empty() {
                self.triangle_buffer.len() - self.triangles_to_commit
            } else {
                0
            };

            // Calculate bounds
            let mut min = Vec2::new(std::f32::MAX, std::f32::MAX);
            let mut max = Vec2::new(-std::f32::MAX, -std::f32::MAX);
            for i in start_triangle..(start_triangle + self.triangles_to_commit) {
                let triangle = &self.triangle_buffer[i];
                for k in triangle.indices.iter() {
                    let vertex = &self.vertex_buffer[*k as usize];

                    min.x = min.x.min(vertex.pos.x);
                    min.y = min.y.min(vertex.pos.y);

                    max.x = max.x.max(vertex.pos.x);
                    max.y = max.y.max(vertex.pos.y);
                }
            }

            let command = Command {
                min,
                max,
                kind,
                brush,
                texture,
                nesting: self.current_nesting,
                start_triangle,
                triangle_count: self.triangles_to_commit,
            };

            self.command_buffer.push(command);
            self.triangles_to_commit = 0;
        }
    }

    pub fn draw_text(&mut self, position: Vec2, formatted_text: &FormattedText) {
        let font = if let Some(font) = formatted_text.get_font() {
            font
        } else {
            println!("Trying to draw text without font!");
            return;
        };

        for element in formatted_text.get_glyphs() {
            let bounds = element.get_bounds();

            let final_bounds = Rect::new(
                position.x + bounds.x, position.y + bounds.y,
                bounds.w, bounds.h);

            self.push_rect_filled(&final_bounds, Some(element.get_tex_coords()));
        }

        self.commit(CommandKind::Geometry, formatted_text.brush(), CommandTexture::Font(font))
    }

    pub fn commit_clip_rect(&mut self, clip_rect: &Rect<f32>) {
        self.push_rect_filled(clip_rect, None);
        let index = self.command_buffer.len();
        self.commit(CommandKind::Clip, Brush::Solid(Color::WHITE), CommandTexture::None);
        self.clip_cmd_stack.push(index);
    }

    pub fn ready_to_draw(&self) -> bool {
        self.clip_cmd_stack.is_empty() && self.triangles_to_commit == 0 && self.opacity_stack.is_empty()
    }

    pub fn revert_clip_geom(&mut self) {
        // Remove last clip command index
        self.clip_cmd_stack.pop();
        if let Some(last_index) = self.clip_cmd_stack.last() {
            if let Some(last_clip_command) = self.command_buffer.get(*last_index) {
                assert_eq!(last_clip_command.kind, CommandKind::Clip);
                // Re-commit last clipping command
                let clip_command = last_clip_command.clone();
                self.command_buffer.push(clip_command);
            }
        }
    }
}
