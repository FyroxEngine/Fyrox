# What is this?
3d shooter written in Rust. This project ports many features from DmitrysEngine (written in C), it will eventually be split for Engine and Game parts. 

WARNING: Some places are semi-complete or just does not implemented yet, this engine is not in production-ready state yet.

## Screenshots
![1](pics/1.png?raw=true "Game 1")

![2](pics/2.png?raw=true "Game 2")

![3](pics/3.png?raw=true "Game 3")

## What is done already?

### Engine

- Window (using glutin)
- Object pool
- Scene graph with pivot, camera, mesh nodes.
- FBX Loader (ASCII only for now, binary coming soon)
- TTF Loader
- Advanced node-based UI with these nodes ("widgets")
	- Border
	- Button
	- Canvas (layout panel)
	- Grid (layout panel)
	- Scroll bar
	- Scroll viewer
	- Scroll content presenter
	- Text
	- Window 
- Fonts
- Position-based physics
- Object visitor (built-in save/load)
- Automatic resource management
	- Texture
	- Models
- Deferred shading
	- Point light
	- Spot light
- Bump mapping

### What will be added soon? 

- Shadows
- Particles systems (soft particles)
- GJK-EPA based collision solver
- Sound
- Ray-cast for physics

### Game

- Player 
- Weapons
- Simple level

## Dependencies

- glutin - window and OpenGL initialization
- image - texture loading
- lexical - fast text -> number parsing for ASCII FBX loader 
- byteorder - read/write builtin rust types (u16, u32, f32, etc.)
- base64 - encode binary data for object visitor's text output 

## Contibuting

Contributions are very welcome!

## Why Rust?

Previously I wrote my engine in C ([DmitrysEngine](https://github.com/mrDIMAS/DmitrysEngine)), but at some point it become relatively hard to maintain it. I thought if it was hard to maintain for me, how hard it would be to use it *correcly* for newcomers? Initially I thought to port engine to modern C++, but C++ is not a silver bullet and won't guarantee memory safety. At my full-time job almost every day we fixing issues related to memory safety and threading bugs, I really tired of this and then I just remembered that Rust provides memory safety and safe concurrency. I've started to learning Rust and it was real pain in the ass at the first few weeks, I even thought to return back to C, but from some point I found ways of doing things without fighting with compiler... and engine started growing, and at some point I found that I crashed (because of segfault) only few times since start and only at unsafe code in OpenGL functions, that was very exciting!