use crate::core::algebra::Vector2;
use crate::{
    brush::Brush,
    core::{
        color::Color,
        math::{self, Rect, TriangleDefinition},
    },
    formatted_text::FormattedText,
    ttf::SharedFont,
    Thickness,
};
use std::{any::Any, ops::Deref, ops::Range, sync::Arc};

#[derive(Clone)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vector2<f32>,
    pub tex_coord: Vector2<f32>,
    pub color: Color,
}

impl Vertex {
    fn new(pos: Vector2<f32>, tex_coord: Vector2<f32>) -> Vertex {
        Vertex {
            pos,
            tex_coord,
            color: Color::WHITE,
        }
    }
}

pub type Texture = dyn Any + Sync + Send;

#[derive(Debug, Clone)]
pub struct SharedTexture(pub Arc<Texture>);

impl<T: Any + Sync + Send> From<Arc<T>> for SharedTexture {
    fn from(arc: Arc<T>) -> Self {
        SharedTexture(arc)
    }
}

#[allow(clippy::vtable_address_comparisons)]
impl PartialEq for SharedTexture {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.deref(), other.0.deref())
    }
}

#[derive(Clone)]
pub enum CommandTexture {
    None,
    Texture(SharedTexture),
    Font(SharedFont),
}

/// A set of triangles that will be used for clipping.
#[derive(Clone)]
pub struct ClippingGeometry {
    pub vertex_buffer: Vec<Vertex>,
    pub triangle_buffer: Vec<TriangleDefinition>,
}

impl Draw for ClippingGeometry {
    #[inline(always)]
    fn push_vertex(&mut self, pos: Vector2<f32>, tex_coord: Vector2<f32>) {
        self.vertex_buffer.push(Vertex::new(pos, tex_coord));
    }

    #[inline(always)]
    fn push_vertex_raw(&mut self, vertex: Vertex) {
        self.vertex_buffer.push(vertex);
    }

    #[inline(always)]
    fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.triangle_buffer.push(TriangleDefinition([a, b, c]));
    }

    #[inline(always)]
    fn last_vertex_index(&self) -> u32 {
        self.vertex_buffer.len() as u32
    }
}

impl ClippingGeometry {
    pub fn is_contains_point(&self, pos: Vector2<f32>) -> bool {
        for triangle in self.triangle_buffer.iter() {
            if let Some((va, vb, vc)) = self.triangle_points(triangle) {
                if math::is_point_inside_2d_triangle(pos, va.pos, vb.pos, vc.pos) {
                    return true;
                }
            }
        }

        false
    }

    pub fn triangle_points(
        &self,
        triangle: &TriangleDefinition,
    ) -> Option<(&Vertex, &Vertex, &Vertex)> {
        let a = self.vertex_buffer.get(triangle[0] as usize)?;
        let b = self.vertex_buffer.get(triangle[1] as usize)?;
        let c = self.vertex_buffer.get(triangle[2] as usize)?;
        Some((a, b, c))
    }
}

#[derive(Clone)]
pub struct Command {
    /// Clipping bounds, should be used for scissor-test. Screen-space.
    pub clip_bounds: Rect<f32>,
    /// Total bounds of command's geometry. Screen-space.
    pub bounds: Rect<f32>,
    /// Brush defines visual appearance of rendered geometry.
    pub brush: Brush,
    pub texture: CommandTexture,
    pub triangles: Range<usize>,
    pub opacity: f32,
    /// A set of triangles that defines clipping region.
    pub clipping_geometry: Option<ClippingGeometry>,
}

pub trait Draw {
    fn push_vertex(&mut self, pos: Vector2<f32>, tex_coord: Vector2<f32>);

    fn push_vertex_raw(&mut self, vertex: Vertex);

    fn push_triangle(&mut self, a: u32, b: u32, c: u32);

    fn last_vertex_index(&self) -> u32;

