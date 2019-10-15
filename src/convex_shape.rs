use rg3d_core::{
    math::vec3::Vec3, math,
    visitor::{Visit, VisitResult, Visitor, VisitError}
};

#[derive(Clone)]
pub struct SphereShape {
    pub radius: f32
}

impl Visit for SphereShape {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;

        visitor.leave_region()
    }
}

impl Default for SphereShape {
    fn default() -> Self {
        Self {
            radius: 0.5,
        }
    }
}

impl SphereShape {
    pub fn new(radius: f32) -> Self {
        Self {
            radius
        }
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    pub fn get_radius(&self) -> f32 {
        self.radius
    }

    pub fn get_farthest_point(&self, direction: Vec3) -> Vec3 {
        let norm_dir = direction.normalized().unwrap_or(Vec3::make(1.0, 0.0, 0.0));
        norm_dir.scale(self.radius)
    }
}

#[derive(Clone)]
pub struct TriangleShape {
    pub vertices: [Vec3; 3]
}

impl Visit for TriangleShape {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.vertices[0].visit("A", visitor)?;
        self.vertices[1].visit("B", visitor)?;
        self.vertices[2].visit("C", visitor)?;

        visitor.leave_region()
    }
}

impl Default for TriangleShape {
    fn default() -> Self {
        Self {
            vertices: [
                Vec3::make(0.0, 0.0, 0.0),
                Vec3::make(1.0, 0.0, 0.0),
                Vec3::make(0.5, 1.0, 0.0)]
        }
    }
}

impl TriangleShape {
    pub fn new(vertices: [Vec3; 3]) -> Self {
        Self {
            vertices
        }
    }

    pub fn get_normal(&self) -> Option<Vec3> {
        (self.vertices[2] - self.vertices[0]).cross(&(self.vertices[1] - self.vertices[0])).normalized()
    }

    pub fn get_farthest_point(&self, direction: Vec3) -> Vec3 {
        math::get_farthest_point(&self.vertices, direction)
    }
}

#[derive(Clone)]
pub struct BoxShape {
    half_extents: Vec3,
}

impl Visit for BoxShape {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.half_extents.visit("HalfExtents", visitor)?;

        visitor.leave_region()
    }
}

impl Default for BoxShape {
    fn default() -> Self {
        Self {
            half_extents: Vec3::make(0.5, 0.5, 0.5)
        }
    }
}

impl BoxShape {
    pub fn new(half_extents: Vec3) -> Self {
        Self {
            half_extents
        }
    }

    pub fn get_farthest_point(&self, direction: Vec3) -> Vec3 {
        Vec3 {
            x: if direction.x >= 0.0 { self.half_extents.x } else { -self.half_extents.x },
            y: if direction.y >= 0.0 { self.half_extents.y } else { -self.half_extents.y },
            z: if direction.z >= 0.0 { self.half_extents.z } else { -self.half_extents.z },
        }
    }

    pub fn get_min(&self) -> Vec3 {
        Vec3 {
            x: -self.half_extents.x,
            y: -self.half_extents.y,
            z: -self.half_extents.z,
        }
    }

    pub fn get_max(&self) -> Vec3 {
        Vec3 {
            x: self.half_extents.x,
            y: self.half_extents.y,
            z: self.half_extents.z,
        }
    }
}

#[derive(Clone)]
pub struct PointCloudShape {
    points: Vec<Vec3>
}

impl Visit for PointCloudShape {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.points.visit("Points", visitor)?;

        visitor.leave_region()
    }
}

impl Default for PointCloudShape {
    fn default() -> Self {
        Self {
            points: Vec::new(),
        }
    }
}

impl PointCloudShape {
    pub fn new(points: Vec<Vec3>) -> Self {
        Self {
            points
        }
    }

    pub fn get_farthest_point(&self, direction: Vec3) -> Vec3 {
        math::get_farthest_point(&self.points, direction)
    }
}

#[derive(Copy, Clone)]
pub enum Axis {
    X = 0,
    Y = 1,
    Z = 2,
}

