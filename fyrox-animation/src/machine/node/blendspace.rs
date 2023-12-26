#![allow(missing_docs)] // TODO

use crate::{
    core::{
        algebra::Vector2,
        math::{self, TriangleDefinition},
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
    },
    machine::{
        node::AnimationEventCollectionStrategy, node::BasePoseNode, AnimationPoseSource, Parameter,
        ParameterContainer, PoseNode,
    },
    Animation, AnimationContainer, AnimationEvent, AnimationPose, EntityId,
};
use fyrox_core::uuid::{uuid, Uuid};
use fyrox_core::TypeUuidProvider;
use spade::{DelaunayTriangulation, Point2, Triangulation};
use std::cmp::Ordering;
use std::{
    cell::{Ref, RefCell},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Visit, Clone, Reflect, PartialEq, Default)]
pub struct BlendSpacePoint<T: EntityId> {
    pub position: Vector2<f32>,
    pub pose_source: Handle<PoseNode<T>>,
}

impl<T: EntityId> TypeUuidProvider for BlendSpacePoint<T> {
    fn type_uuid() -> Uuid {
        uuid!("d163b4b9-aed6-447f-bb93-b7e539099417")
    }
}

#[derive(Debug, Visit, Clone, Reflect, PartialEq)]
pub struct BlendSpace<T: EntityId> {
    base: BasePoseNode<T>,

    #[reflect(hidden)]
    points: Vec<BlendSpacePoint<T>>,

    #[reflect(hidden)]
    triangles: Vec<TriangleDefinition>,

    #[reflect(setter = "set_x_axis_name")]
    x_axis_name: String,

    #[reflect(setter = "set_y_axis_name")]
    y_axis_name: String,

    #[reflect(setter = "set_min_values")]
    min_values: Vector2<f32>,

    #[reflect(setter = "set_max_values")]
    max_values: Vector2<f32>,

    #[reflect(setter = "set_snap_step")]
    snap_step: Vector2<f32>,

    #[reflect(setter = "set_sampling_parameter")]
    sampling_parameter: String,

    #[reflect(hidden)]
    #[visit(skip)]
    pose: RefCell<AnimationPose<T>>,
}

impl<T: EntityId> Default for BlendSpace<T> {
    fn default() -> Self {
        Self {
            base: Default::default(),
            points: vec![],
            triangles: Default::default(),
            x_axis_name: "X".to_string(),
            y_axis_name: "Y".to_string(),
            min_values: Default::default(),
            max_values: Vector2::new(1.0, 1.0),
            snap_step: Vector2::new(0.1, 0.1),
            sampling_parameter: Default::default(),
            pose: Default::default(),
        }
    }
}

impl<T: EntityId> Deref for BlendSpace<T> {
    type Target = BasePoseNode<T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T: EntityId> DerefMut for BlendSpace<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<T: EntityId> AnimationPoseSource<T> for BlendSpace<T> {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        dt: f32,
    ) -> Ref<AnimationPose<T>> {
        let mut pose = self.pose.borrow_mut();

        pose.reset();

        if let Some(Parameter::SamplingPoint(sampling_point)) = params.get(&self.sampling_parameter)
        {
            if let Some(weights) = self.fetch_weights(*sampling_point) {
                let (ia, wa) = weights[0];
                let (ib, wb) = weights[1];
                let (ic, wc) = weights[2];

                if let (Some(pose_a), Some(pose_b), Some(pose_c)) = (
                    nodes.try_borrow(self.points[ia].pose_source),
                    nodes.try_borrow(self.points[ib].pose_source),
                    nodes.try_borrow(self.points[ic].pose_source),
                ) {
                    pose.blend_with(&pose_a.eval_pose(nodes, params, animations, dt), wa);
                    pose.blend_with(&pose_b.eval_pose(nodes, params, animations, dt), wb);
                    pose.blend_with(&pose_c.eval_pose(nodes, params, animations, dt), wc);
                }
            }
        }

        drop(pose);

        self.pose.borrow()
    }

