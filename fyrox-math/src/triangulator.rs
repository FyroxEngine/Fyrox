use nalgebra::{Vector2, Vector3};
use std::fmt;

///
/// Polygon vertex
///
#[derive(Debug)]
struct Vertex {
    position: Vector2<f32>,
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
    #[inline]
    fn remove_vertex(&mut self, index: usize) {
        let next_index = self.vertices[index].next;
        let prev_index = self.vertices[index].prev;

        let prev = &mut self.vertices[prev_index];
        prev.next = next_index;

        let next = &mut self.vertices[next_index];
        next.prev = prev_index;

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
            writeln!(
                f,
                "Vertex {:?}; {} {} {}",
                vertex.position, vertex.prev, vertex.index, vertex.next
            )?;
            i = self.vertices[i].next;
            if i == self.head {
                break;
            }
        }
        Ok(())
    }
}

fn is_ear(poly: &Polygon, prev: &Vertex, ear: &Vertex, next: &Vertex) -> bool {
    // Check if other points are inside triangle
    let mut i = poly.head;
    loop {
        let vertex = &poly.vertices[i];
        if i != prev.index
            && i != ear.index
            && i != next.index
            && crate::is_point_inside_2d_triangle(
                vertex.position,
                prev.position,
                ear.position,
                next.position,
            )
        {
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
pub fn triangulate(vertices: &[Vector3<f32>], out_triangles: &mut Vec<[usize; 3]>) {
    out_triangles.clear();
    if vertices.len() == 3 {
        // Triangulating a triangle?
        out_triangles.push([0, 1, 2]);
    } else if vertices.len() == 4 {
        // Special case for quadrilaterals (much faster than generic)
        let mut start_vertex = 0;
        for i in 0..4 {
            let v = vertices[i];
            let v0 = vertices[(i + 3) % 4];
            if let Some(left) = (v0 - v).try_normalize(f32::EPSILON) {
                let v1 = vertices[(i + 2) % 4];
                if let Some(diag) = (v1 - v).try_normalize(f32::EPSILON) {
                    let v2 = vertices[(i + 1) % 4];
                    if let Some(right) = (v2 - v).try_normalize(f32::EPSILON) {
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
        out_triangles.push([start_vertex, (start_vertex + 1) % 4, (start_vertex + 2) % 4]);
        out_triangles.push([start_vertex, (start_vertex + 2) % 4, (start_vertex + 3) % 4]);
    } else {
        // Ear-clipping for arbitrary polygon (requires one additional memory allocation, so
        // relatively slow)
        if let Ok(normal) = crate::get_polygon_normal(vertices) {
            let plane_class = crate::classify_plane(normal);
            let mut polygon = Polygon {
                vertices: vertices
                    .iter()
                    .enumerate()
                    .map(|(i, point)| Vertex {
                        position: crate::vec3_to_vec2_by_plane(plane_class, normal, *point),
                        index: i,
                        prev: if i == 0 { vertices.len() - 1 } else { i - 1 },
                        next: if i == vertices.len() - 1 { 0 } else { i + 1 },
                    })
                    .collect(),
                head: 0,
                tail: vertices.len() - 1,
            };
            let mut ear_index = polygon.head;
            let mut vertices_left = polygon.vertices.len();
            while vertices_left > 3 {
                let ear = &polygon.vertices[ear_index];
                let prev = &polygon.vertices[ear.prev];
                let next = &polygon.vertices[ear.next];
                if is_ear(&polygon, prev, ear, next) {
                    let prev_index = prev.index;
                    out_triangles.push([prev_index, ear.index, next.index]);
                    polygon.remove_vertex(ear_index);
                    ear_index = prev_index;
                    vertices_left -= 1;
                } else {
                    ear_index = ear.next;
                }
            }
            // Append last triangle.
            if vertices_left > 0 {
                let a = &polygon.vertices[polygon.head];
                let b = &polygon.vertices[a.next];
                out_triangles.push([polygon.head, a.next, b.next]);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use nalgebra::Vector2;

    use crate::triangulator::triangulate;
    use nalgebra::{Point3, Unit, UnitQuaternion, Vector3};

    use super::{Polygon, Vertex};

    #[test]
    fn triangle_triangulation() {
        let polygon = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ];

        let mut ref_indices = Vec::new();
        triangulate(polygon.as_slice(), &mut ref_indices);
        assert_ne!(ref_indices.len(), 0);
    }

    #[test]
    fn quadrilaterals_triangulation_non_concave() {
        let polygon = vec![
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 2.0, 1.0),
            Vector3::new(2.0, 3.0, 1.0),
            Vector3::new(3.0, 2.0, 1.0),
        ];

        let mut ref_indices = Vec::new();
        triangulate(polygon.as_slice(), &mut ref_indices);
        assert_ne!(ref_indices.len(), 0);
    }

    #[test]
    fn quadrilaterals_triangulation_concave() {
        let polygon = vec![
            Vector3::new(0.0, 2.0, 1.0),
            Vector3::new(3.0, 3.0, 1.0),
            Vector3::new(2.0, 2.0, 1.0),
            Vector3::new(3.0, 1.0, 1.0),
        ];

        let mut ref_indices = Vec::new();
        triangulate(polygon.as_slice(), &mut ref_indices);
        assert_ne!(ref_indices.len(), 0);
    }

    #[test]
    fn ear_clip_test() {
        let polygon = vec![
            Vector3::new(-22.760103, 29.051392, 1.377507),
            Vector3::new(-24.6454, 29.051392, 1.377507),
            Vector3::new(-24.640476, 24.873882, 1.377506),
            Vector3::new(-24.637342, 22.215763, 1.377506),
            Vector3::new(-22.760103, 22.215763, 1.377506),
        ];

        // First test flat case
        let mut ref_indices = Vec::new();
        triangulate(polygon.as_slice(), &mut ref_indices);
        assert_ne!(ref_indices.len(), 0);

        // Then compare previous result with series of rotated versions of the polygon.
        for axis in &[
            Unit::new_normalize(Vector3::new(1.0, 0.0, 0.0)),
            Unit::new_normalize(Vector3::new(0.0, 1.0, 0.0)),
            Unit::new_normalize(Vector3::new(0.0, 0.0, 1.0)),
            Unit::new_normalize(Vector3::new(1.0, 1.0, 1.0)),
        ] {
            let mut angle: f32 = 0.0;
            while angle <= 360.0 {
                let mrot =
                    UnitQuaternion::from_axis_angle(axis, angle.to_radians()).to_homogeneous();
                let rotated: Vec<Vector3<f32>> = polygon
                    .iter()
                    .map(|v: &Vector3<f32>| mrot.transform_point(&Point3::from(*v)).coords)
                    .collect();
                let mut new_indices = Vec::new();
                triangulate(rotated.as_slice(), &mut new_indices);
                // We just need to ensure that we have the same amount of triangles as reference triangulation.
                assert_eq!(new_indices.len(), ref_indices.len());
                angle += 36.0;
            }
        }
    }

    #[test]
    fn test_debug_for_polygon() {
        let p = Polygon {
            vertices: vec![
                Vertex {
                    prev: 2,
                    index: 0,
                    next: 1,
                    position: Vector2::new(0.0, 0.0),
                },
                Vertex {
                    prev: 0,
                    index: 1,
                    next: 2,
                    position: Vector2::new(1.0, 0.0),
                },
                Vertex {
                    prev: 1,
                    index: 2,
                    next: 0,
                    position: Vector2::new(0.0, 1.0),
                },
            ],
            head: 0,
            tail: 2,
        };

        assert_eq!(
            format!("{p:?}"),
            r"Vertex [[0.0, 0.0]]; 2 0 1
Vertex [[1.0, 0.0]]; 0 1 2
Vertex [[0.0, 1.0]]; 1 2 0
"
        );
    }
}
