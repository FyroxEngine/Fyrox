#![warn(missing_docs)]

//! Contains all structures and methods to create and manage scenes.
//!
//! Scene is container for graph nodes, animations and physics.

pub mod base;
pub mod camera;
pub mod graph;
pub mod light;
pub mod mesh;
pub mod node;
pub mod particle_system;
pub mod physics;
pub mod sprite;
pub mod transform;

use crate::{
    animation::AnimationContainer,
    core::{
        algebra::{Isometry3, Matrix4, Point3, Translation, Vector2, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, Matrix4Ext},
        pool::{Handle, Pool, PoolIterator, PoolIteratorMut},
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    resource::texture::Texture,
    scene::{base::PhysicsBinding, graph::Graph, node::Node, physics::Physics},
    sound::{context::Context, engine::SoundEngine},
    utils::{lightmap::Lightmap, log::Log, log::MessageKind, navmesh::Navmesh},
};
use std::fmt::{Display, Formatter};
use std::{
    collections::HashMap,
    ops::{Deref, Index, IndexMut, Range},
    path::Path,
    sync::{Arc, Mutex},
};

macro_rules! define_rapier_handle {
    ($(#[$meta:meta])*, $type_name:ident, $dep_type:ty) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $type_name(pub rapier3d::data::arena::Index);

        impl From<$dep_type> for $type_name {
            fn from(inner: $dep_type) -> Self {
                let (index, generation) = inner.into_raw_parts();
                Self(rapier3d::data::arena::Index::from_raw_parts(
                    index, generation,
                ))
            }
        }

        impl From<rapier3d::data::arena::Index> for $type_name {
            fn from(inner: rapier3d::data::arena::Index) -> Self {
                Self(inner)
            }
        }

        impl Into<$dep_type> for $type_name {
            fn into(self) -> $dep_type {
                let (index, generation) = self.0.into_raw_parts();
                <$dep_type>::from_raw_parts(index, generation)
            }
        }

        impl Into<rapier3d::data::arena::Index> for $type_name {
            fn into(self) -> rapier3d::data::arena::Index {
                let (index, generation) = self.0.into_raw_parts();
                rapier3d::data::arena::Index::from_raw_parts(index, generation)
            }
        }

        impl Default for $type_name {
            fn default() -> Self {
                Self(rapier3d::data::arena::Index::from_raw_parts(
                    usize::max_value(),
                    u64::max_value(),
                ))
            }
        }

        impl $type_name {
            /// Checks if handle is invalid.
            pub fn is_none(&self) -> bool {
                *self == Default::default()
            }

            /// Checks if handle is valid.
            pub fn is_some(&self) -> bool {
                !self.is_none()
            }
        }

        impl Visit for $type_name {
            fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
                visitor.enter_region(name)?;

                let (index, mut generation) = self.0.into_raw_parts();
                let mut index = index as u64;

                index.visit("Index", visitor)?;
                generation.visit("Generation", visitor)?;

                if visitor.is_reading() {
                    self.0 =
                        rapier3d::data::arena::Index::from_raw_parts(index as usize, generation);
                }

                visitor.leave_region()
            }
        }
    };
}

define_rapier_handle!(
    #[doc="Rigid body handle wrapper."],
    RigidBodyHandle, rapier3d::dynamics::RigidBodyHandle);

define_rapier_handle!(
    #[doc="Collider handle wrapper."],
    ColliderHandle, rapier3d::geometry::ColliderHandle);

define_rapier_handle!(
    #[doc="Joint handle wrapper."],
    JointHandle, rapier3d::dynamics::JointHandle);

/// Physics binder is used to link graph nodes with rigid bodies. Scene will
/// sync transform of node with its associated rigid body.
#[derive(Clone, Debug)]
pub struct PhysicsBinder {
    /// Mapping Node -> RigidBody.
    forward_map: HashMap<Handle<Node>, RigidBodyHandle>,

    backward_map: HashMap<RigidBodyHandle, Handle<Node>>,

    /// Whether binder is enabled or not. If binder is disabled, it won't synchronize
    /// node's transform with body's transform.
    pub enabled: bool,
}

impl Default for PhysicsBinder {
    fn default() -> Self {
        Self {
            forward_map: Default::default(),
            backward_map: Default::default(),
            enabled: true,
        }
    }
}

impl PhysicsBinder {
    /// Links given graph node with specified rigid body. Returns old linked body.
    pub fn bind(
        &mut self,
        node: Handle<Node>,
        rigid_body: RigidBodyHandle,
    ) -> Option<RigidBodyHandle> {
        let old_body = self.forward_map.insert(node, rigid_body);
        self.backward_map.insert(rigid_body, node);
        old_body
    }

    /// Unlinks given graph node from its associated rigid body (if any).
    pub fn unbind(&mut self, node: Handle<Node>) -> Option<RigidBodyHandle> {
        if let Some(body_handle) = self.forward_map.remove(&node) {
            self.backward_map.remove(&body_handle);
            Some(body_handle)
        } else {
            None
        }
    }

    /// Unlinks given body from a node that is linked with the body.
    pub fn unbind_by_body(&mut self, body: RigidBodyHandle) -> Handle<Node> {
        if let Some(node) = self.backward_map.get(&body) {
            self.forward_map.remove(node);
            *node
        } else {
            Handle::NONE
        }
    }

    /// Returns handle of rigid body associated with given node. It will return
    /// Handle::NONE if given node isn't linked to a rigid body.
    pub fn body_of(&self, node: Handle<Node>) -> Option<RigidBodyHandle> {
        self.forward_map.get(&node).copied()
    }

    /// Tries to find a node for a given rigid body.
    pub fn node_of(&self, body: RigidBodyHandle) -> Option<Handle<Node>> {
        self.backward_map.get(&body).copied()
    }

    /// Removes all bindings.
    pub fn clear(&mut self) {
        self.forward_map.clear();
        self.backward_map.clear();
    }

    /// Returns a shared reference to inner forward mapping.
    pub fn forward_map(&self) -> &HashMap<Handle<Node>, RigidBodyHandle> {
        &self.forward_map
    }

    /// Returns a shared reference to inner backward mapping.
    pub fn backward_map(&self) -> &HashMap<RigidBodyHandle, Handle<Node>> {
        &self.backward_map
    }
}

impl Visit for PhysicsBinder {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.forward_map.visit("Map", visitor)?;
        if self.backward_map.visit("RevMap", visitor).is_err() {
            for (&n, &b) in self.forward_map.iter() {
                self.backward_map.insert(b, n);
            }
        }
        let _ = self.enabled.visit("Enabled", visitor);

        visitor.leave_region()
    }
}