    fn push_triangle_multicolor(&mut self, vertices: [(Vector2<f32>, Color); 3]) {
        let index = self.last_vertex_index();
        for &(pos, color) in &vertices {
            self.push_vertex_raw(Vertex {
                pos,
                tex_coord: Vector2::new(0.0, 0.0),
                color,
            });
        }

        self.push_triangle(index, index + 1, index + 2);
    }

    fn push_triangle_filled(&mut self, vertices: [Vector2<f32>; 3]) {
        let index = self.last_vertex_index();

        for &pos in &vertices {
            self.push_vertex(pos, Default::default());
        }

        self.push_triangle(index, index + 1, index + 2);
    }

    fn push_line(&mut self, a: Vector2<f32>, b: Vector2<f32>, thickness: f32) {
        let index = self.last_vertex_index();
        let perp = get_line_thickness_vector(a, b, thickness);
        self.push_vertex(a - perp, Vector2::new(0.0, 0.0));
        self.push_vertex(b - perp, Vector2::new(1.0, 0.0));
        self.push_vertex(a + perp, Vector2::new(1.0, 1.0));
        self.push_vertex(b + perp, Vector2::new(0.0, 1.0));

        self.push_triangle(index, index + 1, index + 2);
        self.push_triangle(index + 2, index + 1, index + 3);
    }

    fn push_rect(&mut self, rect: &Rect<f32>, thickness: f32) {
        let offset = thickness * 0.5;

        let left_top = Vector2::new(rect.x() + offset, rect.y() + thickness);
        let right_top = Vector2::new(rect.x() + rect.w() - offset, rect.y() + thickness);
        let right_bottom = Vector2::new(
            rect.x() + rect.w() - offset,
            rect.y() + rect.h() - thickness,
        );
        let left_bottom = Vector2::new(rect.x() + offset, rect.y() + rect.h() - thickness);
        let left_top_off = Vector2::new(rect.x(), rect.y() + offset);
        let right_top_off = Vector2::new(rect.x() + rect.w(), rect.y() + offset);
        let right_bottom_off = Vector2::new(rect.x() + rect.w(), rect.y() + rect.h() - offset);
        let left_bottom_off = Vector2::new(rect.x(), rect.y() + rect.h() - offset);

        // Horizontal lines
        self.push_line(left_top_off, right_top_off, thickness);
        self.push_line(right_bottom_off, left_bottom_off, thickness);

        // Vertical line
        self.push_line(right_top, right_bottom, thickness);
        self.push_line(left_bottom, left_top, thickness);
    }

    fn push_rect_vary(&mut self, rect: &Rect<f32>, thickness: Thickness) {
        let left_top = Vector2::new(rect.x() + thickness.left * 0.5, rect.y() + thickness.top);
        let right_top = Vector2::new(
            rect.x() + rect.w() - thickness.right * 0.5,
            rect.y() + thickness.top,
        );
        let right_bottom = Vector2::new(
            rect.x() + rect.w() - thickness.right * 0.5,
            rect.y() + rect.h() - thickness.bottom,
        );
        let left_bottom = Vector2::new(
            rect.x() + thickness.left * 0.5,
            rect.y() + rect.h() - thickness.bottom,
        );
        let left_top_off = Vector2::new(rect.x(), rect.y() + thickness.top * 0.5);
        let right_top_off = Vector2::new(rect.x() + rect.w(), rect.y() + thickness.top * 0.5);
        let right_bottom_off = Vector2::new(
            rect.x() + rect.w(),
            rect.y() + rect.h() - thickness.bottom * 0.5,
        );
        let left_bottom_off = Vector2::new(rect.x(), rect.y() + rect.h() - thickness.bottom * 0.5);

        // Horizontal lines
        self.push_line(left_top_off, right_top_off, thickness.top);
        self.push_line(right_bottom_off, left_bottom_off, thickness.bottom);

        // Vertical lines
        self.push_line(right_top, right_bottom, thickness.right);
        self.push_line(left_bottom, left_top, thickness.left);
    }

