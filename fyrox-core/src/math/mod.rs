// Clippy complains about normal mathematical symbols like A, B, C for quadratic equation.
#![allow(clippy::many_single_char_names)]

pub use fyrox_math::*;

use crate::math::curve::Curve;
use crate::math::curve::CurveKey;
use crate::math::curve::CurveKeyKind;
use crate::Uuid;
use crate::{
    algebra::Scalar,
    math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, plane::Plane},
    num_traits::NumAssign,
    reflect::prelude::*,
    visitor::prelude::*,
};
use fyrox_core_derive::{impl_reflect, impl_visit};
use std::fmt::Debug;

impl_reflect!(
    pub struct Rect<T: Debug> {}
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
    pub struct SmoothAngle {
        angle: f32,
        target: f32,
        speed: f32,
    }
);

impl_reflect!(
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
    #[reflect(hide_all)]
    pub struct Curve {
        pub id: Uuid,
        pub name: String,
        pub keys: Vec<CurveKey>,
    }
);

impl_visit!(
    pub struct Curve {
        #[visit(optional)] // Backward compatibility
        pub id: Uuid,
        #[visit(optional)] // Backward compatibility
        pub name: String,
        pub keys: Vec<CurveKey>,
    }
);