/// Colored line between two points.
#[derive(Clone, Debug)]
pub struct Line {
    /// Beginning of the line.
    pub begin: Vector3<f32>,
    /// End of the line.    
    pub end: Vector3<f32>,
    /// Color of the line.
    pub color: Color,
}

/// Drawing context for simple graphics, it allows you to draw simple figures using
/// set of lines. Most common use is to draw some debug geometry in your game, draw
/// physics info (contacts, meshes, shapes, etc.), draw temporary geometry in editor
/// and so on.
#[derive(Default, Clone, Debug)]
pub struct SceneDrawingContext {
    /// List of lines to draw.
    pub lines: Vec<Line>,
}

impl SceneDrawingContext {
    /// Draws frustum with given color. Drawing is not immediate, it only pushes
    /// lines for frustum into internal buffer. It will be drawn later on in separate
    /// render pass.
    pub fn draw_frustum(&mut self, frustum: &Frustum, color: Color) {
        let left_top_front = frustum.left_top_front_corner();
        let left_bottom_front = frustum.left_bottom_front_corner();
        let right_bottom_front = frustum.right_bottom_front_corner();
        let right_top_front = frustum.right_top_front_corner();

        let left_top_back = frustum.left_top_back_corner();
        let left_bottom_back = frustum.left_bottom_back_corner();
        let right_bottom_back = frustum.right_bottom_back_corner();
        let right_top_back = frustum.right_top_back_corner();

        // Front face
        self.add_line(Line {
            begin: left_top_front,
            end: right_top_front,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: left_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_top_front,
            color,
        });

        // Back face
        self.add_line(Line {
            begin: left_top_back,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_back,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_back,
            end: left_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_back,
            end: left_top_back,
            color,
        });

        // Edges
        self.add_line(Line {
            begin: left_top_front,
            end: left_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_bottom_back,
            color,
        });
    }

    /// Draws axis-aligned bounding box with given color. Drawing is not immediate,
    /// it only pushes lines for bounding box into internal buffer. It will be drawn
    /// later on in separate render pass.
    pub fn draw_aabb(&mut self, aabb: &AxisAlignedBoundingBox, color: Color) {
        let left_bottom_front = Vector3::new(aabb.min.x, aabb.min.y, aabb.max.z);
        let left_top_front = Vector3::new(aabb.min.x, aabb.max.y, aabb.max.z);
        let right_top_front = Vector3::new(aabb.max.x, aabb.max.y, aabb.max.z);
        let right_bottom_front = Vector3::new(aabb.max.x, aabb.min.y, aabb.max.z);

        let left_bottom_back = Vector3::new(aabb.min.x, aabb.min.y, aabb.min.z);
        let left_top_back = Vector3::new(aabb.min.x, aabb.max.y, aabb.min.z);
        let right_top_back = Vector3::new(aabb.max.x, aabb.max.y, aabb.min.z);
        let right_bottom_back = Vector3::new(aabb.max.x, aabb.min.y, aabb.min.z);

        // Front face
        self.add_line(Line {
            begin: left_top_front,
            end: right_top_front,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: left_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_top_front,
            color,
        });

        // Back face
        self.add_line(Line {
            begin: left_top_back,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_back,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_back,
            end: left_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_back,
            end: left_top_back,
            color,
        });

        // Edges
        self.add_line(Line {
            begin: left_top_front,
            end: left_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_bottom_back,
            color,
        });
    }

    /// Draws object-oriented bounding box with given color. Drawing is not immediate,
    /// it only pushes lines for object-oriented bounding box into internal buffer. It
    /// will be drawn later on in separate render pass.
    pub fn draw_oob(
        &mut self,
        aabb: &AxisAlignedBoundingBox,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        let left_bottom_front = transform
            .transform_point(&Point3::new(aabb.min.x, aabb.min.y, aabb.max.z))
            .coords;
        let left_top_front = transform
            .transform_point(&Point3::new(aabb.min.x, aabb.max.y, aabb.max.z))
            .coords;
        let right_top_front = transform
            .transform_point(&Point3::new(aabb.max.x, aabb.max.y, aabb.max.z))
            .coords;
        let right_bottom_front = transform
            .transform_point(&Point3::new(aabb.max.x, aabb.min.y, aabb.max.z))
            .coords;

        let left_bottom_back = transform
            .transform_point(&Point3::new(aabb.min.x, aabb.min.y, aabb.min.z))
            .coords;
        let left_top_back = transform
            .transform_point(&Point3::new(aabb.min.x, aabb.max.y, aabb.min.z))
            .coords;
        let right_top_back = transform
            .transform_point(&Point3::new(aabb.max.x, aabb.max.y, aabb.min.z))
            .coords;
        let right_bottom_back = transform
            .transform_point(&Point3::new(aabb.max.x, aabb.min.y, aabb.min.z))
            .coords;

        // Front face
        self.add_line(Line {
            begin: left_top_front,
            end: right_top_front,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: left_bottom_front,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_top_front,
            color,
        });

        // Back face
        self.add_line(Line {
            begin: left_top_back,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_back,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_back,
            end: left_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_back,
            end: left_top_back,
            color,
        });

        // Edges
        self.add_line(Line {
            begin: left_top_front,
            end: left_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_top_front,
            end: right_top_back,
            color,
        });
        self.add_line(Line {
            begin: right_bottom_front,
            end: right_bottom_back,
            color,
        });
        self.add_line(Line {
            begin: left_bottom_front,
            end: left_bottom_back,
            color,
        });
    }

