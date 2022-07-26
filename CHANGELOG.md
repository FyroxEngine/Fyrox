# 0.27 (WIP)

- Backward compatibility for native engine data formats was dropped - use FyroxEd 0.13 to convert your scenes to newer
version.
- Fixed panic when loading an FBX model with malformed animation curves (when there is only 1 or 2 components animated
instead of 3, X and Y, but not Z for example).
- ABSM editor now have smaller default size and fits on small screens.
- Asset previewer now plays model animations
- Fixed critical FBX importer bug, that caused malformed animations.
- Ability to define "playable" time slice for animations.
- Fixed editor update rate, previously it was very high and that caused some weird issues.
- Proper support for all resource types in Inspector
- Show ABSM resources in the asset browser
- Ability to edit sound import options in the asset browser
- Dynamic type casting for script instances
- Provide access to parameters in ABSM
- Fixed transition instantiation in ABSM - it incorrectly handled "invert rule" flag.
- Prevent panic when deleting a node from script methods.
- Dynamic type casting for plugin instances
- Two-step ABSM instantiation - at first step you load all animations in parallel (async) and on second step you
create actual ABSM instance.
- Wait for all resources to load before initialize scripts - this prevents panicking when trying to access
not yet loaded resource in script methods.
- Default instantiation scaling options for 3D models - allows you to scale 3D models automatically on instantiation.
- Graph event broadcaster - allows you to receive `Added` and `Removed` events for nodes.
- Correctly initialize scripts of nodes that created at runtime.
- Component provider for scripts - allows you to provide access to inner script components via unified interface.
- Disable automatic texture compression - having compression enabled for all kinds of textures is not good, because
there could be some textures with gradients, and they'll have significant distortion.
- `Pool::drain` - allows you to remove all objects from a pool while processing every object via closure.
- `Script::on_deinit` - allows you to execute any code for cleanup.
- Added `NodeHandleMap` - a small wrapper over map that have some methods that makes node handle mapping much 
shorter.
- Correctly handle missing properties in Inspector for various objects.
- Provide access to main application window from plugins.
- Allow chaining `ScriptConstructorContainer::add` calls
- Removed `screen_size` parameter from `UserInterface::new`
- Ability to remove render passes.
- Run the game in a separate process from the editor.
- Provide access to default engine's user interface instance for plugins.
- `--override-scene` parameter for Executor
- `ButtonContent` improvements - it is now possible to re-create button's text field using `ButtonMessage::Content`
- Provide access to control flow switch for plugins.
- `Plugin::on_ui_message`
- Two-step plugins initialization:
  - `PluginConstructor` trait defines a method that creates an instance of `Plugin` trait, instance of plugin
    constructor is used to create plugins on demand. It is needed because engine has deferred plugin initialization.
- `Framework` is removed, its functionality was merged with plugins.
- Simplified `ScriptConstructorContainer::add` definition, there were redundant generic parameters that just add
visual clutter.
- Implemented `Clone+Debug` traits for `NavmeshAgent`
- Fixed spam in log in the editor when any file was changed.
- High DPI screens support for the editor.
- Newly created cameras in the editor are now enabled by default.
- Added "Preview" option for cameras in world viewer.
- Refactored joints:
  - Joints binding now is fully automatic and it is based on world transform of the joint, no need to manually
    set local frames.
  - Rebinding happens when a joint changes its position
  - Joints editing in the editor is now much more intuitive
- Improved debug visualization for physics.
- Read-only mode for NumericUpDown and Vec2/Vec3/Vec4 widgets
- Show global coordinates of current selection in the scene previewer
- BitField widget - it helps you to edit numbers as bit containers, allowing you to switch separate bits
- More compact editors for properties in Inspector
- NumericUpDown widget does not use word wrapping by default anymore
- CheckBox widget can now be switched only by left mouse button
- Ability to disable contacts between connected bodies of a joint
- `style` parameter for project template generator - it defines which scene will be used by default - either `2d` 
or `3d`
- Ability to select portion of the texture to render in `Rectangle` nodes.
- Ability to generate script skeleton for template generator
- HSL color model
- Ability to copy log enties to the clipboard
- `Log` API improvements
- Visualize cameras in the editor
- Context menu for asset items, it is now possible to open, delete, show-in-explorer items and also
to copy file name and full file path to the clipboard.

