use rg3d::{
    core::{
        algebra::{Isometry3, Point3, Translation3},
        color::Color,
        math::aabb::AxisAlignedBoundingBox,
        pool::{ErasedHandle, Handle, Pool},
    },
    physics::data::arena::Index,
    scene::{
        graph::Graph,
        node::Node,
        physics::{ColliderDesc, ColliderShapeDesc, PhysicsDesc, RigidBodyDesc},
        ColliderHandle, Line, RigidBodyHandle, Scene, SceneDrawingContext,
    },
};
use std::collections::HashMap;

pub type RigidBody = RigidBodyDesc<ErasedHandle>;
pub type Collider = ColliderDesc<ErasedHandle>;

/// Editor uses its own data model for physics because engine's is not suitable
/// for editor. Algorithm is very simple:
/// 1) After scene is loaded - convert its physics to editor's
/// 2) Operate with editor's representation
/// 3) On save: convert physics back to engine representation and save.
/// This works ok because we don't need physics simulation while editing scene.
#[derive(Default)]
pub struct Physics {
    pub bodies: Pool<RigidBody>,
    pub colliders: Pool<Collider>,
    pub binder: HashMap<Handle<Node>, Handle<RigidBody>>,
}

impl Physics {
    pub fn new(scene: &Scene) -> Self {
        dbg!(scene.physics.bodies.len());
        dbg!(scene.physics.colliders.len());

        let mut bodies: Pool<RigidBody> = Default::default();

        let mut body_map = HashMap::new();

        for (h, b) in scene.physics.bodies.iter() {
            let pool_handle = bodies.spawn(RigidBodyDesc {
                position: b.position().translation.vector,
                rotation: b.position().rotation,
                linvel: *b.linvel(),
                angvel: *b.angvel(),
                sleeping: b.is_sleeping(),
                status: b.body_status.into(),
                // Filled later.
                colliders: vec![],
                mass: b.mass(),
            });

            body_map.insert(h, pool_handle);
        }

        let mut colliders: Pool<Collider> = Default::default();

        let mut collider_map = HashMap::new();

        for (h, c) in scene.physics.colliders.iter() {
            let pool_handle = colliders.spawn(ColliderDesc {
                shape: ColliderShapeDesc::from_collider_shape(c.shape()),
                parent: ErasedHandle::from(*body_map.get(&c.parent()).unwrap()),
                friction: c.friction,
                density: c.density(),
                restitution: c.restitution,
                is_sensor: c.is_sensor(),
                translation: c.position_wrt_parent().translation.vector,
                rotation: c.position_wrt_parent().rotation,
                collision_groups: c.collision_groups().0,
                solver_groups: c.solver_groups().0,
            });

            collider_map.insert(h, pool_handle);
        }

        for (&old, &new) in body_map.iter() {
            bodies[new].colliders = scene
                .physics
                .bodies
                .get(old)
                .unwrap()
                .colliders()
                .iter()
                .map(|c| ErasedHandle::from(*collider_map.get(c).unwrap()))
                .collect()
        }

        let mut binder = HashMap::new();

        for (&node, body) in scene.physics_binder.node_rigid_body_map.iter() {
            binder.insert(node, *body_map.get(&body.0).unwrap());
        }

        dbg!(&bodies);
        dbg!(&colliders);
        dbg!(&binder);

        Self {
            bodies,
            colliders,
            binder,
        }
    }

    pub fn unbind_by_body(&mut self, body: Handle<RigidBody>) -> Handle<Node> {
        let mut node = Handle::NONE;
        self.binder = self
            .binder
            .clone()
            .into_iter()
            .filter(|&(n, b)| {
                if b == body {
                    node = n;
                    false
                } else {
                    true
                }
            })
            .collect();
        node
    }

