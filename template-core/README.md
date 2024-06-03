# Fyrox Engine Template Generator

This tiny utility handles project and script generation for Fyrox Game Engine. 

## Installation

Install it via `cargo install`:

```shell
cargo install fyrox-template
```

## Generating New Project

`fyrox-template init [--name <name> --style <style>]`

- `name` - a name of new project (default is `my_game`)
- `style` - defines a default scene type, either `2d` or `3d` (default is `3d`)

It creates a workspace with three projects:

- Game - your game project (library)
- Editor - the editor with your game attached as a plugin
- Executor - the "runner" for your game.

It also populates each project with boilerplate code. The main purpose of the project is to reduce amount of time
that is needed to set up a new project.

It will create a new folder with `<project_name>` and it will contain three projects, runnable only two of them:

- `cargo run --package editor --release` - to run your game inside the editor.
- `cargo run --package executor --release` - to run your game as a standalone project. It will also produce final
  binary of your game, that can be shipped to a store.

### Tips

There is nothing special in generated project, so you can tweak them as you wish.

## Adding New Scripts

`fyrox-template script [--name <name>]`

- `name` - a name of your script (default is `MyScript`)

The tool is also capable to generate script skeleton for you, filling it with all required boilerplate. Generated scripts
will be added to `game/src` folder, so you should run the tool from the root folder of your game (where the root Cargo.toml
is located).

Do not forget to add the script to your module tree at required position, you probably will need some small tweaks 
to generated content, it can be easily automated by modern IDEs.