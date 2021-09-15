//! Contains all structures and methods to operate with physics world.

#[cfg(feature = "dim2")]
use rapier2d::{
    dynamics::{CCDSolver, IntegrationParameters, IslandManager, Joint, JointParams, RigidBody},
    geometry::{BroadPhase, Collider, InteractionGroups, NarrowPhase},
    parry::shape::FeatureId,
    pipeline::{EventHandler, PhysicsPipeline, QueryPipeline},
};
#[cfg(feature = "dim3")]
use rapier3d::{
    dynamics::{CCDSolver, IntegrationParameters, IslandManager, Joint, JointParams, RigidBody},
    geometry::{BroadPhase, Collider, InteractionGroups, NarrowPhase},
    parry::shape::FeatureId,
    pipeline::{EventHandler, PhysicsPipeline, QueryPipeline},
};

use crate::{
    body::RigidBodyContainer,
    collider::ColliderContainer,
    desc::{ColliderDesc, JointDesc, PhysicsDesc, RigidBodyDesc},
    joint::JointContainer,
};
use rg3d_core::{arrayvec::ArrayVec, instant, visitor::prelude::*, BiDirHashMap};
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    time::Duration,
};

pub mod body;
pub mod collider;
pub mod desc;
pub mod joint;

#[cfg(feature = "dim3")]
pub use rapier3d as rapier;

#[cfg(feature = "dim2")]
pub use rapier2d as rapier;

#[cfg(feature = "dim3")]
pub type NativeRigidBodyHandle = rapier3d::dynamics::RigidBodyHandle;
#[cfg(feature = "dim3")]
pub type NativeJointHandle = rapier3d::dynamics::JointHandle;
#[cfg(feature = "dim3")]
pub type NativeColliderHandle = rapier3d::geometry::ColliderHandle;
#[cfg(feature = "dim3")]
pub type Vector<N> = rapier3d::prelude::Vector<N>;
#[cfg(feature = "dim3")]
pub type Point<N> = rapier3d::prelude::Point<N>;
#[cfg(feature = "dim3")]
pub type NativeRay = rapier3d::prelude::Ray;
#[cfg(feature = "dim3")]
pub type Isometry<N> = rapier3d::prelude::Isometry<N>;
#[cfg(feature = "dim3")]
pub type Translation<N> = rapier3d::prelude::Translation<N>;
#[cfg(feature = "dim3")]
pub type AngVector<N> = rapier3d::prelude::AngVector<N>;
#[cfg(feature = "dim3")]
pub type Rotation<N> = rapier3d::prelude::Rotation<N>;

#[cfg(feature = "dim2")]
pub type NativeRigidBodyHandle = rapier2d::dynamics::RigidBodyHandle;
#[cfg(feature = "dim2")]
pub type NativeJointHandle = rapier2d::dynamics::JointHandle;
#[cfg(feature = "dim2")]
pub type NativeColliderHandle = rapier2d::geometry::ColliderHandle;
#[cfg(feature = "dim2")]
pub type Vector<N> = rapier2d::prelude::Vector<N>;
#[cfg(feature = "dim2")]
pub type Point<N> = rapier2d::prelude::Point<N>;
#[cfg(feature = "dim2")]
pub type NativeRay = rapier2d::prelude::Ray;
#[cfg(feature = "dim2")]
pub type Isometry<N> = rapier2d::prelude::Isometry<N>;
#[cfg(feature = "dim2")]
pub type Translation<N> = rapier2d::prelude::Translation<N>;
#[cfg(feature = "dim2")]
pub type AngVector<N> = rapier2d::prelude::AngVector<N>;
#[cfg(feature = "dim2")]
pub type Rotation<N> = rapier2d::prelude::Rotation<N>;

