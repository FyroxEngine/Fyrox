use crate::math::vec3::Vec3;
use crate::utils::pool::{Pool, Handle};
use crate::math::ray::Ray;
use crate::math::plane::Plane;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Contact {
    pub body: Handle<Body>,
    pub position: Vec3,
    pub normal: Vec3,
    pub triangle_index: usize,
}

#[derive(Serialize, Deserialize)]
pub struct StaticGeometry {
    triangles: Vec<StaticTriangle>
}

impl StaticGeometry {
    pub fn new() -> StaticGeometry {
        StaticGeometry {
            triangles: Vec::new()
        }
    }

    pub fn add_triangle(&mut self, triangle: StaticTriangle) {
        self.triangles.push(triangle);
    }
}

#[derive(Serialize, Deserialize)]
pub struct StaticTriangle {
    points: [Vec3; 3],
    #[serde(skip)]
    ca: Vec3,
    #[serde(skip)]
    ba: Vec3,
    #[serde(skip)]
    ca_dot_ca: f32,
    #[serde(skip)]
    ca_dot_ba: f32,
    #[serde(skip)]
    ba_dot_ba: f32,
    #[serde(skip)]
    edges: [Ray; 3],
    #[serde(skip)]
    inv_denom: f32,
    #[serde(skip)]
    plane: Plane,
}

impl StaticTriangle {
    ///
    /// Creates static triangle from tree points and precomputes some data
    /// to speedup collision detection in runtime. This function may fail
    /// if degenerated triangle was passed into.
    ///
    pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> Option<StaticTriangle> {
        let ca = c - a;
        let ba = b - a;
        let ca_dot_ca = ca.dot(&ca);
        let ca_dot_ba = ca.dot(&ba);
        let ba_dot_ba = ba.dot(&ba);
        if let Some(plane) = Plane::from_normal_and_point(ba.cross(&ca), a) {
            let ab_ray = Ray::from_two_points(a, b)?;
            let bc_ray = Ray::from_two_points(b, c)?;
            let ca_ray = Ray::from_two_points(c, a)?;
            return Some(StaticTriangle {
                points: [a, b, c],
                ba,
                ca: c - a,
                edges: [ab_ray, bc_ray, ca_ray],
                ca_dot_ca,
                ca_dot_ba,
                ba_dot_ba,
                inv_denom: 1.0 / (ca_dot_ca * ba_dot_ba - ca_dot_ba * ca_dot_ba),
                plane,
            });
        }

        None
    }

    /// Checks if point lies inside or at edge of triangle. Uses a lot of precomputed data.
    pub fn contains_point(&self, p: Vec3) -> bool {
        let vp = p - self.points[0];
        let dot02 = self.ca.dot(&vp);
        let dot12 = self.ba.dot(&vp);
        let u = (self.ba_dot_ba * dot02 - self.ca_dot_ba * dot12) * self.inv_denom;
        let v = (self.ca_dot_ca * dot12 - self.ca_dot_ba * dot02) * self.inv_denom;
        u >= 0.0 && v >= 0.0 && u + v < 1.0
    }
}

#[derive(Serialize, Deserialize)]
pub struct Body {
    position: Vec3,
    last_position: Vec3,
    acceleration: Vec3,
    contacts: Vec<Contact>,
    friction: f32,
    gravity: Vec3,
    radius: f32,
    sqr_radius: f32,
    speed_limit: f32,
}

impl Body {
    pub fn new() -> Body {
        Body {
            position: Vec3::zero(),
            last_position: Vec3::zero(),
            acceleration: Vec3::zero(),
            friction: 0.2,
            gravity: Vec3::make(0.0, -9.81, 0.0),
            radius: 1.0,
            sqr_radius: 1.0,
            contacts: Vec::new(),
            speed_limit: 0.75,
        }
    }

    #[inline]
    pub fn make_copy(&self) -> Body {
        Body {
            position: self.position,
            last_position: self.last_position,
            acceleration: self.acceleration,
            contacts: Vec::new(),
            friction: self.friction,
            gravity: self.gravity,
            radius: self.radius,
            sqr_radius: self.sqr_radius,
            speed_limit: self.speed_limit
        }
    }

    #[inline]
    pub fn get_position(&self) -> Vec3 {
        self.position
    }

    #[inline]
    pub fn set_position(&mut self, p: Vec3) {
        self.position = p;
        self.last_position = p;
    }

    #[inline]
    pub fn move_by(&mut self, v: Vec3) {
        self.position += v;
    }

    #[inline]
    pub fn set_radius(&mut self, r: f32) {
        self.radius = r;
        self.sqr_radius = r * r;
    }

    #[inline]
    pub fn set_friction(&mut self, friction: f32) {
        self.friction = friction;

        if self.friction < 0.0 {
            self.friction = 0.0;
        } else if self.friction > 1.0 {
            self.friction = 1.0;
        }
    }

