# Examples

The engine contains examples for various aspects. Currently, there is not much, but I'm working on it. 
It is better to run examples with the `--release` flag, because Debug is too slow. Also Debug may suffer from
a "glitch" when model loads faster than its textures, it is due asynchronous resource loading.

## Example 01 - Simple Scene

*Difficulty*: Easy.

This example shows how to create a simple scene with an animated model.

![Example 01](screenshots/simple.png?raw=true "Example 01")

## Example 02 - Asynchronous Scene Loading

*Difficulty*: Medium.

This example shows how to load a scene in a separate thread and how to create a standard loading screen which will show progress.

![Example 02_0](screenshots/async_0.png?raw=true "Example 02_0")
![Example 02_1](screenshots/async_1.png?raw=true "Example 02_1")

## Example 03 - 3rd Person Walking Simulator

*Difficulty*: Advanced.

This is based on the async example, because it requires loading a decent amount of resources which might be slow on some machines.

In this example we'll create a simple 3rd person game with a character that can idle, walk, or jump.

It also demonstrates the power of animation blending machines. Animation blending machines are used in all modern games to create complex animations from a set of simple ones.

![Example 03](screenshots/3rd_person.png?raw=true "Example 03")

## Example 04 - User Interface

*Difficulty*: Medium

This example shows how to use the user interface system of the engine. It is based on the simple.rs example because the UI will be used to operate on a model.

![Example 04](screenshots/ui.png?raw=true "Example 04")

## Example 05 - Scene

This example shows how to load a simple scene made in [rusty-editor](https://github.com/mrDIMAS/rusty-editor)

![Example 05](screenshots/scene.png?raw=true "Example 05")

## Example 06 - Save/load

Same as Example 03, but also has "save/load" functionality - F5 and F9 keys respectively.

## Example 07 - Sound

Same as Example 04, but also has foot step sounds and a reverb effect.

## Example 08 - Level of detail

This example shows how to create and use LOD groups to improve performance.
TODO: It still should be improved, it needs to use a more high poly model to show the true power of the technique.

![Example 08](screenshots/lod.png?raw=true "Example 08")

## Example 09 - Lightmap

Lightmap is a texture with precomputed light. This example shows how to load a simple scene made in 
[rusty-editor](https://github.com/mrDIMAS/rusty-editor) and generate a lightmap for it. Lightmaps are still in
active development and not meant to be used.

![Example 09](screenshots/lightmap.png?raw=true "Example 09")

## Example 10 - Instancing

This example shows how to create a simple scene with lots of animated models with a low performance
impact.

![Example 10](screenshots/instancing.jpg?raw=true "Example 10")

## Example 11 - Simple game

- TODO