macro_rules! define_rapier_handle {
    ($(#[$meta:meta])* $type_name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        #[repr(transparent)]
        pub struct $type_name(pub rg3d_core::uuid::Uuid);

        impl From<rg3d_core::uuid::Uuid> for $type_name {
            fn from(inner: rg3d_core::uuid::Uuid) -> Self {
                Self(inner)
            }
        }

        impl Into<rg3d_core::uuid::Uuid> for $type_name {
            fn into(self) -> rg3d_core::uuid::Uuid {
                self.0
            }
        }

        impl Visit for $type_name {
            fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
                visitor.enter_region(name)?;
                self.0.visit("Id", visitor)?;
                visitor.leave_region()
            }
        }
    };
}

define_rapier_handle!(
    /// Rigid body handle wrapper.
    RigidBodyHandle
);

define_rapier_handle!(
    /// Collider handle wrapper.
    ColliderHandle
);

define_rapier_handle!(
    /// Joint handle wrapper.
    JointHandle
);

/// A ray intersection result.
#[derive(Debug, Clone)]
pub struct Intersection {
    /// A handle of the collider with which intersection was detected.
    pub collider: ColliderHandle,

    /// A normal at the intersection position.
    pub normal: Vector<f32>,

    /// A position of the intersection in world coordinates.
    pub position: Point<f32>,

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
    /// A ray for cast.
    pub ray_origin: Point<f32>,

    pub ray_direction: Vector<f32>,

    /// Maximum distance of cast.
    pub max_len: f32,

    /// Groups to check.
    pub groups: InteractionGroups,

    /// Whether to sort intersections from closest to farthest.
    pub sort_results: bool,
}

/// Physics world.
pub struct PhysicsWorld {
    /// Current physics pipeline.
    pipeline: PhysicsPipeline,
    /// Current gravity vector. Default is (0.0, -9.81, 0.0)
    pub gravity: Vector<f32>,
    /// A set of parameters that define behavior of every rigid body.
    pub integration_parameters: IntegrationParameters,
    /// Broad phase performs rough intersection checks.
    pub broad_phase: BroadPhase,
    /// Narrow phase is responsible for precise contact generation.
    pub narrow_phase: NarrowPhase,
    /// A continuous collision detection solver.
    pub ccd_solver: CCDSolver,
    /// Structure responsible for maintaining the set of active rigid-bodies, and putting non-moving
    /// rigid-bodies to sleep to save computation times.
    pub islands: IslandManager,

    /// A container of rigid bodies.
    pub bodies: RigidBodyContainer,

    /// A container of colliders.
    pub colliders: ColliderContainer,

    /// A container of joints.
    pub joints: JointContainer,

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

    pub performance_statistics: PhysicsPerformanceStatistics,
}

impl Debug for PhysicsWorld {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Physics")
    }
}

