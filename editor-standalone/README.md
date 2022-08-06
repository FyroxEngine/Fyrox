# FyroxEd (standalone)

**WARNING:** Standalone version of the editor is not supported, use 
[project template generator](https://fyrox-book.github.io/fyrox/beginning/scripting.html) to utilize the full power
of the editor. Standalone version does not support plugins and scripts, it won't be update in next releases!

A standalone version of FyroxEd - native editor of [Fyrox engine](https://github.com/FyroxEngine/Fyrox). The standalone
version allows you only to create and edit scenes, but **not run your game in the editor**. Please see
[the book](https://fyrox-book.github.io/) to learn how to use the editor in different ways.

## How to install and run

To install the latest stable **standalone** version from crates.io use:

```shell
cargo install fyroxed
```

After that, you can run the editor by simply calling:

```shell
fyroxed
```

If you're on Linux, please make sure that the following dependencies are installed:

```shell
sudo apt install libxcb-shape0-dev libxcb-xfixes0-dev libxcb1-dev libxkbcommon-dev libasound2-dev
```

## Controls

- [Click] - Select
- [W][S][A][D] - Move camera
- [Space][Q]/[E] - Raise/Lower Camera
- [1] - Select interaction mode
- [2] - Move interaction mode
- [3] - Scale interaction mode
- [4] - Rotate interaction mode
- [Ctrl]+[Z] - Undo
- [Ctrl]+[Y] - Redo]()