    /// Draws transform as basis vectors.
    pub fn draw_transform(&mut self, matrix: Matrix4<f32>) {
        let x = matrix.transform_vector(&Vector3::x());
        let y = matrix.transform_vector(&Vector3::y());
        let z = matrix.transform_vector(&Vector3::z());
        let origin = matrix.position();
        self.add_line(Line {
            begin: origin,
            end: origin + x,
            color: Color::RED,
        });
        self.add_line(Line {
            begin: origin,
            end: origin + y,
            color: Color::GREEN,
        });
        self.add_line(Line {
            begin: origin,
            end: origin + z,
            color: Color::BLUE,
        });
    }

    /// Draws a triangle by given points.
    pub fn draw_triangle(
        &mut self,
        a: Vector3<f32>,
        b: Vector3<f32>,
        c: Vector3<f32>,
        color: Color,
    ) {
        self.add_line(Line {
            begin: a,
            end: b,
            color,
        });
        self.add_line(Line {
            begin: b,
            end: c,
            color,
        });
        self.add_line(Line {
            begin: c,
            end: a,
            color,
        });
    }

    /// Draws a wire sphere with given parameters.
    pub fn draw_sphere(
        &mut self,
        position: Vector3<f32>,
        slices: usize,
        stacks: usize,
        radius: f32,
        color: Color,
    ) {
        let d_theta = std::f32::consts::PI / slices as f32;
        let d_phi = 2.0 * std::f32::consts::PI / stacks as f32;

        for i in 0..stacks {
            for j in 0..slices {
                let nj = j + 1;
                let ni = i + 1;

                let k0 = radius * (d_theta * i as f32).sin();
                let k1 = (d_phi * j as f32).cos();
                let k2 = (d_phi * j as f32).sin();
                let k3 = radius * (d_theta * i as f32).cos();

                let k4 = radius * (d_theta * ni as f32).sin();
                let k5 = (d_phi * nj as f32).cos();
                let k6 = (d_phi * nj as f32).sin();
                let k7 = radius * (d_theta * ni as f32).cos();

                if i != (stacks - 1) {
                    self.draw_triangle(
                        position + Vector3::new(k0 * k1, k0 * k2, k3),
                        position + Vector3::new(k4 * k1, k4 * k2, k7),
                        position + Vector3::new(k4 * k5, k4 * k6, k7),
                        color,
                    );
                }

                if i != 0 {
                    self.draw_triangle(
                        position + Vector3::new(k4 * k5, k4 * k6, k7),
                        position + Vector3::new(k0 * k5, k0 * k6, k3),
                        position + Vector3::new(k0 * k1, k0 * k2, k3),
                        color,
                    );
                }
            }
        }
    }

    /// Draws a wire sphere with given parameters.
    pub fn draw_sphere_section(
        &mut self,
        radius: f32,
        theta_range: Range<f32>,
        theta_steps: usize,
        phi_range: Range<f32>,
        phi_steps: usize,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        assert!(theta_range.start < theta_range.end);
        assert!(phi_range.start < phi_range.end);

        assert_ne!(phi_steps, 0);
        assert_ne!(theta_steps, 0);

        let theta_step = (theta_range.end - theta_range.start) / theta_steps as f32;
        let phi_step = (phi_range.end - phi_range.start) / phi_steps as f32;

        fn spherical_to_cartesian(radius: f32, theta: f32, phi: f32) -> Vector3<f32> {
            Vector3::new(
                radius * theta.sin() * phi.cos(),
                radius * theta.cos(),
                radius * theta.sin() * phi.sin(),
            )
        }

        let mut theta = theta_range.start;
        while theta < theta_range.end {
            let mut phi = phi_range.start;
            while phi < phi_range.end {
                let p0 = transform
                    .transform_point(&Point3::from(spherical_to_cartesian(radius, theta, phi)))
                    .coords;
                let p1 = transform
                    .transform_point(&Point3::from(spherical_to_cartesian(
                        radius,
                        theta,
                        phi + phi_step,
                    )))
                    .coords;
                let p2 = transform
                    .transform_point(&Point3::from(spherical_to_cartesian(
                        radius,
                        theta + theta_step,
                        phi + phi_step,
                    )))
                    .coords;
                let p3 = transform
                    .transform_point(&Point3::from(spherical_to_cartesian(
                        radius,
                        theta + theta_step,
                        phi,
                    )))
                    .coords;

                self.draw_triangle(p0, p1, p2, color);
                self.draw_triangle(p0, p2, p3, color);

                phi += phi_step;
            }
            theta += theta_step;
        }
    }

    /// Draws a wire cone with given parameters.
    pub fn draw_cone(
        &mut self,
        sides: usize,
        r: f32,
        h: f32,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        let d_phi = 2.0 * std::f32::consts::PI / sides as f32;

        let half_height = h / 2.0;

        for i in 0..sides {
            let nx0 = (d_phi * i as f32).cos();
            let ny0 = (d_phi * i as f32).sin();
            let nx1 = (d_phi * (i + 1) as f32).cos();
            let ny1 = (d_phi * (i + 1) as f32).sin();

            let x0 = r * nx0;
            let z0 = r * ny0;
            let x1 = r * nx1;
            let z1 = r * ny1;

            // back cap
            self.draw_triangle(
                transform
                    .transform_point(&Point3::new(0.0, -half_height, 0.0))
                    .coords,
                transform
                    .transform_point(&Point3::new(x0, -half_height, z0))
                    .coords,
                transform
                    .transform_point(&Point3::new(x1, -half_height, z1))
                    .coords,
                color,
            );

            // sides
            self.draw_triangle(
                transform
                    .transform_point(&Point3::new(0.0, half_height, 0.0))
                    .coords,
                transform
                    .transform_point(&Point3::new(x1, -half_height, z1))
                    .coords,
                transform
                    .transform_point(&Point3::new(x0, -half_height, z0))
                    .coords,
                color,
            );
        }
    }

