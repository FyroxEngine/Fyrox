use rg3d_core::visitor::{Visit, VisitResult, Visitor, VisitError};
use rg3d_core::math::vec3::Vec3;
use rg3d_core::math::plane::Plane;

pub struct StaticGeometry {
    pub(in crate) triangles: Vec<StaticTriangle>
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

impl Default for StaticGeometry {
    fn default() -> Self {
        Self {
            triangles: Vec::new(),
        }
    }
}

impl Visit for StaticGeometry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.triangles.visit("Triangles", visitor)?;

        visitor.leave_region()
    }
}

pub struct StaticTriangle {
    pub points: [Vec3; 3],
    pub ca: Vec3,
    pub ba: Vec3,
    pub ca_dot_ca: f32,
    pub ca_dot_ba: f32,
    pub ba_dot_ba: f32,
    pub inv_denom: f32,
    pub plane: Plane,
}

impl Default for StaticTriangle {
    fn default() -> Self {
        Self {
            points: Default::default(),
            ca: Default::default(),
            ba: Default::default(),
            ca_dot_ca: 0.0,
            ca_dot_ba: 0.0,
            ba_dot_ba: 0.0,
            inv_denom: 0.0,
            plane: Default::default(),
        }
    }
}

impl Visit for StaticTriangle {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut a = self.points[0];
        a.visit("A", visitor)?;

        let mut b = self.points[1];
        b.visit("B", visitor)?;

        let mut c = self.points[2];
        c.visit("C", visitor)?;

        *self = match Self::from_points(&a, &b, &c) {
            None => return Err(VisitError::User(String::from("invalid triangle"))),
            Some(triangle) => triangle,
        };

        visitor.leave_region()
    }
}

impl StaticTriangle {
    ///
    /// Creates static triangle from tree points and precomputes some data
    /// to speedup collision detection in runtime. This function may fail
    /// if degenerated triangle was passed into.
    ///
    pub fn from_points(a: &Vec3, b: &Vec3, c: &Vec3) -> Option<StaticTriangle> {
        let ca = *c - *a;
        let ba = *b - *a;
        let ca_dot_ca = ca.dot(&ca);
        let ca_dot_ba = ca.dot(&ba);
        let ba_dot_ba = ba.dot(&ba);
        if let Some(plane) = Plane::from_normal_and_point(&ba.cross(&ca), a) {
            return Some(StaticTriangle {
                points: [*a, *b, *c],
                ba,
                ca: *c - *a,
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