# 0.26

This release is mostly to fix critical bugs of 0.25 and add missing functionality that stops you from using scripting
system.

- Added project template generator
- Fixed invisible selected item in drop-down list widget.
- Correctly sync node names in `World Viewer`
- Reset editor's camera projection mode switch when creating new scene
- Fixed doubling scene entities in `World Viewer` when loading scene via `StartupData`
- More logging for renderer
- Fixed shader cache - now the engine won't re-compile shaders each 20 seconds.
- Temporarily disable `Lifetime` property editing because it causes crashes
- Do not show `dirty` flag of `Transform` in the `Inspector`
- Provide access to property editors container for editor's `Inspector` - it is now possible
to register your own property editors
- Fixed panic when syncing `Inspector` for an entity with `Option<Texture>` field.
- Added `handle_object_property_changed` and `handle_collection_property_changed` macros to reduce 
boilerplate code in script property handling.
- Added ability to restore resource handles for scripts
- Fixed selection visualization in `Asset Browser`
- Validation for sky box cube map generator

## Migration guide

There are no breaking changes in this release.

# 0.25

- Static plugin system
- User-defined scripts
- Play mode for the editor
- Animation Blending State Machine (ABSM) editor.
- Some of sound entities were integrated in the scene graph.
- New `Sound` and `Listener` scene nodes.
- Sound buffer import options.
- `ResourceManager::request_sound_buffer` now accepts only path to sound buffer.
- Prefab inheritance improvements - now most of the properties of scene nodes are inheritable.
- Access to simulation properties of the physics.
- Engine and Resource manager are nonserializable anymore, check migration guide to find how to create
save files in the correct way.
- `Node` enumeration was removed and replaced with dynamic dispatch. This allows you to define your own 
types of scene nodes.
- `Base` is not a scene node anymore, it was replaced with `Pivot` node (see migration guide for more info)
- `Base` now has `cast_shadows` property, respective property setters/getters was removed from `Mesh` and 
`Terrain` nodes.
- Ability to bring ListView item into view.
- Logger improvements: event subscriptions + collecting timestamps
- Log panel improvements in the editor: severity filtering, color differentiation.
- Scene nodes now have more or less correct local bounds (a bounding box that can fit the node).
- Improved picking in the editor: now it is using precise hit test against node's geometry.
- "Ignore back faces" option for picking in the editor: allows you to pick through "back" of polygon
faces, especially useful for closed environment.
- Rotation ribbons were replaced with torus, it is much easier to select desired rotation mode.
- New material for gizmos in the editor, that prevent depth issues.
- New expander for TreeView widget, `V` and `>` arrows instead of `+` and `-` signs.
- ScrollBar widget is much thinner by default.
- Editor settings window now based on Inspector widget, which provides uniform way of data visualization.
- `DEFAULT_FONT` singleton was removed, it is replaced with `default_font`
- Shortcuts improvements in the editor.
- Overall UI performance improvements.
- Ability to disable clipping of widget bounds to parent bounds.
- Layout and render transform support for widgets - allows you to scale/rotate/translate widgets.
- Ability to make widget lowermost in hierarchy.
- Animation blending state machine refactoring, optimizations and stability improvements.
- Animation blending state machines are now stored in special container which stored in the Scene.
- Docking manager now shows anchors only for its windows.
- Model previewer now has much more intuitive controls.
- NumericUpDown don't panic anymore on edges of numeric bounds (i.e when trying to do `i32::MAX_VALUE + 1`)
- DoubleClick support for UI.
- Update rate fix for editor, it fixes annoying issue with flickering in text boxes.
- `UserInterface::hit_test_unrestricted` which performs hit test that is not restricted to current 
picking restriction stack.
- WASM renderer fixes.
- `Pool::try_free` which returns `Option<T>` on invalid handles, instead of panicking.
- Light source for model previewer
- Default skybox for editor and model previewer cameras
- `Color` API improvements.
- `#[inspect(expand)]` and `#[inspect(expand_subtree)]` were removed from `Inspect` proc-macro
- Correct field name generation for enum variants
- Ability to draw Bézier curves in the UI.
- Fix for navmesh agent navigation of multilayer navigational meshes.
- Improvements for serializer, now it allows you correctly recover from serialization errors.

