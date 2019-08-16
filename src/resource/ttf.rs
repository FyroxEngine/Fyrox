// TTF loader and rasterizer

use crate::{
    math::{
        vec2::Vec2,
        Rect,
    },
    utils::pool::{Pool, Handle},
};
use std::{
    cmp::Ordering,
    collections::HashMap,
    path::Path,
    fs::File,
    io::Read,
};

const ON_CURVE_POINT: u8 = 1;
const REPEAT_FLAG: u8 = 8;

#[derive(Copy, Clone, Debug)]
struct Point {
    x: f32,
    y: f32,
    flags: u8,
}

#[derive(Debug)]
struct Polygon {
    points: Vec<Point>
}

#[derive(Debug)]
pub struct FontGlyph {
    bitmap_top: f32,
    bitmap_left: f32,
    bitmap_width: f32,
    bitmap_height: f32,
    pixels: Vec<u8>,
    advance: f32,
    has_outline: bool,
    tex_coords: [Vec2; 4],
}

#[derive(Debug)]
struct TtfGlyph {
    num_contours: i16,
    x_min: i16,
    y_min: i16,
    x_max: i16,
    y_max: i16,
    has_outline: bool,
    advance: u16,
    left_side_bearing: i16,
    end_points: Vec<u16>,
    raw_contours: Vec<Polygon>,
    contours: Vec<Polygon>,
}

struct TrueType {
    data: Vec<u8>,
    cmap_table: *const u8,
    loca_table: *const u8,
    maxp_table: *const u8,
    head_table: *const u8,
    glyf_table: *const u8,
    hhea_table: *const u8,
    hmtx_table: *const u8,
    kern_table: *const u8,
    num_glyphs: u16,
    glyphs: Vec<TtfGlyph>,
}

struct Line2 {
    begin: Point,
    end: Point,
}

struct Bitmap {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
}

#[derive(Debug)]
pub struct Font {
    height: f32,
    glyphs: Vec<FontGlyph>,
    ascender: f32,
    descender: f32,
    line_gap: f32,
    char_map: HashMap<u32, usize>,
    atlas: Vec<u8>,
    atlas_size: i32,
    texture_id: u32,
}

#[derive(Debug)]
struct RectPackNode {
    filled: bool,
    split: bool,
    bounds: Rect<i32>,
    left: Handle<RectPackNode>,
    right: Handle<RectPackNode>,
}

impl RectPackNode {
    fn new(bounds: Rect<i32>) -> RectPackNode {
        RectPackNode {
            bounds,
            filled: false,
            split: false,
            left: Handle::none(),
            right: Handle::none(),
        }
    }
}

struct RectPacker {
    nodes: Pool<RectPackNode>,
    root: Handle<RectPackNode>,
}

impl RectPacker {
    fn new(w: i32, h: i32) -> RectPacker {
        let mut nodes = Pool::new();
        let root = nodes.spawn(RectPackNode::new(Rect::new(0, 0, w, h)));
        RectPacker {
            nodes,
            root,
        }
    }

    fn find_free(&mut self, w: i32, h: i32) -> Option<Rect<i32>> {
        let mut unvisited: Vec<Handle<RectPackNode>> = Vec::new();
        unvisited.push(self.root.clone());
        while let Some(node_handle) = unvisited.pop() {
            let mut left_bounds: Rect<i32> = Rect::default();
            let mut right_bounds: Rect<i32> = Rect::default();

            if let Some(node) = self.nodes.borrow_mut(&node_handle) {
                if node.split {
                    unvisited.push(node.right.clone());
                    unvisited.push(node.left.clone());
                    continue;
                } else {
                    if node.filled || node.bounds.w < w || node.bounds.h < h {
                        continue;
                    }

                    if node.bounds.w == w && node.bounds.h == h {
                        node.filled = true;
                        return Some(node.bounds);
                    }

                    // Split and continue
                    node.split = true;
                    if node.bounds.w - w > node.bounds.h - h {
                        left_bounds = Rect::new(node.bounds.x, node.bounds.y, w, node.bounds.h);
                        right_bounds = Rect::new(node.bounds.x + w, node.bounds.y, node.bounds.w - w, node.bounds.h);
                    } else {
                        left_bounds = Rect::new(node.bounds.x, node.bounds.y, node.bounds.w, h);
                        right_bounds = Rect::new(node.bounds.x, node.bounds.y + h, node.bounds.w, node.bounds.h - h);
                    }
                }
            }

            let left = self.nodes.spawn(RectPackNode::new(left_bounds));
            if let Some(node) = self.nodes.borrow_mut(&node_handle) {
                node.left = left.clone();
            }

            let right = self.nodes.spawn(RectPackNode::new(right_bounds));
            if let Some(node) = self.nodes.borrow_mut(&node_handle) {
                node.right = right.clone();
            }

            unvisited.push(left.clone());
        }

        None
    }
}

