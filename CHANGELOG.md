# 0.24

- Three new scene nodes was added: RigidBody, Collider, Joint.
- Since rigid body, collider and joint are graph nodes now, it is possible to have complex hierarchies built with them. 
- It is possible to attach rigid body to any node in scene graph, its position now will be correct in this case (
previously it was possible to have rigid bodies attached only on root scene nodes).
- **LOTS OF OTHER STUFF THAT MUST BE FILLED BEFORE PUBLIC ANNOUNCEMENT**

## Breaking changes

There are lots of breaking changes in this version, however all of them mostly related to the code and scenes made in
previous version _should_ still be loadable.

### Physics

Old physics was replaced with new scene nodes: RigidBody, Collider, Joint. Old scenes will be automatically converted
on load, you should convert your scenes as soon as possible using the editor (open your scene and save it, that will
do the conversion).

Now there are two ways of adding a rigid body to a scene node:

- If you want your object to have a rigid body (for example a crate with box rigid body), your object must be 
**child** object of a rigid body. Graphically it can be represented like this:

```text
- Rigid Body
  - Crate3DModel
  - Cuboid Collider     
```

- If you want your object to have a rigid body that should move together with your object (to simulate hit boxes for 
example), then rigid body must be child object of your object. Additionally it should be marked as `Kinematic`, 
otherwise it will be affected by simulation (simply speaking it will fall on ground). Graphically it can be 
represented like this:

```text
- Limb
  - Rigid Body
     - Capsule Collider
```

#### Migration

This section will help you to migrate to new physics.

##### Rigid bodies

Rigid body and colliders now can be created like so:

```rust
use rg3d::{
    core::{algebra::Vector3, pool::Handle},
    scene::{
        base::BaseBuilder,
        collider::{ColliderBuilder, ColliderShape},
        node::Node,
        rigidbody::RigidBodyBuilder,
        transform::TransformBuilder,
        Scene,
    },
};

fn create_capsule_rigid_body(scene: &mut Scene) -> Handle<Node> {
    RigidBodyBuilder::new(
        BaseBuilder::new()
            .with_local_transform(
                // To position, rotate rigid body you should use engine's transform.
                TransformBuilder::new()
                    .with_local_position(Vector3::new(1.0, 2.0, 3.0))
                    .build(),
            )
            .with_children(&[
                // It is very important to add at least one child collider node, otherwise rigid
                // body will not do collision response.
                ColliderBuilder::new(
                    BaseBuilder::new().with_local_transform(
                        // Colliders can have relative position to their parent rigid bodies.
                        TransformBuilder::new()
                            .with_local_position(Vector3::new(0.0, 0.5, 0.0))
                            .build(),
                    ),
                )
                // Rest of properties can be set almost as before.
                .with_friction(0.2)
                .with_restitution(0.1)
                .with_shape(ColliderShape::capsule_y(0.5, 0.2))
                .build(&mut scene.graph),
            ]),
    )
    // Rest of properties can be set almost as before.
    .with_mass(2.0)
    .with_ang_damping(0.1)
    .with_lin_vel(Vector3::new(2.0, 1.0, 3.0))
    .with_ang_vel(Vector3::new(0.1, 0.1, 0.1))
    .build(&mut scene.graph)
}
```

##### Joints

Joints can be created in a similar way:

```rust
fn create_ball_joint(scene: &mut Scene) -> Handle<Node> {
    JointBuilder::new(BaseBuilder::new())
        .with_params(JointParams::BallJoint(BallJoint {
            local_anchor1: Vector3::new(1.0, 0.0, 0.0),
            local_anchor2: Vector3::new(-1.0, 0.0, 0.0),
            limits_local_axis1: Vector3::new(1.0, 0.0, 0.0),
            limits_local_axis2: Vector3::new(1.0, 0.0, 0.0),
            limits_enabled: true,
            limits_angle: 45.0,
        }))
        .with_body1(create_capsule_rigid_body(scene))
        .with_body2(create_capsule_rigid_body(scene))
        .build(&mut scene.graph)
}
```

##### Raycasting

Raycasting located in `scene.graph.physics`, there were almost no changes to it, except now it returns handles to
scene nodes instead of raw collider handles.

##### Contact info

Contact info can now be queried from the collider node itself, via `contacts()` method.

```rust
fn query_contacts(collider: Handle<Node>, graph: &Graph) -> impl Iterator<Item = ContactPair> {
    graph[collider].as_collider().contacts(&graph.physics)
}
```

### Other

TODO