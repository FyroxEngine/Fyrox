//! Collider is a geometric entity that can be attached to a rigid body to allow participate it
//! participate in contact generation, collision response and proximity queries.

use crate::scene::node::{NodeTrait, SyncContext};
use crate::{
    core::{
        algebra::Vector3,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    impl_directly_inheritable_entity_trait,
    scene::{
        base::{Base, BaseBuilder},
        graph::{
            physics::{CoefficientCombineRule, ContactPair, PhysicsWorld},
            Graph,
        },
        node::Node,
        variable::{InheritError, TemplateVariable},
        DirectlyInheritableEntity,
    },
    utils::log::Log,
};
use fxhash::FxHashMap;
use fyrox_core::math::aabb::AxisAlignedBoundingBox;
use fyrox_core::uuid::Uuid;
use rapier3d::geometry::{self, ColliderHandle};
use std::str::FromStr;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// Ball is an idea sphere shape defined by a single parameters - its radius.
#[derive(Clone, Debug, PartialEq, Visit, Inspect)]
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
#[derive(Clone, Debug, Visit, Inspect, PartialEq)]
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
#[derive(Clone, Debug, Visit, Inspect, PartialEq)]
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
#[derive(Clone, Debug, Visit, Inspect, PartialEq)]
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
#[derive(Clone, Debug, Visit, Inspect, PartialEq)]
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
#[derive(Clone, Debug, Visit, Inspect, PartialEq)]
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
#[derive(Clone, Debug, Visit, Inspect, PartialEq)]
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
#[derive(Default, Clone, Debug, Visit, Inspect, PartialEq)]
pub struct TrimeshShape {
    /// Geometry sources for the shape.
    pub sources: Vec<GeometrySource>,
}

/// Arbitrary height field shape.
#[derive(Default, Clone, Debug, Visit, Inspect, PartialEq)]
pub struct HeightfieldShape {
    /// A handle to terrain scene node.
    pub geometry_source: GeometrySource,
}

/// Arbitrary convex polyhedron shape.
#[derive(Default, Clone, Debug, Visit, Inspect, PartialEq)]
pub struct ConvexPolyhedronShape {
    /// A handle to a mesh node.
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
#[derive(Visit, Debug, Clone, Copy, PartialEq, Inspect)]
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
            ColliderShape::Polyhedron(v) => v.properties(),
        }
    }
}

/// Possible collider shapes.
#[derive(Clone, Debug, PartialEq, Visit, AsRefStr, EnumString, EnumVariantNames)]
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

    #[inspect(getter = "Deref::deref")]
    pub(crate) shape: TemplateVariable<ColliderShape>,

    #[inspect(min_value = 0.0, step = 0.05, getter = "Deref::deref")]
    pub(crate) friction: TemplateVariable<f32>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) density: TemplateVariable<Option<f32>>,

    #[inspect(min_value = 0.0, step = 0.05, getter = "Deref::deref")]
    pub(crate) restitution: TemplateVariable<f32>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) is_sensor: TemplateVariable<bool>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) collision_groups: TemplateVariable<InteractionGroups>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) solver_groups: TemplateVariable<InteractionGroups>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) friction_combine_rule: TemplateVariable<CoefficientCombineRule>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) restitution_combine_rule: TemplateVariable<CoefficientCombineRule>,

    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) native: Cell<ColliderHandle>,
}

impl_directly_inheritable_entity_trait!(Collider;
    shape,
    friction,
    density,
    restitution,
    is_sensor,
    collision_groups,
    solver_groups,
    friction_combine_rule,
    restitution_combine_rule
);

impl Default for Collider {
    fn default() -> Self {
        Self {
            base: Default::default(),
            shape: Default::default(),
            friction: TemplateVariable::new(0.0),
            density: TemplateVariable::new(None),
            restitution: TemplateVariable::new(0.0),
            is_sensor: TemplateVariable::new(false),
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
            // Do not copy.
            native: Cell::new(ColliderHandle::invalid()),
        }
    }
}

impl Collider {
    pub fn type_uuid() -> Uuid {
        Uuid::from_str("bfaa2e82-9c19-4b99-983b-3bc115744a1d").unwrap()
    }

