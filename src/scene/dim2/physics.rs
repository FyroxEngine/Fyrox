//! Scene physics module.

use crate::scene::variable::VariableFlags;
use crate::{
    core::{
        algebra::{
            Isometry2, Matrix4, Point2, Translation2, Unit, UnitComplex, UnitQuaternion,
            UnitVector2, Vector2, Vector3,
        },
        algebra::{Isometry3, Point3, Rotation3, Translation3},
        arrayvec::ArrayVec,
        color::Color,
        inspect::{Inspect, PropertyInfo},
        instant,
        math::Matrix4Ext,
        pool::{Handle, Pool},
        visitor::prelude::*,
        BiDirHashMap,
    },
    scene::{
        self,
        collider::{self},
        debug::{Line, SceneDrawingContext},
        dim2::{collider::ColliderShape, rigidbody::ApplyAction},
        graph::physics::{FeatureId, IntegrationParameters, PhysicsPerformanceStatistics},
        joint::JointChanges,
        node::Node,
    },
    utils::log::{Log, MessageKind},
};
use rapier2d::{
    dynamics::{
        BallJoint, CCDSolver, FixedJoint, IslandManager, JointHandle, JointParams, JointSet,
        PrismaticJoint, RigidBody, RigidBodyActivation, RigidBodyBuilder, RigidBodyHandle,
        RigidBodySet, RigidBodyType,
    },
    geometry::{
        BroadPhase, Collider, ColliderBuilder, ColliderHandle, ColliderSet, Cuboid,
        InteractionGroups, NarrowPhase, Ray, SharedShape, TriMesh,
    },
    pipeline::{EventHandler, PhysicsPipeline, QueryPipeline},
};
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
#[derive(Debug, Clone)]
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

pub(super) struct Container<S, A>
where
    A: Hash + Eq + Clone,
{
    set: S,
    map: BiDirHashMap<A, Handle<Node>>,
}

