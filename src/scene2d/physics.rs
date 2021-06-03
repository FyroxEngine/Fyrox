//! Contains all structures and methods to operate with physics world.

use crate::core::algebra::{Const, DVector};
use crate::{
    core::{
        algebra::{
            Dynamic, Isometry2, Point2, Translation, Translation2, Unit, UnitComplex, VecStorage,
            Vector2,
        },
        arrayvec::ArrayVec,
        instant,
        math::ray::Ray,
        pool::ErasedHandle,
        uuid::Uuid,
        visitor::prelude::*,
        BiDirHashMap,
    },
    engine::{ColliderHandle, JointHandle, RigidBodyHandle},
};
use rapier2d::dynamics::{IslandManager, RigidBodyType};
use rapier2d::{
    dynamics::{
        BallJoint, CCDSolver, FixedJoint, IntegrationParameters, Joint, JointParams, JointSet,
        PrismaticJoint, RigidBody, RigidBodyBuilder, RigidBodySet,
    },
    geometry::{
        BroadPhase, Collider, ColliderBuilder, ColliderSet, InteractionGroups, NarrowPhase,
        Segment, Shape,
    },
    parry::shape::{FeatureId, SharedShape},
    pipeline::{EventHandler, PhysicsPipeline, QueryPipeline},
};
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    hash::Hash,
    time::Duration,
};

/// A ray intersection result.
#[derive(Debug, Clone)]
pub struct Intersection {
    /// A handle of the collider with which intersection was detected.
    pub collider: ColliderHandle,

    /// A normal at the intersection position.
    pub normal: Vector2<f32>,

    /// A position of the intersection in world coordinates.
    pub position: Point2<f32>,

    /// Additional data that contains a kind of the feature with which
    /// intersection was detected as well as its index.    
    pub feature: FeatureId,

    /// Distance from the ray origin.
    pub toi: f32,
}

/// A set of options for the ray cast.
pub struct RayCastOptions {
    /// A ray for cast.
    pub ray: Ray,

    /// Maximum distance of cast.
    pub max_len: f32,

    /// Groups to check.
    pub groups: InteractionGroups,

    /// Whether to sort intersections from closest to farthest.
    pub sort_results: bool,
}

/// Physics world.
pub struct Physics {
    /// Current physics pipeline.
    pipeline: PhysicsPipeline,
    /// Current gravity vector. Default is (0.0, -9.81, 0.0)
    pub gravity: Vector2<f32>,
    /// A set of parameters that define behavior of every rigid body.
    pub integration_parameters: IntegrationParameters,
    /// Broad phase performs rough intersection checks.
    pub broad_phase: BroadPhase,
    /// Narrow phase is responsible for precise contact generation.
    pub narrow_phase: NarrowPhase,
    /// A continuous collision detection solver.
    pub ccd_solver: CCDSolver,
    /// Structure responsible for maintaining the set of active rigid-bodies, and
    /// putting non-moving rigid-bodies to sleep to save computation times.
    pub islands: IslandManager,

    /// A set of rigid bodies.
    bodies: RigidBodySet,

    /// A set of colliders.
    colliders: ColliderSet,

    /// A set of joints.
    joints: JointSet,

    /// Event handler collects info about contacts and proximity events.
    pub event_handler: Box<dyn EventHandler>,

    /// Descriptors have two purposes:
    /// 1) Defer deserialization to resolve stage - the stage where all meshes
    ///    were loaded and there is a possibility to obtain data for trimeshes.
    ///    Resolve stage will drain these vectors. This is normal use case.
    /// 2) Save data from editor: when descriptors are set, only they will be
    ///    written to output. This is a HACK, but I don't know better solution
    ///    yet.
    pub desc: Option<PhysicsDesc>,

    query: RefCell<QueryPipeline>,

    pub(in crate) performance_statistics: PhysicsPerformanceStatistics,

    body_handle_map: BiDirHashMap<RigidBodyHandle, rapier2d::dynamics::RigidBodyHandle>,

    collider_handle_map: BiDirHashMap<ColliderHandle, rapier2d::geometry::ColliderHandle>,

    joint_handle_map: BiDirHashMap<JointHandle, rapier2d::dynamics::JointHandle>,
}

impl Debug for Physics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Physics")
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
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
    pub(in crate) fn reset(&mut self) {
        *self = Default::default();
    }
}

macro_rules! impl_convert_map {
    ($name:ident, $key:ty, $value:ty) => {
        pub fn $name(
            map: &BiDirHashMap<$key, crate::core::pool::ErasedHandle>,
        ) -> BiDirHashMap<$key, $value> {
            map.forward_map()
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        <$value>::from_raw_parts(v.index(), v.generation()),
                    )
                })
                .collect()
        }
    };
}

