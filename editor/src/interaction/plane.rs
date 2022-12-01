use fyrox::core::{algebra::Vector3, math::plane::Plane};

#[derive(Copy, Clone, Debug)]
pub enum PlaneKind {
    X,
    Y,
    Z,
    XY,
    YZ,
    ZX,
    SMART,
}

impl PlaneKind {
    
    pub fn make_plane_from_view(self, look_direction: Vector3<f32>) -> Plane {
        // FIXME: I wonder if look_direction is really needed here
        match self {
            PlaneKind::SMART|PlaneKind::X => Plane::from_normal_and_point(
                &Vector3::new(0.0, look_direction.y, look_direction.z),
                &Default::default(),
            ),
            PlaneKind::Y => Plane::from_normal_and_point(
                &Vector3::new(look_direction.x, 0.0, look_direction.z),
                &Default::default(),
            ),
            PlaneKind::Z => Plane::from_normal_and_point(
                &Vector3::new(look_direction.x, look_direction.y, 0.0),
                &Default::default(),
            ),
            PlaneKind::YZ => Plane::from_normal_and_point(&Vector3::x(), &Default::default()),
            PlaneKind::ZX => Plane::from_normal_and_point(&Vector3::y(), &Default::default()),
            PlaneKind::XY => Plane::from_normal_and_point(&Vector3::z(), &Default::default()),
        }
        .unwrap_or_default()
    }

    pub fn project_point(self, point: Vector3<f32>) -> Vector3<f32> {
        match self {
            PlaneKind::SMART|PlaneKind::X => Vector3::new(point.x, 0.0, 0.0),
            PlaneKind::Y => Vector3::new(0.0, point.y, 0.0),
            PlaneKind::Z => Vector3::new(0.0, 0.0, point.z),
            PlaneKind::XY => Vector3::new(point.x, point.y, 0.0),
            PlaneKind::YZ => Vector3::new(0.0, point.y, point.z),
            PlaneKind::ZX => Vector3::new(point.x, 0.0, point.z),
        }
    }
}