## Migration guide

**WARNING:** This release **does not** provide legacy sound system conversion to new one, which means if 
any of your scene had any sound, they will be lost!

Now there is limited access to `fyrox_sound` entities, there is no way to create sound contexts, sounds, 
effects manually. You have to use respective scene nodes (`Sound`, `Listener`) and `Effect` from 
`fyrox::scene::sound` module (and children modules). 

### Nodes

Since `Node` enumeration was removed, there is a new way of managing nodes:

- `Node` now is just `Box<dyn NodeTrait>` wrapped in a new-type-struct.
- Pattern matching was replaced with `cast` and `cast_mut` methods.
- In addition to `cast/cast_mut` there are two more complex methods for polymorphism: `query_component_ref` and
`query_component_mut` which are able to extract references to internal parts of the nodes. This now has only one
usage - `Light` enumeration was removed and `PointLight`, `SpotLight`, `DirectionalLight` provides unified access
to `BaseLight` component via `query_component_ref/query_component_mut`. `query_component` could be a bit slower,
since it might involve additional branching while attempting to query component. 
- `Base` node was replaced with `Pivot` node (and respective `PivotBuilder`), it happend due to problems with 
`Deref<Target = Base>/DerefMut` implementation, if `Base` is implementing `NodeTrait` then it must implement `Deref`
but implementing `Deref` for `Base` causes infinite deref coercion loop.
- To be able to create custom scene nodes and having the ability to serialize/deserialize scene graph with such
nodes, `NodeConstructorContainer` was added. It contains a simple map `UUID -> NodeConstructor` which allows to
pick the right node constructor based on type uuid at deserialization stage.

#### Replacing `BaseBuilder` with `PivotBuilder`

It is very simply, just wrap `BaseBuilder` with a `PivotBuilder` and call `build` on `PivotBuilder` instance:

```rust
// Before
fn create_pivot_node(graph: &mut Graph) -> Handle<Node> {
    BaseBuilder::new().build(graph)    
}

// After
fn create_pivot_node(graph: &mut Graph) -> Handle<Node> {
    PivotBuilder::new(BaseBuilder::new()).build(graph)
}
```

#### Pattern matching replacement

Pattern matching was replaced with 4 new methods `cast/cast_mut/query_component_ref/query_component_mut`:

```rust
fn set_sprite_color(node: &mut Node, color: Color) {
    // Use `cast_mut` when you are sure about the real node type.
    if let Some(sprite) = node.cast_mut::<Sprite>() {
        sprite.set_color(color);
    }
}

fn set_light_color(node: &mut Node, color: Color) {
    // Use query_component_mut if you unsure what is the exact type of the node.
    // In this example the `node` could be either PointLight, SpotLight, DirectionalLight,
    // since they're all provide access to `BaseLight` via `query_component_x` the function
    // will work with any of those types.
    if let Some(base_light) = node.query_component_mut::<BaseLight>() {
        base_light.set_color(color);
    }
}
```

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

### Engine initialization

