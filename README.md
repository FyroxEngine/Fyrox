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

## What is done already?

- Window and OpenGL context.
- Core library ([rg3d-core](https://github.com/mrDIMAS/rg3d-core)) with some handy data structures  - object pool, vectors, matrices, etc.
- Scene graph with pivot, camera, mesh, light, particle system, sprite nodes.
- FBX Loader - both ASCII and binary. Note: Only 7100 - 7400 versions are supported!
- Advanced node-based UI with these widgets:
	- Border
	- Button
	- Canvas (layout panel)
	- Grid (layout panel)
	- Stack panel
	- Scroll bar
	- Scroll viewer
	- Scroll content presenter
	- Text
	- Text box
	- List box	
	- Window 
- Fonts - TTF Loader (compound characters are not supported yet)
- Built-in save/load using object visitor - save/load state of engine in one call.
- Skinning
- Animation blending - allows you to blend your animations as you want to, i.e. idle animation can be blended with walk.
- Animation retargetting - allows you to remap animation from one model to another.
- Automatic resource management
	- Texture
	- Models
	- Sound buffers
- Deferred shading
	- Point light
	- Spot light
	- Bump mapping
- Particle systems with soft particles.
- Sounds - using [rg3d-sound](https://github.com/mrDIMAS/rg3d-sound) crate.
- Physics - using [rg3d-physics](https://github.com/mrDIMAS/rg3d-physics) crate.

### What will be added soon? 

- Shadows

### Plans

- Optimization - some places of engine lacks optimization - there is still no culling, space partitioning for physics, etc.
- Simple editor - would be so nice to have, but until UI is not stabilized enough there is no point to try to write editor.
- Documentation - it is still incomplete because engine contstantly changing its API.

## Dependencies

- glutin - window and OpenGL initialization
- image - texture loading
- lexical - fast text -> number parsing for ASCII FBX loader 
- byteorder - read/write builtin rust types (u16, u32, f32, etc.)
- base64 - encode binary data for object visitor's text output 
- inflate - to decompress binary FBX data
- rand - to generate random numbers in various places of the engine (mostly in particle systems)

## Contributing

Contributions are very welcome!

## Why Rust?

Previously I wrote my engine in C ([DmitrysEngine](https://github.com/mrDIMAS/DmitrysEngine)), but at some point it become relatively hard to maintain it. I thought if it was hard to maintain for me, how hard it would be to use it *correcly* for newcomers? Initially I thought to port engine to modern C++, but C++ is not a silver bullet and won't guarantee memory safety. At my full-time job almost every day we fixing issues related to memory safety and threading bugs, I really tired of this and then I just remembered that Rust provides memory safety and safe concurrency. I've started to learning Rust and it was real pain in the ass at the first few weeks, I even thought to return back to C, but from some point I found ways of doing things without fighting with compiler... and engine started growing, and at some point I found that I crashed (because of segfault) only few times since start and only at unsafe code in OpenGL functions, that was very exciting!