    fn push_rect_filled(&mut self, rect: &Rect<f32>, tex_coords: Option<&[Vector2<f32>; 4]>) {
        let index = self.last_vertex_index();
        self.push_vertex(
            Vector2::new(rect.x(), rect.y()),
            tex_coords.map_or(Vector2::new(0.0, 0.0), |t| t[0]),
        );
        self.push_vertex(
            Vector2::new(rect.x() + rect.w(), rect.y()),
            tex_coords.map_or(Vector2::new(1.0, 0.0), |t| t[1]),
        );
        self.push_vertex(
            Vector2::new(rect.x() + rect.w(), rect.y() + rect.h()),
            tex_coords.map_or(Vector2::new(1.0, 1.0), |t| t[2]),
        );
        self.push_vertex(
            Vector2::new(rect.x(), rect.y() + rect.h()),
            tex_coords.map_or(Vector2::new(0.0, 1.0), |t| t[3]),
        );

        self.push_triangle(index, index + 1, index + 2);
        self.push_triangle(index, index + 2, index + 3);
    }

    fn push_rect_multicolor(&mut self, rect: &Rect<f32>, colors: [Color; 4]) {
        let index = self.last_vertex_index();
        self.push_vertex_raw(Vertex {
            pos: rect.left_top_corner(),
            tex_coord: Vector2::new(0.0, 0.0),
            color: colors[0],
        });
        self.push_vertex_raw(Vertex {
            pos: rect.right_top_corner(),
            tex_coord: Vector2::new(1.0, 0.0),
            color: colors[1],
        });
        self.push_vertex_raw(Vertex {
            pos: rect.right_bottom_corner(),
            tex_coord: Vector2::new(1.0, 1.0),
            color: colors[2],
        });
        self.push_vertex_raw(Vertex {
            pos: rect.left_bottom_corner(),
            tex_coord: Vector2::new(0.0, 1.0),
            color: colors[3],
        });

        self.push_triangle(index, index + 1, index + 2);
        self.push_triangle(index, index + 2, index + 3);
    }

    fn push_circle(&mut self, origin: Vector2<f32>, radius: f32, segments: usize, color: Color) {
        if segments >= 3 {
            let center_index = self.last_vertex_index();

            self.push_vertex_raw(Vertex {
                pos: origin,
                tex_coord: Vector2::default(),
                color,
            });

            let two_pi = 2.0 * std::f32::consts::PI;
            let delta_angle = two_pi / (segments as f32);
            let mut angle: f32 = 0.0;
            for _ in 0..segments {
                let x = origin.x + radius * angle.cos();
                let y = origin.y + radius * angle.sin();
                self.push_vertex_raw(Vertex {
                    pos: Vector2::new(x, y),
                    tex_coord: Vector2::default(),
                    color,
                });
                angle += delta_angle;
            }

            let first_vertex = center_index + 1;
            for segment in 0..segments {
                self.push_triangle(
                    center_index,
                    first_vertex + segment as u32,
                    first_vertex + (segment as u32 + 1) % segments as u32,
                );
            }
        }
    }
}

pub struct DrawingContext {
    vertex_buffer: Vec<Vertex>,
    triangle_buffer: Vec<TriangleDefinition>,
    command_buffer: Vec<Command>,
    opacity_stack: Vec<f32>,
    triangles_to_commit: usize,
}

fn get_line_thickness_vector(a: Vector2<f32>, b: Vector2<f32>, thickness: f32) -> Vector2<f32> {
    if let Some(dir) = (b - a).try_normalize(f32::EPSILON) {
        Vector2::new(dir.y, -dir.x).scale(thickness * 0.5)
    } else {
        Vector2::default()
    }
}

impl Default for DrawingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Draw for DrawingContext {
    #[inline(always)]
    fn push_vertex(&mut self, pos: Vector2<f32>, tex_coord: Vector2<f32>) {
        self.vertex_buffer.push(Vertex::new(pos, tex_coord));
    }

