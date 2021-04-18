use rg3d::{
    core::{
        algebra::{Isometry3, Point3, Translation, Translation3, Vector3},
        color::Color,
        math::aabb::AxisAlignedBoundingBox,
        pool::{ErasedHandle, Handle, Pool},
        uuid::Uuid,
        BiDirHashMap,
    },
    scene::{
        graph::Graph,
        node::Node,
        physics::{
            ColliderDesc, ColliderShapeDesc, JointDesc, JointParamsDesc, PhysicsDesc, RigidBodyDesc,
        },
        ColliderHandle, JointHandle, Line, RigidBodyHandle, Scene, SceneDrawingContext,
    },
};
use std::collections::HashMap;

pub type RigidBody = RigidBodyDesc<ErasedHandle>;
pub type Collider = ColliderDesc<ErasedHandle>;
pub type Joint = JointDesc<ErasedHandle>;

/// Editor uses its own data model for physics because engine's is not suitable
/// for editor. Algorithm is very simple:
/// 1) After scene is loaded - convert its physics to editor's
/// 2) Operate with editor's representation
/// 3) On save: convert physics back to engine representation and save.
/// This works ok because we don't need physics simulation while editing scene.
///
/// We using Pool to store descriptors because it allows to temporarily move
/// object out, reserving entry for later re-use. This is very handy because
/// handles are not invalidating during this process and it works perfectly
/// with undo/redo.  
#[derive(Default)]
pub struct Physics {
    pub bodies: Pool<RigidBody>,
    pub colliders: Pool<Collider>,
    pub joints: Pool<Joint>,
    pub binder: BiDirHashMap<Handle<Node>, Handle<RigidBody>>,

    body_handle_map: HashMap<Handle<RigidBody>, RigidBodyHandle>,
    collider_handle_map: HashMap<Handle<Collider>, ColliderHandle>,
    joint_handle_map: HashMap<Handle<Joint>, JointHandle>,
}

impl Physics {
    pub fn new(scene: &Scene) -> Self {
        let mut bodies: Pool<RigidBody> = Default::default();
        let mut body_map = HashMap::new();

        let mut body_handle_map = HashMap::new();
        for (h, b) in scene.physics.bodies().iter() {
            let rotation_locked = b.is_rotation_locked();
            let pool_handle = bodies.spawn(RigidBodyDesc {
                position: b.position().translation.vector,
                rotation: b.position().rotation,
                linvel: *b.linvel(),
                angvel: *b.angvel(),
                sleeping: b.is_sleeping(),
                status: b.body_status().into(),
                // Filled later.
                colliders: vec![],
                mass: b.mass(),
                x_rotation_locked: rotation_locked[0],
                y_rotation_locked: rotation_locked[1],
                z_rotation_locked: rotation_locked[1],
                translation_locked: b.is_translation_locked(),
            });

            body_map.insert(h, pool_handle);

            // Remember initial handle of a body.
            body_handle_map.insert(
                pool_handle,
                scene.physics.body_handle_map().key_of(&h).cloned().unwrap(),
            );
        }

        let mut colliders: Pool<Collider> = Default::default();
        let mut collider_map = HashMap::new();

        let mut collider_handle_map = HashMap::new();
        for (h, c) in scene.physics.colliders().iter() {
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
            collider_handle_map.insert(
                pool_handle,
                scene
                    .physics
                    .collider_handle_map()
                    .key_of(&h)
                    .cloned()
                    .unwrap(),
            );
        }

        for (&old, &new) in body_map.iter() {
            bodies[new].colliders = scene
                .physics
                .body(
                    &scene
                        .physics
                        .body_handle_map()
                        .key_of(&old)
                        .cloned()
                        .unwrap(),
                )
                .unwrap()
                .colliders()
                .iter()
                .map(|c| ErasedHandle::from(*collider_map.get(c).unwrap()))
                .collect()
        }

        let mut joints: Pool<Joint> = Pool::new();

        let mut joint_handle_map = HashMap::new();
        for (h, j) in scene.physics.joints().iter() {
            let pool_handle = joints.spawn(JointDesc {
                body1: ErasedHandle::from(*body_map.get(&j.body1).unwrap()),
                body2: ErasedHandle::from(*body_map.get(&j.body2).unwrap()),
                params: JointParamsDesc::from_params(&j.params),
            });
            joint_handle_map.insert(
                pool_handle,
                scene
                    .physics
                    .joint_handle_map()
                    .key_of(&h)
                    .cloned()
                    .unwrap(),
            );
        }

        let mut binder = BiDirHashMap::default();

        for (&node, body) in scene.physics_binder.forward_map().iter() {
            let body_handle = scene
                .physics
                .body_handle_map()
                .value_of(body)
                .cloned()
                .unwrap();
            binder.insert(node, *body_map.get(&body_handle).unwrap());
        }

        Self {
            bodies,
            colliders,
            joints,
            binder,
            body_handle_map,
            collider_handle_map,
            joint_handle_map,
        }
    }

