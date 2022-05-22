# Fyrox Engine Project Template Generator

This tiny utility handles project generation for Fyrox Game Engine. It creates a workspace with three projects:

- Game - your game project (library)
- Editor - the editor with your game attached as a plugin
- Executor - the "runner" for your game.

It also populates each project with boilerplate code. The main purpose of the project is to reduce amount of time
that is needed to set up a new project. 

## Usage

Install it via `cargo` first:

```shell
cargo install fyrox-template
```

Navigate to a folder and call:

```shell
fyrox-template --name <project_name>
```

It will create a new folder with `<project_name>` and it will contain three projects, runnable only two of them:

- `cargo run --package editor --release` - to run your game inside the editor.
- `cargo run --package executor --release` - to run your game as a standalone project. It will also produce final 
binary of your game, that can be shipped to a store.

## Tips

There is nothing special in generated project, so you can tweak them as you wish.