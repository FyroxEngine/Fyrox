/// Gilbert-Johnson-Keerthi (GJK) intersection test +  Expanding Polytope
/// Algorithm (EPA) implementations.
///
/// "Implementing GJK", Casey Muratori:
/// https://www.youtube.com/watch?v=Qupqu1xe7Io
///
/// "GJK + Expanding Polytope Algorithm - Implementation and Visualization"
/// https://www.youtube.com/watch?v=6rgiPrzqt9w
///
/// Some ideas about contact point generation were taken from here
/// https://www.gamedev.net/forums/topic/598678-contact-points-with-epa/

use rg3d_core::{
    math::vec3::Vec3,
    math,
};
use crate::convex_shape::ConvexShape;

pub const GJK_MAX_ITERATIONS: usize = 64;
pub const EPA_TOLERANCE: f32 = 0.0001;
pub const EPA_MAX_ITERATIONS: usize = 64;
pub const EPA_MAX_LOOSE_EDGES: usize = 32;
pub const EPA_MAX_FACES: usize = 64;

/// Vertex in space of Minkowski sum
#[derive(Copy, Clone)]
pub struct MinkowskiVertex {
    /// World space position of vertex of shape A. This position will be used
    /// to compute contact point in world space.
    shape_a_world_space: Vec3,

    /// Minkowski difference between point in shape A and point in shape B.
    /// This position will be used to do all simplex and polytope operations.
    /// https://en.wikipedia.org/wiki/Minkowski_addition
    minkowski_dif: Vec3,
}

impl Default for MinkowskiVertex {
    fn default() -> Self {
        Self {
            shape_a_world_space: Default::default(),
            minkowski_dif: Default::default(),
        }
    }
}

#[derive(Copy, Clone)]
struct PolytopeTriangle {
    vertices: [MinkowskiVertex; 3],
    normal: Vec3,
}

impl Default for PolytopeTriangle {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            normal: Default::default(),
        }
    }
}

#[derive(Copy, Clone)]
struct PolytopeEdge {
    begin: MinkowskiVertex,
    end: MinkowskiVertex,
}

impl Default for PolytopeEdge {
    fn default() -> Self {
        Self {
            begin: Default::default(),
            end: Default::default(),
        }
    }
}

impl PolytopeEdge {
    /// Returns true if edges are equal in combination like this:
    /// ab = ba
    fn eq_ccw(&self, other: &PolytopeEdge) -> bool {
        self.begin.minkowski_dif == other.end.minkowski_dif &&
            self.end.minkowski_dif == other.begin.minkowski_dif
    }
}

pub struct Simplex {
    /// Vertices of simplex.
    /// Important: a is most recently added point (closest to origin)!
    a: MinkowskiVertex,
    b: MinkowskiVertex,
    c: MinkowskiVertex,
    d: MinkowskiVertex,
    /// Rank of simplex
    /// 1 - point
    /// 2 - line
    /// 3 - triangle
    /// 4 - tetrahedron
    rank: usize,
}

impl Default for Simplex {
    fn default() -> Self {
        Self {
            a: Default::default(),
            b: Default::default(),
            c: Default::default(),
            d: Default::default(),
            rank: 0,
        }
    }
}

impl Simplex {
    fn update_triangle(&mut self) -> Vec3 {
        let ca = self.c.minkowski_dif - self.a.minkowski_dif;
        let ba = self.b.minkowski_dif - self.a.minkowski_dif;

        // Direction to origin
        let ao = -self.a.minkowski_dif;

        self.rank = 2;

        let triangle_normal = ba.cross(&ca);

        if ba.cross(&triangle_normal).is_same_direction_as(&ao) {
            // Closest to edge AB
            self.c = self.a;
            return ba.cross(&ao).cross(&ba);
        }

        if triangle_normal.cross(&ca).is_same_direction_as(&ao) {
            // Closest to edge AC
            self.b = self.a;
            return ca.cross(&ao).cross(&ca);
        }

        self.rank = 3;

        if triangle_normal.is_same_direction_as(&ao) {
            // Above triangle
            self.d = self.c;
            self.c = self.b;
            self.b = self.a;
            return triangle_normal;
        }

        // Below triangle
        self.d = self.b;
        self.b = self.a;

        -triangle_normal
    }

