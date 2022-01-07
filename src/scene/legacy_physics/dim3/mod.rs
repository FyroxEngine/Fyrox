use crate::scene::legacy_physics::dim3::body::RigidBodyContainer;
use crate::scene::legacy_physics::dim3::collider::ColliderContainer;
use crate::scene::legacy_physics::dim3::joint::JointContainer;
use rapier3d::{
    dynamics::{CCDSolver, IntegrationParameters, IslandManager},
    geometry::{BroadPhase, NarrowPhase},
    pipeline::EventHandler,
};
use rg3d_core::visitor::prelude::*;
use std::fmt::{Debug, Formatter};

/// Rapier rigid body handle.
pub type NativeRigidBodyHandle = rapier3d::dynamics::RigidBodyHandle;
/// Rapier joint handle.
pub type NativeJointHandle = rapier3d::dynamics::JointHandle;
/// Rapier collider handle.
pub type NativeColliderHandle = rapier3d::geometry::ColliderHandle;
/// N-dimensional vector alias.
pub type Vector<N> = rapier3d::prelude::Vector<N>;
/// N-dimensional point alias.
pub type Point<N> = rapier3d::prelude::Point<N>;
/// Rapier ray alias.
pub type NativeRay = rapier3d::prelude::Ray;
/// N-dimensional isometry alias.
pub type Isometry<N> = rapier3d::prelude::Isometry<N>;
/// N-dimensional translation alias.
pub type Translation<N> = rapier3d::prelude::Translation<N>;
/// N-dimensional angular vector alias.
pub type AngVector<N> = rapier3d::prelude::AngVector<N>;
/// N-dimensional rotation alias.
pub type Rotation<N> = rapier3d::prelude::Rotation<N>;

pub mod body;
pub mod collider;
pub mod desc;
pub mod joint;

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

        impl From<$type_name> for rg3d_core::uuid::Uuid {
            fn from(v: $type_name) -> Self {
                v.0
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

/// Physics world.
pub struct PhysicsWorld {
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

impl PhysicsWorld {
    /// Creates a new instance of the physics world.
    pub fn new() -> Self {
        Self {
            gravity: Vector::new(0.0, -9.81, 0.0),
            integration_parameters: IntegrationParameters::default(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            islands: IslandManager::new(),
            bodies: RigidBodyContainer::new(),
            colliders: ColliderContainer::new(),
            joints: JointContainer::new(),
            event_handler: Box::new(()),
        }
    }
}
