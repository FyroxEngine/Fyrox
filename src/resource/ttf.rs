// TTF loader and rasterizer

const ON_CURVE_POINT: u8 = 1;
const REPEAT_FLAG: u8 = 8;

struct Point {
    x: f32,
    y: f32,
    flags: u8,
}

struct Polygon {
    points: Vec<Point>
}

struct Glyph {
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
    glyphs: Vec<Glyph>,
}

fn get_u16(p: *const u8) -> u16
{
    unsafe {
        let b0 = p.read() as u16;
        let b1 = p.offset(1).read() as u16;
        return b0 * 256 + b1;
    }
}

fn get_i16(p: *const u8) -> i16
{
    unsafe {
        let b0 = p.read() as i16;
        let b1 = p.offset(1).read() as i16;
        return b0 * 256 + b1;
    }
}

fn get_u32(p: *const u8) -> u32
{
    unsafe {
        let b0 = p.read() as u32;
        let b1 = p.offset(1).read() as u32;
        let b2 = p.offset(2).read() as u32;
        let b3 = p.offset(3).read() as u32;
        return (b0 << 24) + (b1 << 16) + (b2 << 8) + b3;
    }
}

fn fourcc(d: u8, c: u8, b: u8, a: u8) -> u32 {
    ((d as u32) << 24) | ((c as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

fn segmented_mapping(subtable: *const u8, unicode: u32) -> u32 {
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
                    return (unicode + get_u16(id_delta.offset(2 * segment as isize)) as u32) & 0xFFFF;
                } else {
                    let offset = 2 * (segment + range_offset + (unicode - start_code));
                    return get_u16(id_range_offset.offset(offset as isize)) as u32;
                }
            }
        }

        return 0;
    }
}

fn direct_mapping(subtable: *const u8, unicode: u32) -> u32 {
    unsafe {
        if unicode < 256 {
            let indices = subtable.offset(6);
            return indices.offset(unicode as isize).read() as u32;
        }
    }

    return 0;
}

fn dense_mapping(subtable: *const u8, unicode: u32) -> u32 {
    unsafe {
        let first = get_u16(subtable.offset(6)) as u32;
        let entry_count = get_u16(subtable.offset(8)) as u32;
        let indices = subtable.offset(10);

        if unicode > first && unicode < first + entry_count {
            return get_u16(indices.offset(2 * (unicode - first) as isize)) as u32;
        }
    }

    return 0;
}

impl TrueType {
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

    fn unicode_to_glyph_index(&self, unicode: u32) -> u32 {
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

        return 0;
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

    fn read_glyphs(&mut self) {
        unsafe {
            self.glyphs = Vec::with_capacity(self.num_glyphs as usize);
            for i in 0..(self.num_glyphs as usize) {
                let offset = self.get_glyph_offset(i);
                let next_offset = self.get_glyph_offset(i + 1);
                let glyph_data = self.glyf_table.offset(offset);

                let mut glyph = Glyph {
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
                        points.get_unchecked_mut(j).flags = pt_flag;
                        flags = flags.offset(1);
                        if (pt_flag & REPEAT_FLAG) != 0 {
                            let repeat_count = flags.read() as usize;
                            flags = flags.offset(1);
                            let mut k = 1;
                            while k <= repeat_count {
                                points.get_unchecked_mut(j + k).flags = pt_flag;
                                k += 1;
                            }
                            j += repeat_count;
                        }
                    }

                    /* Read x-coordinates for each point */
                    let mut coords = flags;
                    let mut x = 0;
                    for j in 0..(point_count as usize) {
                        let pt = points.get_unchecked_mut(j);
                        if (pt.flags & 2) != 0 {
                            let dx = coords.read() as i32;
                            coords = coords.offset(2);
                            x += if (pt.flags & 16) != 0 { dx } else { -dx };
                        } else {
                            if (pt.flags & 16) == 0 {
                                x += get_i16(coords) as i32;
                                coords = coords.offset(2);
                            }
                        }
                        pt.x = x as f32;
                    }

                    /* Read y-coordinates for each point */
                    let mut y = 0;
                    for j in 0..(point_count as usize) {
                        let pt = points.get_unchecked_mut(j);
                        if (pt.flags & 4) != 0 {
                            let dy = coords.read() as i32;
                            coords = coords.offset(1);
                            y += if (pt.flags & 32) != 0 { dy } else { -dy };
                        } else {
                            if (pt.flags & 32) == 0 {
                                y += get_i16(coords) as i32;
                                coords = coords.offset(2);
                            }
                        }
                        pt.y = y as f32;
                    }

                    glyph.prepare_contours(points);
                }

                glyph.fill_horizontal_metrics(self.hhea_table, self.hmtx_table, i);

                self.glyphs.push(glyph);
            }
        }
    }
}

impl Glyph {
    fn fill_horizontal_metrics(&mut self, hhea_table: *const u8, hmtx_table: *const u8, glyph_index: usize) {
        unsafe {
            let num_of_long_hor_metrics = get_u16(hhea_table.offset(34)) as usize;
            if glyph_index < num_of_long_hor_metrics {
                self.advance = get_u16(hmtx_table.offset(4 * glyph_index as isize));
                self.left_side_bearing = get_i16(hmtx_table.offset((4 * glyph_index + 2) as isize));
            } else {
                self.advance = get_u16(hmtx_table.offset(4 * (num_of_long_hor_metrics - 1) as isize));
                self.left_side_bearing = get_i16(hmtx_table.offset((4 * num_of_long_hor_metrics + 2 * (glyph_index - num_of_long_hor_metrics)) as isize));
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
                    &unpacked_contour.points.push(flipped);

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