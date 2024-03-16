//! Debug drawing module.
//!
//! For more info see [`SceneDrawingContext`]

use crate::core::{
    algebra::{Matrix4, Point3, UnitQuaternion, Vector2, Vector3},
    color::{Color, Hsl},
    math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, Matrix4Ext},
};
use std::ops::Range;

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

/// Drawing context for simple graphics, it allows you to draw simple figures using a set of lines. Most
/// common use of the context is to draw some debug geometry in your game, draw physics info (contacts,
/// meshes, shapes, etc.), draw temporary geometry in editor and so on.
///
/// This drawing context is meant to be used only for debugging purposes, it draws everything as set of lines
/// and no solid faces supported.
///
/// It should be noted that the actual drawing is not immediate, provided methods just populate internal array
/// of lines and it will be drawn on special render stage.
///
/// # Example
///
/// The usage of the drawing context is a bit unusual, at the beginning of the frame you should clear the
/// contents of the context and only then call "drawing" methods. Otherwise, the internal buffer will increase
/// in size to values which will take lots of time draw and the FPS will significantly drop with every frame
/// until it reaches zero.
///
/// So typical usage would be:
///
/// ```
/// # use fyrox_impl::scene::debug::SceneDrawingContext;
/// # use fyrox_impl::core::algebra::Matrix4;
/// # use fyrox_impl::core::color::Color;
///
/// fn draw_debug_objects(ctx: &mut SceneDrawingContext) {
///     // Clear at the beginning of the frame.
///     ctx.clear_lines();
///
///     // Draw something.
///     ctx.draw_cone(20, 1.0, 2.0, Matrix4::identity(), Color::WHITE, true);
/// }
///
/// ```
///
/// You could avoid calling `clear_lines` in specific cases where your debug geometry is not changing, then
/// the context could be populated once and rendered multiple times without any issues. Another case when
/// you could not call `clear_lines` each frame, is "tracing" scenario - for example you may need to trace
/// moving objects. In this case call `clear_lines` once in a few seconds, and you'll see the "track" of
/// moving objects.
///
/// # Rendering performance
///
/// The engine renders the entire set of lines in a single draw call, so it very fast - you should be able to draw
/// up to few millions of lines without any significant performance issues.
#[derive(Default, Clone, Debug)]
pub struct SceneDrawingContext {
    /// List of lines to draw.
    pub lines: Vec<Line>,
}

impl rapier2d::pipeline::DebugRenderBackend for SceneDrawingContext {
    fn draw_line(
        &mut self,
        _object: rapier2d::pipeline::DebugRenderObject,
        a: rapier2d::math::Point<rapier2d::math::Real>,
        b: rapier2d::math::Point<rapier2d::math::Real>,
        color: [f32; 4],
    ) {
        self.add_line(Line {
            begin: Vector3::new(a.x, a.y, 0.0),
            end: Vector3::new(b.x, b.y, 0.0),
            color: Color::from(Hsl::new(color[0], color[1], color[2])),
        })
    }
}

impl rapier3d::pipeline::DebugRenderBackend for SceneDrawingContext {
    fn draw_line(
        &mut self,
        _object: rapier3d::pipeline::DebugRenderObject,
        a: rapier3d::math::Point<rapier3d::math::Real>,
        b: rapier3d::math::Point<rapier3d::math::Real>,
        color: [f32; 4],
    ) {
        self.add_line(Line {
            begin: a.coords,
            end: b.coords,
            color: Color::from(Hsl::new(color[0], color[1], color[2])),
        })
    }
}

impl SceneDrawingContext {
    /// Draws frustum with given color.
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

    /// Draws axis-aligned bounding box with given color.
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

    /// Draws object-oriented bounding box with given color.
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

    /// Draws a pyramid by given points.
    pub fn draw_pyramid(
        &mut self,
        top: Vector3<f32>,
        a: Vector3<f32>,
        b: Vector3<f32>,
        c: Vector3<f32>,
        d: Vector3<f32>,
        color: Color,
        transform: Matrix4<f32>,
    ) {
        let top = transform.position() + transform.transform_vector(&top);
        let a = transform.position() + transform.transform_vector(&a);
        let b = transform.position() + transform.transform_vector(&b);
        let c = transform.position() + transform.transform_vector(&c);
        let d = transform.position() + transform.transform_vector(&d);
        self.draw_triangle(top, a, b, color);
        self.draw_triangle(top, b, c, color);
        self.draw_triangle(top, c, d, color);
        self.draw_triangle(top, d, a, color);
    }

