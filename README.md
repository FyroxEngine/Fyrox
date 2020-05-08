# RG3D

3D game engine written in Rust. 

WARNING: Some places are semi-complete or just does not implemented yet, this engine is not in production-ready state yet.

## Screenshots

These screenshots are from [rusty-shooter](https://github.com/mrDIMAS/rusty-shooter) which is a big demo for the engine.

![1](pics/1.jpg?raw=true "Game 1")

![2](pics/2.jpg?raw=true "Game 2")

![3](pics/3.jpg?raw=true "Game 3")

![4](pics/4.jpg?raw=true "Game 4")

![5](pics/5.jpg?raw=true "Game 5")

## [Examples](https://github.com/mrDIMAS/rg3d/tree/master/examples)

## [Editor] (https://github.com/mrDIMAS/rusty-editor/)

## Features

- Deferred shading
	- Directional light
	- Point light + shadows
	- Spot light + shadows
	- Bump mapping
	- Screen-Space Ambient Occlusion (SSAO)
	- Soft shadows
	- Volumetric light (spot, point)
- Scene graph with pivot, camera, mesh, light, particle system, sprite nodes
- Built-in save/load - save/load state of engine in one call
- [High quality binaural sound with HRTF support](https://github.com/mrDIMAS/rg3d-sound)
- Skinning
- Particle systems with soft particles
- A* pathfinder 
- Navmesh
- FBX Loader
- TTF Fonts
- PNG, JPG, TGA, etc. textures
- [Advanced node-based UI](https://github.com/mrDIMAS/rg3d-ui) with lots of widgets.
- Animation blending state machine - similar to Mecanim in Unity Engine
- Animation retargetting - allows you to remap animation from one model to another
- Asset management (textures, models, sound buffers)
- [Simple physics](https://github.com/mrDIMAS/rg3d-physics)
- [Core library](https://github.com/mrDIMAS/rg3d-core)

## Contributing

Contributions are very welcome! Please check Issues to see how you can help project and feel free to create your own issue!

## Limitations

- FBX loader supports versions 7100 - 7400. Binary 7500 is not supported yet, but ASCII is.
- TTF loader does not supports compound characters!