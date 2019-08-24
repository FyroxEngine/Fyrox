# What is this?
3d shooter written in Rust. This project ports many features from DmitrysEngine (written in C), it will eventually be split for Engine and Game parts. 

# Screenshots
- Soon

# What is done already?

## Engine

- Window (using glutin)
- Object pool
- Scene graph with pivot, camera, mesh nodes.
- FBX Loader (ASCII only for now, binary coming soon)
- TTF Loader
- Advanced UI 
- Fonts
- Position-based physics
- Object visitor (built-in save/load)
- Automatic resource management
	- Texture
	- Models

## Game

- Player 
- Weapons
- Simple level

# What will be added soon? 

- Shadows
- Deferred shading
- Particles systems (soft particles)
- GJK-EPA based collision solver
- Sound
- Ray-cast for physics

# Dependencies
glutin - window and OpenGL initialization
image - texture loading
lexical - fast text -> number parsing for ASCII FBX loader 
byteorder - read/write builtin rust types (u16, u32, f32, etc.)
base64 - encode binary data for object visitor's text output 