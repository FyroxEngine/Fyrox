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
use spade::{DelaunayTriangulation, Point2, Triangulation};
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
    /// Sets new points to the blend space and tries to triangulate them. Returns `true` if the triangulation
    /// was successful, `false` - otherwise. Keep in mind, that failed triangulation does not indicate an error -
    /// a blend space could contain any number of points, so for zero, one or two points triangulation is not
    /// defined. In other words, blend space will function ok even with failed triangulation.
    pub fn set_points(&mut self, points: Vec<BlendSpacePoint>) -> bool {
        self.points = points;
        self.triangulate()
    }

    pub fn points(&self) -> &[BlendSpacePoint] {
        &self.points
    }

    pub fn children(&self) -> Vec<Handle<PoseNode>> {
        self.points.iter().map(|p| p.pose_source).collect()
    }

    fn fetch_weights(&self, sampling_point: Vector2<f32>) -> Option<[(usize, f32); 3]> {
        if self.points.is_empty() {
            return None;
        }

        // Single point blend space.
        if self.points.len() == 1 {
            return Some([(0, 1.0), (0, 0.0), (0, 0.0)]);
        }

        // Check if there's an edge that contains a projection of the sampling point.
        if self.points.len() == 2 {
            let edge = self.points[1].position - self.points[0].position;
            let to_point = sampling_point - self.points[0].position;
            let t = to_point.dot(&edge) / edge.dot(&edge);
            if t >= 0.0 && t <= 1.0 {
                return Some([(0, (1.0 - t)), (1, t), (0, 0.0)]);
            }
        }

        // Try to find a triangle that contains the sampling point.
        for triangle in self.triangles.iter() {
            let ia = triangle[0] as usize;
            let ib = triangle[1] as usize;
            let ic = triangle[2] as usize;

            let a = &self.points[ia];
            let b = &self.points[ib];
            let c = &self.points[ic];

            let barycentric_coordinates =
                math::get_barycentric_coords_2d(sampling_point, a.position, b.position, c.position);

            if math::barycentric_is_inside(barycentric_coordinates) {
                let (u, v, w) = barycentric_coordinates;

                return Some([(ia, u), (ib, v), (ic, w)]);
            }
        }

        // If none of the triangles contains the sampling point, then try to find a closest edge of a
        // triangle and calculate weights.
        let mut min_distance = f32::MAX;
        let mut weights = None;

        for triangle in self.triangles.iter() {
            for (a, b) in [
                (triangle[0] as usize, triangle[1] as usize),
                (triangle[1] as usize, triangle[2] as usize),
                (triangle[2] as usize, triangle[0] as usize),
            ] {
                let pt_a = self.points[a].position;
                let pt_b = self.points[b].position;

                let edge = pt_b - pt_a;
                let to_point = sampling_point - pt_a;

                let t = to_point.dot(&edge) / edge.dot(&edge);

                if t >= 0.0 && t <= 1.0 {
                    let projection = pt_a + edge.scale(t);

                    let distance = sampling_point.metric_distance(&projection);

                    if distance < min_distance {
                        min_distance = distance;

                        weights = Some([(a, (1.0 - t)), (b, t), (b, 0.0)]);
                    }
                }
            }
        }

        weights
    }

    fn triangulate(&mut self) -> bool {
        self.triangles.clear();

        if self.points.len() < 3 {
            return false;
        }

        let mut triangulation: DelaunayTriangulation<_> = DelaunayTriangulation::new();

        for point in self.points.iter() {
            if triangulation
                .insert(Point2::new(point.position.x, point.position.y))
                .is_err()
            {
                return false;
            }
        }

        for face in triangulation.inner_faces() {
            let edges = face.adjacent_edges();
            self.triangles.push(TriangleDefinition([
                edges[0].from().index() as u32,
                edges[1].from().index() as u32,
                edges[2].from().index() as u32,
            ]))
        }

        true
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

        // Triangulation must exist.
        assert!(result);

        assert_eq!(
            blend_space.triangles,
            vec![TriangleDefinition([2, 0, 1]), TriangleDefinition([3, 0, 2])]
        )
    }

    #[test]
    fn test_empty_blend_space_sampling() {
        assert!(BlendSpace::default()
            .fetch_weights(Default::default())
            .is_none())
    }

    #[test]
    fn test_single_point_blend_space_sampling() {
        let mut blend_space = BlendSpace::default();

        let triangulated = blend_space.set_points(vec![BlendSpacePoint {
            position: Vector2::new(0.0, 0.0),
            pose_source: Default::default(),
        }]);

        assert!(!triangulated);

        assert_eq!(
            blend_space.fetch_weights(Default::default()),
            Some([(0, 1.0), (0, 0.0), (0, 0.0)])
        );
    }

    #[test]
    fn test_two_points_blend_space_sampling() {
        let mut blend_space = BlendSpace::default();

        let triangulated = blend_space.set_points(vec![
            BlendSpacePoint {
                position: Vector2::new(0.0, 0.0),
                pose_source: Default::default(),
            },
            BlendSpacePoint {
                position: Vector2::new(1.0, 0.0),
                pose_source: Default::default(),
            },
        ]);

        assert!(!triangulated);

        assert_eq!(
            blend_space.fetch_weights(Vector2::new(0.0, 0.0)),
            Some([(0, 1.0), (1, 0.0), (0, 0.0)])
        );

        assert_eq!(
            blend_space.fetch_weights(Vector2::new(0.5, 0.0)),
            Some([(0, 0.5), (1, 0.5), (0, 0.0)])
        );

        assert_eq!(
            blend_space.fetch_weights(Vector2::new(1.0, 0.0)),
            Some([(0, 0.0), (1, 1.0), (0, 0.0)])
        );
    }
}