    /// Draws a wire cylinder with given parameters.
    pub fn draw_cylinder(
        &mut self,
        sides: usize,
        r: f32,
        h: f32,
        caps: bool,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        let d_phi = 2.0 * std::f32::consts::PI / sides as f32;

        let half_height = h / 2.0;

        for i in 0..sides {
            let nx0 = (d_phi * i as f32).cos();
            let ny0 = (d_phi * i as f32).sin();
            let nx1 = (d_phi * (i + 1) as f32).cos();
            let ny1 = (d_phi * (i + 1) as f32).sin();

            let x0 = r * nx0;
            let z0 = r * ny0;
            let x1 = r * nx1;
            let z1 = r * ny1;

            if caps {
                // front cap
                self.draw_triangle(
                    transform
                        .transform_point(&Point3::new(x1, half_height, z1))
                        .coords,
                    transform
                        .transform_point(&Point3::new(x0, half_height, z0))
                        .coords,
                    transform
                        .transform_point(&Point3::new(0.0, half_height, 0.0))
                        .coords,
                    color,
                );

                // back cap
                self.draw_triangle(
                    transform
                        .transform_point(&Point3::new(x0, -half_height, z0))
                        .coords,
                    transform
                        .transform_point(&Point3::new(x1, -half_height, z1))
                        .coords,
                    transform
                        .transform_point(&Point3::new(0.0, -half_height, 0.0))
                        .coords,
                    color,
                );
            }

            // sides
            self.draw_triangle(
                transform
                    .transform_point(&Point3::new(x0, -half_height, z0))
                    .coords,
                transform
                    .transform_point(&Point3::new(x0, half_height, z0))
                    .coords,
                transform
                    .transform_point(&Point3::new(x1, -half_height, z1))
                    .coords,
                color,
            );

            self.draw_triangle(
                transform
                    .transform_point(&Point3::new(x1, -half_height, z1))
                    .coords,
                transform
                    .transform_point(&Point3::new(x0, half_height, z0))
                    .coords,
                transform
                    .transform_point(&Point3::new(x1, half_height, z1))
                    .coords,
                color,
            );
        }
    }

    /// Draws vertical capsule with given radius and height and then applies given transform.
    pub fn draw_capsule(
        &mut self,
        radius: f32,
        height: f32,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        // Top cap
        self.draw_sphere_section(
            radius,
            0.0..std::f32::consts::FRAC_PI_2,
            10,
            0.0..std::f32::consts::TAU,
            10,
            transform * Matrix4::new_translation(&Vector3::new(0.0, height * 0.5 - radius, 0.0)),
            color,
        );

        // Bottom cap
        self.draw_sphere_section(
            radius,
            std::f32::consts::PI..std::f32::consts::PI * 1.5,
            10,
            0.0..std::f32::consts::TAU,
            10,
            transform * Matrix4::new_translation(&Vector3::new(0.0, -height * 0.5 + radius, 0.0)),
            color,
        );

        let cylinder_height = height - 2.0 * radius;

        if cylinder_height > 0.0 {
            self.draw_cylinder(10, radius, cylinder_height, false, transform, color);
        }
    }

    /// Draws capsule between two points with given tesselation and then applies given transform to all points.
    pub fn draw_segment_capsule(
        &mut self,
        begin: Vector3<f32>,
        end: Vector3<f32>,
        radius: f32,
        v_segments: usize,
        h_segments: usize,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        let axis = end - begin;
        let length = axis.norm();

        let z_axis = axis
            .try_normalize(std::f32::EPSILON)
            .unwrap_or_else(Vector3::z);

        let y_axis = z_axis
            .cross(
                &(if z_axis.y != 0.0 || z_axis.z != 0.0 {
                    Vector3::x()
                } else {
                    Vector3::y()
                }),
            )
            .try_normalize(std::f32::EPSILON)
            .unwrap_or_else(Vector3::y);

        let x_axis = z_axis
            .cross(&y_axis)
            .try_normalize(std::f32::EPSILON)
            .unwrap_or_else(Vector3::x); // CHECK

        let shaft_point = |u: f32, v: f32| -> Vector3<f32> {
            transform
                .transform_point(&Point3::from(
                    begin
                        + x_axis.scale((std::f32::consts::TAU * u).cos() * radius)
                        + y_axis.scale((std::f32::consts::TAU * u).sin() * radius)
                        + z_axis.scale(v * length),
                ))
                .coords
        };

        let start_hemisphere_point = |u: f32, v: f32| -> Vector3<f32> {
            let latitude = std::f32::consts::FRAC_PI_2 * (v - 1.0);
            transform
                .transform_point(&Point3::from(
                    begin
                        + x_axis.scale((std::f32::consts::TAU * u).cos() * latitude.cos() * radius)
                        + y_axis.scale((std::f32::consts::TAU * u).sin() * latitude.cos() * radius)
                        + z_axis.scale(latitude.sin() * radius),
                ))
                .coords
        };

        let end_hemisphere_point = |u: f32, v: f32| -> Vector3<f32> {
            let latitude = std::f32::consts::FRAC_PI_2 * v;
            transform
                .transform_point(&Point3::from(
                    end + x_axis.scale((std::f32::consts::TAU * u).cos() * latitude.cos() * radius)
                        + y_axis.scale((std::f32::consts::TAU * u).sin() * latitude.cos() * radius)
                        + z_axis.scale(latitude.sin() * radius),
                ))
                .coords
        };

        let dv = 1.0 / h_segments as f32;
        let du = 1.0 / v_segments as f32;

        let mut u = 0.0;
        while u < 1.0 {
            let sa = shaft_point(u, 0.0);
            let sb = shaft_point(u, 1.0);
            let sc = shaft_point(u + du, 1.0);
            let sd = shaft_point(u + du, 0.0);

            self.draw_triangle(sa, sb, sc, color);
            self.draw_triangle(sa, sc, sd, color);

            u += du;
        }

        u = 0.0;
        while u < 1.0 {
            let mut v = 0.0;
            while v < 1.0 {
                let sa = start_hemisphere_point(u, v);
                let sb = start_hemisphere_point(u, v + dv);
                let sc = start_hemisphere_point(u + du, v + dv);
                let sd = start_hemisphere_point(u + du, v);

                self.draw_triangle(sa, sb, sc, color);
                self.draw_triangle(sa, sc, sd, color);

                let ea = end_hemisphere_point(u, v);
                let eb = end_hemisphere_point(u, v + dv);
                let ec = end_hemisphere_point(u + du, v + dv);
                let ed = end_hemisphere_point(u + du, v);

                self.draw_triangle(ea, eb, ec, color);
                self.draw_triangle(ea, ec, ed, color);

                v += dv;
            }

            u += du;
        }
    }

