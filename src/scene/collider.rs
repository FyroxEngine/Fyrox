#![allow(missing_docs)]

use crate::{
    core::{
        algebra::{DMatrix, Dynamic, Point3, VecStorage, Vector3},
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::rapier::geometry::{
        ColliderHandle, Cuboid, InteractionGroups, Segment, Shape, SharedShape,
    },
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use bitflags::bitflags;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

bitflags! {
    pub(crate) struct ColliderChanges: u32 {
        const NONE = 0;
        const SHAPE = 0b0000_0001;
        const RESTITUTION = 0b0000_0010;
        const COLLISION_GROUPS = 0b0000_0100;
        const FRICTION = 0b0000_1000;
        const FRICTION_COMBINE_RULE = 0b0001_0000;
        const RESTITUTION_COMBINE_RULE = 0b0010_0000;
        const IS_SENSOR = 0b0100_0000;
        const SOLVER_GROUPS = 0b1000_0000;
        const DENSITY = 0b0001_0000_0000;
    }
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct BallShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct CylinderShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct RoundCylinderShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub border_radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct ConeShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct CuboidShape {
    pub half_extents: Vector3<f32>,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct CapsuleShape {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct SegmentShape {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct TriangleShape {
    pub a: Vector3<f32>,
    pub b: Vector3<f32>,
    pub c: Vector3<f32>,
}

#[derive(Default, Clone, PartialEq, Hash, Debug, Visit, Inspect)]
pub struct GeometrySource(pub Handle<Node>);

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct TrimeshShape {
    pub sources: Vec<GeometrySource>,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct HeightfieldShape {
    pub geometry_source: GeometrySource,
}

#[doc(hidden)]
#[derive(Visit, Debug, Clone, Copy, Inspect)]
pub struct InteractionGroupsDesc {
    pub memberships: u32,
    pub filter: u32,
}

impl Default for InteractionGroupsDesc {
    fn default() -> Self {
        Self {
            memberships: u32::MAX,
            filter: u32::MAX,
        }
    }
}

impl From<InteractionGroups> for InteractionGroupsDesc {
    fn from(g: InteractionGroups) -> Self {
        Self {
            memberships: g.memberships,
            filter: g.filter,
        }
    }
}

impl Inspect for ColliderShapeDesc {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            ColliderShapeDesc::Ball(v) => v.properties(),
            ColliderShapeDesc::Cylinder(v) => v.properties(),
            ColliderShapeDesc::RoundCylinder(v) => v.properties(),
            ColliderShapeDesc::Cone(v) => v.properties(),
            ColliderShapeDesc::Cuboid(v) => v.properties(),
            ColliderShapeDesc::Capsule(v) => v.properties(),
            ColliderShapeDesc::Segment(v) => v.properties(),
            ColliderShapeDesc::Triangle(v) => v.properties(),
            ColliderShapeDesc::Trimesh(v) => v.properties(),
            ColliderShapeDesc::Heightfield(v) => v.properties(),
        }
    }
}

#[derive(Clone, Debug, Visit)]
pub enum ColliderShapeDesc {
    Ball(BallShape),
    Cylinder(CylinderShape),
    RoundCylinder(RoundCylinderShape),
    Cone(ConeShape),
    Cuboid(CuboidShape),
    Capsule(CapsuleShape),
    Segment(SegmentShape),
    Triangle(TriangleShape),
    Trimesh(TrimeshShape),
    Heightfield(HeightfieldShape),
}

impl Default for ColliderShapeDesc {
    fn default() -> Self {
        Self::Ball(Default::default())
    }
}

impl ColliderShapeDesc {
    pub(crate) fn from_collider_shape(shape: &dyn Shape) -> Self {
        if let Some(ball) = shape.as_ball() {
            ColliderShapeDesc::Ball(BallShape {
                radius: ball.radius,
            })
        } else if let Some(cuboid) = shape.as_cuboid() {
            ColliderShapeDesc::Cuboid(CuboidShape {
                half_extents: cuboid.half_extents,
            })
        } else if let Some(capsule) = shape.as_capsule() {
            ColliderShapeDesc::Capsule(CapsuleShape {
                begin: capsule.segment.a.coords,
                end: capsule.segment.b.coords,
                radius: capsule.radius,
            })
        } else if let Some(segment) = shape.downcast_ref::<Segment>() {
            ColliderShapeDesc::Segment(SegmentShape {
                begin: segment.a.coords,
                end: segment.b.coords,
            })
        } else if let Some(triangle) = shape.as_triangle() {
            ColliderShapeDesc::Triangle(TriangleShape {
                a: triangle.a.coords,
                b: triangle.b.coords,
                c: triangle.c.coords,
            })
        } else if shape.as_trimesh().is_some() {
            ColliderShapeDesc::Trimesh(TrimeshShape {
                sources: Default::default(),
            })
        } else if shape.as_heightfield().is_some() {
            ColliderShapeDesc::Heightfield(HeightfieldShape {
                geometry_source: Default::default(),
            })
        } else if let Some(cylinder) = shape.as_cylinder() {
            ColliderShapeDesc::Cylinder(CylinderShape {
                half_height: cylinder.half_height,
                radius: cylinder.radius,
            })
        } else if let Some(round_cylinder) = shape.as_round_cylinder() {
            ColliderShapeDesc::RoundCylinder(RoundCylinderShape {
                half_height: round_cylinder.base_shape.half_height,
                radius: round_cylinder.base_shape.radius,
                border_radius: round_cylinder.border_radius,
            })
        } else if let Some(cone) = shape.as_cone() {
            ColliderShapeDesc::Cone(ConeShape {
                half_height: cone.half_height,
                radius: cone.radius,
            })
        } else {
            unreachable!()
        }
    }

    // Converts descriptor in a shared shape.
    pub(crate) fn into_collider_shape(self) -> SharedShape {
        match self {
            ColliderShapeDesc::Ball(ball) => SharedShape::ball(ball.radius),

            ColliderShapeDesc::Cylinder(cylinder) => {
                SharedShape::cylinder(cylinder.half_height, cylinder.radius)
            }
            ColliderShapeDesc::RoundCylinder(rcylinder) => SharedShape::round_cylinder(
                rcylinder.half_height,
                rcylinder.radius,
                rcylinder.border_radius,
            ),
            ColliderShapeDesc::Cone(cone) => SharedShape::cone(cone.half_height, cone.radius),
            ColliderShapeDesc::Cuboid(cuboid) => {
                SharedShape(Arc::new(Cuboid::new(cuboid.half_extents)))
            }
            ColliderShapeDesc::Capsule(capsule) => SharedShape::capsule(
                Point3::from(capsule.begin),
                Point3::from(capsule.end),
                capsule.radius,
            ),
            ColliderShapeDesc::Segment(segment) => {
                SharedShape::segment(Point3::from(segment.begin), Point3::from(segment.end))
            }
            ColliderShapeDesc::Triangle(triangle) => SharedShape::triangle(
                Point3::from(triangle.a),
                Point3::from(triangle.b),
                Point3::from(triangle.c),
            ),
            ColliderShapeDesc::Trimesh(_) => {
                // Create fake trimesh. It will be filled with actual data on resolve stage later on.
                let a = Point3::new(0.0, 0.0, 1.0);
                let b = Point3::new(1.0, 0.0, 1.0);
                let c = Point3::new(1.0, 0.0, 0.0);
                SharedShape::trimesh(vec![a, b, c], vec![[0, 1, 2]])
            }
            ColliderShapeDesc::Heightfield(_) => SharedShape::heightfield(
                {
                    DMatrix::from_data(VecStorage::new(
                        Dynamic::new(2),
                        Dynamic::new(2),
                        vec![0.0, 1.0, 0.0, 0.0],
                    ))
                },
                Default::default(),
            ),
        }
    }
}

#[derive(Inspect, Visit, Debug)]
pub struct Collider {
    base: Base,
    shape: ColliderShapeDesc,
    #[inspect(min_value = 0.0, step = 0.05)]
    friction: f32,
    density: Option<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroupsDesc,
    solver_groups: InteractionGroupsDesc,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) native: ColliderHandle,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) changes: ColliderChanges,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            base: Default::default(),
            shape: Default::default(),
            friction: 0.0,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            collision_groups: Default::default(),
            solver_groups: Default::default(),
            native: ColliderHandle::invalid(),
            changes: ColliderChanges::NONE,
        }
    }
}

impl Deref for Collider {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Collider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

pub struct ColliderShapeRefMut<'a> {
    parent: &'a mut Collider,
}

impl<'a> Drop for ColliderShapeRefMut<'a> {
    fn drop(&mut self) {
        self.parent.changes.insert(ColliderChanges::SHAPE);
    }
}

impl<'a> Deref for ColliderShapeRefMut<'a> {
    type Target = ColliderShapeDesc;

    fn deref(&self) -> &Self::Target {
        &self.parent.shape
    }
}

impl<'a> DerefMut for ColliderShapeRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.parent.shape
    }
}

