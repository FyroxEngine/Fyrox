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
    pub fn make_plane_from_view(self, _look_direction: Vector3<f32>) -> Option<Plane> {
        let normal = match self {
            PlaneKind::SMART => return None,
            PlaneKind::X => Vector3::new(0.0, 1.0,1.0),
            PlaneKind::Y => Vector3::new(1.0, 0.0,1.0),
            PlaneKind::Z => Vector3::new(1.0, 1.0, 0.0),
            PlaneKind::YZ => Vector3::x(),
            PlaneKind::ZX => Vector3::y(),
            PlaneKind::XY => Vector3::z(),
        };
        Plane::from_normal_and_point(&normal, &Default::default())
    }

    pub fn project_point(self, point: Vector3<f32>) -> Vector3<f32> {
        match self {
            PlaneKind::SMART | PlaneKind::X => Vector3::new(point.x, 0.0, 0.0),
            PlaneKind::Y => Vector3::new(0.0, point.y, 0.0),
            PlaneKind::Z => Vector3::new(0.0, 0.0, point.z),
            PlaneKind::XY => Vector3::new(point.x, point.y, 0.0),
            PlaneKind::YZ => Vector3::new(0.0, point.y, point.z),
            PlaneKind::ZX => Vector3::new(point.x, 0.0, point.z),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_plane_in_111() {
        let dir111 = Vector3::new(1.0, 1.0, 1.0);
        assert!(PlaneKind::X.make_plane_from_view(dir111).is_some());
    }
    #[test]
    fn test_look_dir_is_move_dir() {
        let dir_x = Vector3::new(1.0, 0.0, 0.0);
        assert!(PlaneKind::X.make_plane_from_view(dir_x).is_some());
    }
}