    /// Adds single line into internal buffer.
    pub fn add_line(&mut self, line: Line) {
        self.lines.push(line);
    }

    /// Removes all lines from internal buffer. For dynamic drawing you should call it
    /// every update tick of your application.
    pub fn clear_lines(&mut self) {
        self.lines.clear()
    }
}

/// A container for navigational meshes.
#[derive(Default, Clone, Debug)]
pub struct NavMeshContainer {
    pool: Pool<Navmesh>,
}

impl NavMeshContainer {
    /// Adds new navigational mesh to the container and returns its handle.
    pub fn add(&mut self, navmesh: Navmesh) -> Handle<Navmesh> {
        self.pool.spawn(navmesh)
    }

    /// Removes navigational mesh by its handle.
    pub fn remove(&mut self, handle: Handle<Navmesh>) -> Navmesh {
        self.pool.free(handle)
    }

    /// Creates new immutable iterator.
    pub fn iter(&self) -> impl Iterator<Item = &Navmesh> {
        self.pool.iter()
    }

    /// Creates new immutable iterator.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Navmesh> {
        self.pool.iter_mut()
    }

    /// Creates a handle to navmesh from its index.
    pub fn handle_from_index(&self, i: usize) -> Handle<Navmesh> {
        self.pool.handle_from_index(i)
    }

    /// Destroys all navmeshes. All handles will become invalid.
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    /// Checks if given handle is valid.
    pub fn is_valid_handle(&self, handle: Handle<Navmesh>) -> bool {
        self.pool.is_valid_handle(handle)
    }
}

impl Index<Handle<Navmesh>> for NavMeshContainer {
    type Output = Navmesh;

    fn index(&self, index: Handle<Navmesh>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Navmesh>> for NavMeshContainer {
    fn index_mut(&mut self, index: Handle<Navmesh>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}

impl Visit for NavMeshContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}

/// See module docs.
#[derive(Debug)]
pub struct Scene {
    /// Graph is main container for all scene nodes. It calculates global transforms for nodes,
    /// updates them and performs all other important work. See `graph` module docs for more
    /// info.
    pub graph: Graph,

    /// Animations container controls all animation on scene. Each animation can have tracks which
    /// has handles to graph nodes. See `animation` module docs for more info.
    pub animations: AnimationContainer,

    /// Physics world. Allows you create various physics objects such as static geometries and
    /// rigid bodies. Rigid bodies then should be linked with graph nodes using binder.
    pub physics: Physics,

    /// Physics binder is a bridge between physics world and scene graph. If a rigid body is linked
    /// to a graph node, then rigid body will control local transform of node.
    pub physics_binder: PhysicsBinder,

    /// Texture to draw scene to. If empty, scene will be drawn on screen directly.
    /// It is useful to "embed" some scene into other by drawing a quad with this
    /// texture. This can be used to make in-game video conference - you can make
    /// separate scene with your characters and draw scene into texture, then in
    /// main scene you can attach this texture to some quad which will be used as
    /// monitor. Other usage could be previewer of models, like pictogram of character
    /// in real-time strategies, in other words there are plenty of possible uses.
    pub render_target: Option<Texture>,

    /// Drawing context for simple graphics.
    pub drawing_context: SceneDrawingContext,

    /// A sound context that holds all sound sources, effects, etc. belonging to the scene.
    pub sound_context: Context,

    /// A container for navigational meshes.
    pub navmeshes: NavMeshContainer,

    /// Current lightmap.
    lightmap: Option<Lightmap>,

    /// Performance statistics from last `update` call.
    pub performance_statistics: PerformanceStatistics,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            animations: Default::default(),
            physics: Default::default(),
            physics_binder: Default::default(),
            render_target: None,
            lightmap: None,
            drawing_context: Default::default(),
            sound_context: Default::default(),
            navmeshes: Default::default(),
            performance_statistics: Default::default(),
        }
    }
}

fn map_texture(tex: Option<Texture>, rm: ResourceManager) -> Option<Texture> {
    if let Some(shallow_texture) = tex {
        let shallow_texture = shallow_texture.state();
        Some(rm.request_texture(shallow_texture.path()))
    } else {
        None
    }
}

/// A structure that holds times that specific update step took.
#[derive(Copy, Clone, Default, Debug)]
pub struct PerformanceStatistics {
    /// A time (in seconds) which was required to update physics.
    pub physics_time: f32,

    /// A time (in seconds) which was required to update graph.
    pub graph_update_time: f32,

    /// A time (in seconds) which was required to update animations.
    pub animations_update_time: f32,
}

impl Display for PerformanceStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Physics: {} ms\nGraph: {} ms\nAnimations: {} ms",
            self.physics_time * 1000.0,
            self.graph_update_time * 1000.0,
            self.animations_update_time * 1000.0
        )
    }
}