impl Collider {
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            shape: self.shape.clone(),
            friction: self.friction,
            density: self.density,
            restitution: self.restitution,
            is_sensor: self.is_sensor,
            collision_groups: self.collision_groups,
            solver_groups: self.solver_groups,
            // Do not copy.
            native: ColliderHandle::invalid(),
            changes: ColliderChanges::NONE,
        }
    }

    pub fn set_shape(&mut self, shape: ColliderShapeDesc) {
        self.shape = shape;
        self.changes.insert(ColliderChanges::SHAPE);
    }

    pub fn shape(&self) -> &ColliderShapeDesc {
        &self.shape
    }

    pub fn shape_mut(&mut self) -> ColliderShapeRefMut {
        ColliderShapeRefMut { parent: self }
    }

    pub fn set_restitution(&mut self, restitution: f32) {
        self.restitution = restitution;
        self.changes.insert(ColliderChanges::RESTITUTION);
    }

    pub fn restitution(&self) -> f32 {
        self.restitution
    }

    pub fn set_density(&mut self, density: Option<f32>) {
        self.density = density;
        self.changes.insert(ColliderChanges::DENSITY);
    }

    pub fn density(&self) -> Option<f32> {
        self.density
    }

    pub fn set_friction(&mut self, friction: f32) {
        self.friction = friction;
        self.changes.insert(ColliderChanges::FRICTION);
    }

    pub fn friction(&self) -> f32 {
        self.friction
    }

    pub fn set_collision_groups(&mut self, groups: InteractionGroupsDesc) {
        self.collision_groups = groups;
        self.changes.insert(ColliderChanges::COLLISION_GROUPS);
    }

    pub fn collision_groups(&self) -> InteractionGroupsDesc {
        self.collision_groups
    }

    pub fn set_solver_groups(&mut self, groups: InteractionGroupsDesc) {
        self.solver_groups = groups;
        self.changes.insert(ColliderChanges::SOLVER_GROUPS);
    }

    pub fn solver_groups(&self) -> InteractionGroupsDesc {
        self.solver_groups
    }

    pub fn set_is_sensor(&mut self, is_sensor: bool) {
        self.is_sensor = is_sensor;
        self.changes.insert(ColliderChanges::IS_SENSOR);
    }

    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }
}