impl_convert_map!(
    convert_rigid_body_map,
    RigidBodyHandle,
    rapier2d::dynamics::RigidBodyHandle
);
impl_convert_map!(
    convert_joint_map,
    JointHandle,
    rapier2d::dynamics::JointHandle
);
impl_convert_map!(
    convert_collider_map,
    ColliderHandle,
    rapier2d::geometry::ColliderHandle
);

impl Physics {
    pub(in crate) fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: Vector2::new(0.0, 9.81),
            integration_parameters: IntegrationParameters::default(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            islands: IslandManager::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: JointSet::new(),
            event_handler: Box::new(()),
            query: Default::default(),
            desc: Default::default(),
            performance_statistics: Default::default(),
            body_handle_map: Default::default(),
            collider_handle_map: Default::default(),
            joint_handle_map: Default::default(),
        }
    }

    // Deep copy is performed using descriptors.
    pub(in crate) fn deep_copy(&self) -> Self {
        let mut phys = Self::new();
        phys.desc = Some(self.generate_desc());
        phys.resolve();
        phys
    }

    /// TODO
    pub fn bodies(&self) -> &RigidBodySet {
        &self.bodies
    }

    /// TODO
    pub fn colliders(&self) -> &ColliderSet {
        &self.colliders
    }

    /// TODO
    pub fn joints(&self) -> &JointSet {
        &self.joints
    }

    /// TODO
    pub fn body_mut(&mut self, handle: &RigidBodyHandle) -> Option<&mut RigidBody> {
        let bodies = &mut self.bodies;
        self.body_handle_map
            .value_of(handle)
            .and_then(move |&h| bodies.get_mut(h))
    }

    /// TODO
    pub fn body_mut_rapier(
        &mut self,
        handle: rapier2d::dynamics::RigidBodyHandle,
    ) -> Option<&mut RigidBody> {
        self.bodies.get_mut(handle)
    }

    /// TODO
    pub fn body(&self, handle: &RigidBodyHandle) -> Option<&RigidBody> {
        let bodies = &self.bodies;
        self.body_handle_map
            .value_of(handle)
            .and_then(move |&h| bodies.get(h))
    }

    /// TODO
    pub fn body_rapier(&self, handle: rapier2d::dynamics::RigidBodyHandle) -> Option<&RigidBody> {
        self.bodies.get(handle)
    }

    /// TODO
    pub fn contains_body(&self, handle: &RigidBodyHandle) -> bool {
        self.body(handle).is_some()
    }

    /// TODO
    pub fn collider_mut(&mut self, handle: &ColliderHandle) -> Option<&mut Collider> {
        let colliders = &mut self.colliders;
        self.collider_handle_map
            .value_of(handle)
            .and_then(move |&h| colliders.get_mut(h))
    }

    /// TODO
    pub fn collider(&self, handle: &ColliderHandle) -> Option<&Collider> {
        let colliders = &self.colliders;
        self.collider_handle_map
            .value_of(handle)
            .and_then(|&h| colliders.get(h))
    }

    /// TODO
    pub fn collider_rapier(&self, handle: rapier2d::geometry::ColliderHandle) -> Option<&Collider> {
        self.colliders.get(handle)
    }

    /// TODO
    pub fn contains_collider(&self, handle: &ColliderHandle) -> bool {
        self.collider(handle).is_some()
    }

    /// TODO
    pub fn body_handle_map(
        &self,
    ) -> &BiDirHashMap<RigidBodyHandle, rapier2d::dynamics::RigidBodyHandle> {
        &self.body_handle_map
    }

    /// TODO
    pub fn collider_handle_map(
        &self,
    ) -> &BiDirHashMap<ColliderHandle, rapier2d::geometry::ColliderHandle> {
        &self.collider_handle_map
    }

    /// TODO
    pub fn joint_handle_map(&self) -> &BiDirHashMap<JointHandle, rapier2d::dynamics::JointHandle> {
        &self.joint_handle_map
    }

    /// TODO
    pub fn collider_parent(&self, collider: &ColliderHandle) -> Option<&RigidBodyHandle> {
        self.collider(collider)
            .and_then(|c| self.body_handle_map.key_of(&c.parent().unwrap()))
    }

    pub(in crate) fn step(&mut self) {
        let time = instant::Instant::now();

        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joints,
            &mut self.ccd_solver,
            &(),
            &*self.event_handler,
        );

        self.performance_statistics.step_time += instant::Instant::now() - time;
    }

    #[doc(hidden)]
    pub fn generate_desc(&self) -> PhysicsDesc {
        let body_dense_map = self
            .bodies
            .iter()
            .enumerate()
            .map(|(i, (h, _))| {
                (
                    h,
                    rapier2d::dynamics::RigidBodyHandle::from_raw_parts(i as u32, 0),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut body_handle_map = BiDirHashMap::default();
        for (engine_handle, rapier_handle) in self.body_handle_map.forward_map() {
            body_handle_map.insert(*engine_handle, {
                let (index, generation) = body_dense_map[rapier_handle].into_raw_parts();
                ErasedHandle::new(index as u32, generation as u32)
            });
        }

        let collider_dense_map = self
            .colliders
            .iter()
            .enumerate()
            .map(|(i, (h, _))| {
                (
                    h,
                    rapier2d::geometry::ColliderHandle::from_raw_parts(i as u32, 0),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut collider_handle_map = BiDirHashMap::default();
        for (engine_handle, rapier_handle) in self.collider_handle_map.forward_map() {
            collider_handle_map.insert(*engine_handle, {
                let (index, generation) = collider_dense_map[rapier_handle].into_raw_parts();
                ErasedHandle::new(index as u32, generation as u32)
            });
        }

        let joint_dense_map = self
            .joints
            .iter()
            .enumerate()
            .map(|(i, (h, _))| {
                (
                    h,
                    rapier2d::dynamics::JointHandle::from_raw_parts(i as u32, 0),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut joint_handle_map = BiDirHashMap::default();
        for (engine_handle, rapier_handle) in self.joint_handle_map.forward_map() {
            joint_handle_map.insert(*engine_handle, {
                let (index, generation) = joint_dense_map[rapier_handle].into_raw_parts();
                ErasedHandle::new(index as u32, generation as u32)
            });
        }

        PhysicsDesc {
            integration_parameters: self.integration_parameters.clone().into(),

            bodies: self
                .bodies
                .iter()
                .map(|(_, b)| RigidBodyDesc::from_body(b, &self.collider_handle_map))
                .collect::<Vec<_>>(),

            colliders: self
                .colliders
                .iter()
                .map(|(_, c)| ColliderDesc::from_collider(c, &self.body_handle_map))
                .collect::<Vec<_>>(),

            gravity: self.gravity,

            joints: self
                .joints
                .iter()
                .map(|(_, j)| JointDesc::from_joint(j, &self.body_handle_map))
                .collect::<Vec<_>>(),

            body_handle_map,
            collider_handle_map,
            joint_handle_map,
        }
    }

    /// Casts a ray with given options.
    pub fn cast_ray<S: QueryResultsStorage>(
        &self,
        ray_origin: Vector2<f32>,
        ray_direction: Vector2<f32>,
        max_len: f32,
        groups: InteractionGroups,
        sort_results: bool,
        query_buffer: &mut S,
    ) {
        let time = instant::Instant::now();

        let mut query = self.query.borrow_mut();

        // TODO: Ideally this must be called once per frame, but it seems to be impossible because
        // a body can be deleted during the consecutive calls of this method which will most
        // likely end up in panic because of invalid handle stored in internal acceleration
        // structure. This could be fixed by delaying deleting of bodies/collider to the end
        // of the frame.
        query.update(&self.islands, &self.bodies, &self.colliders);

        query_buffer.clear();
        let ray = rapier2d::geometry::Ray::new(
            Point2::from(ray_origin),
            ray_direction
                .try_normalize(f32::EPSILON)
                .unwrap_or_default(),
        );
        query.intersections_with_ray(
            &self.colliders,
            &ray,
            max_len,
            true,
            groups,
            None, // TODO
            |handle, intersection| {
                query_buffer.push(Intersection {
                    collider: self.collider_handle_map.key_of(&handle).cloned().unwrap(),
                    normal: intersection.normal,
                    position: ray.point_at(intersection.toi),
                    feature: intersection.feature,
                    toi: intersection.toi,
                })
            },
        );
        if sort_results {
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

    pub(in crate) fn resolve(&mut self) {
        assert_eq!(self.bodies.len(), 0);
        assert_eq!(self.colliders.len(), 0);
        assert_eq!(self.joints.len(), 0);

        let mut phys_desc = self.desc.take().unwrap();

        self.body_handle_map = convert_rigid_body_map(&phys_desc.body_handle_map);
        self.collider_handle_map = convert_collider_map(&phys_desc.collider_handle_map);
        self.joint_handle_map = convert_joint_map(&phys_desc.joint_handle_map);

        self.integration_parameters = phys_desc.integration_parameters.into();

        for desc in phys_desc.bodies.drain(..) {
            self.bodies.insert(desc.convert_to_body());
        }

        for desc in phys_desc.colliders.drain(..) {
            let (collider, parent) = desc.convert_to_collider();
            self.colliders.insert_with_parent(
                collider,
                self.body_handle_map.value_of(&parent).cloned().unwrap(),
                &mut self.bodies,
            );
        }

        for desc in phys_desc.joints.drain(..) {
            let b1 = self
                .body_handle_map()
                .value_of(&desc.body1)
                .cloned()
                .unwrap();
            let b2 = self
                .body_handle_map()
                .value_of(&desc.body2)
                .cloned()
                .unwrap();
            self.joints.insert(&mut self.bodies, b1, b2, desc.params);
        }
    }

    /// Adds new rigid body.
    pub fn add_body(&mut self, rigid_body: RigidBody) -> RigidBodyHandle {
        let handle = self.bodies.insert(rigid_body);
        let id = RigidBodyHandle::from(Uuid::new_v4());
        self.body_handle_map.insert(id, handle);
        id
    }

    /// Removes a rigid body.
    pub fn remove_body(&mut self, rigid_body: &RigidBodyHandle) -> Option<RigidBody> {
        let bodies = &mut self.bodies;
        let colliders = &mut self.colliders;
        let joints = &mut self.joints;
        let islands = &mut self.islands;
        let result = self
            .body_handle_map
            .value_of(rigid_body)
            .and_then(|&h| bodies.remove(h, islands, colliders, joints));
        if let Some(body) = result.as_ref() {
            for collider in body.colliders() {
                self.collider_handle_map.remove_by_value(collider);
            }
            self.body_handle_map.remove_by_key(rigid_body);
        }
        result
    }

    /// Adds new collider.
    pub fn add_collider(
        &mut self,
        collider: Collider,
        rigid_body: &RigidBodyHandle,
    ) -> ColliderHandle {
        let handle = self.colliders.insert_with_parent(
            collider,
            *self.body_handle_map.value_of(rigid_body).unwrap(),
            &mut self.bodies,
        );
        let id = ColliderHandle::from(Uuid::new_v4());
        self.collider_handle_map.insert(id, handle);
        id
    }

    /// Removes a collider.
    pub fn remove_collider(&mut self, collider_handle: &ColliderHandle) -> Option<Collider> {
        let bodies = &mut self.bodies;
        let colliders = &mut self.colliders;
        let islands = &mut self.islands;
        let result = self
            .collider_handle_map
            .value_of(collider_handle)
            .and_then(|&h| colliders.remove(h, islands, bodies, true));
        self.collider_handle_map.remove_by_key(collider_handle);
        result
    }

    /// Adds new joint.
    pub fn add_joint<J>(
        &mut self,
        body1: &RigidBodyHandle,
        body2: &RigidBodyHandle,
        joint_params: J,
    ) -> JointHandle
    where
        J: Into<JointParams>,
    {
        let handle = self.joints.insert(
            &mut self.bodies,
            *self.body_handle_map.value_of(body1).unwrap(),
            *self.body_handle_map.value_of(body2).unwrap(),
            joint_params,
        );
        let id = JointHandle::from(Uuid::new_v4());
        self.joint_handle_map.insert(id, handle);
        id
    }

    /// Removes a joint.
    pub fn remove_joint(&mut self, joint_handle: &JointHandle, wake_up: bool) -> Option<Joint> {
        let bodies = &mut self.bodies;
        let joints = &mut self.joints;
        let islands = &mut self.islands;
        let result = self
            .joint_handle_map
            .value_of(joint_handle)
            .and_then(|&h| joints.remove(h, islands, bodies, wake_up));
        self.joint_handle_map.remove_by_key(joint_handle);
        result
    }
}

#[derive(Copy, Clone, Debug, Visit)]
#[repr(u32)]
#[doc(hidden)]
pub enum RigidBodyTypeDesc {
    Dynamic = 0,
    Static = 1,
    KinematicPositionBased = 2,
    KinematicVelocityBased = 3,
}

impl Default for RigidBodyTypeDesc {
    fn default() -> Self {
        Self::Dynamic
    }
}

impl From<RigidBodyType> for RigidBodyTypeDesc {
    fn from(s: RigidBodyType) -> Self {
        match s {
            RigidBodyType::Dynamic => Self::Dynamic,
            RigidBodyType::Static => Self::Static,
            RigidBodyType::KinematicPositionBased => Self::KinematicPositionBased,
            RigidBodyType::KinematicVelocityBased => Self::KinematicVelocityBased,
        }
    }
}

impl Into<RigidBodyType> for RigidBodyTypeDesc {
    fn into(self) -> RigidBodyType {
        match self {
            RigidBodyTypeDesc::Dynamic => RigidBodyType::Dynamic,
            RigidBodyTypeDesc::Static => RigidBodyType::Static,
            RigidBodyTypeDesc::KinematicPositionBased => RigidBodyType::KinematicPositionBased,
            RigidBodyTypeDesc::KinematicVelocityBased => RigidBodyType::KinematicVelocityBased,
        }
    }
}

#[derive(Clone, Debug, Visit)]
#[doc(hidden)]
pub struct RigidBodyDesc<C> {
    pub position: Vector2<f32>,
    pub rotation: UnitComplex<f32>,
    pub linvel: Vector2<f32>,
    pub angvel: f32,
    pub sleeping: bool,
    pub status: RigidBodyTypeDesc,
    pub colliders: Vec<C>,
    pub mass: f32,
    pub rotation_locked: bool,
    pub translation_locked: bool,
}

impl<C> Default for RigidBodyDesc<C> {
    fn default() -> Self {
        Self {
            position: Default::default(),
            rotation: UnitComplex::identity(),
            linvel: Default::default(),
            angvel: Default::default(),
            sleeping: false,
            status: Default::default(),
            colliders: vec![],
            mass: 1.0,
            rotation_locked: false,
            translation_locked: false,
        }
    }
}

impl<C: Hash + Clone + Eq> RigidBodyDesc<C> {
    #[doc(hidden)]
    pub fn from_body(
        body: &RigidBody,
        handle_map: &BiDirHashMap<C, rapier2d::geometry::ColliderHandle>,
    ) -> Self {
        Self {
            position: body.position().translation.vector,
            rotation: body.position().rotation,
            linvel: *body.linvel(),
            angvel: body.angvel(),
            status: body.body_type().into(),
            sleeping: body.is_sleeping(),
            colliders: body
                .colliders()
                .iter()
                .map(|c| handle_map.key_of(c).cloned().unwrap())
                .collect(),
            mass: body.mass(),
            rotation_locked: body.is_rotation_locked(),
            translation_locked: body.is_translation_locked(),
        }
    }

    fn convert_to_body(self) -> RigidBody {
        let mut builder = RigidBodyBuilder::new(self.status.into())
            .position(Isometry2 {
                translation: Translation {
                    vector: self.position,
                },
                rotation: self.rotation,
            })
            .additional_mass(self.mass)
            .linvel(self.linvel)
            .angvel(self.angvel);

        if self.translation_locked {
            builder = builder.lock_translations();
        }
        if self.rotation_locked {
            builder = builder.lock_rotations();
        }

        let mut body = builder.build();
        if self.sleeping {
            body.sleep();
        }
        body
    }
}

impl<C> RigidBodyDesc<C> {
    #[doc(hidden)]
    pub fn local_transform(&self) -> Isometry2<f32> {
        Isometry2 {
            rotation: self.rotation,
            translation: Translation {
                vector: self.position,
            },
        }
    }
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct BallDesc {
    pub radius: f32,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct CuboidDesc {
    pub half_extents: Vector2<f32>,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct CapsuleDesc {
    pub begin: Vector2<f32>,
    pub end: Vector2<f32>,
    pub radius: f32,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct SegmentDesc {
    pub begin: Vector2<f32>,
    pub end: Vector2<f32>,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct TriangleDesc {
    pub a: Vector2<f32>,
    pub b: Vector2<f32>,
    pub c: Vector2<f32>,
}

// TODO: for now data of trimesh and heightfield is not serializable.
//  In most cases it is ok, because PhysicsBinder allows to automatically
//  obtain data from associated mesh.
#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct TrimeshDesc;

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct HeightfieldDesc;

#[derive(Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub enum ColliderShapeDesc {
    Ball(BallDesc),
    Cuboid(CuboidDesc),
    Capsule(CapsuleDesc),
    Segment(SegmentDesc),
    Triangle(TriangleDesc),
    Trimesh(TrimeshDesc),
    Heightfield(HeightfieldDesc),
}

impl Default for ColliderShapeDesc {
    fn default() -> Self {
        Self::Ball(Default::default())
    }
}

impl ColliderShapeDesc {
    #[doc(hidden)]
    pub fn from_collider_shape(shape: &dyn Shape) -> Self {
        if let Some(ball) = shape.as_ball() {
            ColliderShapeDesc::Ball(BallDesc {
                radius: ball.radius,
            })
        } else if let Some(cuboid) = shape.as_cuboid() {
            ColliderShapeDesc::Cuboid(CuboidDesc {
                half_extents: cuboid.half_extents,
            })
        } else if let Some(capsule) = shape.as_capsule() {
            ColliderShapeDesc::Capsule(CapsuleDesc {
                begin: capsule.segment.a.coords,
                end: capsule.segment.b.coords,
                radius: capsule.radius,
            })
        } else if let Some(segment) = shape.downcast_ref::<Segment>() {
            ColliderShapeDesc::Segment(SegmentDesc {
                begin: segment.a.coords,
                end: segment.b.coords,
            })
        } else if let Some(triangle) = shape.as_triangle() {
            ColliderShapeDesc::Triangle(TriangleDesc {
                a: triangle.a.coords,
                b: triangle.b.coords,
                c: triangle.c.coords,
            })
        } else if shape.as_trimesh().is_some() {
            ColliderShapeDesc::Trimesh(TrimeshDesc)
        } else if shape.as_heightfield().is_some() {
            ColliderShapeDesc::Heightfield(HeightfieldDesc)
        } else {
            unreachable!()
        }
    }

    fn into_collider_shape(self) -> SharedShape {
        match self {
            ColliderShapeDesc::Ball(ball) => SharedShape::ball(ball.radius),
            ColliderShapeDesc::Cuboid(cuboid) => {
                SharedShape::cuboid(cuboid.half_extents.x, cuboid.half_extents.y)
            }
            ColliderShapeDesc::Capsule(capsule) => SharedShape::capsule(
                Point2::from(capsule.begin),
                Point2::from(capsule.end),
                capsule.radius,
            ),
            ColliderShapeDesc::Segment(segment) => {
                SharedShape::segment(Point2::from(segment.begin), Point2::from(segment.end))
            }
            ColliderShapeDesc::Triangle(triangle) => SharedShape::triangle(
                Point2::from(triangle.a),
                Point2::from(triangle.b),
                Point2::from(triangle.c),
            ),
            ColliderShapeDesc::Trimesh(_) => {
                // Create fake trimesh. It will be filled with actual data on resolve stage later on.
                let a = Point2::new(0.0, 0.0);
                let b = Point2::new(1.0, 0.0);
                let c = Point2::new(1.0, 0.0);
                SharedShape::trimesh(vec![a, b, c], vec![[0, 1, 2]])
            }
            ColliderShapeDesc::Heightfield(_) => SharedShape::heightfield(
                DVector::from_data(VecStorage::new(
                    Dynamic::new(2),
                    Const,
                    vec![0.0, 1.0, 0.0, 0.0],
                )),
                Vector2::new(1.0, 1.0),
            ),
        }
    }
}

#[derive(Clone, Debug, Visit)]
#[doc(hidden)]
pub struct ColliderDesc<R> {
    pub shape: ColliderShapeDesc,
    pub parent: R,
    pub friction: f32,
    pub density: Option<f32>,
    pub restitution: f32,
    pub is_sensor: bool,
    pub translation: Vector2<f32>,
    pub rotation: UnitComplex<f32>,
    pub collision_groups: InteractionGroupsDesc,
    pub solver_groups: InteractionGroupsDesc,
}

#[derive(Visit, Debug, Clone)]
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

impl<R: Default> Default for ColliderDesc<R> {
    fn default() -> Self {
        Self {
            shape: Default::default(),
            parent: Default::default(),
            friction: 0.5,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            translation: Default::default(),
            rotation: UnitComplex::identity(),
            collision_groups: Default::default(),
            solver_groups: Default::default(),
        }
    }
}

impl<R: Hash + Clone + Eq> ColliderDesc<R> {
    fn from_collider(
        collider: &Collider,
        handle_map: &BiDirHashMap<R, rapier2d::dynamics::RigidBodyHandle>,
    ) -> Self {
        Self {
            shape: ColliderShapeDesc::from_collider_shape(collider.shape()),
            parent: handle_map
                .key_of(&collider.parent().unwrap())
                .cloned()
                .unwrap(),
            friction: collider.friction(),
            density: collider.density(),
            restitution: collider.restitution(),
            is_sensor: collider.is_sensor(),
            translation: collider.position_wrt_parent().unwrap().translation.vector,
            rotation: collider.position_wrt_parent().unwrap().rotation,
            collision_groups: collider.collision_groups().into(),
            solver_groups: collider.solver_groups().into(),
        }
    }

    fn convert_to_collider(self) -> (Collider, R) {
        let mut builder = ColliderBuilder::new(self.shape.into_collider_shape())
            .friction(self.friction)
            .restitution(self.restitution)
            .position(Isometry2 {
                translation: Translation2 {
                    vector: self.translation,
                },
                rotation: self.rotation,
            })
            .solver_groups(InteractionGroups::new(
                self.solver_groups.memberships,
                self.solver_groups.filter,
            ))
            .collision_groups(InteractionGroups::new(
                self.collision_groups.memberships,
                self.collision_groups.filter,
            ))
            .sensor(self.is_sensor);
        if let Some(density) = self.density {
            builder = builder.density(density);
        }
        (builder.build(), self.parent)
    }
}

impl Visit for Physics {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut desc = if visitor.is_reading() {
            Default::default()
        } else if let Some(desc) = self.desc.as_ref() {
            desc.clone()
        } else {
            self.generate_desc()
        };
        desc.visit("Desc", visitor)?;

        // Save descriptors for resolve stage.
        if visitor.is_reading() {
            self.desc = Some(desc);
        }

        visitor.leave_region()
    }
}

// Almost full copy of rapier's IntegrationParameters
#[derive(Clone, Debug, Visit)]
#[doc(hidden)]
pub struct IntegrationParametersDesc {
    pub dt: f32,
    pub erp: f32,
    pub min_ccd_dt: f32,
    pub joint_erp: f32,
    pub warmstart_coeff: f32,
    pub warmstart_correction_slope: f32,
    pub velocity_solve_fraction: f32,
    pub velocity_based_erp: f32,
    pub allowed_linear_error: f32,
    pub prediction_distance: f32,
    pub allowed_angular_error: f32,
    pub max_linear_correction: f32,
    pub max_angular_correction: f32,
    pub max_velocity_iterations: u32,
    pub max_position_iterations: u32,
    pub min_island_size: u32,
    pub max_ccd_substeps: u32,
}

impl Default for IntegrationParametersDesc {
    fn default() -> Self {
        Self::from(IntegrationParameters::default())
    }
}

impl From<IntegrationParameters> for IntegrationParametersDesc {
    fn from(params: IntegrationParameters) -> Self {
        Self {
            dt: params.dt,
            erp: params.erp,
            min_ccd_dt: params.min_ccd_dt,
            joint_erp: params.joint_erp,
            warmstart_coeff: params.warmstart_coeff,
            warmstart_correction_slope: params.warmstart_correction_slope,
            velocity_solve_fraction: params.velocity_solve_fraction,
            velocity_based_erp: params.velocity_based_erp,
            allowed_linear_error: params.allowed_linear_error,
            prediction_distance: params.prediction_distance,
            allowed_angular_error: params.allowed_angular_error,
            max_linear_correction: params.max_linear_correction,
            max_angular_correction: params.max_angular_correction,
            max_velocity_iterations: params.max_velocity_iterations as u32,
            max_position_iterations: params.max_position_iterations as u32,
            min_island_size: params.min_island_size as u32,
            max_ccd_substeps: params.max_ccd_substeps as u32,
        }
    }
}

impl Into<IntegrationParameters> for IntegrationParametersDesc {
    fn into(self) -> IntegrationParameters {
        IntegrationParameters {
            dt: self.dt,
            min_ccd_dt: self.min_ccd_dt,
            erp: self.erp,
            joint_erp: self.joint_erp,
            warmstart_coeff: self.warmstart_coeff,
            warmstart_correction_slope: self.warmstart_correction_slope,
            velocity_solve_fraction: self.velocity_solve_fraction,
            velocity_based_erp: self.velocity_based_erp,
            allowed_linear_error: self.allowed_linear_error,
            allowed_angular_error: self.allowed_angular_error,
            max_linear_correction: self.max_linear_correction,
            max_angular_correction: self.max_angular_correction,
            prediction_distance: self.prediction_distance,
            max_velocity_iterations: self.max_velocity_iterations as usize,
            max_position_iterations: self.max_position_iterations as usize,
            min_island_size: self.min_island_size as usize,
            max_ccd_substeps: self.max_ccd_substeps as usize,
        }
    }
}

#[derive(Default, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct BallJointDesc {
    pub local_anchor1: Vector2<f32>,
    pub local_anchor2: Vector2<f32>,
}

#[derive(Clone, Debug, Visit)]
#[doc(hidden)]
pub struct FixedJointDesc {
    pub local_anchor1_translation: Vector2<f32>,
    pub local_anchor1_rotation: UnitComplex<f32>,
    pub local_anchor2_translation: Vector2<f32>,
    pub local_anchor2_rotation: UnitComplex<f32>,
}

impl Default for FixedJointDesc {
    fn default() -> Self {
        Self {
            local_anchor1_translation: Default::default(),
            local_anchor1_rotation: UnitComplex::identity(),
            local_anchor2_translation: Default::default(),
            local_anchor2_rotation: UnitComplex::identity(),
        }
    }
}

#[derive(Default, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct PrismaticJointDesc {
    pub local_anchor1: Vector2<f32>,
    pub local_axis1: Vector2<f32>,
    pub local_anchor2: Vector2<f32>,
    pub local_axis2: Vector2<f32>,
}

#[derive(Default, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct RevoluteJointDesc {
    pub local_anchor1: Vector2<f32>,
    pub local_axis1: Vector2<f32>,
    pub local_anchor2: Vector2<f32>,
    pub local_axis2: Vector2<f32>,
}

#[derive(Clone, Debug, Visit)]
#[doc(hidden)]
pub enum JointParamsDesc {
    BallJoint(BallJointDesc),
    FixedJoint(FixedJointDesc),
    PrismaticJoint(PrismaticJointDesc),
}

impl Default for JointParamsDesc {
    fn default() -> Self {
        Self::BallJoint(Default::default())
    }
}

impl Into<JointParams> for JointParamsDesc {
    fn into(self) -> JointParams {
        match self {
            JointParamsDesc::BallJoint(v) => JointParams::from(BallJoint::new(
                Point2::from(v.local_anchor1),
                Point2::from(v.local_anchor2),
            )),
            JointParamsDesc::FixedJoint(v) => JointParams::from(FixedJoint::new(
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
            )),
            JointParamsDesc::PrismaticJoint(v) => JointParams::from(PrismaticJoint::new(
                Point2::from(v.local_anchor1),
                Unit::<Vector2<f32>>::new_normalize(v.local_axis1),
                Point2::from(v.local_anchor2),
                Unit::<Vector2<f32>>::new_normalize(v.local_axis2),
            )),
        }
    }
}

impl JointParamsDesc {
    #[doc(hidden)]
    pub fn from_params(params: &JointParams) -> Self {
        match params {
            JointParams::BallJoint(v) => Self::BallJoint(BallJointDesc {
                local_anchor1: v.local_anchor1.coords,
                local_anchor2: v.local_anchor2.coords,
            }),
            JointParams::FixedJoint(v) => Self::FixedJoint(FixedJointDesc {
                local_anchor1_translation: v.local_frame1.translation.vector,
                local_anchor1_rotation: v.local_frame1.rotation,
                local_anchor2_translation: v.local_frame2.translation.vector,
                local_anchor2_rotation: v.local_frame2.rotation,
            }),
            JointParams::PrismaticJoint(v) => Self::PrismaticJoint(PrismaticJointDesc {
                local_anchor1: v.local_anchor1.coords,
                local_axis1: v.local_axis1().into_inner(),
                local_anchor2: v.local_anchor2.coords,
                local_axis2: v.local_axis2().into_inner(),
            }),
        }
    }
}

#[derive(Clone, Debug, Default, Visit)]
#[doc(hidden)]
pub struct JointDesc<R> {
    pub body1: R,
    pub body2: R,
    pub params: JointParamsDesc,
}

impl<R: Hash + Clone + Eq> JointDesc<R> {
    #[doc(hidden)]
    pub fn from_joint(
        joint: &Joint,
        handle_map: &BiDirHashMap<R, rapier2d::dynamics::RigidBodyHandle>,
    ) -> Self {
        Self {
            body1: handle_map.key_of(&joint.body1).cloned().unwrap(),
            body2: handle_map.key_of(&joint.body2).cloned().unwrap(),
            params: JointParamsDesc::from_params(&joint.params),
        }
    }
}

#[derive(Default, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct PhysicsDesc {
    pub integration_parameters: IntegrationParametersDesc,
    pub colliders: Vec<ColliderDesc<RigidBodyHandle>>,
    pub bodies: Vec<RigidBodyDesc<ColliderHandle>>,
    pub gravity: Vector2<f32>,
    pub joints: Vec<JointDesc<RigidBodyHandle>>,
    pub body_handle_map: BiDirHashMap<RigidBodyHandle, ErasedHandle>,
    pub collider_handle_map: BiDirHashMap<ColliderHandle, ErasedHandle>,
    pub joint_handle_map: BiDirHashMap<JointHandle, ErasedHandle>,
}