    fn update_tetrahedron(&mut self) -> Result<(), Vec3> {
        // Point a is tip of pyramid, BCD is the base (counterclockwise winding order)

        // Direction to origin
        let ao = -self.a.minkowski_dif;

        // Plane-test origin with 3 faces. This is very inaccurate approach and
        // it would be better to add additional checks for each face of tetrahedron
        // to select search direction more precisely. In this case we assume that
        //we always will produce triangles as final simplex.
        self.rank = 3;

        let ba = self.b.minkowski_dif - self.a.minkowski_dif;
        let ca = self.c.minkowski_dif - self.a.minkowski_dif;

        let abc_normal = ba.cross(&ca);
        if abc_normal.is_same_direction_as(&ao) {
            // In front of ABC
            self.d = self.c;
            self.c = self.b;
            self.b = self.a;
            return Err(abc_normal);
        }

        let da = self.d.minkowski_dif - self.a.minkowski_dif;
        let acd_normal = ca.cross(&da);
        if acd_normal.is_same_direction_as(&ao) {
            // In front of ACD
            self.b = self.a;
            return Err(acd_normal);
        }

        let adb_normal = da.cross(&ba);
        if adb_normal.is_same_direction_as(&ao) {
            // In front of ADB
            self.c = self.d;
            self.d = self.b;
            self.b = self.a;
            return Err(adb_normal);
        }

        // Otherwise origin is inside tetrahedron
        Ok(())
    }
}

fn de_gjk_support(shape1: &ConvexShape, shape1_position: Vec3, shape2: &ConvexShape, shape2_position: Vec3, dir: &Vec3) -> MinkowskiVertex
{
    let shape_a_world_space = shape1.get_farthest_point(shape1_position, *dir);
    let b = shape2.get_farthest_point(shape2_position, -*dir);

    MinkowskiVertex {
        shape_a_world_space,
        minkowski_dif: shape_a_world_space - b,
    }
}

pub fn gjk_is_intersects(shape1: &ConvexShape, shape1_position: Vec3, shape2: &ConvexShape, shape2_position: Vec3) -> Option<Simplex> {
    // This is good enough heuristic to choose initial search direction
    let mut search_dir = shape1_position - shape2_position;

    if search_dir.sqr_len() == 0.0 {
        search_dir.x = 1.0;
    }

    // Get initial point for simplex
    let mut simplex = Simplex::default();
    simplex.c = de_gjk_support(shape1, shape1_position, shape2, shape2_position, &search_dir);
    search_dir = -simplex.c.minkowski_dif; // Search in direction of origin

    // Get second point for a line segment simplex
    simplex.b = de_gjk_support(shape1, shape1_position, shape2, shape2_position, &search_dir);

    if !simplex.b.minkowski_dif.is_same_direction_as(&search_dir) {
        return None;
    }

    let cb = simplex.c.minkowski_dif - simplex.b.minkowski_dif;

    // Search perpendicular to line segment towards origin
    search_dir = cb.cross(&(-simplex.b.minkowski_dif)).cross(&cb);

    // Origin is on this line segment - fix search direction.
    if search_dir.sqr_len() == 0.0 {
        // Perpendicular with x-axis
        search_dir = cb.cross(&Vec3::make(1.0, 0.0, 0.0));
        if search_dir.sqr_len() == 0.0 {
            // Perpendicular with z-axis
            search_dir = cb.cross(&Vec3::make(0.0, 0.0, -1.0));
        }
    }

    simplex.rank = 2;
    for _ in 0..GJK_MAX_ITERATIONS {
        simplex.a = de_gjk_support(shape1, shape1_position, shape2, shape2_position, &search_dir);

        if !simplex.a.minkowski_dif.is_same_direction_as(&search_dir) {
            return None;
        }

        simplex.rank += 1;
        if simplex.rank == 3 {
            search_dir = simplex.update_triangle();
        } else {
            match simplex.update_tetrahedron() {
                Ok(_) => return Some(simplex),
                Err(dir) => search_dir = dir,
            }
        }
    }

    // No convergence - no intersection
    None
}

fn epa_compute_contact_point(closest_triangle: PolytopeTriangle) -> Vec3 {
    // Project origin onto triangle's plane
    let proj = closest_triangle.normal.scale(
        closest_triangle.vertices[0].minkowski_dif.dot(&closest_triangle.normal));

    // Find barycentric coordinates of the projection in Minkowski difference space
    let (u, v, w) = math::get_barycentric_coords(
        &proj, &closest_triangle.vertices[0].minkowski_dif,
        &closest_triangle.vertices[1].minkowski_dif,
        &closest_triangle.vertices[2].minkowski_dif);

    // Use barycentric coordinates to get projection in world space and sum all
    // vectors to get world space contact point
    closest_triangle.vertices[0].shape_a_world_space.scale(u) +
        closest_triangle.vertices[1].shape_a_world_space.scale(v) +
        closest_triangle.vertices[2].shape_a_world_space.scale(w)
}