    pub fn unbind_by_body(&mut self, body: Handle<RigidBody>) -> Handle<Node> {
        self.binder.remove_by_value(&body).unwrap_or_default()
    }

    pub fn generate_engine_desc(&self) -> (PhysicsDesc, HashMap<Handle<Node>, RigidBodyHandle>) {
        let mut editor_body_handle_to_engine_map = BiDirHashMap::default();
        let mut engine_body_handle_rapier_map = BiDirHashMap::default();
        for (i, (handle, _)) in self.bodies.pair_iter().enumerate() {
            let engine_handle = self
                .body_handle_map
                .get(&handle)
                // Use existing handle or generate new for new object.
                .map_or_else(
                    || RigidBodyHandle::from(Uuid::new_v4()),
                    |existing| existing.clone(),
                );
            engine_body_handle_rapier_map.insert(
                engine_handle,
                // Rapier3D handle will become just a simple index.
                rg3d::physics::dynamics::RigidBodyHandle::from_raw_parts(i, 0),
            );
            editor_body_handle_to_engine_map.insert(handle, engine_handle);
        }

        let mut bodies: Vec<RigidBodyDesc<ColliderHandle>> = self
            .bodies
            .pair_iter()
            .map(|(_, r)| {
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
                    x_rotation_locked: r.x_rotation_locked,
                    y_rotation_locked: r.y_rotation_locked,
                    z_rotation_locked: r.z_rotation_locked,
                    translation_locked: r.translation_locked,
                }
            })
            .collect::<Vec<_>>();

        let mut editor_collider_handle_to_engine_map = HashMap::new();
        let mut engine_collider_handle_rapier_map = BiDirHashMap::default();
        for (i, (handle, _)) in self.colliders.pair_iter().enumerate() {
            let engine_handle = self
                .collider_handle_map
                .get(&handle)
                // Use existing handle or generate new for new object.
                .map_or_else(
                    || ColliderHandle::from(Uuid::new_v4()),
                    |existing| existing.clone(),
                );
            engine_collider_handle_rapier_map.insert(
                engine_handle,
                // Rapier3D handle will become just a simple index.
                rg3d::physics::geometry::ColliderHandle::from_raw_parts(i, 0),
            );
            editor_collider_handle_to_engine_map.insert(handle, engine_handle);
        }

