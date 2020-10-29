use rg3d_core::{
    define_is_as,
    math::{
        vec3::Vec3,
        self,
        aabb::AxisAlignedBoundingBox
    },
    visitor::{Visit, VisitResult, Visitor, VisitError},
};

#[derive(Clone, Debug)]
pub struct SphereShape {
    pub radius: f32
}

pub trait CircumRadius {
    fn circumradius(&self) -> f32;
}

impl Visit for SphereShape {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;

        visitor.leave_region()
    }
}

impl CircumRadius for SphereShape {
    fn circumradius(&self) -> f32 {
        self.radius
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
        let norm_dir = direction.normalized().unwrap_or_else(|| Vec3::new(1.0, 0.0, 0.0));
        norm_dir.scale(self.radius)
    }
}

#[derive(Clone, Debug)]
pub struct TriangleShape {
    pub vertices: [Vec3; 3]
}

impl CircumRadius for TriangleShape {
    fn circumradius(&self) -> f32 {
        AxisAlignedBoundingBox::from_points(&self.vertices).half_extents().max_value()
    }
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
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 1.0, 0.0)]
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

#[derive(Clone, Debug)]
pub struct BoxShape {
    half_extents: Vec3,
}

impl CircumRadius for BoxShape {
    fn circumradius(&self) -> f32 {
        self.half_extents.max_value()
    }
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
            half_extents: Vec3::new(0.5, 0.5, 0.5)
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

#[derive(Clone, Debug)]
pub struct PointCloudShape {
    points: Vec<Vec3>
}

impl CircumRadius for PointCloudShape {
    fn circumradius(&self) -> f32 {
        // TODO: Unoptimal, value should be cached.
        AxisAlignedBoundingBox::from_points(&self.points).half_extents().max_value()
    }
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

#[derive(Copy, Clone, Debug)]
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
                0 => Self::X,
                1 => Self::Y,
                2 => Self::Z,
                _ => return Err(VisitError::User("Invalid axis".to_owned()))
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CapsuleShape {
    axis: Axis,
    radius: f32,
    height: f32,
}

impl CircumRadius for CapsuleShape {
    fn circumradius(&self) -> f32 {
        self.radius.max(self.height)
    }
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
                (Vec3::new(half_height, 0.0, 0.0),
                 Vec3::new(-half_height, 0.0, 0.0))
            }
            Axis::Y => {
                (Vec3::new(0.0, half_height, 0.0),
                 Vec3::new(0.0, -half_height, 0.0))
            }
            Axis::Z => {
                (Vec3::new(0.0, 0.0, half_height),
                 Vec3::new(0.0, 0.0, -half_height))
            }
        }
    }

    pub fn get_farthest_point(&self, direction: Vec3) -> Vec3 {
        let norm_dir = direction.normalized().unwrap_or_else(|| Vec3::new(1.0, 0.0, 0.0));
        let half_height = self.height * 0.5;

        let positive_cap_position = match self.axis {
            Axis::X => Vec3::new(half_height, 0.0, 0.0),
            Axis::Y => Vec3::new(0.0, half_height, 0.0),
            Axis::Z => Vec3::new(0.0, 0.0, half_height),
        };

        let mut max = -std::f32::MAX;
        let mut farthest = Vec3::ZERO;
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

#[derive(Clone, Debug)]
pub enum ConvexShape {
    Dummy,
    Box(BoxShape),
    Sphere(SphereShape),
    Capsule(CapsuleShape),
    Triangle(TriangleShape),
    PointCloud(PointCloudShape),
}

impl CircumRadius for ConvexShape {
    fn circumradius(&self) -> f32 {
        match self {
            Self::Dummy => 0.0,
            Self::Box(box_shape) => box_shape.circumradius(),
            Self::Sphere(sphere) => sphere.circumradius(),
            Self::Capsule(capsule) => capsule.circumradius(),
            Self::Triangle(triangle) => triangle.circumradius(),
            Self::PointCloud(point_cloud) => point_cloud.circumradius(),
        }
    }
}

impl ConvexShape {
    pub fn get_farthest_point(&self, position: Vec3, direction: Vec3) -> Vec3 {
        position + match self {
            Self::Dummy => Vec3::ZERO,
            Self::Box(box_shape) => box_shape.get_farthest_point(direction),
            Self::Sphere(sphere) => sphere.get_farthest_point(direction),
            Self::Capsule(capsule) => capsule.get_farthest_point(direction),
            Self::Triangle(triangle) => triangle.get_farthest_point(direction),
            Self::PointCloud(point_cloud) => point_cloud.get_farthest_point(direction),
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            Self::Dummy => 0,
            Self::Box(_) => 1,
            Self::Sphere(_) => 2,
            Self::Capsule(_) => 3,
            Self::Triangle(_) => 4,
            Self::PointCloud(_) => 5,
        }
    }

    pub fn new(id: i32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Dummy),
            1 => Ok(Self::Box(Default::default())),
            2 => Ok(Self::Sphere(Default::default())),
            3 => Ok(Self::Capsule(Default::default())),
            4 => Ok(Self::Triangle(Default::default())),
            5 => Ok(Self::PointCloud(Default::default())),
            _ => Err("Invalid shape id!".to_owned())
        }
    }

    define_is_as!(ConvexShape : Box -> ref BoxShape => fn is_box, fn as_box, fn as_box_mut);
    define_is_as!(ConvexShape : Capsule -> ref CapsuleShape => fn is_capsule, fn as_capsule, fn as_capsule_mut);
    define_is_as!(ConvexShape : Sphere -> ref SphereShape => fn is_sphere, fn as_sphere, fn as_sphere_mut);
    define_is_as!(ConvexShape : Triangle -> ref TriangleShape => fn is_triangle, fn as_triangle, fn as_triangle_mut);
    define_is_as!(ConvexShape : PointCloud -> ref PointCloudShape => fn is_point_cloud, fn as_point_cloud, fn as_point_cloud_mut);
}

impl Visit for ConvexShape {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        match self {
            Self::Dummy => Ok(()),
            Self::Box(box_shape) => box_shape.visit(name, visitor),
            Self::Sphere(sphere) => sphere.visit(name, visitor),
            Self::Capsule(capsule) => capsule.visit(name, visitor),
            Self::Triangle(triangle) => triangle.visit(name, visitor),
            Self::PointCloud(point_cloud) => point_cloud.visit(name, visitor),
        }
    }
}