pub struct ColliderBuilder {
    base_builder: BaseBuilder,
    shape: ColliderShapeDesc,
    friction: f32,
    density: Option<f32>,
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroupsDesc,
    solver_groups: InteractionGroupsDesc,
}

impl ColliderBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            shape: Default::default(),
            friction: 0.0,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            collision_groups: Default::default(),
            solver_groups: Default::default(),
        }
    }

    pub fn with_shape(mut self, shape: ColliderShapeDesc) -> Self {
        self.shape = shape;
        self
    }

    pub fn build_node(self) -> Node {
        let collider = Collider {
            base: self.base_builder.build_base(),
            shape: self.shape,
            friction: self.friction,
            density: self.density,
            restitution: self.restitution,
            is_sensor: self.is_sensor,
            collision_groups: self.collision_groups,
            solver_groups: self.solver_groups,
            native: ColliderHandle::invalid(),
            changes: ColliderChanges::NONE,
        };
        Node::Collider(collider)
    }

    pub fn with_density(mut self, density: Option<f32>) -> Self {
        self.density = density;
        self
    }

    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution;
        self
    }

    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    pub fn with_sensor(mut self, sensor: bool) -> Self {
        self.is_sensor = sensor;
        self
    }

    pub fn with_solver_groups(mut self, solver_groups: InteractionGroupsDesc) -> Self {
        self.solver_groups = solver_groups;
        self
    }

    pub fn with_collision_groups(mut self, collision_groups: InteractionGroupsDesc) -> Self {
        self.collision_groups = collision_groups;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