impl Visit for Axis {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut id = *self as u8;
        id.visit(name, visitor)?;
        if visitor.is_reading() {
            *self = match id {
                0 => Axis::X,
                1 => Axis::Y,
                2 => Axis::Z,
                _ => return Err(VisitError::User("Invalid axis".to_owned()))
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct CapsuleShape {
    axis: Axis,
    radius: f32,
    height: f32,
}

impl Default for CapsuleShape {
    fn default() -> Self {
        Self {
            axis: Axis::X,
            radius: 0.0,
            height: 0.0,
        }
    }
}

impl Visit for CapsuleShape {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.axis.visit("Axis", visitor)?;
        self.height.visit("Height", visitor)?;

        visitor.leave_region()
    }
}

impl CapsuleShape {
    pub fn new(radius: f32, height: f32, axis: Axis) -> Self {
        Self {
            axis,
            radius,
            height
        }
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.abs()
    }

    pub fn get_radius(&self) -> f32 {
        self.radius
    }

    pub fn set_height(&mut self, height: f32) {
        self.height = height.abs()
    }

    pub fn get_height(&self) -> f32 {
        self.height
    }

    pub fn set_axis(&mut self, axis: Axis) {
        self.axis = axis;
    }

    pub fn get_axis(&self) -> Axis {
        self.axis
    }

    pub fn get_cap_centers(&self) -> (Vec3, Vec3) {
        let half_height = self.height * 0.5;

        match self.axis {
            Axis::X => {
                (Vec3::make(half_height, 0.0, 0.0),
                 Vec3::make(-half_height, 0.0, 0.0))
            }
            Axis::Y => {
                (Vec3::make(0.0, half_height, 0.0),
                 Vec3::make(0.0, -half_height, 0.0))
            }
            Axis::Z => {
                (Vec3::make(0.0, 0.0, half_height),
                 Vec3::make(0.0, 0.0, -half_height))
            }
        }
    }

    pub fn get_farthest_point(&self, direction: Vec3) -> Vec3 {
        let norm_dir = direction.normalized().unwrap_or(Vec3::make(1.0, 0.0, 0.0));
        let half_height = self.height * 0.5;

        let positive_cap_position = match self.axis {
            Axis::X => Vec3::make(half_height, 0.0, 0.0),
            Axis::Y => Vec3::make(0.0, half_height, 0.0),
            Axis::Z => Vec3::make(0.0, 0.0, half_height),
        };

        let mut max = -std::f32::MAX;
        let mut farthest = Vec3::zero();
        for cap_center in [positive_cap_position, -positive_cap_position].iter() {
            let vertex = *cap_center + norm_dir.scale(self.radius);
            let dot = norm_dir.dot(&vertex);
            if dot > max {
                max = dot;
                farthest = vertex;
            }
        }

        farthest
    }
}

#[derive(Clone)]
pub enum ConvexShape {
    Dummy,
    Box(BoxShape),
    Sphere(SphereShape),
    Capsule(CapsuleShape),
    Triangle(TriangleShape),
    PointCloud(PointCloudShape),
}

macro_rules! define_is_as {
    ($is:ident, $as_ref:ident, $as_mut:ident, $kind:ident, $result:ty) => {
        #[inline]
        pub fn $is(&self) -> bool {
            match self {
                ConvexShape::$kind(_) => true,
                _ => false
            }
        }

        #[inline]
        pub fn $as_ref(&self) -> &$result {
            match self {
                ConvexShape::$kind(ref val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }

        #[inline]
        pub fn $as_mut(&mut self) -> &mut $result {
            match self {
                ConvexShape::$kind(ref mut val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }
    }
}

impl ConvexShape {
    pub fn get_farthest_point(&self, position: Vec3, direction: Vec3) -> Vec3 {
        position + match self {
            ConvexShape::Dummy => Vec3::zero(),
            ConvexShape::Box(box_shape) => box_shape.get_farthest_point(direction),
            ConvexShape::Sphere(sphere) => sphere.get_farthest_point(direction),
            ConvexShape::Capsule(capsule) => capsule.get_farthest_point(direction),
            ConvexShape::Triangle(triangle) => triangle.get_farthest_point(direction),
            ConvexShape::PointCloud(point_cloud) => point_cloud.get_farthest_point(direction),
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            ConvexShape::Dummy => 0,
            ConvexShape::Box(_) => 1,
            ConvexShape::Sphere(_) => 2,
            ConvexShape::Capsule(_) => 3,
            ConvexShape::Triangle(_) => 4,
            ConvexShape::PointCloud(_) => 5,
        }
    }

    pub fn new(id: i32) -> Result<Self, String> {
        match id {
            0 => Ok(ConvexShape::Dummy),
            1 => Ok(ConvexShape::Box(Default::default())),
            2 => Ok(ConvexShape::Sphere(Default::default())),
            3 => Ok(ConvexShape::Capsule(Default::default())),
            4 => Ok(ConvexShape::Triangle(Default::default())),
            5 => Ok(ConvexShape::PointCloud(Default::default())),
            _ => Err("Invalid shape id!".to_owned())
        }
    }

    define_is_as!(is_box, as_box, as_box_mut, Box, BoxShape);
    define_is_as!(is_capsule, as_capsule, as_capsule_mut, Capsule, CapsuleShape);
    define_is_as!(is_sphere, as_sphere, as_sphere_mut, Sphere, SphereShape);
    define_is_as!(is_triangle, as_triangle, as_triangle_mut, Triangle, TriangleShape);
    define_is_as!(is_point_cloud, as_point_cloud, as_point_cloud_mut, PointCloud, PointCloudShape);
}

impl Visit for ConvexShape {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        match self {
            ConvexShape::Dummy => Ok(()),
            ConvexShape::Box(box_shape) => box_shape.visit(name, visitor),
            ConvexShape::Sphere(sphere) => sphere.visit(name, visitor),
            ConvexShape::Capsule(capsule) => capsule.visit(name, visitor),
            ConvexShape::Triangle(triangle) => triangle.visit(name, visitor),
            ConvexShape::PointCloud(point_cloud) => point_cloud.visit(name, visitor),
        }
    }
}