pub struct PenetrationInfo {
    pub penetration_vector: Vec3,
    pub contact_point: Vec3,
}

pub fn epa_get_penetration_info(simplex: Simplex, shape1: &ConvexShape, shape1_position: Vec3,
                                shape2: &ConvexShape, shape2_position: Vec3) -> Option<PenetrationInfo> {
    let mut triangles = [PolytopeTriangle::default(); EPA_MAX_FACES];

    // Reconstruct polytope from tetrahedron simplex points.

    // ABC
    let ba = simplex.b.minkowski_dif - simplex.a.minkowski_dif;
    let ca = simplex.c.minkowski_dif - simplex.a.minkowski_dif;

    let abc_normal = ba.cross(&ca);
    if let Some(abc_normal) = abc_normal.normalized() {
        triangles[0] = PolytopeTriangle {
            vertices: [simplex.a, simplex.b, simplex.c],
            normal: abc_normal,
        };
    } else {
        return None;
    }

    // ACD
    let da = simplex.d.minkowski_dif - simplex.a.minkowski_dif;

    let acd_normal = ca.cross(&da);
    if let Some(acd_normal) = acd_normal.normalized() {
        triangles[1] = PolytopeTriangle {
            vertices: [simplex.a, simplex.c, simplex.d],
            normal: acd_normal,
        };
    } else {
        return None;
    }

    // ADB
    let adb_normal = da.cross(&ba);
    if let Some(adb_normal) = adb_normal.normalized() {
        triangles[2] = PolytopeTriangle {
            vertices: [simplex.a, simplex.d, simplex.b],
            normal: adb_normal,
        };
    } else {
        return None;
    }

    // BDC
    let db = simplex.d.minkowski_dif - simplex.b.minkowski_dif;
    let cb = simplex.c.minkowski_dif - simplex.b.minkowski_dif;

    let bdc_normal = db.cross(&cb);
    if let Some(bdc_normal) = bdc_normal.normalized() {
        triangles[3] = PolytopeTriangle {
            vertices: [simplex.b, simplex.d, simplex.c],
            normal: bdc_normal,
        };
    } else {
        return None;
    }

    let mut triangle_count = 4;
    let mut closest_triangle_index = 0;

    for _ in 0..EPA_MAX_ITERATIONS {
        // Find triangle that is closest to origin
        let mut min_dist = triangles[0].vertices[0].minkowski_dif.dot(&triangles[0].normal);
        closest_triangle_index = 0;
        for i in 1..triangle_count {
            let triangle = triangles[i];
            let dist = triangle.vertices[0].minkowski_dif.dot(&triangle.normal);
            if dist < min_dist {
                min_dist = dist;
                closest_triangle_index = i;
            }
        }

        // Search normal to triangle that's closest to origin
        let closest_triangle = triangles[closest_triangle_index];
        let search_dir = closest_triangle.normal;
        let new_point = de_gjk_support(shape1, shape1_position, shape2, shape2_position, &search_dir);

        let distance_to_origin = new_point.minkowski_dif.dot(&search_dir);
        if distance_to_origin - min_dist < EPA_TOLERANCE {
            return Some(PenetrationInfo {
                penetration_vector: closest_triangle.normal.scale(distance_to_origin),
                contact_point: epa_compute_contact_point(closest_triangle),
            });
        }

        // Loose edges after we remove triangle must give us list of edges we have
        // to stitch with new point to keep polytope convex.
        let mut loose_edge_count = 0;
        let mut loose_edges = [PolytopeEdge::default(); EPA_MAX_LOOSE_EDGES];

        // Find all triangles that are facing new point and remove them
        let mut i = 0;
        while i < triangle_count {
            let triangle = triangles[i];

            // If triangle i faces new point, remove it. Also search for adjacent edges of it
            // and remove them too to maintain loose edge list in correct state (see below).
            let to_new_point = new_point.minkowski_dif - triangle.vertices[0].minkowski_dif;

            if triangle.normal.dot(&to_new_point) > 0.0 {
                for j in 0..3 {
                    let current_edge = PolytopeEdge {
                        begin: triangle.vertices[j],
                        end: triangle.vertices[(j + 1) % 3],
                    };

                    let mut already_in_list = false;
                    /* Check if current edge is already in list */
                    for k in 0..loose_edge_count {
                        if loose_edges[k].eq_ccw(&current_edge) {
                            // If we found that current edge is same as other loose edge
                            // but in reverse order, then we need to replace the loose
                            // edge by the last loose edge. Lets see at this drawing and
                            // follow it step-by-step. This is pyramid with tip below A
                            // and bottom BCDE divided into 4 triangles by point A. All
                            // triangles given in CCW order.
                            //
                            //       B
                            //      /|\
                            //     / | \
                            //    /  |  \
                            //   /   |   \
                            //  /    |    \
                            // E-----A-----C
                            //  \    |    /
                            //   \   |   /
                            //    \  |  /
                            //     \ | /
                            //      \|/
                            //       D
                            //
                            // We found that triangles we want to remove are ACB (1), ADC (2),
                            // AED (3), and ABE (4). Lets start removing them from triangle ACB.
                            // Also we have to keep list of loose edges for futher linking with
                            // new point.
                            //
                            // 1. AC, CB, BA - just edges of ACB triangle in CCW order.
                            //
                            // 2. AC, CB, BA, AD, DC, (CA) - we see that CA already in list
                            //                               but in reverse order. Do not add it
                            //                               but move DC onto AC position.
                            //    DC, CB, BA, AD - loose edge list for 2nd triangle
                            //
                            // 3. DC, CB, BA, AD, AE, ED, (DA) - again we already have DA in list as AD
                            //                                   move ED to AD position.
                            //    DC, CB, BA, ED, AE
                            //
                            // 4. DC, CB, BA, ED, AE, (AB) - same AB already here as BA, move AE to BA.
                            //    continue adding rest of edges
                            //    DC, CB, AE, ED, BE, (EA) - EA already here as AE, move BE to AE
                            //
                            //    DC, CB, BE, ED - final list of loose edges which gives us
                            //
                            //       B
                            //      / \
                            //     /   \
                            //    /     \
                            //   /       \
                            //  /         \
                            // E           C
                            //  \         /
                            //   \       /
                            //    \     /
                            //     \   /
                            //      \ /
                            //       D
                            //
                            // Viola! We now have contour which we have to patch using new point.
                            loose_edge_count -= 1;
                            loose_edges.swap(k, loose_edge_count);

                            already_in_list = true;
                            break;
                        }
                    }

                    if !already_in_list {
                        // Add current edge to list
                        if loose_edge_count >= EPA_MAX_LOOSE_EDGES {
                            break;
                        }
                        loose_edges[loose_edge_count] = current_edge;
                        loose_edge_count += 1;
                    }
                }

                // Replace current triangle with last in list and discard last, so we will continue
                // processing last triangle and then next to removed. This will effectively reduce
                // amount of triangles in polytope.
                triangle_count -= 1;
                triangles.swap(i, triangle_count);
            } else {
                i += 1;
            }
        }

        // Reconstruct polytope with new point added
        for loose_edge in loose_edges[0..loose_edge_count].iter() {
            if triangle_count >= EPA_MAX_FACES {
                break;
            }
            let new_triangle = triangles.get_mut(triangle_count).unwrap();
            new_triangle.vertices = [loose_edge.begin, loose_edge.end, new_point];

            let edge_vector = loose_edge.begin.minkowski_dif - loose_edge.end.minkowski_dif;
            let begin_to_point = loose_edge.begin.minkowski_dif - new_point.minkowski_dif;

            new_triangle.normal = edge_vector.cross(&begin_to_point).normalized().unwrap_or(Vec3::up());

            // Check for wrong normal to maintain CCW winding
            let bias = 2.0 * std::f32::EPSILON;
            if new_triangle.vertices[0].minkowski_dif.dot(&new_triangle.normal) + bias < 0.0 {
                // Swap vertices to make CCW winding and flip normal.
                new_triangle.vertices.swap(0, 1);
                new_triangle.normal = -new_triangle.normal;
            }
            triangle_count += 1;
        }
    }

    // Return most recent closest point - this is still valid result but less accurate than
    // if we would have total convergence.
    let closest_triangle = triangles[closest_triangle_index];
    Some(PenetrationInfo {
        penetration_vector: closest_triangle.normal.scale(closest_triangle.vertices[0].minkowski_dif.dot(&closest_triangle.normal)),
        contact_point: epa_compute_contact_point(closest_triangle),
    })
}