# 0.25 (future)

- Some of sound entities were integrated in the scene graph.
- New `Sound` and `Listener` scene nodes.
- Sound buffer import options.
- `ResourceManager::request_sound_buffer` now accepts only path to sound buffer.
- Prefab inheritance improvements - now most of the properties of scene nodes are inheritable.
- Access to simulation properties of the physics.
- **WIP**

## Migration guide

**WARNING:** This release **does not** provide legacy sound system conversion to new one, which means if 
any of your scene had any sound, they will be lost!

Now there is limited access to `fyrox_sound` entities, there is no way to create sound contexts, sounds, 
effects manually. You have to use respective scene nodes (`Sound`, `Listener`) and `Effect` from 
`fyrox::scene::sound` module (and children modules). 

### Listener

Now there is no need to manually sync position and orientation of the sound listener, all you need to do
instead is to create `Listener` node and attach it to your primary camera (or other scene node). Keep
in mind that the engine supports only one listener, which means that only one listener can be active 
at a time. The engine will not stop you from having multiple listeners active, however only first (the 
order is undefined) will be used to output sound.

### Sound sources

There is no more 2D/3D separation between sounds, all sounds in 3D by default. Every sound source now is
a scene node and can be created like so:

```rust
let sound = SoundBuilder::new(
    BaseBuilder::new().with_local_transform(
        TransformBuilder::new()
            .with_local_position(position)
            .build(),
    ),
)
.with_buffer(buffer.into())
.with_status(Status::Playing)
.with_play_once(true)
.with_gain(gain)
.with_radius(radius)
.with_rolloff_factor(rolloff_factor)
.build(graph);
```

Its API mimics `fyrox_sound` API so there should be now troubles in migration.

### Effects

Effects got a thin wrapper around `fyrox_sound` to make them compatible with `Sound` scene nodes, a reverb
effect instance can be created like so:

```rust
let reverb = ReverbEffectBuilder::new(BaseEffectBuilder::new().with_gain(0.7))
    .with_wet(0.5)
    .with_dry(0.5)
    .with_decay_time(3.0)
    .build(&mut scene.graph.sound_context);
```

A sound source can be attached to an effect like so:

```rust
graph
    .sound_context
    .effect_mut(self.reverb)
    .inputs_mut()
    .push(EffectInput {
        sound,
        filter: None,
    });
```

### Filters

Effect input filters API remain unchanged.

# 0.24

## Engine

- 2D games support (with 2D physics as well)
- Three new scene nodes was added: RigidBody, Collider, Joint. Since rigid body, collider and joint are graph nodes
now, it is possible to have complex hierarchies built with them.  
- It is possible to attach rigid body to any node in scene graph, its position now will be correct in this case (
previously it was possible to have rigid bodies attached only on root scene nodes).
- New `Inspector` widget + tons of built-in property editors (with the ability to add custom editors)
- `Inspect` trait + proc macro for lightweight reflection
- UI now using dynamic dispatch allowing you to add custom nodes and messages easily
- fyrox-sound optimizations (30% faster)
- Linear interpolation for sound samples when sampling rate != 1.0 (much better quality than before)
- Color fields in material editor now editable
- Window client area is now correctly filled by the renderer on every OS, not just Windows.
- NumericRange removal (replaced with standard Range + extension trait)
- Sort files and directories in FileBrowser/FileSelector widgets
- RawStreaming data source for sound
- Renderer performance improvements (2.5x times faster)
- UI layout performance improvements
- Prevent renderer from eating gigabytes of RAM
- Use `#[inline]` attribute to enable cross-crate inlining
- `ImmutableString` for faster hashing of static strings
- `SparseBuffer` as a lightweight analog for `Pool` (non-generational version)
- Support diffuse color in FBX materials
- Frustum culling fixes for terrain
- Shaders don't print empty lines when compiles successfully.
- `Pool` improvements
- Impl `IntoIterator` for references to `Pool`
- Cascaded shadow maps for directional light sources
- `spawn_at` + `spawn_at_handle` for `Pool`
- Preview for drag'n'drop
- `Grid` widget layout performance optimizations (**1000x** performance improvement - this is not a typo)
- `query_component` for UI widgets
- Curve resource
- Remove all associated widgets of a widget when deleting the widget (do not leave dangling objects)
- World bounding box calculation fix
- Heavy usage of invalidation in UI routines (prevents checking tons of widgets every frame)
- Migrate to `parking-lot` synchronization primitives
- Migrate to `FxHash` (faster hashing)
- `Log::verify` to log errors of `Result<(), Error`
- Custom scene node properties support
- `Alt+Click` prevents selection in `Tree` widget
- Ability to change camera projection (Perspective or Orthographic) 
- Smart position selection for popups (prevents them from appearing outside screen bounds)
- High-quality mip-map generation using Lanczos filter.

## Editor

- `Inspector` widget integration, which allowed to remove tons of boilerplate code
- Middle mouse button camera dragging
- Q/E + Space to move camera up/down
- Working directory message is much less confusing now
- Ability to edit sound sources in the editor
- Checkerboard colorization fix in the world viewer
- Search in the world viewer
- Floating brush panel for terrain editor
- Editor camera has manual exposure (not affected by auto-exposure)
- Curve editor
- Automatically select an newly created instance of a scene node
- Grid snapping fix
- Angle snapping
- Edit properties of multiple selected objects at once.
- Context menu for scene items in world viewer
- `Create child` for scene item context menu
- Import options editor for asset browser
- Hot reload for textures.

## Breaking changes and migration guide

There are lots of breaking changes in this version, however all of them mostly related to the code and scenes made in
previous version _should_ still be loadable.

### Convert old scenes to new format

At first, install the rusty-editor from crates.io and run it:

```shell
cargo install rusty-editor
rusty-editor
```

And then just re-save your scenes one-by-one. After this all your scenes will be converted to the newest version.
Keep in mind that the editor from GitHub repo (0.25+) is not longer have backward compatibility/conversion code!

### 2D scenes

2D scene were completely removed and almost every 2D node were removed, there is only one "2D" node left - Rectangle.
2D now implemented in 3D scenes, you have to use orthographic camera for that. There is no migration guide for 2D scenes
because 2D had rudimentary support, and I highly doubt that there is any project that uses 2D of the engine.

## Resource management

Resource manager has changed its API and gained some useful features that should save you some time. 

`request_texture` now accepts only one argument - path to texture, second argument was used to pass 
`TextureImportOptions`. Import options now should be located in a separate options file. For example, you have a 
`foo.jpg` texture and you want to change its import options (compression, wrapping modes, mip maps, etc.). To do this
you should create `foo.jpg.options` file in the same directory near your file with following content (each field is
optional):

```text
(
    minification_filter: LinearMipMapLinear,
    magnification_filter: Linear,
    s_wrap_mode: Repeat,
    t_wrap_mode: Repeat,
    anisotropy: 16,
    compression: NoCompression,
)
```

The engine will read this file when you'll call `request_texture` and it will apply the options on the first load.
This file is not mandatory, you can always set global import defaults in resource manage by calling 
`set_texture_import_options`.

`request_model` have the same changes, there is only one argument and import options were moved to options file:

```text
(
    material_search_options: RecursiveUp
)
```

Again, all fields aren't mandatory and the entire file can be omitted, global import defaults can be set by calling
`set_model_import_options`.

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
use fyrox::{
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