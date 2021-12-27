//! Contains all structures and methods to operate with physics world.

#![warn(missing_docs)]

#[doc(hidden)]
pub mod legacy;

#[cfg(feature = "dim3")]
pub use rapier3d as rapier;

#[cfg(feature = "dim2")]
pub use rapier2d as rapier;

/// Rapier rigid body handle.
#[cfg(feature = "dim3")]
pub type NativeRigidBodyHandle = rapier3d::dynamics::RigidBodyHandle;
/// Rapier joint handle.
#[cfg(feature = "dim3")]
pub type NativeJointHandle = rapier3d::dynamics::JointHandle;
/// Rapier collider handle.
#[cfg(feature = "dim3")]
pub type NativeColliderHandle = rapier3d::geometry::ColliderHandle;
/// N-dimensional vector alias.
#[cfg(feature = "dim3")]
pub type Vector<N> = rapier3d::prelude::Vector<N>;
/// N-dimensional point alias.
#[cfg(feature = "dim3")]
pub type Point<N> = rapier3d::prelude::Point<N>;
/// Rapier ray alias.
#[cfg(feature = "dim3")]
pub type NativeRay = rapier3d::prelude::Ray;
/// N-dimensional isometry alias.
#[cfg(feature = "dim3")]
pub type Isometry<N> = rapier3d::prelude::Isometry<N>;
/// N-dimensional translation alias.
#[cfg(feature = "dim3")]
pub type Translation<N> = rapier3d::prelude::Translation<N>;
/// N-dimensional angular vector alias.
#[cfg(feature = "dim3")]
pub type AngVector<N> = rapier3d::prelude::AngVector<N>;
/// N-dimensional rotation alias.
#[cfg(feature = "dim3")]
pub type Rotation<N> = rapier3d::prelude::Rotation<N>;

/// Rapier rigid body handle.
#[cfg(feature = "dim2")]
pub type NativeRigidBodyHandle = rapier2d::dynamics::RigidBodyHandle;
/// Rapier joint handle.
#[cfg(feature = "dim2")]
pub type NativeJointHandle = rapier2d::dynamics::JointHandle;
/// Rapier collider handle.
#[cfg(feature = "dim2")]
pub type NativeColliderHandle = rapier2d::geometry::ColliderHandle;
/// N-dimensional vector alias.
#[cfg(feature = "dim2")]
pub type Vector<N> = rapier2d::prelude::Vector<N>;
/// N-dimensional point alias.
#[cfg(feature = "dim2")]
pub type Point<N> = rapier2d::prelude::Point<N>;
/// Rapier ray alias.
#[cfg(feature = "dim2")]
pub type NativeRay = rapier2d::prelude::Ray;
/// N-dimensional isometry alias.
#[cfg(feature = "dim2")]
pub type Isometry<N> = rapier2d::prelude::Isometry<N>;
/// N-dimensional translation alias.
#[cfg(feature = "dim2")]
pub type Translation<N> = rapier2d::prelude::Translation<N>;
/// N-dimensional angular vector alias.
#[cfg(feature = "dim2")]
pub type AngVector<N> = rapier2d::prelude::AngVector<N>;
/// N-dimensional rotation alias.
#[cfg(feature = "dim2")]
pub type Rotation<N> = rapier2d::prelude::Rotation<N>;