`Engine::new` signature has changed to accept `EngineInitParams`, all previous argument were moved to the 
structure. However, there are some new engine initialization parameters, like `serialization_context` and
`resource_manager`. Previously `resource_manager` was created implicitly, currently it has to be created
outside and passed to `EngineInitParams`. This is because of new `SerializationContext` which contains
a set of constructors for various types that may be used in the engine and be added by external plugins.
Typical engine initialization could look something like this:

```rust
use fyrox::engine::{Engine, EngineInitParams};
use fyrox::window::WindowBuilder;
use fyrox::engine::resource_manager::ResourceManager;
use fyrox::event_loop::EventLoop;
use std::sync::Arc;
use fyrox::engine::SerializationContext;

fn init_engine() {
    let evt = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_title("Test")
        .with_fullscreen(None);
    let serialization_context = Arc::new(SerializationContext::new());
    let mut engine = Engine::new(EngineInitParams {
        window_builder,
        resource_manager: ResourceManager::new(serialization_context.clone()),
        serialization_context,
        events_loop: &evt,
        vsync: false,
    })
    .unwrap();
}
```

## Serialization

Engine and ResourceManager both are non-serializable anymore. It changes approach of creating save files in games.
Previously you used something like this (following code snippets are modified versions of `save_load` example):

```rust
const SAVE_FILE: &str = "save.bin";

fn save(game: &mut Game) {
    let mut visitor = Visitor::new();

    game.engine.visit("Engine", visitor)?; // This no longer works
    game.game_scene.visit("GameScene", visitor)?;
    
    visitor.save_binary(Path::new(SAVE_FILE)).unwrap();
}

fn load(game: &mut Game) {
    if Path::new(SAVE_FILE).exists() {
        if let Some(game_scene) = game.game_scene.take() {
            game.engine.scenes.remove(game_scene.scene);
        }

        let mut visitor = block_on(Visitor::load_binary(SAVE_FILE)).unwrap();

        game.engine.visit("Engine", visitor)?; // This no longer works
        game.game_scene.visit("GameScene", visitor)?;
    }
}
```

However, on practice this approach could lead to some undesirable side effects. The main problem with the old 
approach is that when you serialize the engine, it serializes all scenes you have. This fact is more or less
ok if you have only one scene, but if you have two and more scenes (for example one for menu and one for 
game level) it writes/reads redundant data. The second problem is that you cannot load saved games asynchronously
using the old approach, because it takes mutable access of the engine and prevents you from off-threading work.

The new approach is much more flexible and do not have such issues, instead of saving the entire state of the
engine, you just save and load only what you actually need:

```rust
const SAVE_FILE: &str = "save.bin";

fn save(game: &mut Game) {
    if let Some(game_scene) = game.game_scene.as_mut() {
        let mut visitor = Visitor::new();

        // Serialize game scene first.
        game.engine.scenes[game_scene.scene]
            .save("Scene", &mut visitor)
            .unwrap();
        // Then serialize the game scene.
        game_scene.visit("GameScene", &mut visitor).unwrap();

        // And call save method to write everything to disk.
        visitor.save_binary(Path::new(SAVE_FILE)).unwrap();
    }
}

// Notice that load is now async.
async fn load(game: &mut Game) {
    // Try to load saved game.
    if Path::new(SAVE_FILE).exists() {
        // Remove current scene first.
        if let Some(game_scene) = game.game_scene.take() {
            game.engine.scenes.remove(game_scene.scene);
        }

        let mut visitor = Visitor::load_binary(SAVE_FILE).await.unwrap();

        let scene = SceneLoader::load("Scene", &mut visitor)
            .unwrap()
            .finish(game.engine.resource_manager.clone())
            .await;

        let mut game_scene = GameScene::default();
        game_scene.visit("GameScene", &mut visitor).unwrap();

        game_scene.scene = game.engine.scenes.add(scene);
        game.game_scene = Some(game_scene);
    }
}
```

As you can see in the new approach you save your scene and some level data, and on load - you load the scene, add
it to the engine as usual and load level's data. The new approach is a bit more verbose, but it is much more 
flexible.

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