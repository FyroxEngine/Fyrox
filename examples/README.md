# Examples

The engine contains examples for various aspects. Currently, there is not much, but I'm working on it. 
It is better to run examples with `--release` flag, because Debug is too slow. Also Debug may suffer from
a "glitch" when model loads faster than its textures, it is due asynchronous resource loading.

## Example 01 - Simple Scene

*Difficulty*: Easy.

This example shows how to create simple scene with animated model.

![Example 01](screenshots/simple.png?raw=true "Example 01")

## Example 02 - Asynchronous Scene Loading

*Difficulty*: Medium.

This example shows how to load scene in separate thread and how create standard loading screen which will show progress.

![Example 02_0](screenshots/async_0.png?raw=true "Example 02_0")
![Example 02_1](screenshots/async_1.png?raw=true "Example 02_1")

## Example 03 - 3rd Person Walking Simulator

*Difficulty*: Advanced.

This example based on async example, because it requires to load decent amount of resources which might be slow on some machines.

In this example we'll create simple 3rd person game with character that can idle, walk, or jump.

Also this example demonstrates the power of animation blending machines. Animation blending machines are used in all modern games to create complex animations from set of simple ones.

![Example 03](screenshots/3rd_person.png?raw=true "Example 03")

## Example 04 - User Interface

*Difficulty*: Medium

This example shows how to use user interface system of engine. It is based on simple.rs example because UI will be used to operate on model.

![Example 04](screenshots/ui.png?raw=true "Example 04")

## Example 05 - Scene

This example shows how to load simple scene made in [rusty-editor](https://github.com/mrDIMAS/rusty-editor)

![Example 05](screenshots/scene.png?raw=true "Example 05")

## Example 06 - Save/load

Same as Example 03, but also has "save/load" functionality - F5 and F9 keys respectively.

## Example 07 - Sound

Same as Example 04, but also has foot step sounds and reverb effect.

## Example 08 - Level of detail

This example shows how to create and use lod groups to improve performance.
TODO: It still should be improved, it needs to use more high poly model to show true power of the technique.

![Example 08](screenshots/lod.png?raw=true "Example 08")

## Example 09 - Lightmap

Lightmap is a texture with precomputed light. This example shows how to load simple scene made in 
[rusty-editor](https://github.com/mrDIMAS/rusty-editor) and generate lightmap for it. Lightmaps are still in
active development and not meant to be used.

![Example 09](screenshots/lightmap.png?raw=true "Example 09")

## Example 10 - Simple game

- TODO