    #[inline]
    pub fn get_friction(&self) -> f32 {
        self.friction
    }

    #[inline]
    pub fn get_radius(&self) -> f32 {
        self.radius
    }

    #[inline]
    pub fn set_x_velocity(&mut self, x: f32) {
        self.last_position.x = self.position.x - x;
    }

    #[inline]
    pub fn set_y_velocity(&mut self, y: f32) {
        self.last_position.y = self.position.y - y;
    }

    #[inline]
    pub fn set_z_velocity(&mut self, z: f32) {
        self.last_position.z = self.position.z - z;
    }

    #[inline]
    pub fn get_contacts(&self) -> &[Contact] {
        self.contacts.as_slice()
    }

    pub fn verlet(&mut self, sqr_delta_time: f32, air_friction: f32) {
        let friction =
            if self.contacts.len() > 0 {
                self.friction
            } else {
                air_friction
            };

        let k1 = 2.0 - friction;
        let k2 = 1.0 - friction;

        let last_position = self.position;

        // Verlet integration
        self.position = Vec3 {
            x: k1 * self.position.x - k2 * self.last_position.x + self.acceleration.x * sqr_delta_time,
            y: k1 * self.position.y - k2 * self.last_position.y + self.acceleration.y * sqr_delta_time,
            z: k1 * self.position.z - k2 * self.last_position.z + self.acceleration.z * sqr_delta_time,
        };

        self.last_position = last_position;

        self.acceleration = Vec3::zero();

        let velocity = self.last_position - self.position;
        let sqr_speed = velocity.sqr_len();
        if sqr_speed > self.speed_limit * self.speed_limit {
            if let Some(direction) = velocity.normalized() {
                self.last_position = self.position - direction.scale(self.speed_limit);
            }
        }
    }

    /// Checks if body intersects with a triangle.
    /// Returns intersection point if there was intersection.
    pub fn insersect_triangle(&self, triangle: &StaticTriangle) -> Option<Vec3> {
        let distance = triangle.plane.distance(self.position);
        if distance <= self.radius {
            let intersection_point = self.position - triangle.plane.normal.scale(distance);
            if triangle.contains_point(intersection_point) {
                return Some(intersection_point);
            } else {
                // Check intersection with each edge.
                for edge in &triangle.edges {
                    if edge.is_intersect_sphere(self.position, self.radius) {
                        let t = edge.project_point(self.position);
                        if t >= 0.0 && t <= 1.0 {
                            return Some(edge.get_point(t));
                        }
                    }
                }

                // Finally check if body contains any vertex of a triangle.
                for point in &triangle.points {
                    if (*point - self.position).sqr_len() <= self.sqr_radius {
                        return Some(*point);
                    }
                }
            }
        }
        None
    }

    pub fn solve_triangle_collision(&mut self, triangle: &StaticTriangle, triangle_index: usize) {
        if let Some(intersection_point) = self.insersect_triangle(triangle) {
            let (direction, length) = (self.position - intersection_point).normalized_ex();
            if let Some(push_vector) = direction {
                self.position += push_vector.scale(self.radius - length);

                self.contacts.push(Contact {
                    body: Handle::none(),
                    position: intersection_point,
                    normal: push_vector,
                    triangle_index,
                })
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Physics {
    body_pool: Pool<Body>,
    static_geom_pool: Pool<StaticGeometry>,
}

impl Physics {
    pub fn new() -> Physics {
        Physics {
            body_pool: Pool::new(),
            static_geom_pool: Pool::new(),
        }
    }

    pub fn add_body(&mut self, body: Body) -> Handle<Body> {
        self.body_pool.spawn(body)
    }

    pub fn remove_body(&mut self, body_handle: Handle<Body>) {
        self.body_pool.free(body_handle);
    }

    pub fn add_static_geometry(&mut self, static_geom: StaticGeometry) -> Handle<StaticGeometry> {
        self.static_geom_pool.spawn(static_geom)
    }

    pub fn remove_static_geometry(&mut self, static_geom: Handle<StaticGeometry>) {
        self.static_geom_pool.free(static_geom);
    }

    pub fn borrow_body(&self, handle: &Handle<Body>) -> Option<&Body> {
        self.body_pool.borrow(handle)
    }

    pub fn borrow_body_mut(&mut self, handle: &Handle<Body>) -> Option<&mut Body> {
        self.body_pool.borrow_mut(handle)
    }

    pub fn step(&mut self, delta_time: f32) {
        let dt2 = delta_time * delta_time;
        let air_friction = 0.003;

        for body in self.body_pool.iter_mut() {
            body.acceleration += body.gravity;
            body.verlet(dt2, air_friction);

            body.contacts.clear();

            for static_geometry in self.static_geom_pool.iter() {
                for (n, triangle) in static_geometry.triangles.iter().enumerate() {
                    body.solve_triangle_collision(&triangle, n);
                }
            }
        }
    }
}