    #[inline(always)]
    fn push_vertex_raw(&mut self, vertex: Vertex) {
        self.vertex_buffer.push(vertex);
    }

    #[inline(always)]
    fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.triangle_buffer.push(TriangleDefinition([a, b, c]));
        self.triangles_to_commit += 1;
    }

    #[inline(always)]
    fn last_vertex_index(&self) -> u32 {
        self.vertex_buffer.len() as u32
    }
}

impl DrawingContext {
    pub fn new() -> DrawingContext {
        DrawingContext {
            vertex_buffer: Vec::new(),
            triangle_buffer: Vec::new(),
            command_buffer: Vec::new(),
            triangles_to_commit: 0,
            opacity_stack: vec![1.0],
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.vertex_buffer.clear();
        self.triangle_buffer.clear();
        self.command_buffer.clear();
        self.opacity_stack.clear();
        self.opacity_stack.push(1.0);
        self.triangles_to_commit = 0;
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
    pub fn get_commands(&self) -> &Vec<Command> {
        &self.command_buffer
    }

    pub fn push_opacity(&mut self, opacity: f32) {
        self.opacity_stack.push(opacity);
    }

    pub fn pop_opacity(&mut self) {
        self.opacity_stack.pop().unwrap();
    }

    pub fn triangle_points(
        &self,
        triangle: &TriangleDefinition,
    ) -> Option<(&Vertex, &Vertex, &Vertex)> {
        let a = self.vertex_buffer.get(triangle[0] as usize)?;
        let b = self.vertex_buffer.get(triangle[1] as usize)?;
        let c = self.vertex_buffer.get(triangle[2] as usize)?;
        Some((a, b, c))
    }

    pub fn is_command_contains_point(&self, command: &Command, pos: Vector2<f32>) -> bool {
        for i in command.triangles.clone() {
            if let Some(triangle) = self.triangle_buffer.get(i) {
                if let Some((va, vb, vc)) = self.triangle_points(triangle) {
                    if math::is_point_inside_2d_triangle(pos, va.pos, vb.pos, vc.pos) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn pending_range(&self) -> Range<usize> {
        if self.triangle_buffer.is_empty() {
            0..self.triangles_to_commit
        } else {
            (self.triangle_buffer.len() - self.triangles_to_commit)..self.triangle_buffer.len()
        }
    }

    fn bounds_of(&self, range: Range<usize>) -> Rect<f32> {
        let mut bounds = Rect::new(f32::MAX, f32::MAX, 0.0, 0.0);
        for i in range {
            for &k in self.triangle_buffer[i].as_ref() {
                bounds.push(self.vertex_buffer[k as usize].pos);
            }
        }
        bounds
    }

    pub fn commit(
        &mut self,
        clip_bounds: Rect<f32>,
        brush: Brush,
        texture: CommandTexture,
        clipping_geometry: Option<ClippingGeometry>,
    ) {
        if self.triangles_to_commit > 0 {
            let triangles = self.pending_range();
            let bounds = self.bounds_of(triangles.clone());

            let opacity = *self.opacity_stack.last().unwrap();
            self.command_buffer.push(Command {
                clip_bounds,
                bounds,
                brush,
                texture,
                triangles,
                opacity,
                clipping_geometry,
            });
            self.triangles_to_commit = 0;
        }
    }

    pub fn draw_text(
        &mut self,
        clip_bounds: Rect<f32>,
        position: Vector2<f32>,
        formatted_text: &FormattedText,
    ) {
        let font = formatted_text.get_font();

        for element in formatted_text.get_glyphs() {
            let bounds = element.get_bounds();

            let final_bounds = Rect::new(
                position.x + bounds.x(),
                position.y + bounds.y(),
                bounds.w(),
                bounds.h(),
            );

            self.push_rect_filled(&final_bounds, Some(element.get_tex_coords()));
        }

        self.commit(
            clip_bounds,
            formatted_text.brush(),
            CommandTexture::Font(font),
            None,
        )
    }
}
