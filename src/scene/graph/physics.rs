use crate::scene::joint::JointChanges;
use crate::{
    core::{
        algebra::{Isometry3, Point3, Translation3, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        math::aabb::AxisAlignedBoundingBox,
        pool::{Handle, Pool},
        BiDirHashMap,
    },
    physics3d::rapier::{
        dynamics::{
            CCDSolver, IntegrationParameters, IslandManager, JointHandle, JointParams, JointSet,
            RigidBody, RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
        },
        geometry::{
            self, BroadPhase, Collider, ColliderBuilder, ColliderHandle, ColliderSet,
            InteractionGroups, NarrowPhase, Ray, TriMesh,
        },
        pipeline::{EventHandler, PhysicsPipeline, QueryPipeline},
    },
    scene::{
        self,
        collider::{ColliderChanges, InteractionGroupsDesc},
        debug::SceneDrawingContext,
        graph::isometric_global_transform,
        node::Node,
        rigidbody::RigidBodyChanges,
    },
    utils::log::{Log, MessageKind},
};
use rg3d_core::algebra::Vector2;
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    hash::Hash,
    time::Duration,
};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FeatureId {
    /// Shape-dependent identifier of a vertex.
    Vertex(u32),
    /// Shape-dependent identifier of an edge.
    Edge(u32),
    /// Shape-dependent identifier of a face.
    Face(u32),
    /// Unknown identifier.
    Unknown,
}

impl From<geometry::FeatureId> for FeatureId {
    fn from(v: geometry::FeatureId) -> Self {
        match v {
            geometry::FeatureId::Vertex(v) => FeatureId::Vertex(v),
            geometry::FeatureId::Edge(v) => FeatureId::Edge(v),
            geometry::FeatureId::Face(v) => FeatureId::Face(v),
            geometry::FeatureId::Unknown => FeatureId::Unknown,
        }
    }
}

/// Performance statistics for the physics part of the engine.
#[derive(Debug, Default, Clone)]
pub struct PhysicsPerformanceStatistics {
    /// A time that was needed to perform a single simulation step.
    pub step_time: Duration,

    /// A time that was needed to perform all ray casts.
    pub total_ray_cast_time: Cell<Duration>,
}

impl Display for PhysicsPerformanceStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Physics Step Time: {:?}\nPhysics Ray Cast Time: {:?}",
            self.step_time,
            self.total_ray_cast_time.get(),
        )
    }
}

impl PhysicsPerformanceStatistics {
    /// Resets performance statistics to default values.
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// A ray intersection result.
#[derive(Debug, Clone)]
pub struct Intersection {
    /// A handle of the collider with which intersection was detected.
    pub collider: Handle<Node>,

    /// A normal at the intersection position.
    pub normal: Vector3<f32>,

    /// A position of the intersection in world coordinates.
    pub position: Point3<f32>,

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
    pub ray_origin: Point3<f32>,

    /// A ray direction. Can be non-normalized.
    pub ray_direction: Vector3<f32>,

    /// Maximum distance of cast.
    pub max_len: f32,

    /// Groups to check.
    pub groups: InteractionGroupsDesc,

