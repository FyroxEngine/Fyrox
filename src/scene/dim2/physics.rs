//! Scene physics module.

use crate::scene::dim2::joint::JointLocalFrames;
use crate::{
    core::{
        algebra::{
            Isometry2, Isometry3, Matrix4, Point2, Rotation3, Translation2, Translation3,
            UnitComplex, UnitQuaternion, Vector2, Vector3,
        },
        arrayvec::ArrayVec,
        instant,
        log::{Log, MessageKind},
        math::Matrix4Ext,
        parking_lot::Mutex,
        pool::Handle,
        reflect::prelude::*,
        variable::VariableFlags,
        visitor::prelude::*,
        BiDirHashMap,
    },
    scene::{
        self,
        collider::{self},
        debug::SceneDrawingContext,
        dim2::{self, collider::ColliderShape, joint::JointParams, rigidbody::ApplyAction},
        graph::{
            physics::{FeatureId, IntegrationParameters, PhysicsPerformanceStatistics},
            NodePool,
        },
        node::{Node, NodeTrait},
    },
};
use fyrox_core::variable::InheritableVariable;
use rapier2d::{
    dynamics::{
        CCDSolver, GenericJoint, GenericJointBuilder, ImpulseJointHandle, ImpulseJointSet,
        IslandManager, JointAxesMask, JointAxis, MultibodyJointHandle, MultibodyJointSet,
        RigidBody, RigidBodyActivation, RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
        RigidBodyType,
    },
    geometry::{
        BroadPhase, Collider, ColliderBuilder, ColliderHandle, ColliderSet, Cuboid,
        InteractionGroups, NarrowPhase, Ray, SharedShape,
    },
    pipeline::{DebugRenderPipeline, EventHandler, PhysicsPipeline, QueryFilter, QueryPipeline},
};
use std::num::NonZeroUsize;
use std::{
    cell::RefCell,
    cmp::Ordering,
    fmt::{Debug, Formatter},
    hash::Hash,
    sync::Arc,
};

/// A trait for ray cast results storage. It has two implementations: Vec and ArrayVec.
/// Latter is needed for the cases where you need to avoid runtime memory allocations
/// and do everything on stack.
pub trait QueryResultsStorage {
    /// Pushes new intersection in the storage. Returns true if intersection was
    /// successfully inserted, false otherwise.
    fn push(&mut self, intersection: Intersection) -> bool;

    /// Clears the storage.
    fn clear(&mut self);

    /// Sorts intersections by given compare function.
    fn sort_intersections_by<C: FnMut(&Intersection, &Intersection) -> Ordering>(&mut self, cmp: C);
}

impl QueryResultsStorage for Vec<Intersection> {
    fn push(&mut self, intersection: Intersection) -> bool {
        self.push(intersection);
        true
    }

    fn clear(&mut self) {
        self.clear()
    }

    fn sort_intersections_by<C>(&mut self, cmp: C)
    where
        C: FnMut(&Intersection, &Intersection) -> Ordering,
    {
        self.sort_by(cmp);
    }
}

impl<const CAP: usize> QueryResultsStorage for ArrayVec<Intersection, CAP> {
    fn push(&mut self, intersection: Intersection) -> bool {
        self.try_push(intersection).is_ok()
    }

    fn clear(&mut self) {
        self.clear()
    }

    fn sort_intersections_by<C>(&mut self, cmp: C)
    where
        C: FnMut(&Intersection, &Intersection) -> Ordering,
    {
        self.sort_by(cmp);
    }
}

/// A ray intersection result.
#[derive(Debug, Clone, PartialEq)]
pub struct Intersection {
    /// A handle of the collider with which intersection was detected.
    pub collider: Handle<Node>,

    /// A normal at the intersection position.
    pub normal: Vector2<f32>,

    /// A position of the intersection in world coordinates.
    pub position: Point2<f32>,

    /// Additional data that contains a kind of the feature with which
    /// intersection was detected as well as its index.
    ///
    /// # Important notes.
    ///
    /// FeatureId::Face might have index that is greater than amount of triangles in
    /// a triangle mesh, this means that intersection was detected from "back" side of
    /// a face. To "fix" that index, simply subtract amount of triangles of a triangle
    /// mesh from the value.
    pub feature: FeatureId,

    /// Distance from the ray origin.
    pub toi: f32,
}

/// A set of options for the ray cast.
pub struct RayCastOptions {
    /// A ray origin.
    pub ray_origin: Point2<f32>,

    /// A ray direction. Can be non-normalized.
    pub ray_direction: Vector2<f32>,

    /// Maximum distance of cast.
    pub max_len: f32,

    /// Groups to check.
    pub groups: collider::InteractionGroups,

