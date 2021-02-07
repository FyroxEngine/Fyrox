[![RG3D](pics/logo.png)](https://rg3d.rs/)

# Rust Game engine 3D

[![Crates.io](https://img.shields.io/crates/l/rg3d)](https://github.com/mrDIMAS/rg3d/blob/master/LICENSE.md)
[![Crates.io](https://img.shields.io/crates/v/rg3d)](https://crates.io/crates/rg3d)
[![docs.rs](https://img.shields.io/badge/docs-website-blue)](https://docs.rs/rg3d/)
[![Discord](https://img.shields.io/discord/756573453561102427)](https://discord.gg/xENF5Uh)
[![Lines of code](https://tokei.rs/b1/github/mrDIMAS/rg3d)](https://github.com/mrDIMAS/rg3d)

A feature-rich 3D game engine written in Rust with a scene editor and a first person shooter prototype.

## Support

If you want to support the development of the project, click the link below. I'm working on the project full time and
use my savings to drive development forward, I'm looking for any financial support. 

[![Become a patron!](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/mrdimas)

## Community

[Join the Discord server](https://discord.gg/xENF5Uh)

## Screenshots

These screenshots are from [rusty-shooter](https://github.com/mrDIMAS/rusty-shooter) which is a big demo for the engine.

![1](pics/1.jpg?raw=true "Game 1")

![2](pics/2.jpg?raw=true "Game 2")

![3](pics/3.jpg?raw=true "Game 3")

![4](pics/4.jpg?raw=true "Game 4")

![5](pics/5.jpg?raw=true "Game 5")

## [Examples](https://github.com/mrDIMAS/rg3d/tree/master/examples)

There are many examples covering various aspects of the engine. Also don't hesitate to create an issue or ask on Discord if you need help!

## [Editor](https://github.com/mrDIMAS/rusty-editor/)

[![editor](https://raw.githubusercontent.com/mrDIMAS/rusty-editor/master/screenshots/latest.png)](https://github.com/mrDIMAS/rusty-editor/)

## Features

- Deferred shading.
	- Renderer based on OpenGL 3.3 Core (released in 2010) which means that your game will run on almost
	  any relatively modern GPU. 
	- Directional light.
	- Point light + shadows.
	- Spot light + shadows.
	- Bump mapping.
	- Screen-Space Ambient Occlusion (SSAO).
	- Soft shadows.
	- Volumetric light (spot, point).
	- Instancing - render lots of objects without any overhead.
	- FXAA
- Render in texture.
- Sky box.
- Multi-camera rendering.
- Multiple scenes.
- Lightmap generator.
- Level-of-detail (LOD) support.
- Scene graph with pivot, camera, mesh, light, particle system, sprite nodes.
- Built-in save/load - save/load the state of the engine in one call.
- [High quality binaural sound with HRTF support](https://github.com/mrDIMAS/rg3d/tree/master/rg3d-sound).
- Skinning.
- Particle systems with soft particles.
- A* pathfinder.
- Navmesh.
- FBX Loader.
- Full TTF/OTF fonts support (thanks to [fontdue](https://github.com/mooman219/fontdue) and [ttf-parser](https://github.com/RazrFalcon/ttf-parser) crates).
- PNG, JPG, TGA, DDS, etc. textures (thanks to [image](https://github.com/image-rs/image) crate).
- Compressed textures support (DXT1, DXT3, DTX5)
- [Advanced node-based UI](https://github.com/mrDIMAS/rg3d/tree/master/rg3d-ui) with lots of widgets.
- Animation blending state machine - similar to Mecanim in Unity Engine.
- Animation retargetting - allows you to remap animation from one model to another.
- Async asset management (textures, models, sound buffers).
- Advanced physics (thanks to the [rapier](https://github.com/dimforge/rapier) physics engine)
    - Rigid bodies.
    - Rich set of various colliders.
    - Joints.
    - Ray cast.
    - Many other useful features.
- [Core library](https://github.com/mrDIMAS/rg3d/tree/master/rg3d-core).
- Fast iterative compilation 
	- Debug: ~3 seconds
	- Release: ~8 seconds
- Lots of examples.

## Frequently asked questions

**Q:** Does rg3d use ECS?

**A:** No. It uses generational pools (arenas) which are optimized for efficient
memory management to retain more static type safety.

**Q:** Examples running too slow on my PC, FPS is too low, help!

**A:** First, make sure you run examples on the discrete GPU, not on a built-in of your CPU. Built-in GPUs
are very slow and not suitable for rg3d. Second, make sure your discrete GPU is powerful enough to run 
modern games at a decent frame rate.

## Supported Operating Systems

- Windows - **full support**
- Linux - **full support**
- macOS - **full support**
- WebAssembly - **not supported yet**: any help is appreciated.

## Compiler version

rg3d requires the latest stable Rust compiler.

## Contributing

Contributions are very welcome! Feel free to open Issues and Pull Requests.

Check the [good first issue](https://github.com/mrDIMAS/rg3d/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) label to see where you can help.