impl Scene {
    /// Creates new scene with single root node.
    ///
    /// # Notes
    ///
    /// This method differs from Default trait implementation! Scene::default() creates
    /// empty graph with no nodes.
    #[inline]
    pub fn new() -> Self {
        Self {
            // Graph must be created with `new` method because it differs from `default`
            graph: Graph::new(),
            physics: Default::default(),
            animations: Default::default(),
            physics_binder: Default::default(),
            render_target: None,
            lightmap: None,
            drawing_context: Default::default(),
            sound_context: Context::new(),
            navmeshes: Default::default(),
            performance_statistics: Default::default(),
        }
    }

    /// Tries to load scene from given file. File can contain any scene in native engine format.
    /// Such scenes can be made in rusty editor.
    pub async fn from_file<P: AsRef<Path>>(
        path: P,
        resource_manager: ResourceManager,
    ) -> Result<Self, VisitError> {
        let mut scene = Scene::default();
        {
            let mut visitor = Visitor::load_binary(path.as_ref())?;
            scene.visit("Scene", &mut visitor)?;
        }

        // Collect all used resources and wait for them.
        let mut resources = Vec::new();
        for node in scene.graph.linear_iter_mut() {
            if let Some(shallow_resource) = node.resource.clone() {
                let resource = resource_manager
                    .clone()
                    .request_model(&shallow_resource.state().path());
                node.resource = Some(resource.clone());
                resources.push(resource);
            }
        }

        let _ = futures::future::join_all(resources).await;

        // Restore pointers to resources. Scene saves only paths to resources, here we must
        // find real resources instead.

        for node in scene.graph.linear_iter_mut() {
            match node {
                Node::Mesh(mesh) => {
                    for surface in mesh.surfaces_mut() {
                        surface.set_diffuse_texture(map_texture(
                            surface.diffuse_texture(),
                            resource_manager.clone(),
                        ));

                        surface.set_normal_texture(map_texture(
                            surface.normal_texture(),
                            resource_manager.clone(),
                        ));

                        surface.set_specular_texture(map_texture(
                            surface.specular_texture(),
                            resource_manager.clone(),
                        ));

                        surface.set_roughness_texture(map_texture(
                            surface.roughness_texture(),
                            resource_manager.clone(),
                        ));

                        // Do not resolve lightmap texture here, it makes no sense anyway,
                        // it will be resolved below.
                    }
                }
                Node::Sprite(sprite) => {
                    sprite.set_texture(map_texture(sprite.texture(), resource_manager.clone()));
                }
                Node::ParticleSystem(particle_system) => {
                    particle_system.set_texture(map_texture(
                        particle_system.texture(),
                        resource_manager.clone(),
                    ));
                }
                Node::Camera(camera) => {
                    camera.set_environment(map_texture(
                        camera.environment_map(),
                        resource_manager.clone(),
                    ));

                    if let Some(skybox) = camera.skybox_mut() {
                        skybox.bottom =
                            map_texture(skybox.bottom.clone(), resource_manager.clone());
                        skybox.top = map_texture(skybox.top.clone(), resource_manager.clone());
                        skybox.left = map_texture(skybox.left.clone(), resource_manager.clone());
                        skybox.right = map_texture(skybox.right.clone(), resource_manager.clone());
                        skybox.front = map_texture(skybox.front.clone(), resource_manager.clone());
                        skybox.back = map_texture(skybox.back.clone(), resource_manager.clone());
                    }
                }
                _ => (),
            }
        }

        if let Some(lightmap) = scene.lightmap.as_mut() {
            for entries in lightmap.map.values_mut() {
                for entry in entries.iter_mut() {
                    entry.texture = map_texture(entry.texture.clone(), resource_manager.clone());
                }
            }
        }

        // And do resolve to extract correct graphical data and so on.
        scene.resolve();

        Ok(scene)
    }

    fn update_physics(&mut self) {
        self.physics.step();

        // Keep pair when node and body are both alive.
        let graph = &mut self.graph;
        let physics = &mut self.physics;
        self.physics_binder.forward_map.retain(|node, body| {
            graph.is_valid_handle(*node) && physics.bodies.contains(body.clone().into())
        });

        // Sync node positions with assigned physics bodies
        if self.physics_binder.enabled {
            for (&node_handle, &body) in self.physics_binder.forward_map.iter() {
                let body = physics.bodies.get_mut(body.into()).unwrap();
                let node = &mut self.graph[node_handle];
                match node.physics_binding {
                    PhysicsBinding::NodeWithBody => {
                        node.local_transform_mut()
                            .set_position(body.position().translation.vector)
                            .set_rotation(body.position().rotation);
                    }
                    PhysicsBinding::BodyWithNode => {
                        let (r, p) = self.graph.isometric_global_rotation_position(node_handle);
                        body.set_position(
                            Isometry3 {
                                rotation: r,
                                translation: Translation { vector: p },
                            },
                            true,
                        );
                    }
                }
            }
        }
    }

    /// Removes node from scene with all associated entities, like animations etc. This method
    /// should be used all times instead of [Graph::remove_node](crate::scene::graph::Graph::remove_node),     
    ///
    /// # Panics
    ///
    /// Panics if handle is invalid.
    pub fn remove_node(&mut self, handle: Handle<Node>) {
        for descendant in self.graph.traverse_handle_iter(handle) {
            // Remove all associated animations.
            self.animations.retain(|animation| {
                for track in animation.get_tracks() {
                    if track.get_node() == descendant {
                        return false;
                    }
                }
                true
            });

            // Remove all associated physical bodies.
            if let Some(body) = self.physics_binder.body_of(descendant) {
                self.physics.remove_body(body);
                self.physics_binder.unbind(descendant);
            }
        }

        self.graph.remove_node(handle)
    }