    /// Whether to sort intersections from closest to farthest.
    pub sort_results: bool,
}

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

pub struct ContactData {
    /// The contact point in the local-space of the first shape.
    pub local_p1: Vector3<f32>,
    /// The contact point in the local-space of the second shape.
    pub local_p2: Vector3<f32>,
    /// The distance between the two contact points.
    pub dist: f32,
    /// The impulse, along the contact normal, applied by this contact to the first collider's rigid-body.
    /// The impulse applied to the second collider's rigid-body is given by `-impulse`.
    pub impulse: f32,
    /// The friction impulses along the basis orthonormal to the contact normal, applied to the first
    /// collider's rigid-body.
    pub tangent_impulse: Vector2<f32>,
}

pub struct ContactManifold {
    /// The contacts points.
    pub points: Vec<ContactData>,
    /// The contact normal of all the contacts of this manifold, expressed in the local space of the first shape.
    pub local_n1: Vector3<f32>,
    /// The contact normal of all the contacts of this manifold, expressed in the local space of the second shape.
    pub local_n2: Vector3<f32>,
    /// The first rigid-body involved in this contact manifold.
    pub rigid_body1: Handle<Node>,
    /// The second rigid-body involved in this contact manifold.
    pub rigid_body2: Handle<Node>,
    /// The world-space contact normal shared by all the contact in this contact manifold.
    pub normal: Vector3<f32>,
}

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

pub struct PhysicsWorld {
    /// Current physics pipeline.
    pipeline: PhysicsPipeline,
    /// Current gravity vector. Default is (0.0, -9.81, 0.0)
    gravity: Vector3<f32>,
    /// A set of parameters that define behavior of every rigid body.
    integration_parameters: IntegrationParameters,
    /// Broad phase performs rough intersection checks.
    broad_phase: BroadPhase,
    /// Narrow phase is responsible for precise contact generation.
    narrow_phase: NarrowPhase,
    /// A continuous collision detection solver.
    ccd_solver: CCDSolver,
    /// Structure responsible for maintaining the set of active rigid-bodies, and putting non-moving
    /// rigid-bodies to sleep to save computation times.
    islands: IslandManager,

    /// A container of rigid bodies.
    bodies: Container<RigidBodySet, RigidBodyHandle>,

    /// A container of colliders.
    colliders: Container<ColliderSet, ColliderHandle>,

    /// A container of joints.
    joints: Container<JointSet, JointHandle>,

    /// Event handler collects info about contacts and proximity events.
    event_handler: Box<dyn EventHandler>,

    query: RefCell<QueryPipeline>,

    /// Performance statistics of a single simulation step.
    pub performance_statistics: PhysicsPerformanceStatistics,
}

impl PhysicsWorld {
    /// Creates a new instance of the physics world.
    pub(super) fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: Vector3::new(0.0, -9.81, 0.0),
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

