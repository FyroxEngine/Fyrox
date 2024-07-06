use crate::font::FontHeight;
use crate::{
    brush::Brush,
    core::{
        algebra::{Matrix3, Point2, Vector2},
        color::Color,
        math::{self, Rect, TriangleDefinition},
    },
    font::FontResource,
    formatted_text::FormattedText,
    Thickness,
};
use bytemuck::{Pod, Zeroable};
use fyrox_core::math::round_to_step;
use fyrox_resource::untyped::UntypedResource;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
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

#[derive(Clone, Debug)]
pub enum CommandTexture {
    None,
    Texture(UntypedResource),
    Font {
        font: FontResource,
        height: FontHeight,
        page_index: usize,
    },
}

/// A set of triangles that will be used for clipping.
#[derive(Clone, Debug)]
pub struct ClippingGeometry {
    pub vertex_buffer: Vec<Vertex>,
    pub triangle_buffer: Vec<TriangleDefinition>,
    pub transform_stack: TransformStack,
}

impl Draw for ClippingGeometry {
    #[inline(always)]
    fn push_vertex_raw(&mut self, mut vertex: Vertex) {
        vertex.pos = self
            .transform_stack
            .transform
            .transform_point(&Point2::from(vertex.pos))
            .coords;

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

#[derive(Clone, Debug)]
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
    fn push_vertex(&mut self, pos: Vector2<f32>, tex_coord: Vector2<f32>) {
        self.push_vertex_raw(Vertex::new(pos, tex_coord))
    }

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

    fn push_circle_filled(
        &mut self,
        origin: Vector2<f32>,
        radius: f32,
        segments: usize,
        color: Color,
    ) {
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

    fn push_circle(
        &mut self,
        center: Vector2<f32>,
        radius: f32,
        subdivisions: usize,
        thickness: f32,
    ) {
        let start_vertex = self.last_vertex_index();
        let d = std::f32::consts::TAU / subdivisions as f32;

        let half_thickness = thickness * 0.5;

        let mut angle = 0.0;
        while angle < std::f32::consts::TAU {
            let r = Vector2::new(angle.cos(), angle.sin());

            let p0 = center + r.scale(radius - half_thickness);
            self.push_vertex(p0, Default::default());

            let p1 = center + r.scale(radius + half_thickness);
            self.push_vertex(p1, Default::default());

            angle += d;
        }
        let last_vertex_index = self.last_vertex_index();

        self.connect_as_line(start_vertex, last_vertex_index, true)
    }

    fn connect_as_line(&mut self, from: u32, to: u32, closed: bool) {
        if closed {
            let count = to - from;
            for i in (0..count).step_by(2) {
                let i0 = from + i % count;
                let i1 = from + (i + 1) % count;
                let i2 = from + (i + 2) % count;
                let i3 = from + (i + 3) % count;
                self.push_triangle(i0, i1, i2);
                self.push_triangle(i1, i3, i2);
            }
        } else {
            for i in (from..to.saturating_sub(4)).step_by(2) {
                let i0 = i;
                let i1 = i + 1;
                let i2 = i + 2;
                let i3 = i + 3;
                self.push_triangle(i0, i1, i2);
                self.push_triangle(i1, i3, i2);
            }
        }
    }

    fn push_arc(
        &mut self,
        center: Vector2<f32>,
        radius: f32,
        angles: Range<f32>,
        subdivisions: usize,
        thickness: f32,
    ) {
        let start_vertex = self.last_vertex_index();
        self.push_arc_path_with_thickness(center, radius, angles, subdivisions, thickness);
        let last_vertex_index = self.last_vertex_index();

        self.connect_as_line(start_vertex, last_vertex_index, false)
    }

    fn push_arc_path_with_thickness(
        &mut self,
        center: Vector2<f32>,
        radius: f32,
        angles: Range<f32>,
        subdivisions: usize,
        thickness: f32,
    ) {
        let mut start_angle = math::wrap_angle(angles.start);
        let mut end_angle = math::wrap_angle(angles.end);

        if start_angle > end_angle {
            std::mem::swap(&mut start_angle, &mut end_angle);
        }

        let d = (end_angle - start_angle) / subdivisions as f32;

        let half_thickness = thickness * 0.5;

        let mut angle = start_angle;
        while angle <= end_angle {
            let r = Vector2::new(angle.cos(), angle.sin());

            let p0 = center + r.scale(radius - half_thickness);
            self.push_vertex(p0, Default::default());

            let p1 = center + r.scale(radius + half_thickness);
            self.push_vertex(p1, Default::default());

            angle += d;
        }
    }

    fn push_arc_path(
        &mut self,
        center: Vector2<f32>,
        radius: f32,
        angles: Range<f32>,
        subdivisions: usize,
    ) {
        let mut start_angle = math::wrap_angle(angles.start);
        let mut end_angle = math::wrap_angle(angles.end);

        if start_angle > end_angle {
            std::mem::swap(&mut start_angle, &mut end_angle);
        }

        let d = (end_angle - start_angle) / subdivisions as f32;

        let mut angle = start_angle;
        while angle <= end_angle {
            let p0 = center + Vector2::new(angle.cos() * radius, angle.sin() * radius);

            self.push_vertex(p0, Default::default());

            angle += d;
        }
    }

    fn push_line_path(&mut self, a: Vector2<f32>, b: Vector2<f32>) {
        self.push_vertex(a, Default::default());
        self.push_vertex(b, Default::default());
    }

    fn push_line_path_with_thickness(&mut self, a: Vector2<f32>, b: Vector2<f32>, thickness: f32) {
        let perp = get_line_thickness_vector(a, b, thickness);
        self.push_vertex(a - perp, Vector2::new(0.0, 0.0));
        self.push_vertex(a + perp, Vector2::new(1.0, 1.0));
        self.push_vertex(b - perp, Vector2::new(1.0, 0.0));
        self.push_vertex(b + perp, Vector2::new(0.0, 1.0));
    }

    fn push_rounded_rect_filled(
        &mut self,
        rect: &Rect<f32>,
        mut corner_radius: f32,
        corner_subdivisions: usize,
    ) {
        // Restrict corner radius in available rectangle.
        let min_axis = rect.w().min(rect.h());
        corner_radius = corner_radius.min(min_axis * 0.5);

        let center_index = self.last_vertex_index();
        self.push_vertex(rect.center(), Default::default());

        self.push_line_path(
            Vector2::new(rect.x(), rect.y() + rect.h() - corner_radius),
            Vector2::new(rect.x(), rect.y() + corner_radius),
        );

        self.push_arc_path(
            rect.position + Vector2::repeat(corner_radius),
            corner_radius,
            180.0f32.to_radians()..270.0f32.to_radians(),
            corner_subdivisions,
        );

        self.push_line_path(
            Vector2::new(rect.x() + corner_radius, rect.y()),
            Vector2::new(rect.x() + rect.w() - corner_radius, rect.y()),
        );

        self.push_arc_path(
            Vector2::new(
                rect.position.x + rect.w() - corner_radius,
                rect.position.y + corner_radius,
            ),
            corner_radius,
            270.0f32.to_radians()..359.9999f32.to_radians(),
            corner_subdivisions,
        );

        self.push_line_path(
            Vector2::new(rect.x() + rect.w(), rect.y() + corner_radius),
            Vector2::new(rect.x() + rect.w(), rect.y() + rect.h() - corner_radius),
        );

        self.push_arc_path(
            Vector2::new(
                rect.position.x + rect.w() - corner_radius,
                rect.position.y + rect.h() - corner_radius,
            ),
            corner_radius,
            0.0f32.to_radians()..90.0f32.to_radians(),
            corner_subdivisions,
        );

        self.push_line_path(
            Vector2::new(rect.x() + rect.w() - corner_radius, rect.y() + rect.h()),
            Vector2::new(rect.x() + corner_radius, rect.y() + rect.h()),
        );

        self.push_arc_path(
            Vector2::new(
                rect.position.x + corner_radius,
                rect.position.y + rect.h() - corner_radius,
            ),
            corner_radius,
            90.0f32.to_radians()..180.0f32.to_radians(),
            corner_subdivisions,
        );

        // Connect all vertices.
        let first_index = center_index + 1;
        let last_vertex_index = self.last_vertex_index().saturating_sub(1);
        for i in first_index..last_vertex_index {
            let next = i + 1;
            self.push_triangle(i, next, center_index)
        }

        self.push_triangle(last_vertex_index, first_index, center_index);
    }

    fn push_rounded_rect(
        &mut self,
        rect: &Rect<f32>,
        thickness: f32,
        mut corner_radius: f32,
        corner_subdivisions: usize,
    ) {
        // Restrict corner radius in available rectangle.
        let min_axis = rect.w().min(rect.h());
        corner_radius = corner_radius.min(min_axis * 0.5);

        let half_thickness = thickness * 0.5;

        let start_index = self.last_vertex_index();

        self.push_line_path_with_thickness(
            Vector2::new(
                rect.x() + half_thickness,
                rect.y() + rect.h() - thickness - corner_radius,
            ),
            Vector2::new(
                rect.x() + half_thickness,
                rect.y() + thickness + corner_radius,
            ),
            thickness,
        );

        self.push_arc_path_with_thickness(
            rect.position + Vector2::repeat(corner_radius + half_thickness),
            corner_radius,
            180.0f32.to_radians()..270.0f32.to_radians(),
            corner_subdivisions,
            thickness,
        );

        self.push_line_path_with_thickness(
            Vector2::new(
                rect.x() + corner_radius + half_thickness,
                rect.y() + half_thickness,
            ),
            Vector2::new(
                rect.x() + rect.w() - corner_radius - half_thickness,
                rect.y() + half_thickness,
            ),
            thickness,
        );

        self.push_arc_path_with_thickness(
            Vector2::new(
                rect.position.x + rect.w() - corner_radius - half_thickness,
                rect.position.y + corner_radius + half_thickness,
            ),
            corner_radius,
            270.0f32.to_radians()..359.9999f32.to_radians(),
            corner_subdivisions,
            thickness,
        );

        self.push_line_path_with_thickness(
            Vector2::new(
                rect.x() + rect.w() - half_thickness,
                rect.y() + thickness + corner_radius,
            ),
            Vector2::new(
                rect.x() + rect.w() - half_thickness,
                rect.y() + rect.h() - thickness - corner_radius,
            ),
            thickness,
        );

        self.push_arc_path_with_thickness(
            Vector2::new(
                rect.position.x + rect.w() - corner_radius - half_thickness,
                rect.position.y + rect.h() - corner_radius - half_thickness,
            ),
            corner_radius,
            0.0f32.to_radians()..90.0f32.to_radians(),
            corner_subdivisions,
            thickness,
        );

        self.push_line_path_with_thickness(
            Vector2::new(
                rect.x() + rect.w() - corner_radius - half_thickness,
                rect.y() + rect.h() - half_thickness,
            ),
            Vector2::new(
                rect.x() + corner_radius + half_thickness,
                rect.y() + rect.h() - half_thickness,
            ),
            thickness,
        );

        self.push_arc_path_with_thickness(
            Vector2::new(
                rect.position.x + corner_radius + half_thickness,
                rect.position.y + rect.h() - corner_radius - half_thickness,
            ),
            corner_radius,
            90.0f32.to_radians()..180.0f32.to_radians(),
            corner_subdivisions,
            thickness,
        );

        let last_vertex_index = self.last_vertex_index();
        self.connect_as_line(start_index, last_vertex_index, true);
    }

    fn push_bezier(
        &mut self,
        p0: Vector2<f32>,
        p1: Vector2<f32>,
        p2: Vector2<f32>,
        p3: Vector2<f32>,
        subdivisions: usize,
        thickness: f32,
    ) {
        fn cubic_bezier(
            p0: Vector2<f32>,
            p1: Vector2<f32>,
            p2: Vector2<f32>,
            p3: Vector2<f32>,
            t: f32,
        ) -> Vector2<f32> {
            p0.scale((1.0 - t).powi(3))
                + p1.scale(3.0 * t * (1.0 - t).powi(2))
                + p2.scale(3.0 * t.powi(2) * (1.0 - t))
                + p3.scale(t.powi(3))
        }

        let mut prev = cubic_bezier(p0, p1, p2, p3, 0.0);
        for i in 0..subdivisions {
            let t = (i + 1) as f32 / subdivisions as f32;
            let next = cubic_bezier(p0, p1, p2, p3, t);
            // TODO: This could give gaps between segments on sharp turns, it should be either patched
            // or be continuous line instead of separate segments.
            self.push_line(prev, next, thickness);
            prev = next;
        }
    }

    fn push_grid(&mut self, zoom: f32, cell_size: Vector2<f32>, grid_bounds: Rect<f32>) {
        let mut local_left_bottom = grid_bounds.left_top_corner();
        local_left_bottom.x = round_to_step(local_left_bottom.x, cell_size.x);
        local_left_bottom.y = round_to_step(local_left_bottom.y, cell_size.y);

        let mut local_right_top = grid_bounds.right_bottom_corner();
        local_right_top.x = round_to_step(local_right_top.x, cell_size.x);
        local_right_top.y = round_to_step(local_right_top.y, cell_size.y);

        let w = (local_right_top.x - local_left_bottom.x).abs();
        let h = (local_right_top.y - local_left_bottom.y).abs();

        let nw = ((w / cell_size.x).ceil()) as usize;
        let nh = ((h / cell_size.y).ceil()) as usize;

        for ny in 0..=nh {
            let k = ny as f32 / (nh) as f32;
            let y = local_left_bottom.y + k * h;
            self.push_line(
                Vector2::new(local_left_bottom.x - cell_size.x, y),
                Vector2::new(local_right_top.x + cell_size.x, y),
                1.0 / zoom,
            );
        }

        for nx in 0..=nw {
            let k = nx as f32 / (nw) as f32;
            let x = local_left_bottom.x + k * w;
            self.push_line(
                Vector2::new(x, local_left_bottom.y + cell_size.y),
                Vector2::new(x, local_right_top.y - cell_size.y),
                1.0 / zoom,
            );
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransformStack {
    transform: Matrix3<f32>,
    stack: Vec<Matrix3<f32>>,
}

impl Default for TransformStack {
    fn default() -> Self {
        Self {
            transform: Matrix3::identity(),
            stack: vec![],
        }
    }
}

impl TransformStack {
    #[inline]
    pub fn push(&mut self, matrix: Matrix3<f32>) {
        self.transform = matrix;
        self.stack.push(matrix);
    }

    /// Returns the transformation matrix that will be used to transform vertices of drawing context.
    #[inline]
    pub fn transform(&self) -> &Matrix3<f32> {
        &self.transform
    }

    #[inline]
    pub fn pop(&mut self) {
        if let Some(top) = self.stack.pop() {
            self.transform = top;
        }
    }
}

#[derive(Debug, Clone)]
pub struct DrawingContext {
    vertex_buffer: Vec<Vertex>,
    triangle_buffer: Vec<TriangleDefinition>,
    command_buffer: Vec<Command>,
    pub transform_stack: TransformStack,
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
    fn push_vertex_raw(&mut self, mut vertex: Vertex) {
        vertex.pos = self
            .transform_stack
            .transform
            .transform_point(&Point2::from(vertex.pos))
            .coords;

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
            transform_stack: Default::default(),
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

        #[inline(always)]
        fn draw(
            formatted_text: &FormattedText,
            ctx: &mut DrawingContext,
            clip_bounds: Rect<f32>,
            position: Vector2<f32>,
            dilation: f32,
            offset: Vector2<f32>,
            brush: Brush,
            font: &FontResource,
        ) {
            let Some(mut current_page_index) = formatted_text
                .get_glyphs()
                .first()
                .map(|g| g.atlas_page_index)
            else {
                return;
            };

            for element in formatted_text.get_glyphs() {
                // If we've switched to another atlas page, commit the text and start a new batch.
                if current_page_index != element.atlas_page_index {
                    ctx.commit(
                        clip_bounds,
                        brush.clone(),
                        CommandTexture::Font {
                            font: font.clone(),
                            page_index: current_page_index,
                            height: formatted_text.font_size().into(),
                        },
                        None,
                    );
                    current_page_index = element.atlas_page_index;
                }

                let bounds = element.bounds;

                let final_bounds = Rect::new(
                    position.x + bounds.x() + offset.x,
                    position.y + bounds.y() + offset.y,
                    bounds.w(),
                    bounds.h(),
                )
                .inflate(dilation, dilation);

                ctx.push_rect_filled(&final_bounds, Some(&element.tex_coords));
            }

            // Commit the rest.
            ctx.commit(
                clip_bounds,
                brush,
                CommandTexture::Font {
                    font: font.clone(),
                    page_index: current_page_index,
                    height: formatted_text.font_size().into(),
                },
                None,
            );
        }

        // Draw shadow, if any.
        if *formatted_text.shadow {
            draw(
                formatted_text,
                self,
                clip_bounds,
                position,
                *formatted_text.shadow_dilation,
                *formatted_text.shadow_offset,
                (*formatted_text.shadow_brush).clone(),
                &font,
            );
        }

        draw(
            formatted_text,
            self,
            clip_bounds,
            position,
            0.0,
            Default::default(),
            formatted_text.brush(),
            &font,
        );
    }
}
