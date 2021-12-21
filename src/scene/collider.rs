//! Collider is a geometric entity that can be attached to a rigid body to allow participate it
//! participate in contact generation, collision response and proximity queries.

use crate::{
    core::{
        algebra::Vector3,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::rapier::geometry::{self, ColliderHandle},
    scene::{
        base::{Base, BaseBuilder},
        graph::{
            physics::{CoefficientCombineRule, ContactPair, PhysicsWorld},
            Graph,
        },
        node::Node,
    },
};
use bitflags::bitflags;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
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

/// Ball is an idea sphere shape defined by a single parameters - its radius.
#[derive(Clone, Debug, Visit, Inspect)]
pub struct BallShape {
    /// Radius of the sphere.
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

impl Default for BallShape {
    fn default() -> Self {
        Self { radius: 0.5 }
    }
}

/// Cylinder shape aligned in Y axis.
#[derive(Clone, Debug, Visit, Inspect)]
pub struct CylinderShape {
    /// Half height of the cylinder, actual height will be 2 times bigger.
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    /// Radius of the cylinder.
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

impl Default for CylinderShape {
    fn default() -> Self {
        Self {
            half_height: 0.5,
            radius: 0.5,
        }
    }
}

/// Cone shape aligned in Y axis.
#[derive(Clone, Debug, Visit, Inspect)]
pub struct ConeShape {
    /// Half height of the cone, actual height will be 2 times bigger.
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    /// Radius of the cone base.
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

impl Default for ConeShape {
    fn default() -> Self {
        Self {
            half_height: 0.5,
            radius: 0.5,
        }
    }
}

/// Cuboid shape (box).
#[derive(Clone, Debug, Visit, Inspect)]
pub struct CuboidShape {
    /// Half extents of the box. X - half width, Y - half height, Z - half depth.
    /// Actual _size_ will be 2 times bigger.
    pub half_extents: Vector3<f32>,
}

impl Default for CuboidShape {
    fn default() -> Self {
        Self {
            half_extents: Vector3::new(0.5, 0.5, 0.5),
        }
    }
}

/// Arbitrary capsule shape defined by 2 points (which forms axis) and a radius.
#[derive(Clone, Debug, Visit, Inspect)]
pub struct CapsuleShape {
    /// Begin point of the capsule.
    pub begin: Vector3<f32>,
    /// End point of the capsule.
    pub end: Vector3<f32>,
    /// Radius of the capsule.
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

impl Default for CapsuleShape {
    // Y-capsule
    fn default() -> Self {
        Self {
            begin: Default::default(),
            end: Vector3::new(0.0, 1.0, 0.0),
            radius: 0.5,
        }
    }
}

/// Arbitrary segment shape defined by two points.
#[derive(Clone, Debug, Visit, Inspect)]
pub struct SegmentShape {
    /// Begin point of the capsule.
    pub begin: Vector3<f32>,
    /// End point of the capsule.
    pub end: Vector3<f32>,
}

impl Default for SegmentShape {
    fn default() -> Self {
        Self {
            begin: Default::default(),
            end: Vector3::new(0.0, 1.0, 0.0),
        }
    }
}

/// Arbitrary triangle shape.
#[derive(Clone, Debug, Visit, Inspect)]
pub struct TriangleShape {
    /// First point of the triangle shape.
    pub a: Vector3<f32>,
    /// Second point of the triangle shape.
    pub b: Vector3<f32>,
    /// Third point of the triangle shape.
    pub c: Vector3<f32>,
}

impl Default for TriangleShape {
    fn default() -> Self {
        Self {
            a: Default::default(),
            b: Vector3::new(1.0, 0.0, 0.0),
            c: Vector3::new(0.0, 0.0, 1.0),
        }
    }
}

/// Geometry source for colliders with complex geometry.
///
/// # Notes
///
/// Currently there is only one way to set geometry - using a scene node as a source of data.
#[derive(Default, Clone, Copy, PartialEq, Hash, Debug, Visit, Inspect)]
pub struct GeometrySource(pub Handle<Node>);

/// Arbitrary triangle mesh shape.
#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct TrimeshShape {
    /// Geometry sources for the shape.
    pub sources: Vec<GeometrySource>,
}

/// Arbitrary height field shape.
#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct HeightfieldShape {
    /// A handle to terrain scene node.
    pub geometry_source: GeometrySource,
}

/// Pairwise filtering using bit masks.
///
/// This filtering method is based on two 32-bit values:
/// - The interaction groups memberships.
/// - The interaction groups filter.
///
/// An interaction is allowed between two filters `a` and `b` when two conditions
/// are met simultaneously:
/// - The groups membership of `a` has at least one bit set to `1` in common with the groups filter of `b`.
/// - The groups membership of `b` has at least one bit set to `1` in common with the groups filter of `a`.
///
/// In other words, interactions are allowed between two filter iff. the following condition is met:
/// ```ignore
/// (self.memberships & rhs.filter) != 0 && (rhs.memberships & self.filter) != 0
/// ```
#[derive(Visit, Debug, Clone, Copy, Inspect)]
pub struct InteractionGroups {
    /// Groups memberships.
    pub memberships: u32,
    /// Groups filter.
    pub filter: u32,
}

impl InteractionGroups {
    /// Creates new interaction group using given values.
    pub fn new(memberships: u32, filter: u32) -> Self {
        Self {
            memberships,
            filter,
        }
    }
}

impl Default for InteractionGroups {
    fn default() -> Self {
        Self {
            memberships: u32::MAX,
            filter: u32::MAX,
        }
    }
}

impl From<geometry::InteractionGroups> for InteractionGroups {
    fn from(g: geometry::InteractionGroups) -> Self {
        Self {
            memberships: g.memberships,
            filter: g.filter,
        }
    }
}

impl Inspect for ColliderShape {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            ColliderShape::Ball(v) => v.properties(),
            ColliderShape::Cylinder(v) => v.properties(),
            ColliderShape::Cone(v) => v.properties(),
            ColliderShape::Cuboid(v) => v.properties(),
            ColliderShape::Capsule(v) => v.properties(),
            ColliderShape::Segment(v) => v.properties(),
            ColliderShape::Triangle(v) => v.properties(),
            ColliderShape::Trimesh(v) => v.properties(),
            ColliderShape::Heightfield(v) => v.properties(),
        }
    }
}

/// Possible collider shapes.
#[derive(Clone, Debug, Visit)]
pub enum ColliderShape {
    /// See [`BallShape`] docs.
    Ball(BallShape),
    /// See [`CylinderShape`] docs.
    Cylinder(CylinderShape),
    /// See [`ConeShape`] docs.
    Cone(ConeShape),
    /// See [`CuboidShape`] docs.
    Cuboid(CuboidShape),
    /// See [`CapsuleShape`] docs.
    Capsule(CapsuleShape),
    /// See [`SegmentShape`] docs.
    Segment(SegmentShape),
    /// See [`TriangleShape`] docs.
    Triangle(TriangleShape),
    /// See [`TrimeshShape`] docs.
    Trimesh(TrimeshShape),
    /// See [`HeightfieldShape`] docs.
    Heightfield(HeightfieldShape),
}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Ball(Default::default())
    }
}