    /// Whether to sort intersections from closest to farthest.
    pub sort_results: bool,
}

/// Data of the contact.
#[derive(Debug, Clone, PartialEq)]
pub struct ContactData {
    /// The contact point in the local-space of the first shape.
    pub local_p1: Vector2<f32>,
    /// The contact point in the local-space of the second shape.
    pub local_p2: Vector2<f32>,
    /// The distance between the two contact points.
    pub dist: f32,
    /// The impulse, along the contact normal, applied by this contact to the first collider's rigid-body.
    /// The impulse applied to the second collider's rigid-body is given by `-impulse`.
    pub impulse: f32,
    /// The friction impulses along the basis orthonormal to the contact normal, applied to the first
    /// collider's rigid-body.
    pub tangent_impulse: f32,
}

/// A contact manifold between two colliders.
#[derive(Debug, Clone, PartialEq)]
pub struct ContactManifold {
    /// The contacts points.
    pub points: Vec<ContactData>,
    /// The contact normal of all the contacts of this manifold, expressed in the local space of the first shape.
    pub local_n1: Vector2<f32>,
    /// The contact normal of all the contacts of this manifold, expressed in the local space of the second shape.
    pub local_n2: Vector2<f32>,
    /// The first rigid-body involved in this contact manifold.
    pub rigid_body1: Handle<Node>,
    /// The second rigid-body involved in this contact manifold.
    pub rigid_body2: Handle<Node>,
    /// The world-space contact normal shared by all the contact in this contact manifold.
    pub normal: Vector2<f32>,
}

/// Contact info for pair of colliders.
#[derive(Debug, Clone, PartialEq)]
pub struct ContactPair {
    /// The first collider involved in the contact pair.
    pub collider1: Handle<Node>,
    /// The second collider involved in the contact pair.
    pub collider2: Handle<Node>,
    /// The set of contact manifolds between the two colliders.
    /// All contact manifold contain themselves contact points between the colliders.
    pub manifolds: Vec<ContactManifold>,
    /// Is there any active contact in this contact pair?
    pub has_any_active_contact: bool,
}

impl ContactPair {
    fn from_native(c: &rapier2d::geometry::ContactPair, physics: &PhysicsWorld) -> Option<Self> {
        Some(ContactPair {
            collider1: Handle::decode_from_u128(physics.colliders.get(c.collider1)?.user_data),
            collider2: Handle::decode_from_u128(physics.colliders.get(c.collider2)?.user_data),
            manifolds: c
                .manifolds
                .iter()
                .filter_map(|m| {
                    Some(ContactManifold {
                        points: m
                            .points
                            .iter()
                            .map(|p| ContactData {
                                local_p1: p.local_p1.coords,
                                local_p2: p.local_p2.coords,
                                dist: p.dist,
                                impulse: p.data.impulse,
                                tangent_impulse: p.data.tangent_impulse,
                            })
                            .collect(),
                        local_n1: m.local_n1,
                        local_n2: m.local_n2,
                        rigid_body1: m.data.rigid_body1.and_then(|h| {
                            physics
                                .bodies
                                .get(h)
                                .map(|b| Handle::decode_from_u128(b.user_data))
                        })?,
                        rigid_body2: m.data.rigid_body2.and_then(|h| {
                            physics
                                .bodies
                                .get(h)
                                .map(|b| Handle::decode_from_u128(b.user_data))
                        })?,
                        normal: m.data.normal,
                    })
                })
                .collect(),
            has_any_active_contact: c.has_any_active_contact,
        })
    }
}

/// Intersection info for pair of colliders.
#[derive(Debug, Clone, PartialEq)]
pub struct IntersectionPair {
    /// The first collider involved in the contact pair.
    pub collider1: Handle<Node>,
    /// The second collider involved in the contact pair.
    pub collider2: Handle<Node>,
    /// Is there any active contact in this contact pair?
    pub has_any_active_contact: bool,
}

pub(super) struct Container<S, A>
where
    A: Hash + Eq + Clone,
{
    set: S,
    map: BiDirHashMap<A, Handle<Node>>,
}

fn convert_joint_params(
    params: scene::dim2::joint::JointParams,
    local_frame1: Isometry2<f32>,
    local_frame2: Isometry2<f32>,
) -> GenericJoint {
    let locked_axis = match params {
        JointParams::BallJoint(_) => JointAxesMask::LOCKED_REVOLUTE_AXES,
        JointParams::FixedJoint(_) => JointAxesMask::LOCKED_FIXED_AXES,
        JointParams::PrismaticJoint(_) => JointAxesMask::LOCKED_PRISMATIC_AXES,
    };

    let mut joint = GenericJointBuilder::new(locked_axis)
        .local_frame1(local_frame1)
        .local_frame2(local_frame2)
        .build();

    match params {
        scene::dim2::joint::JointParams::BallJoint(v) => {
            if v.limits_enabled {
                joint.set_limits(
                    JointAxis::AngX,
                    [v.limits_angles.start, v.limits_angles.end],
                );
            }
        }
        scene::dim2::joint::JointParams::FixedJoint(_) => {}
        scene::dim2::joint::JointParams::PrismaticJoint(v) => {
            if v.limits_enabled {
                joint.set_limits(JointAxis::X, [v.limits.start, v.limits.end]);
            }
        }
    }

    joint
}

