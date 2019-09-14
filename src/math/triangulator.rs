use crate::{
    math::{
        vec2::*,
        vec3::*,
        self,
    }
};
use std::fmt;

///
/// Polygon vertex
///
#[derive(Debug)]
struct Vertex {
    position: Vec2,
    prev: usize,
    index: usize,
    next: usize,
}

///
/// Linked list of vertices
///
struct Polygon {
    vertices: Vec<Vertex>,
    head: usize,
    tail: usize,
}

impl Polygon {
    ///
    /// Excludes vertex from polygon. Does not remove it from vertices array!
    ///
    fn remove_vertex(&mut self, index: usize) {
        let next_index = self.vertices[index].next;
        let prev_index = self.vertices[index].prev;

        {
            let prev = &mut self.vertices[prev_index];
            prev.next = next_index;
        }

        {
            let next = &mut self.vertices[next_index];
            next.prev = prev_index;
        }

        if index == self.head {
            self.head = next_index;
        }

        if index == self.tail {
            self.tail = prev_index;
        }
    }
}

impl fmt::Debug for Polygon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut i = self.head;
        loop {
            let vertex = &self.vertices[i];
            writeln!(f, "Vertex {:?}; {} {} {}", vertex.position, vertex.prev, vertex.index, vertex.next)?;
            i = self.vertices[i].next;
            if i == self.head {
                break;
            }
        }
        Ok(())
    }
}

fn is_ear(poly: &Polygon, prev: &Vertex, ear: &Vertex, next: &Vertex) -> bool {
    // Check winding
    if math::get_signed_triangle_area(prev.position, ear.position, next.position) >= 0.0 {
        return false;
    }

    // Check if other points are inside triangle
    let mut i = poly.head;
    loop {
        let vertex = &poly.vertices[i];
        if i != prev.index && i != ear.index && i != next.index &&
            math::is_point_inside_2d_triangle(vertex.position, prev.position, ear.position, next.position) {
            return false;
        }
        i = vertex.next;
        if i == poly.head {
            break;
        }
    }

    true
}

///
/// Triangulates specified polygon.
///
pub fn triangulate(vertices: &[Vec3], out_triangles: &mut Vec<(usize, usize, usize)>) {
    out_triangles.clear();
    if vertices.len() == 3 {
        // Triangulating a triangle?
        out_triangles.push((0, 1, 2));
    } else if vertices.len() == 4 {
        // Special case for quadrilaterals (much faster than generic)
        let mut start_vertex = 0;
        for i in 0..4 {
            let v = vertices[i];
            let v0 = vertices[(i + 3) % 4];
            if let Some(left) = (v0 - v).normalized() {
                let v1 = vertices[(i + 2) % 4];
                if let Some(diag) = (v1 - v).normalized() {
                    let v2 = vertices[(i + 1) % 4];
                    if let Some(right) = (v2 - v).normalized() {
                        // Check for concave vertex
                        let angle = left.dot(&diag).acos() + right.dot(&diag).acos();
                        if angle > std::f32::consts::PI {
                            start_vertex = i;
                            break;
                        }
                    }
                }
            }
        }
        out_triangles.push((start_vertex, (start_vertex + 1) % 4, (start_vertex + 2) % 4));
        out_triangles.push((start_vertex, (start_vertex + 2) % 4, (start_vertex + 3) % 4));
    } else {
        // Ear-clipping for arbitrary polygon (requires one additional memory allocation, so
        // relatively slow)
        if let Ok(normal) = math::get_polygon_normal(&vertices) {
            let plane_class = math::classify_plane(normal);
            let mut polygon = Polygon {
                vertices: vertices.iter().enumerate().map(|(i, point)| {
                    Vertex {
                        position: math::vec3_to_vec2_by_plane(plane_class, normal, *point),
                        index: i,
                        prev: if i == 0 { vertices.len() - 1 } else { i - 1 },
                        next: if i == vertices.len() - 1 { 0 } else { i + 1 },
                    }
                }).collect(),
                head: 0,
                tail: vertices.len() - 1,
            };
            let mut ear_index = polygon.head;
            let mut vertices_left = polygon.vertices.len();
            while vertices_left >= 3 {
                if cfg!(test) {
                    println!("{:?}", polygon);
                }
                let ear = &polygon.vertices[ear_index];
                let prev = &polygon.vertices[ear.prev];
                let next = &polygon.vertices[ear.next];
                if is_ear(&polygon, prev, ear, next) {
                    let prev_index = prev.index;
                    out_triangles.push((prev_index, ear.index, next.index));
                    polygon.remove_vertex(ear_index);
                    ear_index = prev_index;
                    vertices_left -= 1;
                } else {
                    ear_index = ear.next;
                }
            }
        }
    }
}