impl Bitmap {
    fn new(w: usize, h: usize) -> Bitmap {
        let mut pixels = Vec::with_capacity(w * h);
        for _ in 0..(w * h) {
            pixels.push(0);
        }
        Bitmap {
            width: w,
            height: h,
            pixels,
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, pixel: u8) {
        if x >= self.width {
            return;
        }
        if y >= self.height {
            return;
        }

        self.pixels[y * self.width + x] = pixel;
    }

    fn get_fpixel(&self, x: usize, y: usize) -> f32 {
        if x >= self.width {
            return 0.0;
        }
        if y >= self.height {
            return 0.0;
        }

        self.pixels[y * self.width + x] as f32 / 255.0
    }
}

fn get_u16(p: *const u8) -> u16
{
    unsafe {
        let b0 = u32::from(*p);
        let b1 = *p.offset(1) as u32;
        (b0 * 256 + b1) as u16
    }
}

fn get_i16(p: *const u8) -> i16
{
    unsafe {
        let b0 = *p as i32;
        let b1 = *p.offset(1) as i32;
        (b0 * 256 + b1) as i16
    }
}

fn get_u32(p: *const u8) -> u32
{
    unsafe {
        let b0 = *p as u32;
        let b1 = *p.offset(1) as u32;
        let b2 = *p.offset(2) as u32;
        let b3 = *p.offset(3) as u32;
        (b0 << 24) + (b1 << 16) + (b2 << 8) + b3
    }
}

fn fourcc(d: u8, c: u8, b: u8, a: u8) -> u32 {
    ((d as u32) << 24) | ((c as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

fn segmented_mapping(subtable: *const u8, unicode: u32) -> usize {
    unsafe {
        let segment_count = (get_u16(subtable.offset(6)) / 2) as u32;
        let end_codes = subtable.offset(14);
        let start_codes = subtable.offset((16 + 2 * segment_count) as isize);
        let id_delta = subtable.offset((16 + 4 * segment_count) as isize);
        let id_range_offset = subtable.offset((16 + 6 * segment_count) as isize);

        let mut segment = 0;
        while segment < segment_count {
            if get_u16(end_codes.offset(2 * segment as isize)) as u32 >= unicode {
                break;
            }
            segment += 1;
        }

        if segment != segment_count {
            let start_code = get_u16(start_codes.offset(2 * segment as isize)) as u32;
            if start_code <= unicode {
                let range_offset = (get_u16(id_range_offset.offset(2 * segment as isize)) / 2) as u32;
                if range_offset == 0 {
                    return ((unicode + get_u16(id_delta.offset(2 * segment as isize)) as u32) & 0xFFFF) as usize;
                } else {
                    let offset = 2 * (segment + range_offset + (unicode - start_code));
                    return get_u16(id_range_offset.offset(offset as isize)) as usize;
                }
            }
        }

        0
    }
}

fn direct_mapping(subtable: *const u8, unicode: u32) -> usize {
    unsafe {
        if unicode < 256 {
            let indices = subtable.offset(6);
            return indices.offset(unicode as isize).read() as usize;
        }
    }

    0
}

fn dense_mapping(subtable: *const u8, unicode: u32) -> usize {
    unsafe {
        let first = get_u16(subtable.offset(6)) as u32;
        let entry_count = get_u16(subtable.offset(8)) as u32;
        let indices = subtable.offset(10);

        if unicode > first && unicode < first + entry_count {
            return get_u16(indices.offset(2 * (unicode - first) as isize)) as usize;
        }
    }

    0
}

fn line_line_intersection(a: &Line2, b: &Line2) -> Option<Point> {
    let s1x = a.end.x - a.begin.x;
    let s1y = a.end.y - a.begin.y;
    let s2x = b.end.x - b.begin.x;
    let s2y = b.end.y - b.begin.y;
    let s = (-s1y * (a.begin.x - b.begin.x) + s1x * (a.begin.y - b.begin.y)) / (-s2x * s1y + s1x * s2y);
    let t = (s2x * (a.begin.y - b.begin.y) - s2y * (a.begin.x - b.begin.x)) / (-s2x * s1y + s1x * s2y);
    if s >= 0.0 && s <= 1.0 && t >= 0.0 && t <= 1.0 {
        Some(Point {
            x: a.begin.x + (t * s1x),
            y: a.begin.y + (t * s1y),
            flags: 0,
        })
    } else {
        None
    }
}

fn polygons_to_scanlines(polys: &Vec<Polygon>, width: f32, height: f32, scale: f32) -> Vec<Line2> {
    let bias = 0.0001;
    let y_oversample = 5.0;
    let y_step = 1.0 / y_oversample;

    let real_width = scale * width;
    let real_height = scale * height;

    let mut intersections: Vec<Point> = Vec::new();

    let mut scanline = Line2 {
        begin: Point { x: -1.0, y: 0.0, flags: 0 },
        end: Point { x: real_width + 2.0, y: 0.0, flags: 0 },
    };

    let mut lines = Vec::new();
    let mut y = bias;
    while y < real_height {
        intersections.clear();

        scanline.begin.y = y;
        scanline.end.y = y;

        /* Find all intersection points for current y */
        for poly in polys.iter() {
            for j in (0..poly.points.len()).step_by(2) {
                let begin = poly.points.get(j).unwrap();
                let end = poly.points.get(j + 1).unwrap();

                let edge = Line2 {
                    begin: Point {
                        x: begin.x * scale,
                        y: begin.y * scale,
                        flags: 0,
                    },
                    end: Point {
                        x: end.x * scale,
                        y: end.y * scale,
                        flags: 0,
                    },
                };

                if let Some(int_point) = line_line_intersection(&scanline, &edge) {
                    intersections.push(int_point);
                }
            }
        }

        intersections.sort_by(|a, b| {
            if a.x < b.x {
                Ordering::Less
            } else if a.x > b.x {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });

        if intersections.len() % 2 != 0 {
            println!("scanline rasterization failed {:?}", intersections);
        }

        /* Convert intersection points into scanlines */
        if intersections.len() > 0 {
            for i in (0..(intersections.len() - 1)).step_by(2) {
                let line = Line2 {
                    begin: Point { x: intersections[i].x, y, flags: 0 },
                    end: Point { x: intersections[i + 1].x, y, flags: 0 },
                };
                lines.push(line);
            }
        }

        y += y_step;
    }

    lines
}

fn raster_scanlines(w: usize, h: usize, lines: &Vec<Line2>) -> Bitmap {
    let mut bitmap = Bitmap::new(w, h);

    /* Antialised scanline rasterization. */
    for scanline in lines.iter() {
        let yi = scanline.begin.y as usize;

        /* Calculate new opacity for pixel at the begin of scanline. */
        let bx = scanline.begin.x;
        let begin_opacity = 0.5 * ((bx.ceil() - bx) + bitmap.get_fpixel(bx as usize, yi));
        bitmap.set_pixel(bx as usize, yi, (255.0 * begin_opacity) as u8);

        /* Calculate new opacity for pixel at the end of scanline. */
        let ex = scanline.end.x;
        let end_opacity = 0.5 * ((ex - ex.floor()) + bitmap.get_fpixel(ex as usize, yi));
        bitmap.set_pixel(ex as usize, yi, (255.0 * end_opacity) as u8);

        /* Modulate rest with opaque color. */
        let begin = bx.ceil() as usize;
        let end = ex.ceil() as usize;
        for x in begin..end {
            let value = 0.5 * (1.0 + bitmap.get_fpixel(x, yi));
            bitmap.set_pixel(x, yi, (255.0 * value) as u8);
        }
    }

    let border = 4;
    let mut out_bitmap = Bitmap::new(bitmap.width + border, bitmap.height + border);

    /* add border to glyph to remove seams due to bilinear filtration on GPU */
    let half_border = border / 2;
    for row in half_border..out_bitmap.height {
        for col in half_border..out_bitmap.width {
            let r = row - half_border;
            let c = col - half_border;
            if r < bitmap.height && c < bitmap.width {
                out_bitmap.pixels[row * out_bitmap.width + col] = bitmap.pixels[r * bitmap.width + c];
            }
        }
    }

    out_bitmap
}

impl TrueType {
    fn new(data: Vec<u8>) -> TrueType {
        let mut ttf = TrueType {
            data,
            cmap_table: std::ptr::null(),
            loca_table: std::ptr::null(),
            maxp_table: std::ptr::null(),
            head_table: std::ptr::null(),
            glyf_table: std::ptr::null(),
            hhea_table: std::ptr::null(),
            hmtx_table: std::ptr::null(),
            kern_table: std::ptr::null(),
            num_glyphs: 0,
            glyphs: Vec::new(),
        };

        ttf.find_tables();
        ttf.read_glyphs();

        ttf
    }

    fn find_tables(&mut self) {
        unsafe {
            let data = self.data.as_ptr();
            let num_tables = get_u16(data.offset(4)) as isize;

            for i in 0..num_tables {
                let subtable = data.offset(12 + i * 16);
                let tag = get_u32(subtable.offset(0));
                let offset = get_u32(subtable.offset(8));
                let table_location = data.offset(offset as isize);

                if tag == fourcc(b'c', b'm', b'a', b'p') {
                    self.cmap_table = table_location;
                } else if tag == fourcc(b'l', b'o', b'c', b'a') {
                    self.loca_table = table_location;
                } else if tag == fourcc(b'm', b'a', b'x', b'p') {
                    self.maxp_table = table_location;
                } else if tag == fourcc(b'h', b'e', b'a', b'd') {
                    self.head_table = table_location;
                } else if tag == fourcc(b'g', b'l', b'y', b'f') {
                    self.glyf_table = table_location;
                } else if tag == fourcc(b'h', b'h', b'e', b'a') {
                    self.hhea_table = table_location;
                } else if tag == fourcc(b'h', b'm', b't', b'x') {
                    self.hmtx_table = table_location;
                } else if tag == fourcc(b'k', b'e', b'r', b'n') {
                    self.kern_table = table_location;
                }
            }
            self.num_glyphs = get_u16(self.maxp_table.offset(4));
        }
    }

    fn unicode_to_glyph_index(&self, unicode: u32) -> usize {
        unsafe {
            let subtable_count = get_u16(self.cmap_table.offset(2));
            for i in 0..subtable_count {
                let subtable_offset = get_u32(self.cmap_table.offset((4 + 4 * i + 4) as isize));
                let subtable = self.cmap_table.offset(subtable_offset as isize);
                let format = get_u16(subtable);
                match format {
                    0 => return direct_mapping(subtable, unicode),
                    4 => return segmented_mapping(subtable, unicode),
                    6 => return dense_mapping(subtable, unicode),
                    _ => () // TODO: Add more mappings
                }
            }
        }

        0
    }

    fn get_glyph_offset(&self, index: usize) -> isize {
        unsafe {
            let index_to_loc_format = get_i16(self.head_table.offset(50));
            if index_to_loc_format & 1 != 0 {
                get_u32(self.loca_table.offset(4 * index as isize)) as isize
            } else {
                (get_u16(self.loca_table.offset(2 * index as isize)) * 2) as isize
            }
        }
    }

    fn em_to_pixels(&self, pixels: f32) -> f32 {
        unsafe {
            let units_per_em = get_u16(self.head_table.offset(18)) as f32;
            pixels / units_per_em
        }
    }

    fn convert_glyph(&self, glyph: &TtfGlyph, scale: f32) -> FontGlyph {
        if glyph.num_contours < 0 {
            return FontGlyph {
                bitmap_top: 0.0,
                bitmap_left: 0.0,
                bitmap_width: 0.0,
                bitmap_height: 0.0,
                pixels: Vec::new(),
                advance: 0.0,
                has_outline: false,
                tex_coords: [Vec2::new(); 4],
            };
        }

        let height = ((scale * (glyph.y_max - glyph.y_min) as f32) + 1.0) as usize;
        let width = ((scale * (glyph.x_max - glyph.x_min) as f32) + 1.0) as usize;

        let lines = polygons_to_scanlines(
            &glyph.contours,
            (glyph.x_max - glyph.x_min) as f32,
            (glyph.y_max - glyph.y_min) as f32,
            scale,
        );

        let final_bitmap = raster_scanlines(width, height, &lines);
        FontGlyph {
            pixels: final_bitmap.pixels,
            bitmap_width: final_bitmap.width as f32,
            bitmap_height: final_bitmap.height as f32,
            advance: glyph.advance as f32 * scale,
            bitmap_left: glyph.x_min as f32 * scale,
            bitmap_top: glyph.y_min as f32 * scale,
            has_outline: glyph.has_outline,
            tex_coords: [Vec2::new(); 4],
        }
    }

    fn get_ascender(&self) -> i16 {
        unsafe {
            get_i16(self.hhea_table.offset(4))
        }
    }

    fn get_descender(&self) -> i16 {
        unsafe {
            get_i16(self.hhea_table.offset(6))
        }
    }

    fn get_line_gap(&self) -> i16 {
        unsafe {
            get_i16(self.hhea_table.offset(8))
        }
    }

    fn read_glyphs(&mut self) {
        unsafe {
            self.glyphs = Vec::with_capacity(self.num_glyphs as usize);
            for i in 0..(self.num_glyphs as usize) {
                let offset = self.get_glyph_offset(i);
                let next_offset = self.get_glyph_offset(i + 1);
                let glyph_data = self.glyf_table.offset(offset);

                let mut glyph = TtfGlyph {
                    num_contours: get_i16(glyph_data),
                    x_min: get_i16(glyph_data.offset(2)),
                    y_min: get_i16(glyph_data.offset(4)),
                    x_max: get_i16(glyph_data.offset(6)),
                    y_max: get_i16(glyph_data.offset(8)),
                    has_outline: (next_offset - offset) != 0,
                    advance: 0,
                    left_side_bearing: 0,
                    end_points: Vec::new(),
                    raw_contours: Vec::new(),
                    contours: Vec::new(),
                };

                if glyph.num_contours < 0 {
                    /* TODO: Implement compound glyph support. */
                } else {
                    /* Simple glyph */
                    /* Read end contour points */
                    let mut point_count = 0;
                    for j in 0..glyph.num_contours {
                        let end_point = get_u16(glyph_data.offset((10 + j * 2) as isize));
                        glyph.end_points.push(end_point);
                        if end_point > point_count {
                            point_count = end_point;
                        }
                    }
                    point_count += 1;

                    /* Alloc points */
                    let mut points: Vec<Point> = Vec::new();
                    for _ in 0..point_count {
                        points.push(Point { x: 0.0, y: 0.0, flags: 0 });
                    }

                    /* TODO: Skip instructions for now. Simple interpreter would be nice. */
                    let instructions = get_u16(glyph_data.offset(10 + 2 * glyph.num_contours as isize)) as isize;

                    /* Read flags for each point */
                    let mut flags = glyph_data.offset(10 + 2 * glyph.num_contours as isize + 2 + instructions);

                    let mut j = 0;
                    while j < points.len() {
                        let pt_flag = flags.read();
                        points.get_mut(j).unwrap().flags = pt_flag;
                        flags = flags.offset(1);
                        if (pt_flag & REPEAT_FLAG) != 0 {
                            let repeat_count = *flags as usize;
                            flags = flags.offset(1);
                            let mut k = 1;
                            while k <= repeat_count {
                                points.get_mut(j + k).unwrap().flags = pt_flag;
                                k += 1;
                            }
                            j += repeat_count;
                        }
                        j += 1;
                    }

                    /* Read x-coordinates for each point */
                    let mut coords = flags;
                    let mut x = 0;
                    for j in 0..(point_count as usize) {
                        let pt = points.get_mut(j).unwrap();
                        if (pt.flags & 2) != 0 {
                            let dx = *coords as i32;
                            coords = coords.offset(1);
                            x += if (pt.flags & 16) != 0 { dx } else { -dx };
                        } else if (pt.flags & 16) == 0 {
                            x += get_i16(coords) as i32;
                            coords = coords.offset(2);
                        }
                        pt.x = x as f32;
                    }

                    /* Read y-coordinates for each point */
                    let mut y = 0;
                    for j in 0..(point_count as usize) {
                        let pt = points.get_mut(j).unwrap();
                        if (pt.flags & 4) != 0 {
                            let dy = *coords as i32;
                            coords = coords.offset(1);
                            y += if (pt.flags & 32) != 0 { dy } else { -dy };
                        } else if (pt.flags & 32) == 0 {
                            y += get_i16(coords) as i32;
                            coords = coords.offset(2);
                        }

                        pt.y = y as f32;
                    }

                    glyph.prepare_contours(points);
                }

                glyph.fill_horizontal_metrics(self.hhea_table, self.hmtx_table, i);
                glyph.convert_curves_to_line_set();

                self.glyphs.push(glyph);
            }
        }
    }
}

fn eval_quad_bezier(p0: &Point, p1: &Point, p2: &Point, steps: usize) -> Vec<Point> {
    let step = 1.0 / steps as f32;

    let mut result = Vec::new();
    let mut t = 0.0;
    while t <= 1.0 {
        let inv_t = 1.0 - t;
        let k0 = inv_t * inv_t;
        let k1 = 2.0 * t * inv_t;
        let k2 = t * t;
        let pt = Point {
            x: k0 * p0.x + k1 * p1.x + k2 * p2.x,
            y: k0 * p0.y + k1 * p1.y + k2 * p2.y,
            flags: 0,
        };
        result.push(pt);
        t += step;
    }

    result
}

impl TtfGlyph {
    fn fill_horizontal_metrics(&mut self, hhea_table: *const u8, hmtx_table: *const u8, glyph_index: usize) {
        unsafe {
            let num_of_long_hor_metrics = get_u16(hhea_table.add(34)) as usize;
            if glyph_index < num_of_long_hor_metrics {
                self.advance = get_u16(hmtx_table.add(4 * glyph_index));
                self.left_side_bearing = get_i16(hmtx_table.add(4 * glyph_index + 2));
            } else {
                self.advance = get_u16(hmtx_table.add(4 * (num_of_long_hor_metrics - 1)));
                self.left_side_bearing = get_i16(hmtx_table.add(4 * num_of_long_hor_metrics + 2 * (glyph_index - num_of_long_hor_metrics)));
            }
        }
    }

    fn convert_curves_to_line_set(&mut self) {
        unsafe {
            for _ in 0..self.num_contours {
                self.contours.push(Polygon { points: Vec::new() });
            }

            for i in 0..(self.num_contours as usize) {
                let contour = self.contours.get_unchecked_mut(i);
                let raw_contour = self.raw_contours.get_unchecked(i);

                /* Extract vertices */
                let mut j = 0;
                while j < raw_contour.points.len() {
                    let p0 = raw_contour.points.get_unchecked(j);
                    let p1 = raw_contour.points.get_unchecked((j + 1) % raw_contour.points.len());
                    let p2 = raw_contour.points.get_unchecked((j + 2) % raw_contour.points.len());

                    let p0_on = (p0.flags & ON_CURVE_POINT) != 0;
                    let p1_on = (p1.flags & ON_CURVE_POINT) != 0;
                    let p2_on = (p2.flags & ON_CURVE_POINT) != 0;

                    if p0_on && !p1_on && p2_on {
                        let points = eval_quad_bezier(p0, p1, p2, 6);
                        for k in 0..(points.len() - 1) {
                            contour.points.push(*points.get_unchecked(k));
                            contour.points.push(*points.get_unchecked(k + 1));
                        }
                        j += 2;
                    } else if p0_on && p1_on {
                        contour.points.push(*p0);
                        contour.points.push(*p1);
                        j += 1
                    } else {
                        j += 2;
                        println!("Invalid point sequence! Probably a bug in de_ttf_prepare_contours");
                    }
                }
            }
        }
    }

    fn prepare_contours(&mut self, points: Vec<Point>) {
        unsafe {
            let glyph_height = self.y_max - self.y_min;

            /* Extract contours */
            for _ in 0..self.num_contours {
                self.raw_contours.push(Polygon { points: Vec::new() });
            }

            let mut prev_end_pt = 0;
            for j in 0..self.num_contours {
                let end_pt = self.end_points[j as usize];
                let contour = self.raw_contours.get_unchecked_mut(j as usize);

                /* Extract vertices */
                let mut k = prev_end_pt;
                while k <= end_pt {
                    let pt = points.get_unchecked(k as usize);

                    let off_pt = Point {
                        x: pt.x - self.x_min as f32,
                        y: pt.y - self.y_min as f32,
                        flags: pt.flags,
                    };

                    contour.points.push(off_pt);

                    k += 1;
                }

                prev_end_pt = end_pt + 1;
            }

            /* Unpack contours */
            for j in 0..self.num_contours {
                let raw_contour = self.raw_contours.get_unchecked_mut(j as usize);
                let mut unpacked_contour = Polygon { points: Vec::new() };

                let start_off = (raw_contour.points[0].flags & ON_CURVE_POINT) == 0;

                let to =
                    if start_off {
                        /* when first point is off-curve we should add middle point between first and last points */
                        let first = raw_contour.points.first().unwrap();
                        let last = raw_contour.points.last().unwrap();

                        let middle = Point {
                            flags: ON_CURVE_POINT,
                            x: (first.x + last.x) / 2.0,
                            y: glyph_height as f32 - (first.y + last.y) / 2.0,
                        };

                        unpacked_contour.points.push(middle);

                        /* also make sure to iterate not to the end - we already added point */
                        raw_contour.points.len() - 1
                    } else {
                        raw_contour.points.len()
                    };

                for k in 0..to {
                    let p0 = raw_contour.points.get_unchecked(k as usize);
                    let p1 = raw_contour.points.get_unchecked((k + 1) % raw_contour.points.len());

                    let p0_off_curve = (p0.flags & ON_CURVE_POINT) == 0;
                    let p1_off_curve = (p1.flags & ON_CURVE_POINT) == 0;

                    let flipped = Point {
                        flags: p0.flags,
                        x: p0.x,
                        y: glyph_height as f32 - p0.y,
                    };
                    unpacked_contour.points.push(flipped);

                    if p0_off_curve && p1_off_curve {
                        let middle = Point {
                            flags: ON_CURVE_POINT,
                            x: (p0.x + p1.x) / 2.0,
                            y: glyph_height as f32 - (p0.y + p1.y) / 2.0,
                        };
                        unpacked_contour.points.push(middle);
                    }
                }

                *raw_contour = unpacked_contour;
            }
        }
    }
}

impl Font {
    pub fn load(path: &Path, height: f32, char_set: Vec<u32>) -> Option<Font> {
        if let Ok(ref mut file) = File::open(path) {
            let mut file_content: Vec<u8> = Vec::with_capacity(file.metadata().unwrap().len() as usize);
            file.read_to_end(&mut file_content).unwrap();

            let ttf = TrueType::new(file_content);

            let scale = ttf.em_to_pixels(height);

            let mut font = Font {
                height,
                glyphs: Vec::new(),
                ascender: scale * ttf.get_ascender() as f32,
                descender: scale * ttf.get_descender() as f32,
                line_gap: scale * ttf.get_line_gap() as f32,
                char_map: HashMap::new(),
                texture_id: 0,
                atlas: Vec::new(),
                atlas_size: 0,
            };

            for ttf_glyph in ttf.glyphs.iter() {
                font.glyphs.push(ttf.convert_glyph(ttf_glyph, scale));
            }

            font.pack();

            for unicode in char_set {
                let index = ttf.unicode_to_glyph_index(unicode);
                font.char_map.insert(unicode, index);
            }

            Some(font)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_glyph(&self, unicode: u32) -> Option<&FontGlyph> {
        match self.char_map.get(&unicode) {
            Some(glyph_index) => self.glyphs.get(*glyph_index),
            None => None
        }
    }

    #[inline]
    pub fn get_height(&self) -> f32 {
        self.height
    }

    #[inline]
    pub fn get_ascender(&self) -> f32 {
        self.ascender
    }

    #[inline]
    pub fn get_descender(&self) -> f32 {
        self.descender
    }

    #[inline]
    pub fn set_texture_id(&mut self, id: u32) {
        self.texture_id = id;
    }

    #[inline]
    pub fn get_texture_id(&self) -> u32 {
        self.texture_id
    }

    #[inline]
    pub fn get_atlas_pixels(&self) -> &[u8] {
        self.atlas.as_slice()
    }

    #[inline]
    pub fn get_atlas_size(&self) -> i32 {
        self.atlas_size
    }

    #[inline]
    fn compute_atlas_size(&self) -> i32 {
        let mut area = 0.0;
        for glyph in self.glyphs.iter() {
            area += glyph.bitmap_height * glyph.bitmap_width;
        }
        (1.05 * area.sqrt()) as i32
    }

    fn pack(&mut self) {
        self.atlas_size = self.compute_atlas_size();

        self.atlas = vec![0; (self.atlas_size * self.atlas_size) as usize];

        let mut rect_packer = RectPacker::new(self.atlas_size, self.atlas_size);
        for glyph in self.glyphs.iter_mut() {
            if let Some(bounds) = rect_packer.find_free(glyph.bitmap_width as i32, glyph.bitmap_height as i32) {
                let w = bounds.w as f32 / self.atlas_size as f32;
                let h = bounds.h as f32 / self.atlas_size as f32;
                let x = bounds.x as f32 / self.atlas_size as f32;
                let y = bounds.y as f32 / self.atlas_size as f32;

                glyph.tex_coords[0] = Vec2 { x, y };
                glyph.tex_coords[1] = Vec2 { x: x + w, y };
                glyph.tex_coords[2] = Vec2 { x: x + w, y: y + h };
                glyph.tex_coords[3] = Vec2 { x, y: y + h };

                let row_end = bounds.y + bounds.h;
                let col_end = bounds.x + bounds.w;

                // Copy glyph pixels to atlas pixels
                let mut row = bounds.y;
                let mut src_row = 0;
                while row < row_end {
                    let mut col = bounds.x;
                    let mut src_col = 0;
                    while col < col_end {
                        self.atlas[(row * self.atlas_size + col) as usize] = glyph.pixels[(src_row * bounds.w + src_col) as usize];
                        col += 1;
                        src_col += 1;
                    }

                    row += 1;
                    src_row += 1;
                }
            } else {
                println!("Insufficient atlas size!");
            }
        }
    }
}

impl FontGlyph {
    #[inline]
    pub fn get_bitmap_top(&self) -> f32 {
        self.bitmap_top
    }

    #[inline]
    pub fn get_bitmap_left(&self) -> f32 {
        self.bitmap_left
    }

    #[inline]
    pub fn get_bitmap_width(&self) -> f32 {
        self.bitmap_width
    }

    #[inline]
    pub fn get_bitmap_height(&self) -> f32 {
        self.bitmap_height
    }

    #[inline]
    pub fn get_pixels(&self) -> &[u8] {
        self.pixels.as_slice()
    }

    #[inline]
    pub fn get_advance(&self) -> f32 {
        self.advance
    }

    #[inline]
    pub fn has_outline(&self) -> bool {
        self.has_outline
    }

    #[inline]
    pub fn get_tex_coords(&self) -> &[Vec2; 4] {
        &self.tex_coords
    }
}

#[test]
fn font_test() {
    use image::ColorType;
    use std::path::PathBuf;

    let font = Font::load(Path::new("data/fonts/font.ttf"), 40.0, (0..255).collect()).unwrap();
    let raster_path = Path::new("data/raster");
    if !raster_path.exists() {
        std::fs::create_dir(raster_path).unwrap();
    }
    let path = PathBuf::from("data/raster/_atlas.png");
    image::save_buffer(path, font.atlas.as_slice(),
                       font.atlas_size as u32,
                       font.atlas_size as u32,
                       ColorType::Gray(8)).unwrap();
}