// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
            Self::SMART => return None,
            Self::X => {
                let r = Vector3::new(0.0, look_direction.y, look_direction.z);
                if !r.is_zero() {
                    r
                } else {
                    Vector3::new(0.0, 1.0, 1.0)
                }
            }
            Self::Y => {
                let r = Vector3::new(look_direction.x, 0.0, look_direction.z);
                if !r.is_zero() {
                    r
                } else {
                    Vector3::new(1.0, 0.0, 1.0)
                }
            }
            Self::Z => {
                let r = Vector3::new(look_direction.x, look_direction.y, 0.0);
                if !r.is_zero() {
                    r
                } else {
                    Vector3::new(1.0, 1.0, 0.0)
                }
            }
            Self::YZ => Vector3::x(),
            Self::ZX => Vector3::y(),
            Self::XY => Vector3::z(),
        };
        Plane::from_normal_and_point(&normal, &Default::default())
    }

    pub fn project_point(self, point: Vector3<f32>) -> Vector3<f32> {
        match self {
            Self::SMART | Self::X => Vector3::new(point.x, 0.0, 0.0),
            Self::Y => Vector3::new(0.0, point.y, 0.0),
            Self::Z => Vector3::new(0.0, 0.0, point.z),
            Self::XY => Vector3::new(point.x, point.y, 0.0),
            Self::YZ => Vector3::new(0.0, point.y, point.z),
            Self::ZX => Vector3::new(point.x, 0.0, point.z),
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