    pub(super) fn update(&mut self) {
        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
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

    pub(super) fn add_body(&mut self, owner: Handle<Node>, body: RigidBody) -> RigidBodyHandle {
        let handle = self.bodies.set.insert(body);
        self.bodies.map.insert(handle, owner);
        handle
    }

    pub(super) fn remove_body(&mut self, handle: RigidBodyHandle) {
        assert!(self.bodies.map.remove_by_key(&handle).is_some());
        self.bodies.set.remove(
            handle,
            &mut self.islands,
            &mut self.colliders.set,
            &mut self.joints.set,
        );
    }

    pub(super) fn add_collider(
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

    pub(super) fn remove_collider(&mut self, handle: ColliderHandle) -> bool {
        if self
            .colliders
            .set
            .remove(handle, &mut self.islands, &mut self.bodies.set, true)
            .is_some()
        {
            assert!(self.colliders.map.remove_by_key(&handle).is_some());
            true
        } else {
            false
        }
    }

    pub(super) fn add_joint(
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

    pub(super) fn remove_joint(&mut self, handle: JointHandle) {
        assert!(self.joints.map.remove_by_key(&handle).is_some());
        self.joints
            .set
            .remove(handle, &mut self.islands, &mut self.bodies.set, true);
    }

    /// Draws physics world. Very useful for debugging, it allows you to see where are
    /// rigid bodies, which colliders they have and so on.
    pub fn draw(&self, context: &mut SceneDrawingContext) {
        for (_, body) in self.bodies.set.iter() {
            context.draw_transform(body.position().to_homogeneous());
        }

        for (_, collider) in self.colliders.set.iter() {
            let body = self.bodies.set.get(collider.parent().unwrap()).unwrap();
            let collider_local_transform = collider.position_wrt_parent().unwrap().to_homogeneous();
            let transform = body.position().to_homogeneous() * collider_local_transform;
            if let Some(trimesh) = collider.shape().as_trimesh() {
                let trimesh: &TriMesh = trimesh;
                for triangle in trimesh.triangles() {
                    let a = transform.transform_point(&triangle.a);
                    let b = transform.transform_point(&triangle.b);
                    let c = transform.transform_point(&triangle.c);
                    context.draw_triangle(
                        a.coords,
                        b.coords,
                        c.coords,
                        Color::opaque(200, 200, 200),
                    );
                }
            } else if let Some(cuboid) = collider.shape().as_cuboid() {
                let min = -cuboid.half_extents;
                let max = cuboid.half_extents;
                context.draw_oob(
                    &AxisAlignedBoundingBox::from_min_max(min, max),
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(ball) = collider.shape().as_ball() {
                context.draw_sphere(
                    body.position().translation.vector,
                    10,
                    10,
                    ball.radius,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(cone) = collider.shape().as_cone() {
                context.draw_cone(
                    10,
                    cone.radius,
                    cone.half_height * 2.0,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(cylinder) = collider.shape().as_cylinder() {
                context.draw_cylinder(
                    10,
                    cylinder.radius,
                    cylinder.half_height * 2.0,
                    true,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(round_cylinder) = collider.shape().as_round_cylinder() {
                context.draw_cylinder(
                    10,
                    round_cylinder.base_shape.radius,
                    round_cylinder.base_shape.half_height * 2.0,
                    false,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(triangle) = collider.shape().as_triangle() {
                context.draw_triangle(
                    triangle.a.coords,
                    triangle.b.coords,
                    triangle.c.coords,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(capsule) = collider.shape().as_capsule() {
                context.draw_segment_capsule(
                    capsule.segment.a.coords,
                    capsule.segment.b.coords,
                    capsule.radius,
                    10,
                    10,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(heightfield) = collider.shape().as_heightfield() {
                for triangle in heightfield.triangles() {
                    let a = transform.transform_point(&triangle.a);
                    let b = transform.transform_point(&triangle.b);
                    let c = transform.transform_point(&triangle.c);
                    context.draw_triangle(
                        a.coords,
                        b.coords,
                        c.coords,
                        Color::opaque(200, 200, 200),
                    );
                }
            }
        }
    }

    /// Casts a ray with given options.
    pub fn cast_ray<S: QueryResultsStorage>(&self, opts: RayCastOptions, query_buffer: &mut S) {
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
    }

    pub(super) fn sync_rigid_body_node(&mut self, rigid_body: &mut scene::rigidbody::RigidBody) {
        if let Some(native) = self.bodies.set.get_mut(rigid_body.native.get()) {
            rigid_body
                .local_transform
                .set_position(native.position().translation.vector)
                .set_rotation(native.position().rotation);
            rigid_body.lin_vel = *native.linvel();
            rigid_body.ang_vel = *native.angvel();
        }
    }

    pub(super) fn sync_to_rigid_body_node(
        &mut self,
        handle: Handle<Node>,
        rigid_body_node: &scene::rigidbody::RigidBody,
    ) {
        if let Some(native) = self.bodies.set.get_mut(rigid_body_node.native.get()) {
            // Sync transform.
            if rigid_body_node.transform_modified.get() {
                // Transform was changed by user, sync native rigid body with node's position.
                native.set_position(
                    Isometry3 {
                        rotation: **rigid_body_node.local_transform().rotation(),
                        translation: Translation3 {
                            vector: **rigid_body_node.local_transform().position(),
                        },
                    },
                    true,
                );
                rigid_body_node.transform_modified.set(false);
            }

            // Sync native rigid body's properties with scene node's in case if they
            // were changed by user.
            let mut changes = rigid_body_node.changes.get();
            if changes.contains(RigidBodyChanges::BODY_TYPE) {
                native.set_body_type(rigid_body_node.body_type.into());
                changes.remove(RigidBodyChanges::BODY_TYPE);
            }
            if changes.contains(RigidBodyChanges::LIN_VEL) {
                native.set_linvel(rigid_body_node.lin_vel, true);
                changes.remove(RigidBodyChanges::LIN_VEL);
            }
            if changes.contains(RigidBodyChanges::ANG_VEL) {
                native.set_angvel(rigid_body_node.ang_vel, true);
                changes.remove(RigidBodyChanges::ANG_VEL);
            }
            if changes.contains(RigidBodyChanges::MASS) {
                let mut props = *native.mass_properties();
                props.set_mass(rigid_body_node.mass, true);
                native.set_mass_properties(props, true);
                changes.remove(RigidBodyChanges::MASS);
            }
            if changes.contains(RigidBodyChanges::LIN_DAMPING) {
                native.set_linear_damping(rigid_body_node.lin_damping);
                changes.remove(RigidBodyChanges::LIN_DAMPING);
            }
            if changes.contains(RigidBodyChanges::ANG_DAMPING) {
                native.set_angular_damping(rigid_body_node.ang_damping);
                changes.remove(RigidBodyChanges::ANG_DAMPING);
            }
            if changes.contains(RigidBodyChanges::ROTATION_LOCKED) {
                native.restrict_rotations(
                    rigid_body_node.x_rotation_locked,
                    rigid_body_node.y_rotation_locked,
                    rigid_body_node.z_rotation_locked,
                    true,
                );
                changes.remove(RigidBodyChanges::ROTATION_LOCKED);
            }

            if changes != RigidBodyChanges::NONE {
                Log::writeln(
                    MessageKind::Warning,
                    format!("Unhandled rigid body changes! Mask: {:?}", changes),
                );
            }

            rigid_body_node.changes.set(changes);
        } else {
            let mut builder = RigidBodyBuilder::new(rigid_body_node.body_type.into())
                .position(Isometry3 {
                    rotation: **rigid_body_node.local_transform().rotation(),
                    translation: Translation3 {
                        vector: **rigid_body_node.local_transform().position(),
                    },
                })
                .additional_mass(rigid_body_node.mass)
                .angvel(rigid_body_node.ang_vel)
                .linvel(rigid_body_node.lin_vel)
                .linear_damping(rigid_body_node.lin_damping)
                .angular_damping(rigid_body_node.ang_damping)
                .restrict_rotations(
                    rigid_body_node.x_rotation_locked,
                    rigid_body_node.y_rotation_locked,
                    rigid_body_node.z_rotation_locked,
                );

            if rigid_body_node.translation_locked {
                builder = builder.lock_translations();
            }

            rigid_body_node
                .native
                .set(self.add_body(handle, builder.build()));

            Log::writeln(
                MessageKind::Information,
                format!(
                    "Native rigid body was created for node {}",
                    rigid_body_node.name()
                ),
            );
        }
    }

    pub(super) fn sync_to_collider_node(
        &mut self,
        nodes: &Pool<Node>,
        handle: Handle<Node>,
        collider_node: &scene::collider::Collider,
    ) {
        // The collider node may lack backing native physics collider in case if it
        // is not attached to a rigid body.
        if let Some(native) = self.colliders.set.get_mut(collider_node.native.get()) {
            if collider_node.transform_modified.get() {
                // Transform was changed by user, sync native rigid body with node's position.
                native.set_position(Isometry3 {
                    rotation: **collider_node.local_transform().rotation(),
                    translation: Translation3 {
                        vector: **collider_node.local_transform().position(),
                    },
                });
                collider_node.transform_modified.set(false);
            }

            let mut changes = collider_node.changes.get();
            if changes.contains(ColliderChanges::SHAPE) {
                let inv_global_transform = isometric_global_transform(nodes, handle)
                    .try_inverse()
                    .unwrap();
                if let Some(shape) = collider_node.shape().clone().into_native_shape(
                    inv_global_transform,
                    handle,
                    nodes,
                ) {
                    native.set_shape(shape);
                    changes.remove(ColliderChanges::SHAPE);
                }
            }
            if changes.contains(ColliderChanges::RESTITUTION) {
                native.set_restitution(collider_node.restitution());
                changes.remove(ColliderChanges::RESTITUTION);
            }
            if changes.contains(ColliderChanges::COLLISION_GROUPS) {
                native.set_collision_groups(InteractionGroups::new(
                    collider_node.collision_groups().memberships,
                    collider_node.collision_groups().filter,
                ));
                changes.remove(ColliderChanges::COLLISION_GROUPS);
            }
            if changes.contains(ColliderChanges::SOLVER_GROUPS) {
                native.set_solver_groups(InteractionGroups::new(
                    collider_node.solver_groups().memberships,
                    collider_node.solver_groups().filter,
                ));
                changes.remove(ColliderChanges::SOLVER_GROUPS);
            }
            if changes.contains(ColliderChanges::FRICTION) {
                native.set_friction(collider_node.friction());
                changes.remove(ColliderChanges::FRICTION);
            }
            if changes.contains(ColliderChanges::IS_SENSOR) {
                native.set_sensor(collider_node.is_sensor());
                changes.remove(ColliderChanges::IS_SENSOR);
            }

            if changes != ColliderChanges::NONE {
                Log::writeln(
                    MessageKind::Warning,
                    format!("Unhandled collider changes! Mask: {:?}", changes),
                );
            }

            collider_node.changes.set(changes);
            // TODO: Handle RESTITUTION_COMBINE_RULE + FRICTION_COMBINE_RULE
        } else if let Some(Node::RigidBody(parent_body)) = nodes.try_borrow(collider_node.parent())
        {
            if parent_body.native.get() != RigidBodyHandle::invalid() {
                let inv_global_transform = isometric_global_transform(nodes, handle)
                    .try_inverse()
                    .unwrap();
                let rigid_body_native = parent_body.native.get();
                if let Some(shape) = collider_node.shape().clone().into_native_shape(
                    inv_global_transform,
                    handle,
                    nodes,
                ) {
                    let mut builder = ColliderBuilder::new(shape)
                        .position(Isometry3 {
                            rotation: **collider_node.local_transform().rotation(),
                            translation: Translation3 {
                                vector: **collider_node.local_transform().position(),
                            },
                        })
                        .friction(collider_node.friction())
                        .restitution(collider_node.restitution())
                        .collision_groups(InteractionGroups::new(
                            collider_node.collision_groups().memberships,
                            collider_node.collision_groups().filter,
                        ))
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

    pub(super) fn sync_to_joint_node(
        &mut self,
        nodes: &Pool<Node>,
        handle: Handle<Node>,
        joint: &scene::joint::Joint,
    ) {
        if let Some(native) = self.joints.set.get_mut(joint.native.get()) {
            let mut changes = joint.changes.get();
            if changes.contains(JointChanges::PARAMS) {
                native.params = joint.params().clone().into();
                changes.remove(JointChanges::PARAMS);
            }
            if changes.contains(JointChanges::BODY1) {
                if let Some(Node::RigidBody(rigid_body_node)) = nodes.try_borrow(joint.body1()) {
                    native.body1 = rigid_body_node.native.get();
                }
                changes.remove(JointChanges::BODY1);
            }
            if changes.contains(JointChanges::BODY2) {
                if let Some(Node::RigidBody(rigid_body_node)) = nodes.try_borrow(joint.body2()) {
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
            if let (Some(Node::RigidBody(body1)), Some(Node::RigidBody(body2))) = (
                nodes.try_borrow(body1_handle),
                nodes.try_borrow(body2_handle),
            ) {
                let native_body1 = body1.native.get();
                let native_body2 = body2.native.get();

                let native = self.add_joint(handle, native_body1, native_body2, params.into());

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
