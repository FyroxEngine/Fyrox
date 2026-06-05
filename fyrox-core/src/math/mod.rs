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

// Clippy complains about normal mathematical symbols like A, B, C for quadratic equation.
#![allow(clippy::many_single_char_names)]

pub use fyrox_math::*;

use crate::math::curve::Curve;
use crate::math::curve::CurveKey;
use crate::math::curve::CurveKeyKind;
use crate::{
    algebra::Scalar,
    math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, plane::Plane},
    num_traits::NumAssign,
    reflect::prelude::*,
    visitor::prelude::*,
};
use fyrox_core_derive::{impl_reflect, impl_visit};

impl_reflect!(
    #[reflect(type_uuid = "64ef33b7-d05a-4d8d-b0c7-99bcc919271f")]
    pub struct Rect<T: Reflect + Copy + PartialEq> {}
);

impl<T> Visit for Rect<T>
where
    T: NumAssign + Scalar + Visit + PartialOrd + Copy + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.position.x.visit("X", &mut region)?;
        self.position.y.visit("Y", &mut region)?;
        self.size.x.visit("W", &mut region)?;
        self.size.y.visit("H", &mut region)?;

        Ok(())
    }
}

impl Visit for TriangleDefinition {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.0[0].visit("A", &mut region)?;
        self.0[1].visit("B", &mut region)?;
        self.0[2].visit("C", &mut region)?;

        Ok(())
    }
}

impl Visit for AxisAlignedBoundingBox {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.min.visit("Min", &mut region)?;
        self.max.visit("Max", &mut region)?;

        Ok(())
    }
}

impl Visit for Frustum {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.planes[0].visit("Left", &mut region)?;
        self.planes[1].visit("Right", &mut region)?;
        self.planes[2].visit("Top", &mut region)?;
        self.planes[3].visit("Bottom", &mut region)?;
        self.planes[4].visit("Far", &mut region)?;
        self.planes[5].visit("Near", &mut region)?;

        Ok(())
    }
}

impl Visit for Plane {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.normal.visit("Normal", &mut region)?;
        self.d.visit("D", &mut region)?;

        Ok(())
    }
}

impl_reflect!(
    #[reflect(type_uuid = "f317d328-8a0d-4404-8681-7aae272a0023")]
    pub struct TriangleDefinition(pub [u32; 3]);
);

impl_visit!(
    pub struct SmoothAngle {
        angle: f32,
        target: f32,
        speed: f32,
    }
);

impl_reflect!(
    #[reflect(type_uuid = "b20a5c3f-902b-4943-9de2-defbd9709dc2")]
    pub struct SmoothAngle {
        angle: f32,
        target: f32,
        speed: f32,
    }
);

impl_reflect!(
    #[reflect(type_uuid = "e7e30ddb-9ae9-4bb1-9d68-726b5d99d895")]
    pub enum CurveKeyKind {
        Constant,
        Linear,
        Cubic {
            left_tangent: f32,
            right_tangent: f32,
        },
    }
);

impl_visit!(
    pub enum CurveKeyKind {
        Constant,
        Linear,
        Cubic {
            left_tangent: f32,
            right_tangent: f32,
        },
    }
);

impl_visit!(
    pub struct CurveKey {
        pub id: Uuid,
        location: f32,
        pub value: f32,
        pub kind: CurveKeyKind,
    }
);

impl_reflect!(
    #[reflect(hide_all, type_uuid = "d42bcb0c-92c6-4fc8-8a6f-2fa5604f0d22")]
    pub struct Curve {
        pub id: Uuid,
        pub name: String,
        pub keys: Vec<CurveKey>,
    }
);

impl_visit!(
    pub struct Curve {
        pub id: Uuid,
        pub name: String,
        pub keys: Vec<CurveKey>,
    }
);