fn convert_joint_params(params: scene::dim2::joint::JointParams) -> JointParams {
    match params {
        scene::dim2::joint::JointParams::BallJoint(v) => {
            let mut ball_joint =
                BallJoint::new(Point2::from(v.local_anchor1), Point2::from(v.local_anchor2));

            ball_joint.limits_enabled = v.limits_enabled;
            ball_joint.limits_local_axis1 = UnitVector2::new_normalize(v.limits_local_axis1);
            ball_joint.limits_local_axis2 = UnitVector2::new_normalize(v.limits_local_axis2);
            ball_joint.limits_angle = v.limits_angle;

            JointParams::from(ball_joint)
        }
        scene::dim2::joint::JointParams::FixedJoint(v) => {
            let fixed_joint = FixedJoint::new(
                Isometry2 {
                    translation: Translation2 {
                        vector: v.local_anchor1_translation,
                    },
                    rotation: v.local_anchor1_rotation,
                },
                Isometry2 {
                    translation: Translation2 {
                        vector: v.local_anchor2_translation,
                    },
                    rotation: v.local_anchor2_rotation,
                },
            );

            JointParams::from(fixed_joint)
        }
        scene::dim2::joint::JointParams::PrismaticJoint(v) => {
            let mut prismatic_joint = PrismaticJoint::new(
                Point2::from(v.local_anchor1),
                Unit::<Vector2<f32>>::new_normalize(v.local_axis1),
                Point2::from(v.local_anchor2),
                Unit::<Vector2<f32>>::new_normalize(v.local_axis2),
            );

            prismatic_joint.limits = v.limits;
            prismatic_joint.limits_enabled = v.limits_enabled;

            JointParams::from(prismatic_joint)
        }
    }
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
#[derive(Visit, Inspect)]
pub struct PhysicsWorld {
    /// A flag that defines whether physics simulation is enabled or not.
    pub enabled: bool,

    /// A set of parameters that define behavior of every rigid body.
    pub integration_parameters: IntegrationParameters,

    /// Current gravity vector. Default is (0.0, -9.81)
    pub gravity: Vector2<f32>,

    /// Performance statistics of a single simulation step.
    #[visit(skip)]
    #[inspect(skip)]
    pub performance_statistics: PhysicsPerformanceStatistics,

    // Current physics pipeline.
    #[visit(skip)]
    #[inspect(skip)]
    pipeline: PhysicsPipeline,
    // Broad phase performs rough intersection checks.
    #[visit(skip)]
    #[inspect(skip)]
    broad_phase: BroadPhase,
    // Narrow phase is responsible for precise contact generation.
    #[visit(skip)]
    #[inspect(skip)]
    narrow_phase: NarrowPhase,
    // A continuous collision detection solver.
    #[visit(skip)]
    #[inspect(skip)]
    ccd_solver: CCDSolver,
    // Structure responsible for maintaining the set of active rigid-bodies, and putting non-moving
    // rigid-bodies to sleep to save computation times.
    #[visit(skip)]
    #[inspect(skip)]
    islands: IslandManager,
    // A container of rigid bodies.
    #[visit(skip)]
    #[inspect(skip)]
    bodies: Container<RigidBodySet, RigidBodyHandle>,
    // A container of colliders.
    #[visit(skip)]
    #[inspect(skip)]
    colliders: Container<ColliderSet, ColliderHandle>,
    // A container of joints.
    #[visit(skip)]
    #[inspect(skip)]
    joints: Container<JointSet, JointHandle>,
    // Event handler collects info about contacts and proximity events.
    #[visit(skip)]
    #[inspect(skip)]
    event_handler: Box<dyn EventHandler>,
    #[visit(skip)]
    #[inspect(skip)]
    query: RefCell<QueryPipeline>,
}

impl PhysicsWorld {
    /// Creates a new instance of the physics world.
    pub(crate) fn new() -> Self {
        Self {
            enabled: true,
            pipeline: PhysicsPipeline::new(),
            gravity: Vector2::new(0.0, -9.81),
            integration_parameters: IntegrationParameters::default(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            islands: IslandManager::new(),
            bodies: Container {
                set: RigidBodySet::new(),
                map: Default::default(),
            },
            colliders: Container {
                set: ColliderSet::new(),
                map: Default::default(),
            },
            joints: Container {
                set: JointSet::new(),
                map: Default::default(),
            },
            event_handler: Box::new(()),
            query: RefCell::new(Default::default()),
            performance_statistics: Default::default(),
        }
    }

    pub(crate) fn update(&mut self) {
        let time = instant::Instant::now();

        if self.enabled {
            let integration_parameters = rapier2d::dynamics::IntegrationParameters {
                dt: self.integration_parameters.dt,
                min_ccd_dt: self.integration_parameters.min_ccd_dt,
                erp: self.integration_parameters.erp,
                joint_erp: self.integration_parameters.joint_erp,
                warmstart_coeff: self.integration_parameters.warmstart_coeff,
                warmstart_correction_slope: self.integration_parameters.warmstart_correction_slope,
                velocity_solve_fraction: self.integration_parameters.velocity_solve_fraction,
                velocity_based_erp: self.integration_parameters.velocity_based_erp,
                allowed_linear_error: self.integration_parameters.allowed_linear_error,
                prediction_distance: self.integration_parameters.prediction_distance,
                allowed_angular_error: self.integration_parameters.allowed_angular_error,
                max_linear_correction: self.integration_parameters.max_linear_correction,
                max_angular_correction: self.integration_parameters.max_angular_correction,
                max_velocity_iterations: self.integration_parameters.max_velocity_iterations
                    as usize,
                max_position_iterations: self.integration_parameters.max_position_iterations
                    as usize,
                min_island_size: self.integration_parameters.min_island_size as usize,
                max_ccd_substeps: self.integration_parameters.max_ccd_substeps as usize,
            };

            self.pipeline.step(
                &self.gravity,
                &integration_parameters,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.bodies.set,
                &mut self.colliders.set,
                &mut self.joints.set,
                &mut self.ccd_solver,
                &(),
                &*self.event_handler,
            );
        }

        self.performance_statistics.step_time += instant::Instant::now() - time;
    }

    pub(crate) fn add_body(&mut self, owner: Handle<Node>, body: RigidBody) -> RigidBodyHandle {
        let handle = self.bodies.set.insert(body);
        self.bodies.map.insert(handle, owner);
        handle
    }

    pub(crate) fn remove_body(&mut self, handle: RigidBodyHandle) {
        assert!(self.bodies.map.remove_by_key(&handle).is_some());
        self.bodies.set.remove(
            handle,
            &mut self.islands,
            &mut self.colliders.set,
            &mut self.joints.set,
        );
    }

    pub(crate) fn add_collider(
        &mut self,
        owner: Handle<Node>,
        parent_body: RigidBodyHandle,
        collider: Collider,
    ) -> ColliderHandle {
        let handle =
            self.colliders
                .set
                .insert_with_parent(collider, parent_body, &mut self.bodies.set);
        self.colliders.map.insert(handle, owner);
        handle
    }

    pub(crate) fn remove_collider(&mut self, handle: ColliderHandle) -> bool {
        if self
            .colliders
            .set
            .remove(handle, &mut self.islands, &mut self.bodies.set, false)
            .is_some()
        {
            assert!(self.colliders.map.remove_by_key(&handle).is_some());
            true
        } else {
            false
        }
    }

    pub(crate) fn add_joint(
        &mut self,
        owner: Handle<Node>,
        body1: RigidBodyHandle,
        body2: RigidBodyHandle,
        params: JointParams,
    ) -> JointHandle {
        let handle = self.joints.set.insert(body1, body2, params);
        self.joints.map.insert(handle, owner);
        handle
    }

    pub(crate) fn remove_joint(&mut self, handle: JointHandle) {
        assert!(self.joints.map.remove_by_key(&handle).is_some());
        self.joints
            .set
            .remove(handle, &mut self.islands, &mut self.bodies.set, false);
    }

    /// Draws physics world. Very useful for debugging, it allows you to see where are
    /// rigid bodies, which colliders they have and so on.
    pub fn draw(&self, context: &mut SceneDrawingContext) {
        for (_, body) in self.bodies.set.iter() {
            context.draw_transform(isometry2_to_mat4(body.position()));
        }

        for (_, collider) in self.colliders.set.iter() {
            let body = self.bodies.set.get(collider.parent().unwrap()).unwrap();
            let collider_local_transform =
                isometry2_to_mat4(collider.position_wrt_parent().unwrap());
            let transform = isometry2_to_mat4(body.position()) * collider_local_transform;
            if let Some(trimesh) = collider.shape().as_trimesh() {
                let trimesh: &TriMesh = trimesh;
                for triangle in trimesh.triangles() {
                    let a = transform
                        .transform_point(&Point3::from(triangle.a.to_homogeneous()))
                        .coords;
                    let b = transform
                        .transform_point(&Point3::from(triangle.b.to_homogeneous()))
                        .coords;
                    let c = transform
                        .transform_point(&Point3::from(triangle.c.to_homogeneous()))
                        .coords;
                    context.draw_triangle(a, b, c, Color::opaque(200, 200, 200));
                }
            } else if let Some(cuboid) = collider.shape().as_cuboid() {
                context.draw_rectangle(
                    cuboid.half_extents.x,
                    cuboid.half_extents.y,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(ball) = collider.shape().as_ball() {
                context.draw_circle(
                    body.position().translation.vector.to_homogeneous(),
                    ball.radius,
                    10,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(triangle) = collider.shape().as_triangle() {
                context.draw_triangle(
                    triangle.a.to_homogeneous(),
                    triangle.b.to_homogeneous(),
                    triangle.c.to_homogeneous(),
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(capsule) = collider.shape().as_capsule() {
                context.draw_segment_flat_capsule(
                    capsule.segment.a.coords,
                    capsule.segment.b.coords,
                    capsule.radius,
                    10,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(heightfield) = collider.shape().as_heightfield() {
                for segment in heightfield.segments() {
                    let a = transform
                        .transform_point(&Point3::from(segment.a.to_homogeneous()))
                        .coords;
                    let b = transform
                        .transform_point(&Point3::from(segment.b.to_homogeneous()))
                        .coords;
                    context.add_line(Line {
                        begin: a,
                        end: b,
                        color: Color::opaque(200, 200, 200),
                    });
                }
            }
        }
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
        query.update(&self.islands, &self.bodies.set, &self.colliders.set);

        query_buffer.clear();
        let ray = Ray::new(
            opts.ray_origin,
            opts.ray_direction
                .try_normalize(f32::EPSILON)
                .unwrap_or_default(),
        );
        query.intersections_with_ray(
            &self.colliders.set,
            &ray,
            opts.max_len,
            true,
            InteractionGroups::new(opts.groups.memberships, opts.groups.filter),
            None, // TODO
            |handle, intersection| {
                query_buffer.push(Intersection {
                    collider: self.colliders.map.value_of(&handle).cloned().unwrap(),
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
        if let Some(native) = self.bodies.set.get_mut(rigid_body.native.get()) {
            let global_rotation = UnitComplex::from_angle(
                Rotation3::from_matrix(&new_global_transform.basis())
                    .euler_angles()
                    .2,
            );
            let global_position = Vector2::new(new_global_transform[12], new_global_transform[13]);

            native.set_position(
                Isometry2 {
                    translation: Translation2::from(global_position),
                    rotation: global_rotation,
                },
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
        if self.enabled {
            if let Some(native) = self.bodies.set.get(rigid_body.native.get()) {
                if native.body_type() == RigidBodyType::Dynamic {
                    let local_transform: Matrix4<f32> = parent_transform
                        .try_inverse()
                        .unwrap_or_else(Matrix4::identity)
                        * isometry2_to_mat4(native.position());

                    let local_rotation = UnitQuaternion::from_matrix(&local_transform.basis());
                    let local_position =
                        Vector3::new(local_transform[12], local_transform[13], 0.0);

                    rigid_body
                        .local_transform
                        .set_position(local_position)
                        .set_rotation(local_rotation);

                    rigid_body
                        .lin_vel
                        .set_with_flags(*native.linvel(), VariableFlags::MODIFIED);
                    rigid_body
                        .ang_vel
                        .set_with_flags(native.angvel(), VariableFlags::MODIFIED);
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
        // Important notes!
        // 1) `get_mut` is **very** expensive because it forces physics engine to recalculate contacts
        //    and a lot of other stuff, this is why we need `anything_changed` flag.
        if rigid_body_node.native.get() != RigidBodyHandle::invalid() {
            let mut actions = rigid_body_node.actions.lock();
            if rigid_body_node.need_sync_model() || !actions.is_empty() {
                if let Some(native) = self.bodies.set.get_mut(rigid_body_node.native.get()) {
                    // Sync native rigid body's properties with scene node's in case if they
                    // were changed by user.
                    rigid_body_node
                        .body_type
                        .try_sync_model(|v| native.set_body_type(v.into()));
                    rigid_body_node
                        .lin_vel
                        .try_sync_model(|v| native.set_linvel(v, false));
                    rigid_body_node
                        .ang_vel
                        .try_sync_model(|v| native.set_angvel(v, false));
                    rigid_body_node.mass.try_sync_model(|v| {
                        let mut props = *native.mass_properties();
                        props.set_mass(v, true);
                        native.set_mass_properties(props, true)
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
                        let mut activation = native.activation_mut();
                        if v {
                            activation.threshold = RigidBodyActivation::default_threshold()
                        } else {
                            activation.sleeping = false;
                            activation.threshold = -1.0;
                        };
                    });
                    rigid_body_node
                        .translation_locked
                        .try_sync_model(|v| native.lock_translations(v, false));
                    rigid_body_node.rotation_locked.try_sync_model(|v| {
                        // Logic is inverted here:
                        // See https://github.com/dimforge/rapier/pull/265
                        native.restrict_rotations(v, v, v, false);
                    });

                    while let Some(action) = actions.pop_front() {
                        match action {
                            ApplyAction::Force(force) => native.apply_force(force, false),
                            ApplyAction::Torque(torque) => native.apply_torque(torque, false),
                            ApplyAction::ForceAtPoint { force, point } => {
                                native.apply_force_at_point(force, Point2::from(point), false)
                            }
                            ApplyAction::Impulse(impulse) => native.apply_impulse(impulse, false),
                            ApplyAction::TorqueImpulse(impulse) => {
                                native.apply_torque_impulse(impulse, false)
                            }
                            ApplyAction::ImpulseAtPoint { impulse, point } => {
                                native.apply_impulse_at_point(impulse, Point2::from(point), false)
                            }
                            ApplyAction::WakeUp => native.wake_up(false),
                        }
                    }
                }
            }
        } else {
            let mut builder = RigidBodyBuilder::new(rigid_body_node.body_type().into())
                .position(Isometry2 {
                    rotation: UnitComplex::from_angle(
                        rigid_body_node
                            .local_transform()
                            .rotation()
                            .euler_angles()
                            .2,
                    ),
                    translation: Translation2 {
                        vector: rigid_body_node.local_transform().position().xy(),
                    },
                })
                .ccd_enabled(rigid_body_node.is_ccd_enabled())
                .additional_mass(rigid_body_node.mass())
                .angvel(*rigid_body_node.ang_vel)
                .linvel(*rigid_body_node.lin_vel)
                .linear_damping(*rigid_body_node.lin_damping)
                .angular_damping(*rigid_body_node.ang_damping)
                .can_sleep(rigid_body_node.is_can_sleep())
                .sleeping(rigid_body_node.is_sleeping());

            if rigid_body_node.is_translation_locked() {
                builder = builder.lock_translations();
            }

            let mut body = builder.build();

            // Logic is inverted here:
            // See https://github.com/dimforge/rapier/pull/265
            body.restrict_rotations(
                rigid_body_node.is_rotation_locked(),
                rigid_body_node.is_rotation_locked(),
                rigid_body_node.is_rotation_locked(),
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
        nodes: &Pool<Node>,
        handle: Handle<Node>,
        collider_node: &scene::dim2::collider::Collider,
    ) {
        let anything_changed =
            collider_node.transform_modified.get() || collider_node.needs_sync_model();

        // Important notes!
        // 1) The collider node may lack backing native physics collider in case if it
        //    is not attached to a rigid body.
        // 2) `get_mut` is **very** expensive because it forces physics engine to recalculate contacts
        //    and a lot of other stuff, this is why we need `anything_changed` flag.
        if collider_node.native.get() != ColliderHandle::invalid() {
            if anything_changed {
                if let Some(native) = self.colliders.set.get_mut(collider_node.native.get()) {
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
                        native.set_collision_groups(InteractionGroups::new(v.memberships, v.filter))
                    });
                    collider_node.solver_groups.try_sync_model(|v| {
                        native.set_solver_groups(InteractionGroups::new(v.memberships, v.filter))
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
        } else if let Some(Node::RigidBody2D(parent_body)) =
            nodes.try_borrow(collider_node.parent())
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
                            collider_node.collision_groups().memberships,
                            collider_node.collision_groups().filter,
                        ))
                        .friction_combine_rule(collider_node.friction_combine_rule().into())
                        .restitution_combine_rule(collider_node.restitution_combine_rule().into())
                        .solver_groups(InteractionGroups::new(
                            collider_node.solver_groups().memberships,
                            collider_node.solver_groups().filter,
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
        nodes: &Pool<Node>,
        handle: Handle<Node>,
        joint: &scene::dim2::joint::Joint,
    ) {
        if let Some(native) = self.joints.set.get_mut(joint.native.get()) {
            let mut changes = joint.changes.get();
            if changes.contains(JointChanges::PARAMS) {
                native.params = convert_joint_params(joint.params().clone());
                changes.remove(JointChanges::PARAMS);
            }
            if changes.contains(JointChanges::BODY1) {
                if let Some(Node::RigidBody2D(rigid_body_node)) = nodes.try_borrow(joint.body1()) {
                    native.body1 = rigid_body_node.native.get();
                }
                changes.remove(JointChanges::BODY1);
            }
            if changes.contains(JointChanges::BODY2) {
                if let Some(Node::RigidBody2D(rigid_body_node)) = nodes.try_borrow(joint.body2()) {
                    native.body2 = rigid_body_node.native.get();
                }
                changes.remove(JointChanges::BODY2);
            }

            if changes != JointChanges::NONE {
                Log::writeln(
                    MessageKind::Warning,
                    format!("Unhandled joint changes! Mask: {:?}", changes),
                );
            }

            joint.changes.set(changes);
        } else {
            let body1_handle = joint.body1();
            let body2_handle = joint.body2();
            let params = joint.params().clone();

            // A native joint can be created iff both rigid bodies are correctly assigned.
            if let (Some(Node::RigidBody2D(body1)), Some(Node::RigidBody2D(body2))) = (
                nodes.try_borrow(body1_handle),
                nodes.try_borrow(body2_handle),
            ) {
                let native_body1 = body1.native.get();
                let native_body2 = body2.native.get();

                let native = self.add_joint(
                    handle,
                    native_body1,
                    native_body2,
                    convert_joint_params(params),
                );

                joint.native.set(native);

                Log::writeln(
                    MessageKind::Information,
                    format!("Native joint was created for node {}", joint.name()),
                );
            }
        }
    }

    pub(crate) fn contacts_with(
        &self,
        collider: ColliderHandle,
    ) -> impl Iterator<Item = ContactPair> + '_ {
        self.narrow_phase
            .contacts_with(collider)
            .map(|c| ContactPair {
                collider1: self
                    .colliders
                    .map
                    .value_of(&c.collider1)
                    .cloned()
                    .unwrap_or_default(),
                collider2: self
                    .colliders
                    .map
                    .value_of(&c.collider2)
                    .cloned()
                    .unwrap_or_default(),
                manifolds: c
                    .manifolds
                    .iter()
                    .map(|m| ContactManifold {
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
                        rigid_body1: m
                            .data
                            .rigid_body1
                            .and_then(|h| self.bodies.map.value_of(&h).cloned())
                            .unwrap_or_default(),
                        rigid_body2: m
                            .data
                            .rigid_body2
                            .and_then(|h| self.bodies.map.value_of(&h).cloned())
                            .unwrap_or_default(),
                        normal: m.data.normal,
                    })
                    .collect(),
                has_any_active_contact: c.has_any_active_contact,
            })
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
