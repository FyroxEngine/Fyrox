extern crate rg3d_core;
#[macro_use]
extern crate bitflags;

use rg3d_core::{
    math::{
        vec3::Vec3,
        ray::Ray,
    },
    pool::{
        Pool,
        Handle,
    },
    visitor::{
        Visit,
        VisitResult,
        Visitor,
    },
};
use std::{
    cmp::Ordering,
    cell::RefCell
};
use crate::{
    rigid_body::RigidBody,
    static_geometry::StaticGeometry,
    convex_shape::{
        ConvexShape,
        CircumRadius
    }
};
use rg3d_core::pool::Ticket;

pub mod gjk_epa;
pub mod convex_shape;
pub mod rigid_body;
pub mod contact;
pub mod static_geometry;

pub enum HitKind {
    Body(Handle<RigidBody>),
    StaticTriangle {
        static_geometry: Handle<StaticGeometry>,
        triangle_index: usize,
    },
}

pub struct RayCastOptions {
    pub ignore_bodies: bool,
    pub ignore_static_geometries: bool,
    pub sort_results: bool,
}

impl Default for RayCastOptions {
    fn default() -> Self {
        Self {
            ignore_bodies: false,
            ignore_static_geometries: false,
            sort_results: true,
        }
    }
}

pub struct RayCastResult {
    pub kind: HitKind,
    pub position: Vec3,
    pub normal: Vec3,
    pub sqr_distance: f32,
}

#[derive(Debug)]
pub struct Physics {
    bodies: Pool<RigidBody>,
    static_geoms: Pool<StaticGeometry>,
    query_buffer: RefCell<Vec<u32>>,
    enabled: bool
}

impl Visit for Physics {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.bodies.visit("Bodies", visitor)?;
        self.static_geoms.visit("StaticGeoms", visitor)?;
        let _ = self.enabled.visit("Enabled", visitor); // let _ for backward compatibility.

        visitor.leave_region()
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Physics {
    fn clone(&self) -> Self {
        Self {
            bodies: self.bodies.clone(),
            static_geoms: self.static_geoms.clone(),
            query_buffer: Default::default(),
            enabled: self.enabled
        }
    }
}

impl Physics {
    pub fn new() -> Self {
        Self {
            bodies: Pool::new(),
            static_geoms: Pool::new(),
            query_buffer: Default::default(),
            enabled: true
        }
    }

    pub fn add_body(&mut self, body: RigidBody) -> Handle<RigidBody> {
        self.bodies.spawn(body)
    }

    pub fn remove_body(&mut self, body_handle: Handle<RigidBody>) {
        self.bodies.free(body_handle);
    }

    pub fn add_static_geometry(&mut self, static_geom: StaticGeometry) -> Handle<StaticGeometry> {
        self.static_geoms.spawn(static_geom)
    }

    pub fn borrow_static_geometry(&self, static_geom: Handle<StaticGeometry>) -> &StaticGeometry {
        &self.static_geoms[static_geom]
    }

    pub fn borrow_static_geometry_mut(&mut self, static_geom: Handle<StaticGeometry>) -> &mut StaticGeometry {
        &mut self.static_geoms[static_geom]
    }

    pub fn is_static_geometry_handle_valid(&self, static_geom: Handle<StaticGeometry>) -> bool {
        self.static_geoms.is_valid_handle(static_geom)
    }

    pub fn remove_static_geometry(&mut self, static_geom: Handle<StaticGeometry>) {
        self.static_geoms.free(static_geom);
    }

    pub fn borrow_body(&self, handle: Handle<RigidBody>) -> &RigidBody {
        self.bodies.borrow(handle)
    }

    pub fn borrow_body_mut(&mut self, handle: Handle<RigidBody>) -> &mut RigidBody {
        self.bodies.borrow_mut(handle)
    }

    pub fn is_valid_body_handle(&self, handle: Handle<RigidBody>) -> bool {
        self.bodies.is_valid_handle(handle)
    }

    pub fn take_reserve_body(&mut self, handle: Handle<RigidBody>) -> (Ticket<RigidBody>, RigidBody) {
        self.bodies.take_reserve(handle)
    }

    pub fn put_body_back(&mut self, ticket: Ticket<RigidBody>, body: RigidBody) -> Handle<RigidBody> {
        self.bodies.put_back(ticket, body)
    }

