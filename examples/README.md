# Examples

The engine contains examples for various aspects. Currently there is not much, but I'm working on it. 
It is better to run examples with `--release` flag, because Debug version is too slow.

## How to build

Make sure you have latest `rg3d` dependecies near `rg3d` crate: this is needed because engine split into multiple crates which are decoupled from each other. You need to get `rg3d-ui` `rg3d-sound` `rg3d-core` and `rg3d-physics` crates. So typical build script can be:

```
git clone https://github.com/mrDIMAS/rg3d
git clone https://github.com/mrDIMAS/rg3d-ui
git clone https://github.com/mrDIMAS/rg3d-core
git clone https://github.com/mrDIMAS/rg3d-sound
git clone https://github.com/mrDIMAS/rg3d-physics
cd rg3d
cargo run --example <example_name> --release
```

Or if you already have all dependencies, you can do:

```
cd rg3d
git pull
cd ../rg3d-ui
git pull
cd ../rg3d-core
git pull
cd ../rg3d-sound
git pull
cd ../rg3d-physics
git pull
cd ../rg3d
cargo run --example <example_name> --release
```

## Example 01 - Simple Scene

*Difficulty*: Easy.

This example shows how to create simple scene with animated model.

![Example 01](screenshots/simple.png?raw=true "Example 01")

## Example 02 - Asynchronous Scene Loading

*Difficulty*: Medium.

This example shows how to load scene in separate thread and how create standard loading screen which will show progress.

![Example 02_0](screenshots/async_0.png?raw=true "Example 02_0")
![Example 02_1](screenshots/async_1.png?raw=true "Example 02_1")

## Example 03 - 3rd Person Walking Sumilator

*Difficulty*: Advanced.

This example based on async example, because it requires to load decent amount of resources which might be slow on some machines.

In this example we'll create simple 3rd person game with character that can idle, walk, or jump.

Also this example demonstrates the power of animation blending machines. Animation blending machines are used in all modern games to create complex animations from set of simple ones.

![Example 03](screenshots/3rd_person.png?raw=true "Example 03")

## Example 04 - User Interface

*Difficulty*: Medium

This example shows how to use user interface system of engine. It is based on simple.rs example because UI will be used to operate on model.

![Example 04](screenshots/ui.png?raw=true "Example 04")

## Example 05 - Save/load

- TODO

## Example 06 - Sound

- TODO

## Example 07 - Simple game

- TODO