    /// Draws a sphere as a set of three circles around each axes.
    pub fn draw_wire_sphere(
        &mut self,
        position: Vector3<f32>,
        radius: f32,
        segments: usize,
        color: Color,
    ) {
        let translation = Matrix4::new_translation(&position);
        self.draw_circle(Default::default(), radius, segments, translation, color);
        self.draw_circle(
            Default::default(),
            radius,
            segments,
            translation
                * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians())
                    .to_homogeneous(),
            color,
        );
        self.draw_circle(
            Default::default(),
            radius,
            segments,
            translation
                * UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 90.0f32.to_radians())
                    .to_homogeneous(),
            color,
        );
    }

    /// Draws a circle at given world-space position with given radius. `segments` could be used
    /// to control quality of the circle.
    pub fn draw_circle(
        &mut self,
        position: Vector3<f32>,
        radius: f32,
        segments: usize,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        let d_phi = 2.0 * std::f32::consts::PI / segments as f32;
        for i in 0..segments {
            let x1 = position.x + radius * (d_phi * i as f32).cos();
            let y1 = position.y + radius * (d_phi * i as f32).sin();
            let x2 = position.x + radius * (d_phi * (i + 1) as f32).cos();
            let y2 = position.y + radius * (d_phi * (i + 1) as f32).sin();

            self.add_line(Line {
                begin: transform.transform_point(&Point3::new(x1, y1, 0.0)).coords,
                end: transform.transform_point(&Point3::new(x2, y2, 0.0)).coords,
                color,
            })
        }
    }

    /// Draws a circle segment between two given angles. Center of the segment is defined by `position`,
    /// `segments` defines quality of the shape.
    pub fn draw_circle_segment(
        &mut self,
        position: Vector3<f32>,
        radius: f32,
        segments: usize,
        begin_angle: f32,
        end_angle: f32,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        let d_angle = 2.0 * std::f32::consts::PI / segments as f32;
        let mut angle = begin_angle;
        while angle < end_angle {
            let x1 = position.x + radius * (angle).cos();
            let y1 = position.y + radius * (angle).sin();
            let x2 = position.x + radius * (angle + d_angle).cos();
            let y2 = position.y + radius * (angle + d_angle).sin();

            self.add_line(Line {
                begin: transform.transform_point(&Point3::new(x1, y1, 0.0)).coords,
                end: transform.transform_point(&Point3::new(x2, y2, 0.0)).coords,
                color,
            });

            angle += d_angle;
        }
    }

    /// Draws a rectangle with given width and height.
    pub fn draw_rectangle(
        &mut self,
        half_width: f32,
        half_height: f32,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        let a = transform
            .transform_point(&Point3::new(-half_width, half_height, 0.0))
            .coords;
        let b = transform
            .transform_point(&Point3::new(half_width, half_height, 0.0))
            .coords;
        let c = transform
            .transform_point(&Point3::new(half_width, -half_height, 0.0))
            .coords;
        let d = transform
            .transform_point(&Point3::new(-half_width, -half_height, 0.0))
            .coords;
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
            end: d,
            color,
        });
        self.add_line(Line {
            begin: d,
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

    /// Draws a wire Y oriented cone where the tip is on +Y with given parameters.
    pub fn draw_cone(
        &mut self,
        sides: usize,
        r: f32,
        h: f32,
        transform: Matrix4<f32>,
        color: Color,
        cap: bool,
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
            if cap {
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
            }

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

    /// Draws a flat capsule with given height and radius. `segments` defines quality of the shape.
    pub fn draw_flat_capsule(
        &mut self,
        radius: f32,
        height: f32,
        segments: usize,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        self.draw_circle_segment(
            Vector3::new(0.0, height * 0.5, 0.0),
            radius,
            segments,
            0.0,
            std::f32::consts::PI,
            transform,
            color,
        );

        self.draw_circle_segment(
            Vector3::new(0.0, -height * 0.5, 0.0),
            radius,
            segments,
            std::f32::consts::PI,
            std::f32::consts::TAU,
            transform,
            color,
        );

        self.add_line(Line {
            begin: transform
                .transform_point(&Point3::new(-radius, height * 0.5, 0.0))
                .coords,
            end: transform
                .transform_point(&Point3::new(-radius, -height * 0.5, 0.0))
                .coords,
            color,
        });
        self.add_line(Line {
            begin: transform
                .transform_point(&Point3::new(radius, height * 0.5, 0.0))
                .coords,
            end: transform
                .transform_point(&Point3::new(radius, -height * 0.5, 0.0))
                .coords,
            color,
        });
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

    /// Draws a flat capsule between two points with given radius. `segments` defines quality of
    /// the shape.
    pub fn draw_segment_flat_capsule(
        &mut self,
        begin: Vector2<f32>,
        end: Vector2<f32>,
        radius: f32,
        segments: usize,
        transform: Matrix4<f32>,
        color: Color,
    ) {
        // Draw as two circles and a rectangle
        // TODO: Draw this correctly
        self.draw_circle(
            Vector3::new(begin.x, begin.y, 0.0),
            radius,
            segments,
            transform,
            color,
        );
        self.draw_circle(
            Vector3::new(end.x, end.y, 0.0),
            radius,
            segments,
            transform,
            color,
        );
        let perp = (end - begin)
            .try_normalize(f32::EPSILON)
            .map(|v| Vector2::new(v.y, -v.x).scale(radius))
            .unwrap_or_default();

        self.add_line(Line {
            begin: transform
                .transform_point(&Point3::from((begin - perp).to_homogeneous()))
                .coords,
            end: transform
                .transform_point(&Point3::from((end - perp).to_homogeneous()))
                .coords,
            color,
        });
        self.add_line(Line {
            begin: transform
                .transform_point(&Point3::from((begin + perp).to_homogeneous()))
                .coords,
            end: transform
                .transform_point(&Point3::from((end + perp).to_homogeneous()))
                .coords,
            color,
        });
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

        let z_axis = axis.try_normalize(f32::EPSILON).unwrap_or_else(Vector3::z);

        let y_axis = z_axis
            .cross(
                &(if z_axis.y != 0.0 || z_axis.z != 0.0 {
                    Vector3::x()
                } else {
                    Vector3::y()
                }),
            )
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(Vector3::y);

        let x_axis = z_axis
            .cross(&y_axis)
            .try_normalize(f32::EPSILON)
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

    /// Draws an Y+ oriented arrow with the given parameters.
    pub fn draw_arrow(
        &mut self,
        sides: usize,
        color: Color,
        length: f32,
        radius: f32,
        transform: Matrix4<f32>,
    ) {
        self.draw_cylinder(sides, radius, length, true, transform, color);

        let head_radius = radius * 2.0;
        let head_height = radius * 4.0;

        self.draw_cone(
            sides,
            head_radius,
            head_height,
            transform
                * Matrix4::new_translation(&Vector3::new(0.0, (length + head_height) * 0.5, 0.0)),
            color,
            true,
        );
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