    pub fn forget_ticket(&mut self, ticket: Ticket<RigidBody>) {
        self.bodies.forget_ticket(ticket)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn step(&mut self, delta_time: f32) {
        if !self.enabled {
            return;
        }

        let dt2 = delta_time * delta_time;
        let air_friction = 0.003;

        // Take second mutable reference to bodies, this is safe because:
        // 1. We won't modify collection while iterating over it.
        // 2. Simultaneous access to a body won't happen because of
        //    pointer equality check down below.
        let other_bodies = unsafe { &mut *(&mut self.bodies as *mut Pool<RigidBody>) };

        for (body_handle, body) in self.bodies.pair_iter_mut() {
            if let Some(ref mut lifetime) = body.lifetime {
                *lifetime -= delta_time;
            }

            body.acceleration += body.gravity;
            body.verlet(dt2, air_friction);

            body.contacts.clear();

            for (other_body_handle, other_body) in other_bodies.pair_iter_mut() {
                // Enforce borrowing rules at runtime.
                if !std::ptr::eq(body, other_body) &&
                    ((other_body.collision_group & body.collision_mask) != 0) &&
                    ((body.collision_group & other_body.collision_mask) != 0) {
                    body.solve_rigid_body_collision(body_handle,other_body, other_body_handle);
                }
            }

            for (handle, static_geometry) in self.static_geoms.pair_iter() {
                let mut query_buffer = self.query_buffer.borrow_mut();
                static_geometry.octree.sphere_query(body.position, body.shape.circumradius(), &mut query_buffer);

                for n in query_buffer.iter().map(|i| *i as usize) {
                    let triangle = static_geometry.triangles.get(n).unwrap();
                    body.solve_triangle_collision(&triangle, n, handle);
                }
            }
        }

        self.bodies.retain(|body| body.lifetime.is_none() || body.lifetime.unwrap() > 0.0);
    }

    pub fn ray_cast(&self, ray: &Ray, options: RayCastOptions, result: &mut Vec<RayCastResult>) -> bool {
        result.clear();

        // Check bodies
        if !options.ignore_bodies {
            for body_index in 0..self.bodies.get_capacity() {
                let body = if let Some(body) = self.bodies.at(body_index) {
                    body
                } else {
                    continue;
                };

                let body_handle = self.bodies.handle_from_index(body_index);

                match &body.shape {
                    ConvexShape::Dummy => {}
                    ConvexShape::Box(box_shape) => {
                        if let Some(points) = ray.box_intersection_points(&box_shape.get_min(), &box_shape.get_max()) {
                            for point in points.iter() {
                                result.push(RayCastResult {
                                    kind: HitKind::Body(body_handle),
                                    position: *point,
                                    normal: *point - body.position, // TODO: Fix normal
                                    sqr_distance: point.sqr_distance(&ray.origin),
                                })
                            }
                        }
                    }
                    ConvexShape::Sphere(sphere_shape) => {
                        if let Some(points) = ray.sphere_intersection_points(&body.position, sphere_shape.radius) {
                            for point in points.iter() {
                                result.push(RayCastResult {
                                    kind: HitKind::Body(body_handle),
                                    position: *point,
                                    normal: *point - body.position,
                                    sqr_distance: point.sqr_distance(&ray.origin),
                                })
                            }
                        }
                    }
                    ConvexShape::Capsule(capsule_shape) => {
                        let (pa, pb) = capsule_shape.get_cap_centers();
                        let pa = pa + body.position;
                        let pb = pb + body.position;

                        if let Some(points) = ray.capsule_intersection(&pa, &pb, capsule_shape.get_radius()) {
                            for point in points.iter() {
                                result.push(RayCastResult {
                                    kind: HitKind::Body(body_handle),
                                    position: *point,
                                    normal: *point - body.position,
                                    sqr_distance: point.sqr_distance(&ray.origin),
                                })
                            }
                        }
                    }
                    ConvexShape::Triangle(triangle_shape) => {
                        if let Some(point) = ray.triangle_intersection(&triangle_shape.vertices) {
                            result.push(RayCastResult {
                                kind: HitKind::Body(body_handle),
                                position: point,
                                normal: triangle_shape.get_normal().unwrap(),
                                sqr_distance: point.sqr_distance(&ray.origin),
                            })
                        }
                    }
                    ConvexShape::PointCloud(_point_cloud) => {
                        // TODO: Implement this. This requires to build convex hull from point cloud first
                        // i.e. by gift wrapping algorithm or some other more efficient algorithms -
                        // https://dccg.upc.edu/people/vera/wp-content/uploads/2014/11/GA2014-ConvexHulls3D-Roger-Hernando.pdf
                    }
                }
            }
        }

        // Check static geometries
        if !options.ignore_static_geometries {
            for (handle, geom) in self.static_geoms.pair_iter() {
                let mut query_buffer = self.query_buffer.borrow_mut();
                geom.octree.ray_query(ray, &mut query_buffer);

                for triangle_index in query_buffer.iter().map(|i| *i as usize) {
                    let triangle = geom.triangles.get(triangle_index).unwrap();
                    if let Some(point) = ray.triangle_intersection(&triangle.points) {
                        result.push(RayCastResult {
                            kind: HitKind::StaticTriangle {
                                static_geometry: handle,
                                triangle_index,
                            },
                            position: point,
                            normal: triangle.plane.normal,
                            sqr_distance: point.sqr_distance(&ray.origin),
                        })
                    }
                }
            }
        }

        if options.sort_results {
            result.sort_by(|a, b| {
                if a.sqr_distance > b.sqr_distance {
                    Ordering::Greater
                } else if a.sqr_distance < b.sqr_distance {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
        }

        !result.is_empty()
    }
}