    pub(in crate) fn resolve(&mut self) {
        Log::writeln(MessageKind::Information, "Starting resolve...".to_owned());

        self.graph.resolve();
        self.animations.resolve(&self.graph);

        self.graph.update_hierarchical_data();
        self.physics.resolve(&self.physics_binder, &self.graph);

        // Re-apply lightmap if any. This has to be done after resolve because we must patch surface
        // data at this stage, but if we'd do this before we wouldn't be able to do this because
        // meshes contains invalid surface data.
        if let Some(lightmap) = self.lightmap.as_mut() {
            // Patch surface data first. To do this we gather all surface data instances and
            // look in patch data if we have patch for data.
            let mut unique_data_set = HashMap::new();
            for &handle in lightmap.map.keys() {
                if let Node::Mesh(mesh) = &mut self.graph[handle] {
                    for surface in mesh.surfaces() {
                        let data = surface.data();
                        let key = &*data as *const _ as u64;
                        unique_data_set.entry(key).or_insert(data);
                    }
                }
            }

            for (_, data) in unique_data_set.into_iter() {
                let mut data = data.write().unwrap();
                if let Some(patch) = lightmap.patches.get(&data.id()) {
                    data.triangles = patch.triangles.clone();
                    for &v in patch.additional_vertices.iter() {
                        let vertex = data.vertices[v as usize];
                        data.vertices.push(vertex);
                    }
                    assert_eq!(data.vertices.len(), patch.second_tex_coords.len());
                    for (v, &tex_coord) in
                        data.vertices.iter_mut().zip(patch.second_tex_coords.iter())
                    {
                        v.second_tex_coord = tex_coord;
                    }
                } else {
                    Log::writeln(
                        MessageKind::Warning,
                        "Failed to get surface data patch while resolving lightmap!\
                    This means that surface has changed and lightmap must be regenerated!"
                            .to_owned(),
                    );
                }
            }

            // Apply textures.
            for (&handle, entries) in lightmap.map.iter_mut() {
                if let Node::Mesh(mesh) = &mut self.graph[handle] {
                    for (entry, surface) in entries.iter_mut().zip(mesh.surfaces_mut()) {
                        surface.set_lightmap_texture(entry.texture.clone());
                    }
                }
            }
        }

        Log::writeln(MessageKind::Information, "Resolve succeeded!".to_owned());
    }

    /// Tries to set new lightmap to scene.
    pub fn set_lightmap(&mut self, lightmap: Lightmap) -> Result<Option<Lightmap>, &'static str> {
        // Assign textures to surfaces.
        for (handle, lightmaps) in lightmap.map.iter() {
            if let Node::Mesh(mesh) = &mut self.graph[*handle] {
                if mesh.surfaces().len() != lightmaps.len() {
                    return Err("failed to set lightmap, surface count mismatch");
                }

                for (surface, entry) in mesh.surfaces_mut().iter_mut().zip(lightmaps) {
                    // This unwrap() call must never panic in normal conditions, because texture wrapped in Option
                    // only to implement Default trait to be serializable.
                    let texture = entry.texture.clone().unwrap();
                    surface.set_lightmap_texture(Some(texture))
                }
            }
        }
        Ok(std::mem::replace(&mut self.lightmap, Some(lightmap)))
    }

    /// Performs single update tick with given delta time from last frame. Internally
    /// it updates physics, animations, and each graph node. In most cases there is
    /// no need to call it directly, engine automatically updates all available scenes.
    pub fn update(&mut self, frame_size: Vector2<f32>, dt: f32) {
        let last = std::time::Instant::now();
        self.update_physics();
        self.performance_statistics.physics_time = (std::time::Instant::now() - last).as_secs_f32();

        let last = std::time::Instant::now();
        self.animations.update_animations(dt);
        self.performance_statistics.animations_update_time =
            (std::time::Instant::now() - last).as_secs_f32();

        let last = std::time::Instant::now();
        self.graph.update_nodes(frame_size, dt);
        self.performance_statistics.graph_update_time =
            (std::time::Instant::now() - last).as_secs_f32();
    }

    /// Creates deep copy of a scene, filter predicate allows you to filter out nodes
    /// by your criteria.
    pub fn clone<F>(&self, filter: &mut F) -> (Self, HashMap<Handle<Node>, Handle<Node>>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let (graph, old_new_map) = self.graph.clone(filter);
        let mut animations = self.animations.clone();
        for animation in animations.iter_mut() {
            // Remove all tracks for nodes that were filtered out.
            animation.retain_tracks(|track| old_new_map.contains_key(&track.get_node()));
            // Remap track nodes.
            for track in animation.get_tracks_mut() {
                track.set_node(old_new_map[&track.get_node()]);
            }
        }
        // It is ok to use old binder here, because handles maps one-to-one.
        let physics = self.physics.deep_copy(&self.physics_binder, &graph);
        let mut physics_binder = PhysicsBinder::default();
        for (node, &body) in self.physics_binder.forward_map.iter() {
            // Make sure we bind existing node with new physical body.
            if let Some(&new_node) = old_new_map.get(node) {
                // Re-use of body handle is fine here because physics copy bodies
                // directly and handles from previous pool is still suitable for copy.
                physics_binder.bind(new_node, body);
            }
        }
        (
            Self {
                graph,
                animations,
                physics,
                physics_binder,
                // Render target is intentionally not copied, because it does not makes sense - a copy
                // will redraw frame completely.
                render_target: Default::default(),
                lightmap: self.lightmap.clone(),
                drawing_context: self.drawing_context.clone(),
                sound_context: self.sound_context.deep_clone(),
                navmeshes: self.navmeshes.clone(),
                performance_statistics: Default::default(),
            },
            old_new_map,
        )
    }
}

impl Visit for Scene {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        self.physics_binder.visit("PhysicsBinder", visitor)?;
        self.graph.visit("Graph", visitor)?;
        self.animations.visit("Animations", visitor)?;
        self.physics.visit("Physics", visitor)?;
        let _ = self.lightmap.visit("Lightmap", visitor);
        let _ = self.sound_context.visit("SoundContext", visitor);
        let _ = self.navmeshes.visit("NavMeshes", visitor);
        // Backward compatibility.
        if self.sound_context.is_invalid() {
            self.sound_context = Context::new();
        }
        visitor.leave_region()
    }
}

/// Container for scenes in the engine.
#[derive(Default)]
pub struct SceneContainer {
    pool: Pool<Scene>,
    sound_engine: Arc<Mutex<SoundEngine>>,
}

