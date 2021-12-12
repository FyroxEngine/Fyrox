# rusty editor

Scene editor for [rg3d engine](https://github.com/rg3dengine/rg3d).

[![CI Status](https://github.com/rg3dengine/rusty-editor/actions/workflows/ci.yml/badge.svg)](https://github.com/rg3dengine/rusty-editor/actions/workflows/ci.yml)
[![Discord](https://img.shields.io/discord/756573453561102427)](https://discord.gg/xENF5Uh)
[![Lines of code](https://tokei.rs/b1/github/rg3dengine/rusty-editor)](https://github.com/rg3dengine/rusty-editor)


## Limitations

It should be noted that this editor is the **scene** editor, it does **not** allow you to run your game inside like many other editors do (Unreal Engine, Unity, etc.). This fact means that each prototyping iteration of your game will take more time. Having the ability to run game inside editor would be nice indeed, but this is too much work for one person and I just don't want to spend time on this.

## How to build

### Platform specific

#### Linux

```shell
sudo apt install libxcb-shape0-dev libxcb-xfixes0-dev libxcb1-dev libxkbcommon-dev libasound2-dev
```

### Clean build

```shell
git clone https://github.com/mrDIMAS/rg3d
git clone https://github.com/mrDIMAS/rusty-editor
cd rusty-editor
cargo run --release
```

### Update to latest and run

```shell
cd rg3d
git pull
cd ../rusty-editor
git pull
cargo run --release
```

## Screenshots

![1](screenshots/latest.png?raw=true "Editor")

## Controls

- [Click] - Select
- [W][S][A][D] - Move camera
- [Space][Q]/[E] - Raise/Lower Camera
- [1] - Select interaction mode
- [2] - Move interaction mode
- [3] - Scale interaction mode
- [4] - Rotate interaction mode
- [Ctrl]+[Z] - Undo
- [Ctrl]+[Y] - Redo

## How to use produced scenes.

`rgs` files can be loaded as standard model resources:

```rust
use rg3d::scene::Scene;

// Create test scene, this step is unnecessary, if you already have some scene
// you can instantiate model into your scene.
let mut scene = Scene::default();

// There is no difference between scene created in rusty-editor and any other
// model file, so any scene can be used directly as resource.
let root = resource_manager
	.request_model("your_scene.rgs")
	.await
	.unwrap()
	.lock()
	.unwrap()
	.instantiate(&mut scene)
	.root;

let scene_handle = engine.scenes.add(scene);

```

Alternatively `rgs` can be loaded by `Scene::from_file` method:

```rust
use rg3d::core::visitor::Visitor;
use rg3d::scene::Scene;

// Load scene
let mut scene = Scene::from_file("your_scene.rgs", &mut engine.resource_manager.lock().unwrap()).await.unwrap();

...

// and add to engine
let scene_handle = engine.scenes.add(scene);

```

## Plan

- [x] Interaction modes.
	- [x] Select.
	- [x] Move.
	  	- [x] Grid snapping
	- [x] Scale.
	    - [ ] Grid snapping
	- [x] Rotate.
		- [ ] Grid snapping
	- [x] Navmesh
	- [x] Terrain
- [x] Undo/redo.
- [x] Camera controller.
- [x] Save scene.
  	- [x] Validation
- [x] Load scene.
- [x] Docking windows.
- [x] Scene preview
- [x] Side bar with interaction modes.
- [x] Multi selection
- [x] Menu
	- [x] File
		- [x] New scene
		- [x] Save
		- [x] Save as
		- [x] Load
		- [x] Close scene
		- [x] Exit
	- [x] Edit
		- [x] Undo
		- [x] Redo
	- [x] Create
		- [x] Mesh
			- [x] Cube
			- [x] Sphere
			- [x] Cone
			- [x] Cylinder
			- [x] Quad
		- [x] Light
		  	- [x] Directional light
			- [x] Spot light
			- [x] Point light
        - [x] Sounds
            - [x] 2D
            - [x] 3D
		- [x] Particle system
		- [x] Camera
		- [x] Sprite
		- [x] Pivot
		- [x] Terrain
- [ ] World outliner
	- [x] Syncing with graph.
	- [x] Syncing selection with scene selection and vice versa.
	- [x] Drag'n'drop hierarchy edit.
	- [x] Icons for nodes
	- [x] Visibility switch
	- [x] Nodes context menu
- [ ] Node properties editor
	- [x] Base node
		- [x] Name
		- [x] Position
		- [x] Rotation
		- [x] Scale
		- [x] Physical body
		- [x] Physical binding
	- [x] Light node
		- [x] Cast shadows
		- [x] Enable scatter
		- [x] Scatter
		- [x] Color
		- [x] Point light
			- [x] Radius
		- [x] Spot light
			- [x] Hotspot angle
			- [x] Falloff angle delta
			- [x] Distance
		- [x] Directional light
	- [x] Camera node
		- [x] Fov
		- [x] Z near
		- [x] Z far
	- [ ] Particle system node.
		- [x] Acceleration
		- [x] Select emitter
		- [x] Add/remove emitter
		- [x] Position
		- [x] Spawn Rate
		- [x] Max Particles
		- [x] Min Lifetime
		- [x] Max Lifetime
		- [x] Min Size Modifier
		- [x] Max Size Modifier
		- [x] Min X Velocity
		- [x] Max X Velocity
		- [x] Min Y Velocity
		- [x] Max Y Velocity
		- [x] Min Z Velocity
		- [x] Max Z Velocity
		- [x] Min Rotation Speed
		- [x] Max Rotation Speed
		- [x] Min Rotation
		- [x] Max Rotation
		- [x] Resurrect Particles
		- [x] Sphere emitter
		- [ ] Color-over-lifetime curve (requires curve editor)
	- [x] Sprite node.
		- [x] Size
		- [x] Rotation
		- [x] Color
	- [x] Mesh node
		- [x] Cast shadows
	- [x] Joints
		- [x] Ball joint
		- [x] Prismatic joint
		- [x] Fixed joint
		- [x] Revolute joint
	- [x] Colliders
	  	- [ ] Multiple colliders per body
	  	- [x] Translation
	  	- [x] Rotation
	- [x] Rigid body
- [ ] Shape editors - many things in the engine has shapes: colliders, emitters, etc. There should be shape editor
  that will allow to edit them using mouse. Currently, editing is performed by setting values directly in side bar.
- [ ] Curve editor - many parameters can be expressed as a curve, we need a way to edit such curves.
- [ ] Sound - we need a way to add/remove/edit sounds.
    - [ ] Move via Move interaction mode
	- [x] Properties  	
- [x] Navmesh editor
- [x] Terrain editor
    - [x] Push/pull 
	- [x] Draw on mask
	- [x] Add/remove layers
- [x] Configurator
  	- [x] History
- [x] Settings window
    - [x] Graphics 
	- [x] Debugging
	- [x] Move interaction mode
	- [ ] Rotate interaction mode
	- [ ] Scale interaction mode	
	- [ ] Navmesh interaction mode	
	- [ ] Terrain interaction mode	
- [x] Asset browser.
	- [x] Asset previewer
	- [x] Folder view
	- [x] Asset view
	- [x] Drag'n'drop resource import to scene
	- [ ] Sync with file system on changes
- [x] Command stack viewer
  	- [x] Undo/redo buttons
    - [x] Command stack visualization
- [ ] Animation graph editor
	- [ ] Simple node-based animation blending machine editor is needed.
- [ ] Scene preview in runtime. Currently, an editable scene is static: no physics simulation, particle systems are
	frozen, etc. We need an ability to clone a current scene and put it into the engine for preview with free camera.
  	This is somewhat similar to "play mode" in Unity, Unreal Engine, etc. but much more restrictive because it should
  	allow only to observe your scene in dynamics.

... Lots of stuff.