        let colliders = self
            .colliders
            .pair_iter()
            .map(|(_, c)| {
                ColliderDesc {
                    shape: c.shape,
                    // Remap from sparse handle to dense.
                    parent: *editor_body_handle_to_engine_map
                        .value_of(&c.parent.into())
                        .unwrap(),
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
        for (engine_handle, &dense_handle) in engine_body_handle_rapier_map.forward_map().iter() {
            let editor_handle = editor_body_handle_to_engine_map
                .key_of(engine_handle)
                .cloned()
                .unwrap();
            let body = &self.bodies[editor_handle];
            bodies[dense_handle.into_raw_parts().0].colliders = body
                .colliders
                .iter()
                .map(|&collider_sparse| {
                    *editor_collider_handle_to_engine_map
                        .get(&collider_sparse.into())
                        .unwrap()
                })
                .collect();
        }

        let mut binder = HashMap::new();

        for (&node, body) in self.binder.forward_map().iter() {
            binder.insert(
                node,
                *editor_body_handle_to_engine_map.value_of(body).unwrap(),
            );
        }

        let mut engine_joint_handle_rapier_map = BiDirHashMap::default();
        for (i, (handle, _)) in self.joints.pair_iter().enumerate() {
            let engine_handle = self
                .joint_handle_map
                .get(&handle)
                // Use existing handle or generate new for new object.
                .map_or_else(
                    || JointHandle::from(Uuid::new_v4()),
                    |existing| existing.clone(),
                );
            engine_joint_handle_rapier_map.insert(
                engine_handle,
                // Rapier3D handle will become just a simple index.
                rg3d::physics::dynamics::JointHandle::from_raw_parts(i, 0),
            );
        }
        let joints = self
            .joints
            .iter()
            .map(|j| JointDesc {
                body1: *editor_body_handle_to_engine_map
                    .value_of(&j.body1.into())
                    .unwrap(),
                body2: *editor_body_handle_to_engine_map
                    .value_of(&j.body2.into())
                    .unwrap(),
                params: j.params.clone(),
            })
            .collect();

        (
            PhysicsDesc {
                colliders,
                bodies,
                joints,
                body_handle_map: engine_body_handle_rapier_map,
                collider_handle_map: engine_collider_handle_rapier_map,
                joint_handle_map: engine_joint_handle_rapier_map,
                gravity: Default::default(),
                integration_parameters: Default::default(),
            },
            binder,
        )
    }

    /// Searches joint by its **first** body.
    pub fn find_joint(&self, body1: Handle<RigidBody>) -> Handle<Joint> {
        for (handle, joint) in self.joints.pair_iter() {
            if joint.body1 == body1.into() {
                return handle;
            }
        }
        Handle::NONE
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

        let color = Color::opaque(255, 0, 255);

        for collider in self.colliders.iter() {
            let parent = collider.parent.into();
            let body = self.bodies.borrow(parent);

            let body_global_transform = Isometry3 {
                rotation: body.rotation,
                translation: Translation3 {
                    vector: body.position,
                },
            }
            .to_homogeneous();

            let collider_local_tranform = Isometry3 {
                rotation: collider.rotation,
                translation: Translation3 {
                    vector: collider.translation,
                },
            }
            .to_homogeneous();

            let transform = if let Some(&node) = self.binder.key_of(&parent) {
                let (rotation, position) = graph.isometric_global_rotation_position(node);
                Isometry3 {
                    rotation,
                    translation: Translation { vector: position },
                }
                .to_homogeneous()
                    * collider_local_tranform
            } else {
                body_global_transform * collider_local_tranform
            };

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
                    let min = -cuboid.half_extents;
                    let max = cuboid.half_extents;
                    context.draw_oob(
                        &AxisAlignedBoundingBox::from_min_max(min, max),
                        transform,
                        color,
                    );
                }
                ColliderShapeDesc::Capsule(capsule) => context.draw_segment_capsule(
                    capsule.begin,
                    capsule.end,
                    capsule.radius,
                    10,
                    10,
                    transform,
                    color,
                ),
                ColliderShapeDesc::Segment(segment) => {
                    context.add_line(Line {
                        begin: segment.begin,
                        end: segment.end,
                        color,
                    });
                }
                ColliderShapeDesc::Triangle(triangle) => {
                    context.draw_triangle(triangle.a, triangle.b, triangle.c, color);
                }
                ColliderShapeDesc::Trimesh(_) => {
                    let mut node = Handle::NONE;
                    for (&n, &b) in self.binder.forward_map().iter() {
                        if b == parent {
                            node = n;
                            break;
                        }
                    }

                    if node.is_some() {
                        let mut stack = vec![node];
                        while let Some(handle) = stack.pop() {
                            let node = &graph[handle];
                            if let Node::Mesh(mesh) = node {
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
                            stack.extend_from_slice(node.children());
                        }
                    }
                }
                ColliderShapeDesc::Heightfield(_) => {} // TODO
            }
        }

        for joint in self.joints.iter() {
            match &joint.params {
                JointParamsDesc::BallJoint(ball) => {
                    let mut draw_anchor = |local_anchor: Vector3<f32>| -> Option<Vector3<f32>> {
                        if joint.body1.is_some() {
                            let frame_of_reference =
                                self.bodies[joint.body1.into()].local_transform();

                            let anchor = frame_of_reference
                                .transform_point(&Point3::from(local_anchor))
                                .coords;
                            context.draw_sphere(anchor, 6, 6, 0.2, Color::BLUE);
                            Some(anchor)
                        } else {
                            None
                        }
                    };

                    let anchor1 = draw_anchor(ball.local_anchor1);
                    let anchor2 = draw_anchor(-ball.local_anchor2);

                    if let (Some(anchor1), Some(anchor2)) = (anchor1, anchor2) {
                        context.add_line(Line {
                            begin: anchor1,
                            end: anchor2,
                            color: Color::BLUE,
                        })
                    }
                }
                JointParamsDesc::FixedJoint(_) => {}
                JointParamsDesc::PrismaticJoint(_) => {}
                JointParamsDesc::RevoluteJoint(_) => {}
            }
        }
    }
}
