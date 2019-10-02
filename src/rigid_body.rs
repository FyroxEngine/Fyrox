use rg3d_core::{
    math::vec3::Vec3,
    visitor::{Visit, VisitResult, Visitor},
    pool::Handle
};
use crate::{
    contact::Contact,
    convex_shape::{ConvexShape, TriangleShape},
    gjk_epa,
    static_geometry::StaticTriangle
};

pub struct RigidBody {
    pub(in crate) position: Vec3,
    pub(in crate) shape: ConvexShape,
    pub(in crate) last_position: Vec3,
    pub(in crate) acceleration: Vec3,
    pub(in crate) contacts: Vec<Contact>,
    pub(in crate) friction: f32,
    pub(in crate) gravity: Vec3,
    pub(in crate) speed_limit: f32,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self::new(ConvexShape::Dummy)
    }
}

impl Visit for RigidBody {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.shape.id();
        id.visit("ShapeKind", visitor)?;
        if visitor.is_reading() {
            self.shape = ConvexShape::new(id)?;
        }
        self.shape.visit("Shape", visitor)?;

        self.position.visit("Position", visitor)?;
        self.last_position.visit("LastPosition", visitor)?;
        self.acceleration.visit("Acceleration", visitor)?;
        self.contacts.visit("Contacts", visitor)?;
        self.friction.visit("Friction", visitor)?;
        self.gravity.visit("Gravity", visitor)?;
        self.speed_limit.visit("SpeedLimit", visitor)?;

        visitor.leave_region()
    }
}

impl RigidBody {
    pub fn new(shape: ConvexShape) -> RigidBody {
        RigidBody {
            position: Vec3::zero(),
            last_position: Vec3::zero(),
            acceleration: Vec3::zero(),
            friction: 0.2,
            gravity: Vec3::make(0.0, -9.81, 0.0),
            shape,
            contacts: Vec::new(),
            speed_limit: 0.75,
        }
    }

    #[inline]
    pub fn make_copy(&self) -> RigidBody {
        RigidBody {
            position: self.position,
            last_position: self.last_position,
            acceleration: self.acceleration,
            contacts: Vec::new(),
            friction: self.friction,
            gravity: self.gravity,
            shape: self.shape.clone(),
            speed_limit: self.speed_limit,
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
    pub fn set_shape(&mut self, shape: ConvexShape) {
        self.shape = shape;
    }

    #[inline]
    pub fn get_shape(&self) -> &ConvexShape {
        &self.shape
    }

    #[inline]
    pub fn get_shape_mut(&mut self) -> &mut ConvexShape {
        &mut self.shape
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
            if !self.contacts.is_empty() {
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

    pub fn solve_triangle_collision(&mut self, triangle: &StaticTriangle, triangle_index: usize) {
        let triangle_shape = ConvexShape::Triangle(TriangleShape {
            vertices: triangle.points
        });

        if let Some(simplex) = gjk_epa::gjk_is_intersects(&self.shape, self.position, &triangle_shape, Vec3::zero()) {
            if let Some(penetration_info) = gjk_epa::epa_get_penetration_info(simplex, &self.shape, self.position, &triangle_shape, Vec3::zero()) {
                self.position -= penetration_info.penetration_vector;

                self.contacts.push(Contact {
                    body: Handle::none(),
                    position: penetration_info.contact_point,
                    normal: (-penetration_info.penetration_vector).normalized().unwrap_or(Vec3::up()),
                    triangle_index: triangle_index as u32,
                })
            }
        }
    }
}