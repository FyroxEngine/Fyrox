use crate::fyrox::core::{algebra::Vector3, math::plane::Plane, num_traits::Zero};
use strum_macros::EnumIter;

#[derive(Copy, Clone, Debug, EnumIter)]
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
    pub fn make_plane_from_view(self, look_direction: Vector3<f32>) -> Option<Plane> {
        let normal = match self {
            PlaneKind::SMART => return None,
            PlaneKind::X => {
                let r = Vector3::new(0.0, look_direction.y, look_direction.z);
                if !r.is_zero() {
                    r
                } else {
                    Vector3::new(0.0, 1.0, 1.0)
                }
            }
            PlaneKind::Y => {
                let r = Vector3::new(look_direction.x, 0.0, look_direction.z);
                if !r.is_zero() {
                    r
                } else {
                    Vector3::new(1.0, 0.0, 1.0)
                }
            }
            PlaneKind::Z => {
                let r = Vector3::new(look_direction.x, look_direction.y, 0.0);
                if !r.is_zero() {
                    r
                } else {
                    Vector3::new(1.0, 1.0, 0.0)
                }
            }
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
    use strum::IntoEnumIterator;

    #[test]
    fn test_look_dir_is_move_dir() {
        let dirs = vec![
            Vector3::new(1.0, 1.0, 1.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];
        for dir_x in dirs {
            for kind in PlaneKind::iter() {
                match kind {
                    PlaneKind::SMART => {}
                    _ => {
                        let plane = kind.make_plane_from_view(dir_x);
                        assert!(plane.is_some());
                    }
                }
            }
        }
    }
}
