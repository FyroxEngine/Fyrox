//! Collider is a geometric entity that can be attached to a rigid body to allow participate it
//! participate in contact generation, collision response and proximity queries.

use crate::{
    core::{
        algebra::Vector3,
        log::Log,
        math::aabb::AxisAlignedBoundingBox,
        num_traits::{NumCast, One, ToPrimitive, Zero},
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    scene::{
        base::{Base, BaseBuilder},
        graph::{
            physics::{CoefficientCombineRule, ContactPair, IntersectionPair, PhysicsWorld},
            Graph,
        },
        node::{Node, NodeTrait, SyncContext},
        rigidbody::RigidBody,
        Scene,
    },
};
use fyrox_core::uuid_provider;
use fyrox_graph::BaseSceneGraph;
use rapier3d::geometry::{self, ColliderHandle};
use std::{
    cell::Cell,
    ops::{Add, BitAnd, BitOr, Deref, DerefMut, Mul, Not, Shl},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Ball is an idea sphere shape defined by a single parameters - its radius.
#[derive(Clone, Debug, PartialEq, Visit, Reflect)]
pub struct BallShape {
    /// Radius of the sphere.
    #[reflect(min_value = 0.001, step = 0.05)]
    pub radius: f32,
}

impl Default for BallShape {
    fn default() -> Self {
        Self { radius: 0.5 }
    }
}

/// Cylinder shape aligned in Y axis.
#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
pub struct CylinderShape {
    /// Half height of the cylinder, actual height will be 2 times bigger.
    #[reflect(min_value = 0.001, step = 0.05)]
    pub half_height: f32,
    /// Radius of the cylinder.
    #[reflect(min_value = 0.001, step = 0.05)]
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
#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
pub struct ConeShape {
    /// Half height of the cone, actual height will be 2 times bigger.
    #[reflect(min_value = 0.001, step = 0.05)]
    pub half_height: f32,
    /// Radius of the cone base.
    #[reflect(min_value = 0.001, step = 0.05)]
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
#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
pub struct CuboidShape {
    /// Half extents of the box. X - half width, Y - half height, Z - half depth.
    /// Actual _size_ will be 2 times bigger.
    #[reflect(min_value = 0.001, step = 0.05)]
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
#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
pub struct CapsuleShape {
    /// Begin point of the capsule.
    pub begin: Vector3<f32>,
    /// End point of the capsule.
    pub end: Vector3<f32>,
    /// Radius of the capsule.
    #[reflect(min_value = 0.001, step = 0.05)]
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
#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
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
#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
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
#[derive(Default, Clone, Copy, PartialEq, Hash, Debug, Visit, Reflect, Eq)]
pub struct GeometrySource(pub Handle<Node>);

uuid_provider!(GeometrySource = "6fea7c72-c488-48a1-935f-2752a8a10e9a");

/// Arbitrary triangle mesh shape.
#[derive(Default, Clone, Debug, Visit, Reflect, PartialEq, Eq)]
pub struct TrimeshShape {
    /// Geometry sources for the shape.
    pub sources: Vec<GeometrySource>,
}

/// Arbitrary height field shape.
#[derive(Default, Clone, Debug, Visit, Reflect, PartialEq, Eq)]
pub struct HeightfieldShape {
    /// A handle to terrain scene node.
    pub geometry_source: GeometrySource,
}

/// Arbitrary convex polyhedron shape.
#[derive(Default, Clone, Debug, Visit, Reflect, PartialEq, Eq)]
pub struct ConvexPolyhedronShape {
    /// A handle to a mesh node.
    pub geometry_source: GeometrySource,
}

/// A set of bits used for pairwise collision filtering.
#[derive(Clone, Copy, Default, PartialEq, Debug, Reflect, Eq)]
pub struct BitMask(pub u32);

uuid_provider!(BitMask = "f2db0c2a-921b-4728-9ce4-2506d95c60fa");

impl Visit for BitMask {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl BitOr for BitMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for BitMask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Mul for BitMask {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl One for BitMask {
    fn one() -> Self {
        Self(1)
    }
}

impl Add for BitMask {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Zero for BitMask {
    fn zero() -> Self {
        Self(0)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Shl for BitMask {
    type Output = Self;

    fn shl(self, rhs: Self) -> Self::Output {
        Self(self.0 << rhs.0)
    }
}

impl Not for BitMask {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl ToPrimitive for BitMask {
    fn to_i64(&self) -> Option<i64> {
        Some(self.0 as i64)
    }

    fn to_u64(&self) -> Option<u64> {
        Some(self.0 as u64)
    }
}

impl NumCast for BitMask {
    fn from<T: ToPrimitive>(n: T) -> Option<Self> {
        n.to_u32().map(Self)
    }
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
#[derive(Visit, Debug, Clone, Copy, PartialEq, Reflect, Eq)]
pub struct InteractionGroups {
    /// Groups memberships.
    pub memberships: BitMask,
    /// Groups filter.
    pub filter: BitMask,
}

impl InteractionGroups {
    /// Creates new interaction group using given values.
    pub fn new(memberships: BitMask, filter: BitMask) -> Self {
        Self {
            memberships,
            filter,
        }
    }
}

impl Default for InteractionGroups {
    fn default() -> Self {
        Self {
            memberships: BitMask(u32::MAX),
            filter: BitMask(u32::MAX),
        }
    }
}

impl From<geometry::InteractionGroups> for InteractionGroups {
    fn from(g: geometry::InteractionGroups) -> Self {
        Self {
            memberships: BitMask(g.memberships.bits()),
            filter: BitMask(g.filter.bits()),
        }
    }
}

bitflags::bitflags! {
    #[derive(Default, Copy, Clone)]
    /// Flags for excluding whole sets of colliders from a scene query.
    pub struct QueryFilterFlags: u32 {
        /// Exclude from the query any collider attached to a fixed rigid-body and colliders with no rigid-body attached.
        const EXCLUDE_FIXED = 1 << 1;
        /// Exclude from the query any collider attached to a kinematic rigid-body.
        const EXCLUDE_KINEMATIC = 1 << 2;
        /// Exclude from the query any collider attached to a dynamic rigid-body.
        const EXCLUDE_DYNAMIC = 1 << 3;
        /// Exclude from the query any collider that is a sensor.
        const EXCLUDE_SENSORS = 1 << 4;
        /// Exclude from the query any collider that is not a sensor.
        const EXCLUDE_SOLIDS = 1 << 5;
        /// Excludes all colliders not attached to a dynamic rigid-body.
        const ONLY_DYNAMIC = Self::EXCLUDE_FIXED.bits() | Self::EXCLUDE_KINEMATIC.bits();
        /// Excludes all colliders not attached to a kinematic rigid-body.
        const ONLY_KINEMATIC = Self::EXCLUDE_DYNAMIC.bits() | Self::EXCLUDE_FIXED.bits();
        /// Exclude all colliders attached to a non-fixed rigid-body
        /// (this will not exclude colliders not attached to any rigid-body).
        const ONLY_FIXED = Self::EXCLUDE_DYNAMIC.bits() | Self::EXCLUDE_KINEMATIC.bits();
    }
}

/// The status of the time-of-impact computation algorithm.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TOIStatus {
    /// The TOI algorithm ran out of iterations before achieving convergence.
    ///
    /// The content of the `TOI` will still be a conservative approximation of the actual result so
    /// it is often fine to interpret this case as a success.
    OutOfIterations,
    /// The TOI algorithm converged successfully.
    Converged,
    /// Something went wrong during the TOI computation, likely due to numerical instabilities.
    ///
    /// The content of the `TOI` will still be a conservative approximation of the actual result so
    /// it is often fine to interpret this case as a success.
    Failed,
    /// The two shape already overlap at the time 0.
    ///
    /// The witness points and normals provided by the `TOI` will have undefined values.
    Penetrating,
}

impl From<rapier3d::parry::query::ShapeCastStatus> for TOIStatus {
    fn from(value: rapier3d::parry::query::ShapeCastStatus) -> Self {
        match value {
            rapier3d::parry::query::ShapeCastStatus::OutOfIterations => Self::OutOfIterations,
            rapier3d::parry::query::ShapeCastStatus::Converged => Self::Converged,
            rapier3d::parry::query::ShapeCastStatus::Failed => Self::Failed,
            rapier3d::parry::query::ShapeCastStatus::PenetratingOrWithinTargetDist => {
                Self::Penetrating
            }
        }
    }
}

impl From<rapier2d::parry::query::ShapeCastStatus> for TOIStatus {
    fn from(value: rapier2d::parry::query::ShapeCastStatus) -> Self {
        match value {
            rapier2d::parry::query::ShapeCastStatus::OutOfIterations => Self::OutOfIterations,
            rapier2d::parry::query::ShapeCastStatus::Converged => Self::Converged,
            rapier2d::parry::query::ShapeCastStatus::Failed => Self::Failed,
            rapier2d::parry::query::ShapeCastStatus::PenetratingOrWithinTargetDist => {
                Self::Penetrating
            }
        }
    }
}

/// Possible collider shapes.
#[derive(Clone, Debug, PartialEq, Visit, Reflect, AsRefStr, EnumString, VariantNames)]
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
    /// See [`ConvexPolyhedronShape`] docs.
    Polyhedron(ConvexPolyhedronShape),
}

uuid_provider!(ColliderShape = "2e627337-71ea-4b33-a5f1-be697f705a86");

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
#[derive(Reflect, Visit, Debug)]
pub struct Collider {
    base: Base,

    #[reflect(setter = "set_shape")]
    pub(crate) shape: InheritableVariable<ColliderShape>,

    #[reflect(min_value = 0.0, step = 0.05, setter = "set_friction")]
    pub(crate) friction: InheritableVariable<f32>,

    #[reflect(setter = "set_density")]
    pub(crate) density: InheritableVariable<Option<f32>>,

    #[reflect(min_value = 0.0, step = 0.05, setter = "set_restitution")]
    pub(crate) restitution: InheritableVariable<f32>,

    #[reflect(setter = "set_is_sensor")]
    pub(crate) is_sensor: InheritableVariable<bool>,

    #[reflect(setter = "set_collision_groups")]
    pub(crate) collision_groups: InheritableVariable<InteractionGroups>,

    #[reflect(setter = "set_solver_groups")]
    pub(crate) solver_groups: InheritableVariable<InteractionGroups>,

    #[reflect(setter = "set_friction_combine_rule")]
    pub(crate) friction_combine_rule: InheritableVariable<CoefficientCombineRule>,

    #[reflect(setter = "set_restitution_combine_rule")]
    pub(crate) restitution_combine_rule: InheritableVariable<CoefficientCombineRule>,

    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) native: Cell<ColliderHandle>,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            base: Default::default(),
            shape: Default::default(),
            friction: InheritableVariable::new_modified(0.0),
            density: InheritableVariable::new_modified(None),
            restitution: InheritableVariable::new_modified(0.0),
            is_sensor: InheritableVariable::new_modified(false),
            collision_groups: Default::default(),
            solver_groups: Default::default(),
            friction_combine_rule: Default::default(),
            restitution_combine_rule: Default::default(),
            native: Cell::new(ColliderHandle::invalid()),
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

impl Clone for Collider {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            shape: self.shape.clone(),
            friction: self.friction.clone(),
            density: self.density.clone(),
            restitution: self.restitution.clone(),
            is_sensor: self.is_sensor.clone(),
            collision_groups: self.collision_groups.clone(),
            solver_groups: self.solver_groups.clone(),
            friction_combine_rule: self.friction_combine_rule.clone(),
            restitution_combine_rule: self.restitution_combine_rule.clone(),
            // Do not copy. The copy will have its own native representation (for example - Rapier's collider)
            native: Cell::new(ColliderHandle::invalid()),
        }
    }
}

impl TypeUuidProvider for Collider {
    fn type_uuid() -> Uuid {
        uuid!("bfaa2e82-9c19-4b99-983b-3bc115744a1d")
    }
}

impl Collider {
    /// Sets the new shape to the collider.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_shape(&mut self, shape: ColliderShape) -> ColliderShape {
        self.shape.set_value_and_mark_modified(shape)
    }

    /// Returns shared reference to the collider shape.
    pub fn shape(&self) -> &ColliderShape {
        &self.shape
    }

    /// Returns a copy of the collider shape.
    pub fn shape_value(&self) -> ColliderShape {
        (*self.shape).clone()
    }

    /// Returns mutable reference to the current collider shape.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn shape_mut(&mut self) -> &mut ColliderShape {
        self.shape.get_value_mut_and_mark_modified()
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
    pub fn set_restitution(&mut self, restitution: f32) -> f32 {
        self.restitution.set_value_and_mark_modified(restitution)
    }

    /// Returns current restitution value of the collider.
    pub fn restitution(&self) -> f32 {
        *self.restitution
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
    pub fn set_density(&mut self, density: Option<f32>) -> Option<f32> {
        self.density.set_value_and_mark_modified(density)
    }

    /// Returns current density of the collider.
    pub fn density(&self) -> Option<f32> {
        *self.density
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
    pub fn set_friction(&mut self, friction: f32) -> f32 {
        self.friction.set_value_and_mark_modified(friction)
    }

    /// Return current friction of the collider.
    pub fn friction(&self) -> f32 {
        *self.friction
    }

    /// Sets the new collision filtering options. See [`InteractionGroups`] docs for more info.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_collision_groups(&mut self, groups: InteractionGroups) -> InteractionGroups {
        self.collision_groups.set_value_and_mark_modified(groups)
    }

    /// Returns current collision filtering options.
    pub fn collision_groups(&self) -> InteractionGroups {
        *self.collision_groups
    }

    /// Sets the new joint solver filtering options. See [`InteractionGroups`] docs for more info.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_solver_groups(&mut self, groups: InteractionGroups) -> InteractionGroups {
        self.solver_groups.set_value_and_mark_modified(groups)
    }

    /// Returns current solver groups.
    pub fn solver_groups(&self) -> InteractionGroups {
        *self.solver_groups
    }

    /// If true is passed, the method makes collider a sensor. Sensors will not participate in
    /// collision response, but it is still possible to query contact information from them.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_is_sensor(&mut self, is_sensor: bool) -> bool {
        self.is_sensor.set_value_and_mark_modified(is_sensor)
    }

    /// Returns true if the collider is sensor, false - otherwise.
    pub fn is_sensor(&self) -> bool {
        *self.is_sensor
    }

    /// Sets the new friction combine rule. See [`CoefficientCombineRule`] docs for more info.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_friction_combine_rule(
        &mut self,
        rule: CoefficientCombineRule,
    ) -> CoefficientCombineRule {
        self.friction_combine_rule.set_value_and_mark_modified(rule)
    }

    /// Returns current friction combine rule of the collider.
    pub fn friction_combine_rule(&self) -> CoefficientCombineRule {
        *self.friction_combine_rule
    }

    /// Sets the new restitution combine rule. See [`CoefficientCombineRule`] docs for more info.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_restitution_combine_rule(
        &mut self,
        rule: CoefficientCombineRule,
    ) -> CoefficientCombineRule {
        self.restitution_combine_rule
            .set_value_and_mark_modified(rule)
    }

    /// Returns current restitution combine rule of the collider.
    pub fn restitution_combine_rule(&self) -> CoefficientCombineRule {
        *self.restitution_combine_rule
    }

    /// Returns an iterator that yields contact information for the collider.
    /// Contacts checks between two regular colliders
    pub fn contacts<'a>(
        &self,
        physics: &'a PhysicsWorld,
    ) -> impl Iterator<Item = ContactPair> + 'a {
        physics.contacts_with(self.native.get())
    }

    /// Returns an iterator that yields intersection information for the collider.
    /// Intersections checks between regular colliders and sensor colliders
    pub fn intersects<'a>(
        &self,
        physics: &'a PhysicsWorld,
    ) -> impl Iterator<Item = IntersectionPair> + 'a {
        physics.intersections_with(self.native.get())
    }

    pub(crate) fn needs_sync_model(&self) -> bool {
        self.shape.need_sync()
            || self.friction.need_sync()
            || self.density.need_sync()
            || self.restitution.need_sync()
            || self.is_sensor.need_sync()
            || self.collision_groups.need_sync()
            || self.solver_groups.need_sync()
            || self.friction_combine_rule.need_sync()
            || self.restitution_combine_rule.need_sync()
    }
}

impl NodeTrait for Collider {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn on_removed_from_graph(&mut self, graph: &mut Graph) {
        graph.physics.remove_collider(self.native.get());
        self.native.set(ColliderHandle::invalid());

        Log::info(format!(
            "Native collider was removed for node: {}",
            self.name()
        ));
    }

    fn on_unlink(&mut self, graph: &mut Graph) {
        if graph.physics.remove_collider(self.native.get()) {
            // Remove native collider when detaching a collider node from rigid body node.
            self.native.set(ColliderHandle::invalid());
        }
    }

    fn sync_native(&self, self_handle: Handle<Node>, context: &mut SyncContext) {
        context
            .physics
            .sync_to_collider_node(context.nodes, self_handle, self);
    }

    fn validate(&self, scene: &Scene) -> Result<(), String> {
        if scene
            .graph
            .try_get(self.parent())
            .and_then(|p| p.query_component_ref::<RigidBody>())
            .is_none()
        {
            Err(
                "3D Collider must be a direct child of a 3D Rigid Body node, \
            otherwise it will not have any effect!"
                    .to_string(),
            )
        } else {
            Ok(())
        }
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
    pub fn build_collider(self) -> Collider {
        Collider {
            base: self.base_builder.build_base(),
            shape: self.shape.into(),
            friction: self.friction.into(),
            density: self.density.into(),
            restitution: self.restitution.into(),
            is_sensor: self.is_sensor.into(),
            collision_groups: self.collision_groups.into(),
            solver_groups: self.solver_groups.into(),
            friction_combine_rule: self.friction_combine_rule.into(),
            restitution_combine_rule: self.restitution_combine_rule.into(),
            native: Cell::new(ColliderHandle::invalid()),
        }
    }

    /// Creates collider node, but does not add it to a graph.
    pub fn build_node(self) -> Node {
        Node::new(self.build_collider())
    }

    /// Creates collider node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::core::algebra::Vector2;
    use crate::scene::{
        base::BaseBuilder,
        collider::{ColliderBuilder, ColliderShape},
        graph::Graph,
        rigidbody::{RigidBodyBuilder, RigidBodyType},
    };

    #[test]
    fn test_collider_intersect() {
        let mut graph = Graph::new();

        let mut create_rigid_body = |is_sensor| {
            let cube_half_size = 0.5;
            let collider_sensor = ColliderBuilder::new(BaseBuilder::new())
                .with_shape(ColliderShape::cuboid(
                    cube_half_size,
                    cube_half_size,
                    cube_half_size,
                ))
                .with_sensor(is_sensor)
                .build(&mut graph);

            RigidBodyBuilder::new(BaseBuilder::new().with_children(&[collider_sensor]))
                .with_body_type(RigidBodyType::Static)
                .build(&mut graph);

            collider_sensor
        };

        let collider_sensor = create_rigid_body(true);
        let collider_non_sensor = create_rigid_body(false);

        // need to call two times for the physics engine to execute
        graph.update(Vector2::new(800.0, 600.0), 1.0, Default::default());
        graph.update(Vector2::new(800.0, 600.0), 1.0, Default::default());

        // we don't expect contact between regular body and sensor
        assert_eq!(
            0,
            graph[collider_sensor]
                .as_collider()
                .contacts(&graph.physics)
                .count()
        );
        assert_eq!(
            0,
            graph[collider_non_sensor]
                .as_collider()
                .contacts(&graph.physics)
                .count()
        );

        // we expect intersection between regular body and sensor
        assert_eq!(
            1,
            graph[collider_sensor]
                .as_collider()
                .intersects(&graph.physics)
                .count()
        );
        assert_eq!(
            1,
            graph[collider_non_sensor]
                .as_collider()
                .intersects(&graph.physics)
                .count()
        );
    }
}