impl SceneContainer {
    pub(in crate) fn new(sound_engine: Arc<Mutex<SoundEngine>>) -> Self {
        Self {
            pool: Pool::new(),
            sound_engine,
        }
    }

    /// Returns pair iterator which yields (handle, scene_ref) pairs.
    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Scene>, &Scene)> {
        self.pool.pair_iter()
    }

    /// Creates new iterator over scenes in container.
    #[inline]
    pub fn iter(&self) -> PoolIterator<Scene> {
        self.pool.iter()
    }

    /// Creates new mutable iterator over scenes in container.
    #[inline]
    pub fn iter_mut(&mut self) -> PoolIteratorMut<Scene> {
        self.pool.iter_mut()
    }

    /// Adds new scene into container.
    #[inline]
    pub fn add(&mut self, scene: Scene) -> Handle<Scene> {
        self.sound_engine
            .lock()
            .unwrap()
            .add_context(scene.sound_context.clone());
        self.pool.spawn(scene)
    }

    /// Removes all scenes from container.
    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    /// Removes given scene from container.
    #[inline]
    pub fn remove(&mut self, handle: Handle<Scene>) {
        self.sound_engine
            .lock()
            .unwrap()
            .remove_context(self.pool[handle].sound_context.clone());
        self.pool.free(handle);
    }
}

impl Index<Handle<Scene>> for SceneContainer {
    type Output = Scene;

    #[inline]
    fn index(&self, index: Handle<Scene>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Scene>> for SceneContainer {
    #[inline]
    fn index_mut(&mut self, index: Handle<Scene>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}

impl Visit for SceneContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;
        let _ = self.sound_engine.visit("SoundEngine", visitor);

        visitor.leave_region()
    }
}

/// Visibility cache stores information about objects visibility for a single frame.
/// Allows you to quickly check if object is visible or not.
#[derive(Default, Debug)]
pub struct VisibilityCache {
    map: HashMap<Handle<Node>, bool>,
}

impl From<HashMap<Handle<Node>, bool>> for VisibilityCache {
    fn from(map: HashMap<Handle<Node>, bool>) -> Self {
        Self { map }
    }
}

impl VisibilityCache {
    /// Replaces internal map with empty and returns previous value. This trick is useful
    /// to reuse hash map to prevent redundant memory allocations.
    pub fn invalidate(&mut self) -> HashMap<Handle<Node>, bool> {
        std::mem::take(&mut self.map)
    }

    /// Updates visibility cache - checks visibility for each node in given graph, also performs
    /// frustum culling if frustum specified.
    pub fn update(
        &mut self,
        graph: &Graph,
        view_matrix: Matrix4<f32>,
        z_far: f32,
        frustum: Option<&Frustum>,
    ) {
        self.map.clear();

        let view_position = view_matrix.position();

        // Check LODs first, it has priority over other visibility settings.
        for node in graph.linear_iter() {
            if let Some(lod_group) = node.lod_group() {
                for level in lod_group.levels.iter() {
                    for &object in level.objects.iter() {
                        let normalized_distance =
                            view_position.metric_distance(&graph[object].global_position()) / z_far;
                        let visible = normalized_distance >= level.begin()
                            && normalized_distance <= level.end();
                        self.map.insert(object, visible);
                    }
                }
            }
        }

        // Fill rest of data from global visibility flag of nodes.
        for (handle, node) in graph.pair_iter() {
            // We care only about meshes.
            if let Node::Mesh(mesh) = node {
                // We need to fill only unfilled entries, none of visibility flags of a node can
                // make it visible again if lod group hid it.
                self.map.entry(handle).or_insert_with(|| {
                    let mut visibility = node.global_visibility();
                    if visibility {
                        if let Some(frustum) = frustum {
                            visibility = mesh.is_intersect_frustum(graph, frustum);
                        }
                    }
                    visibility
                });
            }
        }
    }

    /// Checks if given node is visible or not.
    pub fn is_visible(&self, node: Handle<Node>) -> bool {
        self.map.get(&node).cloned().unwrap_or(false)
    }
}

/// A wrapper for a variable that hold additional flag that tells that
/// initial value was changed in runtime.
#[derive(Debug)]
pub struct TemplateVariable<T> {
    /// Actual value.
    value: T,

    /// A marker that tells that initial value was changed.
    custom: bool,
}

impl<T: Clone> Clone for TemplateVariable<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            custom: self.custom,
        }
    }
}

impl<T: PartialEq> PartialEq for TemplateVariable<T> {
    fn eq(&self, other: &Self) -> bool {
        // `custom` flag intentionally ignored!
        self.value.eq(&other.value)
    }
}

impl<T: Eq> Eq for TemplateVariable<T> {}

impl<T: Copy> Copy for TemplateVariable<T> {}

impl<T: Default> Default for TemplateVariable<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            custom: false,
        }
    }
}

impl<T: Clone> TemplateVariable<T> {
    /// Clones wrapped value.
    pub fn clone_inner(&self) -> T {
        self.value.clone()
    }
}

impl<T> TemplateVariable<T> {
    /// Creates new non-custom variable from given value.
    pub fn new(value: T) -> Self {
        Self {
            value,
            custom: false,
        }
    }

    /// Creates new custom variable from given value.
    pub fn new_custom(value: T) -> Self {
        Self {
            value,
            custom: true,
        }
    }

    /// Replaces value and also raises the `custom` flag.
    pub fn set(&mut self, value: T) -> T {
        self.custom = true;
        std::mem::replace(&mut self.value, value)
    }

    /// Returns a reference to wrapped value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Returns true if value has changed.
    pub fn is_custom(&self) -> bool {
        self.custom
    }
}

impl<T> Deref for TemplateVariable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Visit for TemplateVariable<T>
where
    T: Visit,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.value.visit("Value", visitor)?;
        self.custom.visit("IsCustom", visitor)?;

        visitor.leave_region()
    }
}