// Converts descriptor in a shared shape.
fn collider_shape_into_native_shape(shape: &ColliderShape) -> Option<SharedShape> {
    match shape {
        ColliderShape::Ball(ball) => Some(SharedShape::ball(ball.radius)),
        ColliderShape::Cuboid(cuboid) => {
            Some(SharedShape(Arc::new(Cuboid::new(cuboid.half_extents))))
        }
        ColliderShape::Capsule(capsule) => Some(SharedShape::capsule(
            Point2::from(capsule.begin),
            Point2::from(capsule.end),
            capsule.radius,
        )),
        ColliderShape::Segment(segment) => Some(SharedShape::segment(
            Point2::from(segment.begin),
            Point2::from(segment.end),
        )),
        ColliderShape::Triangle(triangle) => Some(SharedShape::triangle(
            Point2::from(triangle.a),
            Point2::from(triangle.b),
            Point2::from(triangle.c),
        )),
        ColliderShape::Trimesh(_) => {
            None // TODO
        }
        ColliderShape::Heightfield(_) => {
            None // TODO
        }
    }
}

fn isometry2_to_mat4(isometry: &Isometry2<f32>) -> Matrix4<f32> {
    Isometry3 {
        rotation: UnitQuaternion::from_euler_angles(0.0, 0.0, isometry.rotation.angle()),
        translation: Translation3 {
            vector: Vector3::new(isometry.translation.x, isometry.translation.y, 0.0),
        },
    }
    .to_homogeneous()
}

/// Physics world is responsible for physics simulation in the engine. There is a very few public
/// methods, mostly for ray casting. You should add physical entities using scene graph nodes, such
/// as RigidBody, Collider, Joint.
#[derive(Visit, Reflect)]
pub struct PhysicsWorld {
    /// A flag that defines whether physics simulation is enabled or not.
    pub enabled: InheritableVariable<bool>,

    /// A set of parameters that define behavior of every rigid body.
    pub integration_parameters: InheritableVariable<IntegrationParameters>,

    /// Current gravity vector. Default is (0.0, -9.81)
    pub gravity: InheritableVariable<Vector2<f32>>,

    /// Performance statistics of a single simulation step.
    #[visit(skip)]
    #[reflect(hidden)]
    pub performance_statistics: PhysicsPerformanceStatistics,

    // Current physics pipeline.
    #[visit(skip)]
    #[reflect(hidden)]
    pipeline: PhysicsPipeline,
    // Broad phase performs rough intersection checks.
    #[visit(skip)]
    #[reflect(hidden)]
    broad_phase: BroadPhase,
    // Narrow phase is responsible for precise contact generation.
    #[visit(skip)]
    #[reflect(hidden)]
    narrow_phase: NarrowPhase,
    // A continuous collision detection solver.
    #[visit(skip)]
    #[reflect(hidden)]
    ccd_solver: CCDSolver,
    // Structure responsible for maintaining the set of active rigid-bodies, and putting non-moving
    // rigid-bodies to sleep to save computation times.
    #[visit(skip)]
    #[reflect(hidden)]
    islands: IslandManager,
    // A container of rigid bodies.
    #[visit(skip)]
    #[reflect(hidden)]
    bodies: RigidBodySet,
    // A container of colliders.
    #[visit(skip)]
    #[reflect(hidden)]
    colliders: ColliderSet,
    // A container of impulse joints.
    #[visit(skip)]
    #[reflect(hidden)]
    joints: Container<ImpulseJointSet, ImpulseJointHandle>,
    // A container of multibody joints.
    #[visit(skip)]
    #[reflect(hidden)]
    multibody_joints: Container<MultibodyJointSet, MultibodyJointHandle>,
    // Event handler collects info about contacts and proximity events.
    #[visit(skip)]
    #[reflect(hidden)]
    event_handler: Box<dyn EventHandler>,
    #[visit(skip)]
    #[reflect(hidden)]
    query: RefCell<QueryPipeline>,
    #[visit(skip)]
    #[reflect(hidden)]
    debug_render_pipeline: Mutex<DebugRenderPipeline>,
}