impl ColliderShape {
    /// Initializes a ball shape defined by its radius.
    pub fn ball(radius: f32) -> Self {
        Self::Ball(BallShape { radius })
    }

    /// Initializes a cylindrical shape defined by its half-height (along along the y axis) and its
    /// radius.
    pub fn cylinder(half_height: f32, radius: f32) -> Self {
        Self::Cylinder(CylinderShape {
            half_height,
            radius,
        })
    }

    /// Initializes a cone shape defined by its half-height (along along the y axis) and its basis
    /// radius.
    pub fn cone(half_height: f32, radius: f32) -> Self {
        Self::Cone(ConeShape {
            half_height,
            radius,
        })
    }

    /// Initializes a cuboid shape defined by its half-extents.
    pub fn cuboid(hx: f32, hy: f32, hz: f32) -> Self {
        Self::Cuboid(CuboidShape {
            half_extents: Vector3::new(hx, hy, hz),
        })
    }

    /// Initializes a capsule shape from its endpoints and radius.
    pub fn capsule(begin: Vector3<f32>, end: Vector3<f32>, radius: f32) -> Self {
        Self::Capsule(CapsuleShape { begin, end, radius })
    }

    /// Initializes a new collider builder with a capsule shape aligned with the `x` axis.
    pub fn capsule_x(half_height: f32, radius: f32) -> Self {
        let p = Vector3::x() * half_height;
        Self::capsule(-p, p, radius)
    }

    /// Initializes a new collider builder with a capsule shape aligned with the `y` axis.
    pub fn capsule_y(half_height: f32, radius: f32) -> Self {
        let p = Vector3::y() * half_height;
        Self::capsule(-p, p, radius)
    }

    /// Initializes a new collider builder with a capsule shape aligned with the `z` axis.
    pub fn capsule_z(half_height: f32, radius: f32) -> Self {
        let p = Vector3::z() * half_height;
        Self::capsule(-p, p, radius)
    }

