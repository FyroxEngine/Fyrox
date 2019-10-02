extern crate rg3d_core;

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
use std::cmp::Ordering;
use crate::{
    rigid_body::RigidBody,
    static_geometry::StaticGeometry,
    convex_shape::ConvexShape
};

pub mod gjk_epa;
pub mod convex_shape;
pub mod rigid_body;
pub mod contact;
pub mod static_geometry;

pub enum HitKind {
    Body(Handle<RigidBody>),
    StaticTriangle {
        static_geometry: Handle<StaticGeometry>,
        triangle_index: usize
    }
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
            sort_results: true
        }
    }
}

pub struct RayCastResult {
    pub kind: HitKind,
    pub position: Vec3,
    pub normal: Vec3,
    pub sqr_distance: f32,
}

pub struct Physics {
    bodies: Pool<RigidBody>,
    static_geoms: Pool<StaticGeometry>,
}

impl Visit for Physics {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.bodies.visit("Bodies", visitor)?;
        self.static_geoms.visit("StaticGeoms", visitor)?;

        visitor.leave_region()
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

impl Physics {
    pub fn new() -> Physics {
        Physics {
            bodies: Pool::new(),
            static_geoms: Pool::new(),
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

    pub fn remove_static_geometry(&mut self, static_geom: Handle<StaticGeometry>) {
        self.static_geoms.free(static_geom);
    }

    pub fn borrow_body(&self, handle: Handle<RigidBody>) -> Option<&RigidBody> {
        self.bodies.borrow(handle)
    }

    pub fn borrow_body_mut(&mut self, handle: Handle<RigidBody>) -> Option<&mut RigidBody> {
        self.bodies.borrow_mut(handle)
    }

    pub fn step(&mut self, delta_time: f32) {
        let dt2 = delta_time * delta_time;
        let air_friction = 0.003;

        for body in self.bodies.iter_mut() {
            body.acceleration += body.gravity;
            body.verlet(dt2, air_friction);

            body.contacts.clear();

            for static_geometry in self.static_geoms.iter() {
                for (n, triangle) in static_geometry.triangles.iter().enumerate() {
                    body.solve_triangle_collision(&triangle, n);
                }
            }
        }
    }

    pub fn ray_cast(&self, ray: &Ray, options: RayCastOptions, result: &mut Vec<RayCastResult>) -> bool {
        result.clear();

        /* Check bodies */
        if !options.ignore_bodies {
            for body_index in 0..self.bodies.get_capacity() {
                let body = if let Some(body) = self.bodies.at(body_index) {
                    body
                } else {
                    continue;
                };

                let body_handle = self.bodies.handle_from_index(body_index);

                match &body.shape {
                    ConvexShape::Dummy => {},
                    ConvexShape::Box(box_shape) => {
                        if let Some(points) = ray.box_intersection_points(&box_shape.get_min(), &box_shape.get_max()) {
                            for point in points.iter() {
                                result.push(RayCastResult {
                                    kind: HitKind::Body(body_handle),
                                    position: *point,
                                    normal: *point - body.position, // TODO: Fix normal
                                    sqr_distance: point.sqr_distance(&ray.origin)
                                })
                            }
                        }
                    },
                    ConvexShape::Sphere(sphere_shape) => {
                        if let Some(points) = ray.sphere_intersection_points(&body.position, sphere_shape.radius) {
                            for point in points.iter() {
                                result.push(RayCastResult {
                                    kind: HitKind::Body(body_handle),
                                    position: *point,
                                    normal: *point - body.position,
                                    sqr_distance: point.sqr_distance(&ray.origin)
                                })
                            }
                        }
                    },
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
                                    sqr_distance: point.sqr_distance(&ray.origin)
                                })
                            }
                        }
                    },
                    ConvexShape::Triangle(triangle_shape) => {
                        if let Some(point) = ray.triangle_intersection(&triangle_shape.vertices) {
                            result.push(RayCastResult {
                                kind: HitKind::Body(body_handle),
                                position: point,
                                normal: triangle_shape.get_normal().unwrap(),
                                sqr_distance: point.sqr_distance(&ray.origin)
                            })
                        }
                    },
                    ConvexShape::PointCloud(_point_cloud) => {
                        // TODO: Implement this. This requires to build convex hull from point cloud first
                        // i.e. by gift wrapping algorithm or some other more efficient algorithms -
                        // https://dccg.upc.edu/people/vera/wp-content/uploads/2014/11/GA2014-ConvexHulls3D-Roger-Hernando.pdf
                    },
                }
            }
        }

        /* Check static geometries */
        if !options.ignore_static_geometries {
            for index in 0..self.static_geoms.get_capacity() {
                let geom = if let Some(geom) = self.static_geoms.at(index) {
                    geom
                } else {
                    continue;
                };

                for (triangle_index, triangle) in geom.triangles.iter().enumerate() {
                    if let Some(point) = ray.triangle_intersection(&triangle.points) {
                        result.push(RayCastResult {
                            kind: HitKind::StaticTriangle {
                                static_geometry: self.static_geoms.handle_from_index(index),
                                triangle_index
                            },
                            position: point,
                            normal: triangle.plane.normal,
                            sqr_distance: point.sqr_distance(&ray.origin)
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