fn isometry_from_global_transform(transform: &Matrix4<f32>) -> Isometry2<f32> {
    Isometry2 {
        translation: Translation2::new(transform[12], transform[13]),
        rotation: UnitComplex::from_angle(
            Rotation3::from_matrix_eps(&transform.basis(), f32::EPSILON, 16, Rotation3::identity())
                .euler_angles()
                .2,
        ),
    }
}

fn calculate_local_frames(
    joint: &dyn NodeTrait,
    body1: &dyn NodeTrait,
    body2: &dyn NodeTrait,
) -> (Isometry2<f32>, Isometry2<f32>) {
    let joint_isometry = isometry_from_global_transform(&joint.global_transform());

    (
        joint_isometry * isometry_from_global_transform(&body1.global_transform()).inverse(),
        joint_isometry * isometry_from_global_transform(&body2.global_transform()).inverse(),
    )
}

fn u32_to_group(v: u32) -> rapier2d::geometry::Group {
    rapier2d::geometry::Group::from_bits(v).unwrap_or_else(rapier2d::geometry::Group::all)
}

impl PhysicsWorld {
    /// Creates a new instance of the physics world.
    pub(crate) fn new() -> Self {
        Self {
            enabled: true.into(),
            pipeline: PhysicsPipeline::new(),
            gravity: Vector2::new(0.0, -9.81).into(),
            integration_parameters: IntegrationParameters::default().into(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            islands: IslandManager::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: Container {
                set: ImpulseJointSet::new(),
                map: Default::default(),
            },
            multibody_joints: Container {
                set: MultibodyJointSet::new(),
                map: Default::default(),
            },
            event_handler: Box::new(()),
            query: RefCell::new(Default::default()),
            performance_statistics: Default::default(),
            debug_render_pipeline: Default::default(),
        }
    }

    pub(crate) fn update(&mut self, dt: f32) {
        let time = instant::Instant::now();

        if *self.enabled {
            let integration_parameters = rapier2d::dynamics::IntegrationParameters {
                dt: self.integration_parameters.dt.unwrap_or(dt),
                min_ccd_dt: self.integration_parameters.min_ccd_dt,
                erp: self.integration_parameters.erp,
                damping_ratio: self.integration_parameters.damping_ratio,
                joint_erp: self.integration_parameters.joint_erp,
                joint_damping_ratio: self.integration_parameters.joint_damping_ratio,
                allowed_linear_error: self.integration_parameters.allowed_linear_error,
                max_penetration_correction: self.integration_parameters.max_penetration_correction,
                prediction_distance: self.integration_parameters.prediction_distance,
                num_solver_iterations: NonZeroUsize::new(
                    self.integration_parameters.num_solver_iterations,
                )
                .unwrap(),
                num_additional_friction_iterations: self
                    .integration_parameters
                    .num_additional_friction_iterations,
                num_internal_pgs_iterations: self
                    .integration_parameters
                    .num_internal_pgs_iterations,
                min_island_size: self.integration_parameters.min_island_size as usize,
                max_ccd_substeps: self.integration_parameters.max_ccd_substeps as usize,
            };

            self.pipeline.step(
                &self.gravity,
                &integration_parameters,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.joints.set,
                &mut self.multibody_joints.set,
                &mut self.ccd_solver,
                // In Rapier 0.17 passing query pipeline here sometimes causing panic in numeric overflow,
                // so we keep updating it manually.
                None,
                &(),
                &*self.event_handler,
            );
        }

        self.performance_statistics.step_time += instant::Instant::now() - time;
    }

    pub(crate) fn add_body(&mut self, owner: Handle<Node>, mut body: RigidBody) -> RigidBodyHandle {
        body.user_data = owner.encode_to_u128();
        self.bodies.insert(body)
    }

    pub(crate) fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.joints.set,
            &mut self.multibody_joints.set,
            true,
        );
    }

    pub(crate) fn add_collider(
        &mut self,
        owner: Handle<Node>,
        parent_body: RigidBodyHandle,
        mut collider: Collider,
    ) -> ColliderHandle {
        collider.user_data = owner.encode_to_u128();
        self.colliders
            .insert_with_parent(collider, parent_body, &mut self.bodies)
    }

    pub(crate) fn remove_collider(&mut self, handle: ColliderHandle) -> bool {
        self.colliders
            .remove(handle, &mut self.islands, &mut self.bodies, false)
            .is_some()
    }

    pub(crate) fn add_joint(
        &mut self,
        owner: Handle<Node>,
        body1: RigidBodyHandle,
        body2: RigidBodyHandle,
        params: GenericJoint,
    ) -> ImpulseJointHandle {
        let handle = self.joints.set.insert(body1, body2, params, false);
        self.joints.map.insert(handle, owner);
        handle
    }

    pub(crate) fn remove_joint(&mut self, handle: ImpulseJointHandle) {
        if self.joints.set.remove(handle, false).is_some() {
            assert!(self.joints.map.remove_by_key(&handle).is_some());
        }
    }

    /// Draws physics world. Very useful for debugging, it allows you to see where are
    /// rigid bodies, which colliders they have and so on.
    pub fn draw(&self, context: &mut SceneDrawingContext) {
        self.debug_render_pipeline.lock().render(
            context,
            &self.bodies,
            &self.colliders,
            &self.joints.set,
            &self.multibody_joints.set,
            &self.narrow_phase,
        );
    }

    /// Casts a ray with given options.
    pub fn cast_ray<S: QueryResultsStorage>(&self, opts: RayCastOptions, query_buffer: &mut S) {
        let time = instant::Instant::now();

        let mut query = self.query.borrow_mut();

        // TODO: Ideally this must be called once per frame, but it seems to be impossible because
        // a body can be deleted during the consecutive calls of this method which will most
        // likely end up in panic because of invalid handle stored in internal acceleration
        // structure. This could be fixed by delaying deleting of bodies/collider to the end
        // of the frame.
        query.update(&self.bodies, &self.colliders);

        query_buffer.clear();
        let ray = Ray::new(
            opts.ray_origin,
            opts.ray_direction
                .try_normalize(f32::EPSILON)
                .unwrap_or_default(),
        );
        query.intersections_with_ray(
            &self.bodies,
            &self.colliders,
            &ray,
            opts.max_len,
            true,
            QueryFilter::new().groups(InteractionGroups::new(
                u32_to_group(opts.groups.memberships.0),
                u32_to_group(opts.groups.filter.0),
            )),
            |handle, intersection| {
                query_buffer.push(Intersection {
                    collider: Handle::decode_from_u128(
                        self.colliders.get(handle).unwrap().user_data,
                    ),
                    normal: intersection.normal,
                    position: ray.point_at(intersection.toi),
                    feature: intersection.feature.into(),
                    toi: intersection.toi,
                })
            },
        );
        if opts.sort_results {
            query_buffer.sort_intersections_by(|a, b| {
                if a.toi > b.toi {
                    Ordering::Greater
                } else if a.toi < b.toi {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
        }

        self.performance_statistics.total_ray_cast_time.set(
            self.performance_statistics.total_ray_cast_time.get()
                + (instant::Instant::now() - time),
        );
    }

    pub(crate) fn set_rigid_body_position(
        &mut self,
        rigid_body: &scene::dim2::rigidbody::RigidBody,
        new_global_transform: &Matrix4<f32>,
    ) {
        if let Some(native) = self.bodies.get_mut(rigid_body.native.get()) {
            native.set_position(
                isometry_from_global_transform(new_global_transform),
                // Do not wake up body, it is too expensive and must be done **only** by explicit
                // `wake_up` call!
                false,
            );
        }
    }

    pub(crate) fn sync_rigid_body_node(
        &mut self,
        rigid_body: &mut scene::dim2::rigidbody::RigidBody,
        parent_transform: Matrix4<f32>,
    ) {
        if *self.enabled {
            if let Some(native) = self.bodies.get(rigid_body.native.get()) {
                if native.body_type() == RigidBodyType::Dynamic {
                    let local_transform: Matrix4<f32> = parent_transform
                        .try_inverse()
                        .unwrap_or_else(Matrix4::identity)
                        * isometry2_to_mat4(native.position());

                    let local_rotation = UnitQuaternion::from_matrix_eps(
                        &local_transform.basis(),
                        f32::EPSILON,
                        16,
                        UnitQuaternion::identity(),
                    );
                    let local_position =
                        Vector3::new(local_transform[12], local_transform[13], 0.0);

                    rigid_body
                        .local_transform
                        .set_position(local_position)
                        .set_rotation(local_rotation);

                    rigid_body
                        .lin_vel
                        .set_value_with_flags(*native.linvel(), VariableFlags::MODIFIED);
                    rigid_body
                        .ang_vel
                        .set_value_with_flags(native.angvel(), VariableFlags::MODIFIED);
                    rigid_body.sleeping = native.is_sleeping();
                }
            }
        }
    }

    pub(crate) fn sync_to_rigid_body_node(
        &mut self,
        handle: Handle<Node>,
        rigid_body_node: &scene::dim2::rigidbody::RigidBody,
    ) {
        if !rigid_body_node.is_globally_enabled() {
            self.remove_body(rigid_body_node.native.get());
            rigid_body_node.native.set(Default::default());
            return;
        }

        // Important notes!
        // 1) `get_mut` is **very** expensive because it forces physics engine to recalculate contacts
        //    and a lot of other stuff, this is why we need `anything_changed` flag.
        if rigid_body_node.native.get() != RigidBodyHandle::invalid() {
            let mut actions = rigid_body_node.actions.lock();
            if rigid_body_node.need_sync_model() || !actions.is_empty() {
                if let Some(native) = self.bodies.get_mut(rigid_body_node.native.get()) {
                    // Sync native rigid body's properties with scene node's in case if they
                    // were changed by user.
                    rigid_body_node
                        .body_type
                        .try_sync_model(|v| native.set_body_type(v.into(), false));
                    rigid_body_node
                        .lin_vel
                        .try_sync_model(|v| native.set_linvel(v, false));
                    rigid_body_node
                        .ang_vel
                        .try_sync_model(|v| native.set_angvel(v, false));
                    rigid_body_node.mass.try_sync_model(|v| {
                        native.set_additional_mass(v, true);
                    });
                    rigid_body_node
                        .lin_damping
                        .try_sync_model(|v| native.set_linear_damping(v));
                    rigid_body_node
                        .ang_damping
                        .try_sync_model(|v| native.set_angular_damping(v));
                    rigid_body_node
                        .ccd_enabled
                        .try_sync_model(|v| native.enable_ccd(v));
                    rigid_body_node.can_sleep.try_sync_model(|v| {
                        let activation = native.activation_mut();
                        if v {
                            activation.linear_threshold =
                                RigidBodyActivation::default_linear_threshold();
                            activation.angular_threshold =
                                RigidBodyActivation::default_angular_threshold();
                        } else {
                            activation.sleeping = false;
                            activation.linear_threshold = -1.0;
                            activation.angular_threshold = -1.0;
                        };
                    });
                    rigid_body_node
                        .translation_locked
                        .try_sync_model(|v| native.lock_translations(v, false));
                    rigid_body_node.rotation_locked.try_sync_model(|v| {
                        native.set_enabled_rotations(!v, !v, !v, false);
                    });
                    rigid_body_node
                        .dominance
                        .try_sync_model(|v| native.set_dominance_group(v));
                    rigid_body_node
                        .gravity_scale
                        .try_sync_model(|v| native.set_gravity_scale(v, false));

                    // We must reset any forces applied at previous update step, otherwise physics engine
                    // will keep pushing the rigid body infinitely.
                    if rigid_body_node.reset_forces.replace(false) {
                        native.reset_forces(false);
                        native.reset_torques(false);
                    }

                    while let Some(action) = actions.pop_front() {
                        match action {
                            ApplyAction::Force(force) => {
                                native.add_force(force, false);
                                rigid_body_node.reset_forces.set(true);
                            }
                            ApplyAction::Torque(torque) => {
                                native.add_torque(torque, false);
                                rigid_body_node.reset_forces.set(true);
                            }
                            ApplyAction::ForceAtPoint { force, point } => {
                                native.add_force_at_point(force, Point2::from(point), false);
                                rigid_body_node.reset_forces.set(true);
                            }
                            ApplyAction::Impulse(impulse) => native.apply_impulse(impulse, false),
                            ApplyAction::TorqueImpulse(impulse) => {
                                native.apply_torque_impulse(impulse, false)
                            }
                            ApplyAction::ImpulseAtPoint { impulse, point } => {
                                native.apply_impulse_at_point(impulse, Point2::from(point), false)
                            }
                            ApplyAction::WakeUp => native.wake_up(true),
                        }
                    }
                }
            }
        } else {
            let mut builder = RigidBodyBuilder::new(rigid_body_node.body_type().into())
                .position(isometry_from_global_transform(
                    &rigid_body_node.global_transform(),
                ))
                .ccd_enabled(rigid_body_node.is_ccd_enabled())
                .additional_mass(rigid_body_node.mass())
                .angvel(*rigid_body_node.ang_vel)
                .linvel(*rigid_body_node.lin_vel)
                .linear_damping(*rigid_body_node.lin_damping)
                .angular_damping(*rigid_body_node.ang_damping)
                .can_sleep(rigid_body_node.is_can_sleep())
                .sleeping(rigid_body_node.is_sleeping())
                .dominance_group(rigid_body_node.dominance())
                .gravity_scale(rigid_body_node.gravity_scale());

            if rigid_body_node.is_translation_locked() {
                builder = builder.lock_translations();
            }

            let mut body = builder.build();

            body.set_enabled_rotations(
                !rigid_body_node.is_rotation_locked(),
                !rigid_body_node.is_rotation_locked(),
                !rigid_body_node.is_rotation_locked(),
                false,
            );

            rigid_body_node.native.set(self.add_body(handle, body));

            Log::writeln(
                MessageKind::Information,
                format!(
                    "Native rigid body was created for node {}",
                    rigid_body_node.name()
                ),
            );
        }
    }

    pub(crate) fn sync_to_collider_node(
        &mut self,
        nodes: &NodePool,
        handle: Handle<Node>,
        collider_node: &scene::dim2::collider::Collider,
    ) {
        if !collider_node.is_globally_enabled() {
            self.remove_collider(collider_node.native.get());
            collider_node.native.set(Default::default());
            return;
        }

        let anything_changed =
            collider_node.transform_modified.get() || collider_node.needs_sync_model();

        // Important notes!
        // 1) The collider node may lack backing native physics collider in case if it
        //    is not attached to a rigid body.
        // 2) `get_mut` is **very** expensive because it forces physics engine to recalculate contacts
        //    and a lot of other stuff, this is why we need `anything_changed` flag.
        if collider_node.native.get() != ColliderHandle::invalid() {
            if anything_changed {
                if let Some(native) = self.colliders.get_mut(collider_node.native.get()) {
                    if collider_node.transform_modified.get() {
                        native.set_position_wrt_parent(Isometry2 {
                            rotation: UnitComplex::from_angle(
                                collider_node.local_transform().rotation().euler_angles().2,
                            ),
                            translation: Translation2 {
                                vector: collider_node.local_transform().position().xy(),
                            },
                        });
                    }

                    collider_node.shape.try_sync_model(|v| {
                        if let Some(shape) = collider_shape_into_native_shape(&v) {
                            native.set_shape(shape);
                        }
                    });
                    collider_node
                        .restitution
                        .try_sync_model(|v| native.set_restitution(v));
                    collider_node.collision_groups.try_sync_model(|v| {
                        native.set_collision_groups(InteractionGroups::new(
                            u32_to_group(v.memberships.0),
                            u32_to_group(v.filter.0),
                        ))
                    });
                    collider_node.solver_groups.try_sync_model(|v| {
                        native.set_solver_groups(InteractionGroups::new(
                            u32_to_group(v.memberships.0),
                            u32_to_group(v.filter.0),
                        ))
                    });
                    collider_node
                        .friction
                        .try_sync_model(|v| native.set_friction(v));
                    collider_node
                        .is_sensor
                        .try_sync_model(|v| native.set_sensor(v));
                    collider_node
                        .friction_combine_rule
                        .try_sync_model(|v| native.set_friction_combine_rule(v.into()));
                    collider_node
                        .restitution_combine_rule
                        .try_sync_model(|v| native.set_restitution_combine_rule(v.into()));
                }
            }
        } else if let Some(parent_body) = nodes
            .try_borrow(collider_node.parent())
            .and_then(|n| n.cast::<dim2::rigidbody::RigidBody>())
        {
            if parent_body.native.get() != RigidBodyHandle::invalid() {
                let rigid_body_native = parent_body.native.get();
                if let Some(shape) = collider_shape_into_native_shape(collider_node.shape()) {
                    let mut builder = ColliderBuilder::new(shape)
                        .position(Isometry2 {
                            rotation: UnitComplex::from_angle(
                                collider_node.local_transform().rotation().euler_angles().2,
                            ),
                            translation: Translation2 {
                                vector: collider_node.local_transform().position().xy(),
                            },
                        })
                        .friction(collider_node.friction())
                        .restitution(collider_node.restitution())
                        .collision_groups(InteractionGroups::new(
                            u32_to_group(collider_node.collision_groups().memberships.0),
                            u32_to_group(collider_node.collision_groups().filter.0),
                        ))
                        .friction_combine_rule(collider_node.friction_combine_rule().into())
                        .restitution_combine_rule(collider_node.restitution_combine_rule().into())
                        .solver_groups(InteractionGroups::new(
                            u32_to_group(collider_node.solver_groups().memberships.0),
                            u32_to_group(collider_node.solver_groups().filter.0),
                        ))
                        .sensor(collider_node.is_sensor());

                    if let Some(density) = collider_node.density() {
                        builder = builder.density(density);
                    }

                    let native_handle =
                        self.add_collider(handle, rigid_body_native, builder.build());

                    collider_node.native.set(native_handle);

                    Log::writeln(
                        MessageKind::Information,
                        format!(
                            "Native collider was created for node {}",
                            collider_node.name()
                        ),
                    );
                }
            }
        }
    }

    pub(crate) fn sync_to_joint_node(
        &mut self,
        nodes: &NodePool,
        handle: Handle<Node>,
        joint: &scene::dim2::joint::Joint,
    ) {
        if !joint.is_globally_enabled() {
            self.remove_joint(joint.native.get());
            joint.native.set(ImpulseJointHandle(Default::default()));
            return;
        }

        if let Some(native) = self.joints.set.get_mut(joint.native.get()) {
            joint.body1.try_sync_model(|v| {
                if let Some(rigid_body_node) = nodes
                    .try_borrow(v)
                    .and_then(|n| n.cast::<dim2::rigidbody::RigidBody>())
                {
                    native.body1 = rigid_body_node.native.get();
                }
            });
            joint.body2.try_sync_model(|v| {
                if let Some(rigid_body_node) = nodes
                    .try_borrow(v)
                    .and_then(|n| n.cast::<dim2::rigidbody::RigidBody>())
                {
                    native.body2 = rigid_body_node.native.get();
                }
            });
            joint.params.try_sync_model(|v| {
                native.data =
                    // Preserve local frames.
                    convert_joint_params(v, native.data.local_frame1, native.data.local_frame2)
            });
            joint.contacts_enabled.try_sync_model(|v| {
                native.data.set_contacts_enabled(v);
            });
            let mut local_frames = joint.local_frames.borrow_mut();
            if local_frames.is_none() {
                if let (Some(body1), Some(body2)) = (
                    nodes
                        .try_borrow(joint.body1())
                        .and_then(|n| n.cast::<scene::rigidbody::RigidBody>()),
                    nodes
                        .try_borrow(joint.body2())
                        .and_then(|n| n.cast::<scene::rigidbody::RigidBody>()),
                ) {
                    let (local_frame1, local_frame2) = calculate_local_frames(joint, body1, body2);
                    native.data =
                        convert_joint_params((*joint.params).clone(), local_frame1, local_frame2);
                    *local_frames = Some(JointLocalFrames::new(&local_frame1, &local_frame2));
                }
            }
        } else {
            let body1_handle = joint.body1();
            let body2_handle = joint.body2();
            let params = joint.params().clone();

            // A native joint can be created iff both rigid bodies are correctly assigned.
            if let (Some(body1), Some(body2)) = (
                nodes
                    .try_borrow(body1_handle)
                    .and_then(|n| n.cast::<dim2::rigidbody::RigidBody>()),
                nodes
                    .try_borrow(body2_handle)
                    .and_then(|n| n.cast::<dim2::rigidbody::RigidBody>()),
            ) {
                // Calculate local frames first (if needed).
                let mut local_frames = joint.local_frames.borrow_mut();
                let (local_frame1, local_frame2) = local_frames
                    .clone()
                    .map(|frames| {
                        (
                            Isometry2 {
                                rotation: frames.body1.rotation,
                                translation: Translation2 {
                                    vector: frames.body1.position,
                                },
                            },
                            Isometry2 {
                                rotation: frames.body2.rotation,
                                translation: Translation2 {
                                    vector: frames.body2.position,
                                },
                            },
                        )
                    })
                    .unwrap_or_else(|| calculate_local_frames(joint, body1, body2));

                let native_body1 = body1.native.get();
                let native_body2 = body2.native.get();

                assert!(self.bodies.get(native_body1).is_some());
                assert!(self.bodies.get(native_body2).is_some());

                let mut native_joint = convert_joint_params(params, local_frame1, local_frame2);
                native_joint.contacts_enabled = joint.is_contacts_enabled();
                let native_handle =
                    self.add_joint(handle, native_body1, native_body2, native_joint);

                joint.native.set(native_handle);
                *local_frames = Some(JointLocalFrames::new(&local_frame1, &local_frame2));

                Log::writeln(
                    MessageKind::Information,
                    format!("Native joint was created for node {}", joint.name()),
                );
            }
        }
    }

    /// Intersections checks between regular colliders and sensor colliders
    pub(crate) fn intersections_with(
        &self,
        collider: ColliderHandle,
    ) -> impl Iterator<Item = IntersectionPair> + '_ {
        self.narrow_phase.intersection_pairs_with(collider).map(
            |(collider1, collider2, intersecting)| IntersectionPair {
                collider1: Handle::decode_from_u128(
                    self.colliders.get(collider1).unwrap().user_data,
                ),
                collider2: Handle::decode_from_u128(
                    self.colliders.get(collider2).unwrap().user_data,
                ),
                has_any_active_contact: intersecting,
            },
        )
    }

    /// Contacts checks between two regular colliders
    pub(crate) fn contacts_with(
        &self,
        collider: ColliderHandle,
    ) -> impl Iterator<Item = ContactPair> + '_ {
        self.narrow_phase
            .contact_pairs_with(collider)
            .filter_map(|c| ContactPair::from_native(c, self))
    }

    /// Returns an iterator over all contact pairs generated in this frame.
    pub fn contacts(&self) -> impl Iterator<Item = ContactPair> + '_ {
        self.narrow_phase
            .contact_pairs()
            .filter_map(|c| ContactPair::from_native(c, self))
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for PhysicsWorld {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PhysicsWorld")
    }
}
