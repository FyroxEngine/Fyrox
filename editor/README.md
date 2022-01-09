# rusty editor

Scene editor for [Fyrox engine](https://github.com/FyroxEngine/Fyrox).

## Limitations

It should be noted that this editor is the **scene** editor, it does **not** allow you to run your game inside 
like many other editors do (Unreal Engine, Unity, etc.). This fact means that each prototyping iteration of your
game will take more time. Having the ability to run game inside editor would be nice indeed, but this is too much 
work for one person and I just don't want to spend time on this.

## How to install and run

To install the latest stable version from crates.io use:

```shell
cargo install rusty-editor
```

After that, you can run the editor by simply calling:

```shell
rusty-editor
```

If you're on Linux, please make sure that the following dependencies are installed:

```shell
sudo apt install libxcb-shape0-dev libxcb-xfixes0-dev libxcb1-dev libxkbcommon-dev libasound2-dev
```

## How to build latest unstable version

If you want to tackle the latest _unstable_ version of the editor the following instructions will
help you:

```shell
cd ../
cargo run --release --package rusty-editor
```

### Platform specific

#### Linux

On Linux you need to install additional dependencies first:

```shell
sudo apt install libxcb-shape0-dev libxcb-xfixes0-dev libxcb1-dev libxkbcommon-dev libasound2-dev
```

## Screenshots

![1](screenshots/latest.png?raw=true "Editor")

## Controls

- [Click] - Select
- [W][S][A][D] - Move camera
- [Space][Q]/[E] - Raise/Lower Camera
- [1] - Select interaction mode
- [2] - Move interaction mode
- [3] - Scale interaction mode
- [4] - Rotate interaction mode
- [Ctrl]+[Z] - Undo
- [Ctrl]+[Y] - Redo