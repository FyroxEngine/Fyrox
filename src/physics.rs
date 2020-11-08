use rg3d::{
    core::pool::{ErasedHandle, Handle, Pool},
    physics::data::arena::Index,
    scene::{
        node::Node,
        physics::ColliderShapeDesc,
        physics::{ColliderDesc, PhysicsDesc, RigidBodyDesc},
        ColliderHandle, RigidBodyHandle, Scene,
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
                position: b.position.translation.vector,
                rotation: b.position.rotation,
                linvel: b.linvel,
                angvel: b.angvel,
                sleeping: b.is_sleeping(),
                status: b.body_status.into(),
                // Filled later.
                colliders: vec![],
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
                translation: c.position().translation.vector,
                rotation: c.position().rotation,
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
}
