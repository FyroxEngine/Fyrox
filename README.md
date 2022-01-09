<div align="center">
  <a href="https://rg3d.rs/">
    <img src="pics/logo.png" alt="Fyrox" />
  </a>

  <h1>Fyrox - a modern Rust game engine</h1>
</div>

[![License (MIT)](https://img.shields.io/crates/l/rg3d)](https://github.com/FyroxEngine/Fyrox/blob/master/LICENSE.md)
[![CI Status](https://github.com/FyroxEngine/Fyrox/actions/workflows/ci.yml/badge.svg)](https://github.com/FyroxEngine/Fyrox/actions/workflows/ci.yml)
[![audit](https://github.com/FyroxEngine/Fyrox/actions/workflows/audit.yml/badge.svg)](https://github.com/FyroxEngine/Fyrox/actions/workflows/audit.yml)
[![Dependency status](https://deps.rs/repo/github/FyroxEngine/Fyrox/status.svg)](https://deps.rs/repo/github/FyroxEngine/Fyrox)
[![Crates.io](https://img.shields.io/crates/v/rg3d)](https://crates.io/crates/rg3d)
[![docs.rs](https://img.shields.io/badge/docs-website-blue)](https://docs.rs/Fyrox/)
[![Discord](https://img.shields.io/discord/756573453561102427)](https://discord.gg/xENF5Uh)
[![Lines of code](https://tokei.rs/b1/github/FyroxEngine/Fyrox)](https://github.com/FyroxEngine/Fyrox)

A feature-rich, production-ready, general purpose 2D/3D game engine written in Rust with a scene editor.

## Support

If you want to support the development of the project, click the link below. I'm working on the project full time and
use my savings to drive development forward, I'm looking for any financial support.

[![Become a patron!](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/mrdimas)

## Community

[Join the Discord server](https://discord.gg/xENF5Uh)

## [Learning materials](https://rg3d-book.github.io)

[Read the official Fyrox book here.](https://rg3d-book.github.io) It is in early development stage, but it should
help you start using the engine, also the book contains a series of tutorials that should help you to create your
first game.

## Features

[![Video](pics/video.png)](https://www.youtube.com/watch?v=N8kmZ9aBtZs)

### General

- Exceptional safety, reliability, and speed.
- PC (Windows, Linux, macOS) and [Web (WebAssembly) support](https://rg3d.rs/assets/webexample/index.html).
- Modern 3D rendering pipeline.
- Comprehensive [documentation](https://docs.rs/rg3d).
- [Guide book](https://rg3d-book.github.io) 
- 2D support.
- [Scene editor](https://github.com/FyroxEngine/Fyrox/tree/master/editor).
- Fast iterative compilation.
- Classic object-oriented design.
- Lots of examples.

### Rendering

- Custom shaders, materials, and rendering techniques.
- Physically-based rendering.
- Metallic workflow.
- High dynamic range (HDR) rendering.
- Tone mapping.
- Color grading.
- Auto-exposure.
- Gamma correction.
- Deferred shading.
- Directional light.
- Point lights + shadows.
- Spotlights + shadows.
- Screen-Space Ambient Occlusion (SSAO).
- Soft shadows.
- Volumetric light (spot, point).
- Batching.
- Instancing.
- Fast Approximate Anti-Aliasing (FXAA).
- Normal mapping.
- Parallax mapping.
- Render in texture.
- Forward rendering for transparent objects.
- Sky box.
- Deferred decals.
- Multi-camera rendering.
- Lightmapping.
- Soft particles.
- Fully customizable vertex format.
- Compressed textures support.
- High-quality mip-map on-demand generation.

### Scene

- Multiple scenes.
- Full-featured scene graph.
- Level-of-detail (LOD) support.
- GPU Skinning.
- Various scene nodes:
  - Pivot.
  - Camera.
  - Decal.
  - Mesh.
  - Particle system.
  - Sprite.
  - Multilayer terrain.
  - Rectangle (2D Sprites)
  - Rigid body + Rigid Body 2D
  - Collider + Collider 2D
  - Joint + Joint 2D

### Sound

- [High quality binaural sound with HRTF support](https://github.com/FyroxEngine/Fyrox/tree/master/fyrox-sound).
- Generic and spatial sound sources.
- Built-in streaming for large sounds.
- Raw samples playback support.
- WAV/OGG format support.
- HRTF support for excellent positioning and binaural effects.
- Reverb effect.

### Serialization

- Powerful serialization system
- Almost every entity of the engine can be serialized
- No need to write your own serialization.

### Animation

- Animation blending state machine - similar to Mecanim in Unity Engine.
- Animation retargetting - allows you to remap animation from one model to another.

### Asset management

- Advanced asset manager.
- Fully asynchronous asset loading.
- PNG, JPG, TGA, DDS, etc. textures.
- FBX models loader.
- WAV, OGG sound formats.
- Compressed textures support (DXT1, DXT3, DTX5).

### Artificial Intelligence (AI)

- A* pathfinder.
- Navmesh.
- Behavior trees.

### User Interface (UI)

- [Advanced node-based UI](https://github.com/FyroxEngine/Fyrox/tree/master/fyrox-ui) with lots of widgets.
- More than 32 widgets
- Powerful layout system.
- Full TTF/OTF fonts support.
- Based on message passing.
- Fully customizable.
- GAPI-agnostic.
- OS-agnostic.
- Button widget.
- Border widget.
- Canvas widget.
- Color picker widget.
- Color field widget.
- Check box widget.
- Decorator widget.
- Drop-down list widget.
- Grid widget.
- Image widget.
- List view widget.
- Popup widget.
- Progress bar widget.
- Scroll bar widget.
- Scroll panel widget.
- Scroll viewer widget.
- Stack panel widget.
- Tab control widget.
- Text widget.
- Text box widget.
- Tree widget.
- Window widget.
- File browser widget.
- File selector widget.
- Docking manager widget.
- NumericUpDown widget.
- `Vector3<f32>` editor widget.
- Menu widget.
- Menu item widget.
- Message box widget.
- Wrap panel widget.
- Curve editor widget.
- User defined widget.

### Physics

- Advanced physics (thanks to the [rapier](https://github.com/dimforge/rapier) physics engine)
- Rigid bodies.
- Rich set of various colliders.
- Joints.
- Ray cast.
- Many other useful features.
- 2D support.

## Screenshots

These screenshots are from [Station Iapetus](https://github.com/mrDIMAS/StationIapetus) which is a commercial project
made with the engine.

![1](pics/1.jpg?raw=true "Game 1")

![2](pics/2.jpg?raw=true "Game 2")

These screenshots are from [rusty-shooter](https://github.com/mrDIMAS/rusty-shooter) which is a big demo for the engine.

![3](pics/3.jpg?raw=true "Game 3")

![4](pics/4.jpg?raw=true "Game 4")

![5](pics/5.jpg?raw=true "Game 5")

## [Examples](https://github.com/FyroxEngine/Fyrox/tree/master/examples)

There are many examples covering various aspects of the engine. Also don't hesitate to create an issue or ask on Discord if you need help!

## [Editor](https://github.com/FyroxEngine/Fyrox/tree/master/editor)

[![editor](https://raw.githubusercontent.com/FyroxEngine/Fyrox/master/editor/screenshots/latest.png)](https://github.com/FyroxEngine/Fyrox/tree/master/editor)

## Dependencies

### Linux

```shell
sudo apt install libxcb-shape0-dev libxcb-xfixes0-dev libxcb1-dev libxkbcommon-dev libasound2-dev
```

## Contributing

Contributions are very welcome! Feel free to open Issues and Pull Requests.

Check the [good first issue](https://github.com/FyroxEngine/Fyrox/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) label to see where you can help.

## Sponsors

### JetBrains

JetBrains provided an open-source all-products license for their products which drastically helps in development of the engine.

<img src="https://resources.jetbrains.com/storage/products/company/brand/logos/jb_beam.png" alt="JetBrains logo." width="200" height="200">

_Copyright © 2000-2021 [JetBrains](https://jb.gg/OpenSource) s.r.o. JetBrains and the JetBrains logo are registered trademarks of JetBrains s.r.o._