#[test]
fn quadrilaterals_triangulation_non_concave() {
    let polygon = vec![
        Vec3::make(0.0, 0.0, 1.0),
        Vec3::make(1.0, 2.0, 1.0),
        Vec3::make(2.0, 3.0, 1.0),
        Vec3::make(3.0, 2.0, 1.0)
    ];

    let mut ref_indices: Vec<(usize, usize, usize)> = Vec::new();
    triangulate(polygon.as_slice(), &mut ref_indices);
    println!("{:?}", ref_indices);
    assert_ne!(ref_indices.len(), 0);
}

#[test]
fn quadrilaterals_triangulation_concave() {
    let polygon = vec![
        Vec3::make(0.0, 2.0, 1.0),
        Vec3::make(3.0, 3.0, 1.0),
        Vec3::make(2.0, 2.0, 1.0),
        Vec3::make(3.0, 1.0, 1.0)
    ];

    let mut ref_indices: Vec<(usize, usize, usize)> = Vec::new();
    triangulate(polygon.as_slice(), &mut ref_indices);
    println!("{:?}", ref_indices);
    assert_ne!(ref_indices.len(), 0);
}

#[test]
fn ear_clip_test() {
    let polygon = vec![
        Vec3::make(0.0, 0.0, 1.0),
        Vec3::make(1.0, 2.0, 1.0),
        Vec3::make(2.0, 4.0, 1.0),
        Vec3::make(3.0, 2.0, 1.0),
        Vec3::make(4.0, 1.0, 1.0),
        Vec3::make(3.0, 0.0, 1.0),
        Vec3::make(2.0, 0.5, 1.0),
    ];

    // First test flat case
    let mut ref_indices: Vec<(usize, usize, usize)> = Vec::new();
    triangulate(polygon.as_slice(), &mut ref_indices);
    println!("{:?}", ref_indices);
    assert_ne!(ref_indices.len(), 0);

    use crate::math::{mat4::Mat4, quat::Quat};

    // Then compare previous result with series of rotated versions of the polygon
    // This could give false fails because of not sufficient precision of f32 when
    // there is a polygon with an edge containing other polygon vertex or if trying to
    // triangulate non-flat polygon - in this case there will be sligthly different
    // order of indices but visually result stays correct. So for test I'm not using
    // such polygons just to not trigger false fails.
    for axis in &[
        Vec3::make(1.0, 0.0, 0.0),
        Vec3::make(0.0, 1.0, 0.0),
        Vec3::make(0.0, 0.0, 1.0),
        Vec3::make(1.0, 1.0, 1.0)] {
        let mut angle: f32 = 0.0;
        while angle <= 360.0 {
            let mrot = Mat4::from_quat(Quat::from_axis_angle(*axis, angle.to_radians()));
            let rotated: Vec<Vec3> = polygon.iter().map(|v| mrot.transform_vector(*v)).collect();
            let mut new_indices: Vec<(usize, usize, usize)> = Vec::new();
            triangulate(rotated.as_slice(), &mut new_indices);
            println!("angle: {} {:?}", angle, new_indices);
            assert_eq!(new_indices, ref_indices);
            angle += 36.0;
        }
    }
}