    /// Initializes a segment shape from its endpoints.
    pub fn segment(begin: Vector3<f32>, end: Vector3<f32>) -> Self {
        Self::Segment(SegmentShape { begin, end })
    }

    /// Initializes a triangle shape.
    pub fn triangle(a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>) -> Self {
        Self::Triangle(TriangleShape { a, b, c })
    }

    /// Initializes a triangle mesh shape defined by a set of handles to mesh nodes that will be
    /// used to create physical shape.
    pub fn trimesh(geometry_sources: Vec<GeometrySource>) -> Self {
        Self::Trimesh(TrimeshShape {
            sources: geometry_sources,
        })
    }

    /// Initializes a heightfield shape defined by a handle to terrain node.
    pub fn heightfield(geometry_source: GeometrySource) -> Self {
        Self::Heightfield(HeightfieldShape { geometry_source })
    }
}

/// Collider is a geometric entity that can be attached to a rigid body to allow participate it
/// participate in contact generation, collision response and proximity queries.
#[derive(Inspect, Visit, Debug)]
pub struct Collider {
    base: Base,
    shape: ColliderShape,
    #[inspect(min_value = 0.0, step = 0.05)]
    friction: f32,
    density: Option<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroups,
    solver_groups: InteractionGroups,
    friction_combine_rule: CoefficientCombineRule,
    restitution_combine_rule: CoefficientCombineRule,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) native: Cell<ColliderHandle>,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) changes: Cell<ColliderChanges>,
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
            friction_combine_rule: Default::default(),
            restitution_combine_rule: Default::default(),
            native: Cell::new(ColliderHandle::invalid()),
            changes: Cell::new(ColliderChanges::NONE),
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

impl Collider {
    /// Creates a raw copy of the collider. This method is for internal use only!
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
            friction_combine_rule: self.friction_combine_rule,
            restitution_combine_rule: self.restitution_combine_rule,
            // Do not copy.
            native: Cell::new(ColliderHandle::invalid()),
            changes: Cell::new(ColliderChanges::NONE),
        }
    }

    /// Sets the new shape to the collider.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_shape(&mut self, shape: ColliderShape) {
        self.shape = shape;
        self.changes.get_mut().insert(ColliderChanges::SHAPE);
    }

    /// Returns shared reference to the collider shape.
    pub fn shape(&self) -> &ColliderShape {
        &self.shape
    }

    /// Returns a copy of the collider shape.
    pub fn shape_value(&self) -> ColliderShape {
        self.shape.clone()
    }

    /// Returns mutable reference to the current collider shape.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn shape_mut(&mut self) -> &mut ColliderShape {
        self.changes.get_mut().insert(ColliderChanges::SHAPE);
        &mut self.shape
    }

    /// Sets the new restitution value. The exact meaning of possible values is somewhat complex,
    /// check [Wikipedia page](https://en.wikipedia.org/wiki/Coefficient_of_restitution) for more
    /// info.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_restitution(&mut self, restitution: f32) {
        self.restitution = restitution;
        self.changes.get_mut().insert(ColliderChanges::RESTITUTION);
    }

    /// Returns current restitution value of the collider.
    pub fn restitution(&self) -> f32 {
        self.restitution
    }

    /// Sets the new density value of the collider. Density defines actual mass of the rigid body to
    /// which the collider is attached. Final mass will be a sum of `ColliderVolume * ColliderDensity`
    /// of each collider. In case if density is undefined, the mass of the collider will be zero,
    /// which will lead to two possible effects:
    ///
    /// 1) If a rigid body to which collider is attached have no additional mass, then the rigid body
    ///    won't rotate, only move.
    /// 2) If the rigid body have some additional mass, then the rigid body will have normal behaviour.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_density(&mut self, density: Option<f32>) {
        self.density = density;
        self.changes.get_mut().insert(ColliderChanges::DENSITY);
    }

    /// Returns current density of the collider.
    pub fn density(&self) -> Option<f32> {
        self.density
    }

    /// Sets friction coefficient for the collider. The greater value is the more kinematic energy
    /// will be converted to heat (in other words - lost), the parent rigid body will slowdown much
    /// faster and so on.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_friction(&mut self, friction: f32) {
        self.friction = friction;
        self.changes.get_mut().insert(ColliderChanges::FRICTION);
    }

    /// Return current friction of the collider.
    pub fn friction(&self) -> f32 {
        self.friction
    }

    /// Sets the new collision filtering options. See [`InteractionGroups`] docs for more info.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_collision_groups(&mut self, groups: InteractionGroups) {
        self.collision_groups = groups;
        self.changes
            .get_mut()
            .insert(ColliderChanges::COLLISION_GROUPS);
    }

    /// Returns current collision filtering options.
    pub fn collision_groups(&self) -> InteractionGroups {
        self.collision_groups
    }

    /// Sets the new joint solver filtering options. See [`InteractionGroups`] docs for more info.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_solver_groups(&mut self, groups: InteractionGroups) {
        self.solver_groups = groups;
        self.changes
            .get_mut()
            .insert(ColliderChanges::SOLVER_GROUPS);
    }

    /// Returns current solver groups.
    pub fn solver_groups(&self) -> InteractionGroups {
        self.solver_groups
    }

    /// If true is passed, the method makes collider a sensor. Sensors will not participate in
    /// collision response, but it is still possible to query contact information from them.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_is_sensor(&mut self, is_sensor: bool) {
        self.is_sensor = is_sensor;
        self.changes.get_mut().insert(ColliderChanges::IS_SENSOR);
    }

    /// Returns true if the collider is sensor, false - otherwise.
    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }

    /// Sets the new friction combine rule. See [`CoefficientCombineRule`] docs for more info.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_friction_combine_rule(&mut self, rule: CoefficientCombineRule) {
        self.friction_combine_rule = rule;
        self.changes
            .get_mut()
            .insert(ColliderChanges::FRICTION_COMBINE_RULE);
    }

    /// Returns current friction combine rule of the collider.
    pub fn friction_combine_rule(&self) -> CoefficientCombineRule {
        self.friction_combine_rule
    }

    /// Sets the new restitution combine rule. See [`CoefficientCombineRule`] docs for more info.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_restitution_combine_rule(&mut self, rule: CoefficientCombineRule) {
        self.restitution_combine_rule = rule;
        self.changes
            .get_mut()
            .insert(ColliderChanges::RESTITUTION_COMBINE_RULE);
    }

    /// Returns current restitution combine rule of the collider.
    pub fn restitution_combine_rule(&self) -> CoefficientCombineRule {
        self.restitution_combine_rule
    }

    /// Returns an iterator that yields contact information for the collider.
    pub fn contacts<'a>(
        &self,
        physics: &'a PhysicsWorld,
    ) -> impl Iterator<Item = ContactPair> + 'a {
        physics.contacts_with(self.native.get())
    }
}

