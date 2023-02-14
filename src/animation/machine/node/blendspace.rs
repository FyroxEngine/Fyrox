#![allow(missing_docs)] // TODO

use crate::{
    animation::{
        machine::{node::BasePoseNode, EvaluatePose, Parameter, ParameterContainer, PoseNode},
        AnimationContainer, AnimationPose,
    },
    core::{
        algebra::Vector2,
        math::{self, TriangleDefinition},
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
    },
};
use spade::{DelaunayTriangulation, InsertionError, Point2, Triangulation};
use std::{
    cell::{Ref, RefCell},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Visit, Clone, Reflect, PartialEq, Default)]
pub struct BlendSpacePoint {
    position: Vector2<f32>,
    pose_source: Handle<PoseNode>,
}

#[derive(Debug, Visit, Clone, Reflect, PartialEq, Default)]
pub struct BlendSpace {
    base: BasePoseNode,

    points: Vec<BlendSpacePoint>,
    triangles: Vec<TriangleDefinition>,

    min_values: Vector2<f32>,
    max_values: Vector2<f32>,
    snap_step: Vector2<f32>,
    parameter: String,

    #[reflect(hidden)]
    #[visit(skip)]
    pose: RefCell<AnimationPose>,
}

impl Deref for BlendSpace {
    type Target = BasePoseNode;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BlendSpace {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl EvaluatePose for BlendSpace {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        _animations: &AnimationContainer,
        _dt: f32,
    ) -> Ref<AnimationPose> {
        let mut pose = self.pose.borrow_mut();

        pose.reset();

        if let Some(Parameter::SamplingPoint(sampling_point)) = params.get(&self.parameter) {
            if let Some(weights) = self.fetch_weights(*sampling_point) {
                let (ia, wa) = weights[0];
                let (ib, wb) = weights[1];
                let (ic, wc) = weights[2];

                if let (Some(pose_a), Some(pose_b), Some(pose_c)) = (
                    nodes.try_borrow(self.points[ia].pose_source),
                    nodes.try_borrow(self.points[ib].pose_source),
                    nodes.try_borrow(self.points[ic].pose_source),
                ) {
                    pose.blend_with(&pose_a.pose(), wa);
                    pose.blend_with(&pose_b.pose(), wb);
                    pose.blend_with(&pose_c.pose(), wc);
                }
            }
        }

        self.pose.borrow()
    }

    fn pose(&self) -> Ref<AnimationPose> {
        self.pose.borrow()
    }
}

impl BlendSpace {
    pub fn set_points(&mut self, points: Vec<BlendSpacePoint>) -> bool {
        self.points = points;
        self.triangulate().is_ok()
    }

    pub fn points(&self) -> &[BlendSpacePoint] {
        &self.points
    }

    pub fn children(&self) -> Vec<Handle<PoseNode>> {
        self.points.iter().map(|p| p.pose_source).collect()
    }

    fn fetch_weights(&self, point: Vector2<f32>) -> Option<[(usize, f32); 3]> {
        for triangle in self.triangles.iter() {
            let ia = triangle[0] as usize;
            let ib = triangle[1] as usize;
            let ic = triangle[2] as usize;

            let a = &self.points[ia];
            let b = &self.points[ib];
            let c = &self.points[ic];

            let barycentric_coordinates =
                math::get_barycentric_coords_2d(point, a.position, b.position, c.position);

            if math::barycentric_is_inside(barycentric_coordinates) {
                let (u, v, w) = barycentric_coordinates;

                return Some([(ia, u), (ib, v), (ic, w)]);
            }
        }

        // TODO: If none of the triangles contains sampling point, then try to find closes edge and
        // calculate weights.

        None
    }

    fn triangulate(&mut self) -> Result<(), InsertionError> {
        self.triangles.clear();

        let mut triangulation: DelaunayTriangulation<_> = DelaunayTriangulation::new();

        for point in self.points.iter() {
            triangulation.insert(Point2::new(point.position.x, point.position.y))?;
        }

        for face in triangulation.inner_faces() {
            let edges = face.adjacent_edges();
            self.triangles.push(TriangleDefinition([
                edges[0].from().index() as u32,
                edges[1].from().index() as u32,
                edges[2].from().index() as u32,
            ]))
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        animation::machine::node::blendspace::{BlendSpace, BlendSpacePoint},
        core::{algebra::Vector2, math::TriangleDefinition},
    };

    #[test]
    fn test_blend_space_triangulation() {
        let mut blend_space = BlendSpace::default();

        let result = blend_space.set_points(vec![
            BlendSpacePoint {
                position: Vector2::new(0.0, 0.0),
                pose_source: Default::default(),
            },
            BlendSpacePoint {
                position: Vector2::new(1.0, 0.0),
                pose_source: Default::default(),
            },
            BlendSpacePoint {
                position: Vector2::new(1.0, 1.0),
                pose_source: Default::default(),
            },
            BlendSpacePoint {
                position: Vector2::new(0.0, 1.0),
                pose_source: Default::default(),
            },
        ]);

        assert!(result);

        assert_eq!(
            blend_space.triangles,
            vec![TriangleDefinition([2, 0, 1]), TriangleDefinition([3, 0, 2])]
        )
    }
}