    /// Sets the new shape to the collider.
    ///
    /// # Performance
    ///
    /// This is relatively expensive operation - it forces the physics engine to recalculate contacts,
    /// perform collision response, etc. Try avoid calling this method each frame for better
    /// performance.
    pub fn set_shape(&mut self, shape: ColliderShape) {
        self.shape.set(shape);
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
        self.shape.get_mut()
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
        self.restitution.set(restitution);
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
    pub fn set_density(&mut self, density: Option<f32>) {
        self.density.set(density);
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
    pub fn set_friction(&mut self, friction: f32) {
        self.friction.set(friction);
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
    pub fn set_collision_groups(&mut self, groups: InteractionGroups) {
        self.collision_groups.set(groups);
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
    pub fn set_solver_groups(&mut self, groups: InteractionGroups) {
        self.solver_groups.set(groups);
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
    pub fn set_is_sensor(&mut self, is_sensor: bool) {
        self.is_sensor.set(is_sensor);
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
    pub fn set_friction_combine_rule(&mut self, rule: CoefficientCombineRule) {
        self.friction_combine_rule.set(rule);
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
    pub fn set_restitution_combine_rule(&mut self, rule: CoefficientCombineRule) {
        self.restitution_combine_rule.set(rule);
    }

    /// Returns current restitution combine rule of the collider.
    pub fn restitution_combine_rule(&self) -> CoefficientCombineRule {
        *self.restitution_combine_rule
    }

    /// Returns an iterator that yields contact information for the collider.
    pub fn contacts<'a>(
        &self,
        physics: &'a PhysicsWorld,
    ) -> impl Iterator<Item = ContactPair> + 'a {
        physics.contacts_with(self.native.get())
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
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    // Prefab inheritance resolving.
    fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        self.base.inherit_properties(parent)?;
        if let Some(parent) = parent.cast::<Self>() {
            self.try_inherit_self_properties(parent)?;
        }
        Ok(())
    }

    fn reset_inheritable_properties(&mut self) {
        self.base.reset_inheritable_properties();
        self.reset_self_inheritable_properties();
    }

    fn restore_resources(&mut self, _resource_manager: ResourceManager) {}

    fn remap_handles(&mut self, old_new_mapping: &FxHashMap<Handle<Node>, Handle<Node>>) {
        self.base.remap_handles(old_new_mapping);

        match self.shape.get_mut_silent() {
            ColliderShape::Trimesh(ref mut trimesh) => {
                for source in trimesh.sources.iter_mut() {
                    if let Some(entry) = old_new_mapping.get(&source.0) {
                        source.0 = *entry;
                    } else {
                        Log::warn(format!(
                            "Unable to remap geometry source of a Trimesh collider {} shape. Handle is {}!",
                            *self.base.name,
                            source.0
                        ))
                    }
                }
            }
            ColliderShape::Heightfield(ref mut heightfield) => {
                if let Some(entry) = old_new_mapping.get(&heightfield.geometry_source.0) {
                    heightfield.geometry_source.0 = *entry;
                } else {
                    Log::warn(format!(
                        "Unable to remap geometry source of a Height Field collider {} shape. Handle is {}!",
                        *self.base.name,
                        heightfield.geometry_source.0
                    ))
                }
            }
            _ => (),
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn clean_up(&mut self, graph: &mut Graph) {
        graph.physics.remove_collider(self.native.get());

        Log::info(format!(
            "Native collider was removed for node: {}",
            self.name()
        ));
    }

    fn sync_native(&self, self_handle: Handle<Node>, context: &mut SyncContext) {
        context
            .physics
            .sync_to_collider_node(context.nodes, self_handle, self);
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
    use crate::scene::node::NodeTrait;
    use crate::scene::{
        base::{test::check_inheritable_properties_equality, BaseBuilder},
        collider::{ColliderBuilder, ColliderShape, InteractionGroups},
        graph::physics::CoefficientCombineRule,
        node::Node,
    };

    #[test]
    fn test_collider_inheritance() {
        let parent = ColliderBuilder::new(BaseBuilder::new())
            .with_shape(ColliderShape::ball(1.0))
            .with_friction(1.0)
            .with_restitution(1.0)
            .with_density(Some(2.0))
            .with_sensor(true)
            .with_restitution_combine_rule(CoefficientCombineRule::Max)
            .with_friction_combine_rule(CoefficientCombineRule::Max)
            .with_collision_groups(InteractionGroups::new(1, 2))
            .with_solver_groups(InteractionGroups::new(1, 2))
            .build_node();

        let mut child = ColliderBuilder::new(BaseBuilder::new()).build_collider();

        child.inherit(&parent).unwrap();

        if let Node::Collider(parent) = parent {
            check_inheritable_properties_equality(&child.base, &parent.base);
            check_inheritable_properties_equality(&child, &parent);
        } else {
            unreachable!();
        }
    }
}
