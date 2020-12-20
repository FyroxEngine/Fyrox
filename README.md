# rusty editor

Scene editor for [rg3d engine](https://github.com/mrDIMAS/rg3d).

## Motivation

rg3d engine getting bigger, but still does not have scene editor what makes creation of scenes harder - you have to use 3d editors (like Blender, 3ds Max, etc.) to create scenes in them, no need to say that this looks like "hack" instead of normal solution. This editor is planned to be relatively small; not tied to any type of game. It will be used to compose scene from existing 3d models, setup physics, and all such stuff.

## Limitations

It should be noted that this editor is the **scene** editor, it does **not** allow you to run your game inside like many other editors do (Unreal Engine, Unity, etc.). This fact means that each prototyping iteration of your game will take more time. Having the ability to run game inside editor would be nice indeed, but this is too much work for one person and I just don't want to spend time on this.

## Screenshots

![1](screenshots/latest.png?raw=true "Editor")

## Controls

- [Click] - Select
- [W][S][A][D] - Move camera
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
let mut scene = Scene::from_file("your_scene.rgs", &mut engine.resource_manager.lock().unwrap()).unwrap();

...

// and add to engine
let scene_handle = engine.scenes.add(scene);

```

## Plan

- [x] Interaction modes.
	- [x] Select.
	- [x] Move.
	- [x] Scale.
	- [x] Rotate.
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
		- [x] Light
		  	- [x] Directional light
			- [x] Spot light
			- [x] Point light
		- [x] Particle system
		- [x] Camera
		- [x] Sprite
		- [x] Pivot
- [ ] World outliner
	- [x] Syncing with graph.
	- [x] Syncing selection with scene selection and vice versa.
	- [x] Drag'n'drop hierarchy edit.
	- [x] Icons for nodes
	- [x] Visibility switch
	- [ ] Nodes context menu
- [ ] Node properties editor
	- [x] Base node
		- [x] Name
		- [x] Position
		- [x] Rotation
		- [x] Scale
		- [x] Physical body
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
	- [ ] Properties  	
- [ ] Navmesh editor
- [x] Configurator
  	- [x] History
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