/// Collider builder allows you to build a collider node in declarative mannner.
pub struct ColliderBuilder {
    base_builder: BaseBuilder,
    shape: ColliderShape,
    friction: f32,
    density: Option<f32>,
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroups,
    solver_groups: InteractionGroups,
    friction_combine_rule: CoefficientCombineRule,
    restitution_combine_rule: CoefficientCombineRule,
}

impl ColliderBuilder {
    /// Creates new collider builder.
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
            friction_combine_rule: Default::default(),
            restitution_combine_rule: Default::default(),
        }
    }

    /// Sets desired shape of the collider.
    pub fn with_shape(mut self, shape: ColliderShape) -> Self {
        self.shape = shape;
        self
    }

    /// Sets desired density value.
    pub fn with_density(mut self, density: Option<f32>) -> Self {
        self.density = density;
        self
    }

    /// Sets desired restitution value.
    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution;
        self
    }

    /// Sets desired friction value.    
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    /// Sets whether this collider will be used a sensor or not.
    pub fn with_sensor(mut self, sensor: bool) -> Self {
        self.is_sensor = sensor;
        self
    }

    /// Sets desired solver groups.    
    pub fn with_solver_groups(mut self, solver_groups: InteractionGroups) -> Self {
        self.solver_groups = solver_groups;
        self
    }

    /// Sets desired collision groups.
    pub fn with_collision_groups(mut self, collision_groups: InteractionGroups) -> Self {
        self.collision_groups = collision_groups;
        self
    }

    /// Sets desired friction combine rule.
    pub fn with_friction_combine_rule(mut self, rule: CoefficientCombineRule) -> Self {
        self.friction_combine_rule = rule;
        self
    }

    /// Sets desired restitution combine rule.
    pub fn with_restitution_combine_rule(mut self, rule: CoefficientCombineRule) -> Self {
        self.restitution_combine_rule = rule;
        self
    }

    /// Creates collider node, but does not add it to a graph.
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
            friction_combine_rule: self.friction_combine_rule,
            restitution_combine_rule: self.restitution_combine_rule,
            native: Cell::new(ColliderHandle::invalid()),
            changes: Cell::new(ColliderChanges::NONE),
        };
        Node::Collider(collider)
    }

    /// Creates collider node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