    fn pose(&self) -> Ref<AnimationPose<T>> {
        self.pose.borrow()
    }

    fn collect_animation_events(
        &self,
        nodes: &Pool<PoseNode<T>>,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        strategy: AnimationEventCollectionStrategy,
    ) -> Vec<(Handle<Animation<T>>, AnimationEvent)> {
        if let Some(Parameter::SamplingPoint(sampling_point)) = params.get(&self.sampling_parameter)
        {
            if let Some(weights) = self.fetch_weights(*sampling_point) {
                let (ia, wa) = weights[0];
                let (ib, wb) = weights[1];
                let (ic, wc) = weights[2];

                if let (Some(pose_a), Some(pose_b), Some(pose_c)) = (
                    nodes.try_borrow(self.points[ia].pose_source),
                    nodes.try_borrow(self.points[ib].pose_source),
                    nodes.try_borrow(self.points[ic].pose_source),
                ) {
                    match strategy {
                        AnimationEventCollectionStrategy::All => {
                            let mut events = Vec::new();
                            for pose in [pose_a, pose_b, pose_c] {
                                events.extend(
                                    pose.collect_animation_events(
                                        nodes, params, animations, strategy,
                                    ),
                                );
                            }
                            return events;
                        }
                        AnimationEventCollectionStrategy::MaxWeight => {
                            if let Some((max_weight_pose, _)) =
                                [(pose_a, wa), (pose_b, wb), (pose_c, wc)].iter().max_by(
                                    |(_, w1), (_, w2)| {
                                        w1.partial_cmp(w2).unwrap_or(Ordering::Equal)
                                    },
                                )
                            {
                                return max_weight_pose
                                    .collect_animation_events(nodes, params, animations, strategy);
                            }
                        }
                        AnimationEventCollectionStrategy::MinWeight => {
                            if let Some((min_weight_pose, _)) =
                                [(pose_a, wa), (pose_b, wb), (pose_c, wc)].iter().min_by(
                                    |(_, w1), (_, w2)| {
                                        w1.partial_cmp(w2).unwrap_or(Ordering::Equal)
                                    },
                                )
                            {
                                return min_weight_pose
                                    .collect_animation_events(nodes, params, animations, strategy);
                            }
                        }
                    }
                }
            }
        }

        Default::default()
    }
}

pub struct PointsMut<'a, T: EntityId> {
    blend_space: &'a mut BlendSpace<T>,
}

impl<'a, T: EntityId> Deref for PointsMut<'a, T> {
    type Target = Vec<BlendSpacePoint<T>>;

    fn deref(&self) -> &Self::Target {
        &self.blend_space.points
    }
}

impl<'a, T: EntityId> DerefMut for PointsMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.blend_space.points
    }
}

impl<'a, T: EntityId> Drop for PointsMut<'a, T> {
    fn drop(&mut self) {
        self.blend_space.triangulate();
    }
}

impl<T: EntityId> BlendSpace<T> {
    pub fn add_point(&mut self, point: BlendSpacePoint<T>) -> bool {
        self.points.push(point);
        self.triangulate()
    }

    /// Sets new points to the blend space.
    pub fn set_points(&mut self, points: Vec<BlendSpacePoint<T>>) -> bool {
        self.points = points;
        self.triangulate()
    }

    pub fn clear_points(&mut self) {
        self.points.clear();
        self.triangles.clear();
    }

    pub fn points(&self) -> &[BlendSpacePoint<T>] {
        &self.points
    }

    pub fn points_mut(&mut self) -> PointsMut<T> {
        PointsMut { blend_space: self }
    }

    pub fn triangles(&self) -> &[TriangleDefinition] {
        &self.triangles
    }

    pub fn children(&self) -> Vec<Handle<PoseNode<T>>> {
        self.points.iter().map(|p| p.pose_source).collect()
    }