    pub fn generate_engine_desc(&self) -> (PhysicsDesc, HashMap<Handle<Node>, RigidBodyHandle>) {
        let mut body_map = HashMap::new();

        let mut bodies: Vec<RigidBodyDesc<ColliderHandle>> = self
            .bodies
            .pair_iter()
            .enumerate()
            .map(|(i, (h, r))| {
                // Sparse to dense mapping.
                let dense_handle = RigidBodyHandle::from(Index::from_raw_parts(i, 0));
                body_map.insert(h, dense_handle);
                RigidBodyDesc {
                    position: r.position,
                    rotation: r.rotation,
                    linvel: r.linvel,
                    angvel: r.angvel,
                    sleeping: r.sleeping,
                    status: r.status,
                    // Filled later.
                    colliders: vec![],
                    mass: r.mass,
                }
            })
            .collect::<Vec<_>>();

        let mut collider_map = HashMap::new();

        let colliders = self
            .colliders
            .pair_iter()
            .enumerate()
            .map(|(i, (h, c))| {
                // Sparse to dense mapping.
                let dense_handle = ColliderHandle::from(Index::from_raw_parts(i, 0));
                collider_map.insert(h, dense_handle);
                ColliderDesc {
                    shape: c.shape.clone(),
                    // Remap from sparse handle to dense.
                    parent: *body_map.get(&c.parent.into()).unwrap(),
                    friction: c.friction,
                    density: c.density,
                    restitution: c.restitution,
                    is_sensor: c.is_sensor,
                    translation: c.translation,
                    rotation: c.rotation,
                    collision_groups: c.collision_groups,
                    solver_groups: c.solver_groups,
                }
            })
            .collect();

        // Find colliders for each remapped body.
        for (&sparse_handle, &dense_handle) in body_map.iter() {
            let body = &self.bodies[sparse_handle];
            bodies[dense_handle.0.into_raw_parts().0].colliders = body
                .colliders
                .iter()
                .map(|&collider_sparse| *collider_map.get(&collider_sparse.into()).unwrap())
                .collect();
        }

        let mut binder = HashMap::new();

        for (&node, body) in self.binder.iter() {
            binder.insert(node, *body_map.get(body).unwrap());
        }

        (
            PhysicsDesc {
                colliders,
                bodies,
                ..Default::default()
            },
            binder,
        )
    }

    pub fn draw(&self, context: &mut SceneDrawingContext, graph: &Graph) {
        for body in self.bodies.iter() {
            context.draw_transform(
                Isometry3 {
                    rotation: body.rotation,
                    translation: Translation3 {
                        vector: body.position,
                    },
                }
                .to_homogeneous(),
            );
        }

        let color = Color::opaque(200, 200, 200);

        for collider in self.colliders.iter() {
            let parent = collider.parent.into();
            let body = self.bodies.borrow(parent);
            let transform = Isometry3 {
                rotation: body.rotation,
                translation: Translation3 {
                    vector: body.position,
                },
            }
            .to_homogeneous();
            match &collider.shape {
                ColliderShapeDesc::Ball(ball) => {
                    context.draw_sphere(body.position, 10, 10, ball.radius, color);
                }
                ColliderShapeDesc::Cylinder(cylinder) => {
                    context.draw_cylinder(
                        10,
                        cylinder.radius,
                        cylinder.half_height * 2.0,
                        true,
                        transform,
                        color,
                    );
                }
                ColliderShapeDesc::RoundCylinder(round_cylinder) => {
                    context.draw_cylinder(
                        10,
                        round_cylinder.radius,
                        round_cylinder.half_height * 2.0,
                        false,
                        transform,
                        color,
                    );
                }
                ColliderShapeDesc::Cone(cone) => {
                    context.draw_cone(10, cone.radius, cone.half_height * 2.0, transform, color);
                }
                ColliderShapeDesc::Cuboid(cuboid) => {
                    let max = cuboid.half_extents.scale(2.0);
                    let min = -max;
                    context.draw_oob(
                        &AxisAlignedBoundingBox::from_min_max(min, max),
                        transform,
                        color,
                    );
                }
                ColliderShapeDesc::Capsule(capsule) => {
                    // TODO: Draw as it should be.
                    context.draw_sphere(capsule.begin, 10, 10, capsule.radius, color);
                    context.draw_sphere(capsule.end, 10, 10, capsule.radius, color);
                }
                ColliderShapeDesc::Segment(segment) => {
                    context.add_line(Line {
                        begin: segment.begin,
                        end: segment.end,
                        color: color,
                    });
                }
                ColliderShapeDesc::Triangle(triangle) => {
                    context.draw_triangle(triangle.a, triangle.b, triangle.c, color);
                }
                ColliderShapeDesc::Trimesh(_) => {
                    let mut node = Handle::NONE;
                    for (&n, &b) in self.binder.iter() {
                        if b == parent {
                            node = n;
                            break;
                        }
                    }
                    if node.is_some() {
                        if let Node::Mesh(mesh) = &graph[node] {
                            // Trimesh's transform is special - it has transform baked into vertices.
                            // We have to emulate it here.
                            let transform = mesh.global_transform();
                            for surface in mesh.surfaces() {
                                let data = surface.data();
                                let data = data.read().unwrap();
                                for triangle in data.triangles() {
                                    let a = transform.transform_point(&Point3::from(
                                        data.get_vertices()[triangle[0] as usize].position,
                                    ));
                                    let b = transform.transform_point(&Point3::from(
                                        data.get_vertices()[triangle[1] as usize].position,
                                    ));
                                    let c = transform.transform_point(&Point3::from(
                                        data.get_vertices()[triangle[2] as usize].position,
                                    ));
                                    context.draw_triangle(a.coords, b.coords, c.coords, color);
                                }
                            }
                        }
                    }
                }
                ColliderShapeDesc::Heightfield(_) => {} // TODO
            }
        }
    }
}
