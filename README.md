# rusty editor

Scene editor for [rg3d engine](https://github.com/mrDIMAS/rg3d). **It is not ready for use yet.**

## Motivation

rg3d engine getting bigger, but still does not have scene editor what makes creation of scenes harder - you have to use 3d editors (like Blender, 3ds Max, etc.) to create scenes in them, no need to say that this looks like "hack" instead of normal solution. This editor is planned to be relatively small; not tied to any type of game. It will be used to compose scene from existing 3d models, setup physics, and all such stuff.

## Screenshots

![1](screenshots/1.png?raw=true "Editor")

## Controls

- [Click] - Select
- [W][S][A][D] - Move camera
- [1] - Move interaction mode
- [2] - Scale interaction mode
- [3] - Rotate interaction mode
- [Z] - Undo
- [Y] - Redo

## How to use produced scenes.

`rgs` can be loaded by standard object visitor like this:

```rust
use rg3d::core::visitor::Visitor;
use rg3d::scene::Scene;

// Load scene
let mut visitor = Visitor::load_binary("your_scene.rgs").unwrap();
let mut scene = Scene::default();
scene.visit("Scene", &mut visitor).unwrap();

...

// and add to engine
let scene_handle = engine.scenes.add(scene);

```

## Plan

- [x] Interaction modes.
	- [x] Move.
	- [x] Scale.
	- [x] Rotate.
- [x] Undo/redo.
- [x] Camera controller.
- [x] Save scene.
- [x] Load scene.
- [x] Docking windows.
- [x] Scene preview
- [x] Side bar with interaction modes.
- [ ] Commands
	- [x] Move.
	- [x] Scale.
	- [x] Rotate.
	- [x] Delete node.
	- [x] Create node.
	- [x] Link nodes.
	- [ ] Other?
- [ ] World outliner
	- [x] Syncing with graph.
	- [x] Syncing selection with scene selection and vice versa.
	- [x] Drag'n'drop hierarchy edit.
	- [ ] Nodes context menu
- [ ] Node properties editor
	- [ ] Base node
		- [x] Show name.
		- [ ] Edit name.
		- [x] Edit position.
		- [x] Edit rotation.
		- [x] Edit scale.
	- [ ] Light node
	- [ ] Particle system node.
		- [ ] Particle system properties.
	- [ ] Sprite node.
		- [ ] Sprite properties.
	- [ ] Mesh node.
		- [ ] Mesh properties.
- [ ] Asset browser.

... Lots of stuff.