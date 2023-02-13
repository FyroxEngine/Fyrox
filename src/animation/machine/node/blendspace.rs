#![allow(missing_docs)] // TODO

use crate::{
    animation::{
        machine::{node::BasePoseNode, EvaluatePose, ParameterContainer, PoseNode},
        AnimationContainer, AnimationPose,
    },
    core::{
        algebra::Vector2,
        math::TriangleDefinition,
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
    },
    utils::log::Log,
};
use spade::{DelaunayTriangulation, InsertionError, Point2, Triangulation};
use std::{
    cell::{Ref, RefCell},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Visit, Clone, Reflect, PartialEq, Default)]
pub struct BlendSpacePoint {
    position: Vector2<u32>,
    pose_source: Handle<PoseNode>,
}

#[derive(Debug, Visit, Clone, Reflect, PartialEq, Default)]
pub struct BlendSpace {
    base: BasePoseNode,

    points: Vec<BlendSpacePoint>,
    triangles: Vec<TriangleDefinition>,

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
        animations: &AnimationContainer,
        dt: f32,
    ) -> Ref<AnimationPose> {
        todo!()
    }

    fn pose(&self) -> Ref<AnimationPose> {
        self.pose.borrow()
    }
}

impl BlendSpace {
    pub fn set_points(&mut self, points: Vec<BlendSpacePoint>) {
        self.points = points;
        Log::verify(self.triangulate());
    }

    pub fn points(&self) -> &[BlendSpacePoint] {
        &self.points
    }

    pub fn children(&self) -> Vec<Handle<PoseNode>> {
        self.points.iter().map(|p| p.pose_source).collect()
    }

    fn triangulate(&mut self) -> Result<(), InsertionError> {
        self.triangles.clear();

        let mut triangulation: DelaunayTriangulation<_> = DelaunayTriangulation::new();

        for point in self.points.iter() {
            triangulation.insert(Point2::new(
                point.position.x as f32,
                point.position.y as f32,
            ))?;
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