impl Default for PhysicsWorld {
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
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            #[cfg(feature = "dim3")]
            gravity: Vector::new(0.0, -9.81, 0.0),
            #[cfg(feature = "dim2")]
            gravity: Vector::new(0.0, -9.81),
            integration_parameters: IntegrationParameters::default(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            islands: IslandManager::new(),
            bodies: RigidBodyContainer::new(),
            colliders: ColliderContainer::new(),
            joints: JointContainer::new(),
            event_handler: Box::new(()),
            query: Default::default(),
            desc: Default::default(),
            performance_statistics: Default::default(),
        }
    }

    /// Tries to get a parent of collider.
    pub fn collider_parent(&self, collider: &ColliderHandle) -> Option<&RigidBodyHandle> {
        self.colliders
            .get(collider)
            .and_then(|c| self.bodies.handle_map().key_of(&c.parent().unwrap()))
    }

    pub fn step(&mut self) {
        let time = instant::Instant::now();

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

        self.performance_statistics.step_time += instant::Instant::now() - time;
    }

    #[doc(hidden)]
    pub fn generate_desc(&self) -> PhysicsDesc {
        let body_dense_map = self
            .bodies
            .set
            .iter()
            .enumerate()
            .map(|(i, (h, _))| (h, NativeRigidBodyHandle::from_raw_parts(i as u32, 0)))
            .collect::<HashMap<_, _>>();

        let mut body_handle_map = BiDirHashMap::default();
        for (engine_handle, rapier_handle) in self.bodies.handle_map().forward_map() {
            body_handle_map.insert(*engine_handle, body_dense_map[rapier_handle]);
        }

        let collider_dense_map = self
            .colliders
            .set
            .iter()
            .enumerate()
            .map(|(i, (h, _))| (h, NativeColliderHandle::from_raw_parts(i as u32, 0)))
            .collect::<HashMap<_, _>>();

        let mut collider_handle_map = BiDirHashMap::default();
        for (engine_handle, rapier_handle) in self.colliders.handle_map().forward_map() {
            collider_handle_map.insert(*engine_handle, collider_dense_map[rapier_handle]);
        }

        let joint_dense_map = self
            .joints
            .set
            .iter()
            .enumerate()
            .map(|(i, (h, _))| (h, NativeJointHandle::from_raw_parts(i as u32, 0)))
            .collect::<HashMap<_, _>>();

        let mut joint_handle_map = BiDirHashMap::default();
        for (engine_handle, rapier_handle) in self.joints.handle_map.forward_map() {
            joint_handle_map.insert(*engine_handle, joint_dense_map[rapier_handle]);
        }

        PhysicsDesc {
            integration_parameters: self.integration_parameters.into(),

            bodies: self
                .bodies
                .iter()
                .map(|b| RigidBodyDesc::from_body(b, self.colliders.handle_map()))
                .collect::<Vec<_>>(),

            colliders: self
                .colliders
                .iter()
                .map(|c| ColliderDesc::from_collider(c, self.bodies.handle_map()))
                .collect::<Vec<_>>(),

            gravity: self.gravity,

            joints: self
                .joints
                .iter()
                .map(|j| JointDesc::from_joint(j, self.bodies.handle_map()))
                .collect::<Vec<_>>(),

            body_handle_map,
            collider_handle_map,
            joint_handle_map,
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
        let ray = NativeRay::new(
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
            opts.groups,
            None, // TODO
            |handle, intersection| {
                query_buffer.push(Intersection {
                    collider: self
                        .colliders
                        .handle_map()
                        .key_of(&handle)
                        .cloned()
                        .unwrap(),
                    normal: intersection.normal,
                    position: ray.point_at(intersection.toi),
                    feature: intersection.feature,
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

    /// Adds new rigid body.
    pub fn add_body(&mut self, rigid_body: RigidBody) -> RigidBodyHandle {
        self.bodies.add(rigid_body)
    }

    /// Removes a rigid body.
    pub fn remove_body(&mut self, rigid_body: &RigidBodyHandle) -> Option<RigidBody> {
        self.bodies.remove(
            rigid_body,
            &mut self.colliders,
            &mut self.joints,
            &mut self.islands,
        )
    }

    /// Adds new collider.
    pub fn add_collider(
        &mut self,
        collider: Collider,
        rigid_body: &RigidBodyHandle,
    ) -> ColliderHandle {
        self.colliders.add(collider, rigid_body, &mut self.bodies)
    }

    /// Removes a collider.
    pub fn remove_collider(&mut self, collider_handle: &ColliderHandle) -> Option<Collider> {
        self.colliders
            .remove(collider_handle, &mut self.bodies, &mut self.islands)
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
        self.joints
            .add(body1, body2, joint_params, &mut self.bodies)
    }

    /// Removes a joint.
    pub fn remove_joint(&mut self, joint_handle: &JointHandle, wake_up: bool) -> Option<Joint> {
        self.joints
            .remove(joint_handle, &mut self.bodies, &mut self.islands, wake_up)
    }
}

impl Visit for PhysicsWorld {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut desc = if visitor.is_reading() {
            Default::default()
        } else if let Some(desc) = self.desc.as_ref() {
            desc.clone()
        } else {
            self.generate_desc()
        };
        desc.visit(name, visitor)?;

        // Save descriptors for resolve stage.
        if visitor.is_reading() {
            self.desc = Some(desc);
        }

        Ok(())
    }
}