    pub fn set_min_values(&mut self, min_values: Vector2<f32>) {
        self.min_values = min_values;
        self.max_values = self.max_values.sup(&self.min_values);
    }

    pub fn min_values(&self) -> Vector2<f32> {
        self.min_values
    }

    pub fn set_max_values(&mut self, max_values: Vector2<f32>) {
        self.max_values = max_values;
        self.min_values = self.min_values.inf(&self.max_values);
    }

    pub fn max_values(&self) -> Vector2<f32> {
        self.max_values
    }

    pub fn set_snap_step(&mut self, step: Vector2<f32>) {
        self.snap_step = step;
    }

    pub fn snap_step(&self) -> Vector2<f32> {
        self.snap_step
    }

    pub fn set_sampling_parameter(&mut self, parameter: String) {
        self.sampling_parameter = parameter;
    }

    pub fn sampling_parameter(&self) -> &str {
        &self.sampling_parameter
    }

    pub fn set_x_axis_name(&mut self, name: String) -> String {
        std::mem::replace(&mut self.x_axis_name, name)
    }

    pub fn x_axis_name(&self) -> &str {
        &self.x_axis_name
    }

    pub fn set_y_axis_name(&mut self, name: String) -> String {
        std::mem::replace(&mut self.y_axis_name, name)
    }

    pub fn y_axis_name(&self) -> &str {
        &self.y_axis_name
    }

    pub fn try_snap_points(&mut self) {
        for point in self.points.iter_mut() {
            let x = math::round_to_step(point.position.x, self.snap_step.x)
                .clamp(self.min_values.x, self.max_values.x);
            let y = math::round_to_step(point.position.y, self.snap_step.y)
                .clamp(self.min_values.y, self.max_values.y);
            point.position = Vector2::new(x, y);
        }
    }

    pub fn fetch_weights(&self, sampling_point: Vector2<f32>) -> Option<[(usize, f32); 3]> {
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
            if (0.0..=1.0).contains(&t) {
                return Some([(0, (1.0 - t)), (1, t), (0, 0.0)]);
            }
        }

        let triangles = &self.triangles;

        // Try to find a triangle that contains the sampling point.
        for triangle in triangles.iter() {
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

        for triangle in triangles.iter() {
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

                if (0.0..=1.0).contains(&t) {
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
        core::{algebra::Vector2, math::TriangleDefinition},
        machine::node::blendspace::{BlendSpace, BlendSpacePoint},
    };
    use fyrox_core::pool::ErasedHandle;

    #[test]
    fn test_blend_space_triangulation() {
        let mut blend_space = BlendSpace::<ErasedHandle>::default();

        blend_space.set_points(vec![
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

        blend_space.fetch_weights(Default::default());

        assert_eq!(
            blend_space.triangles,
            vec![TriangleDefinition([2, 0, 1]), TriangleDefinition([3, 0, 2])]
        )
    }

    #[test]
    fn test_empty_blend_space_sampling() {
        assert!(BlendSpace::<ErasedHandle>::default()
            .fetch_weights(Default::default())
            .is_none())
    }

    #[test]
    fn test_single_point_blend_space_sampling() {
        let mut blend_space = BlendSpace::<ErasedHandle>::default();

        blend_space.set_points(vec![BlendSpacePoint {
            position: Vector2::new(0.0, 0.0),
            pose_source: Default::default(),
        }]);

        assert_eq!(
            blend_space.fetch_weights(Default::default()),
            Some([(0, 1.0), (0, 0.0), (0, 0.0)])
        );
    }

    #[test]
    fn test_two_points_blend_space_sampling() {
        let mut blend_space = BlendSpace::<ErasedHandle>::default();

        blend_space.set_points(vec![
            BlendSpacePoint {
                position: Vector2::new(0.0, 0.0),
                pose_source: Default::default(),
            },
            BlendSpacePoint {
                position: Vector2::new(1.0, 0.0),
                pose_source: Default::default(),
            },
        ]);

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
