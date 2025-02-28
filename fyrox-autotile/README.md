# fyrox-autotile

An implementation of autotiling algorithms for use with regular tile grids. This includes:

- Interactive terrain-based autotiling for tile map editing. The user picks a broad category of tile for each cell. These categories are called *terrains* and each terrain may have many tiles. The algorithm picks specific tiles for each cell from its terrain so that the tiles fit naturally together, thus sparing the user from having to manually select matching tiles.

    The algorithm is based upon a Godot library for the same purpose: https://github.com/dandeliondino/terrain-autotiler

- Randomized tile generation using Wave Function Collapse. Unlike terrain-based autotiling where the purpose is to automate tedious work for an artist, the purpose of Wave Function Collapse is to replace the artist entirely by generating creative tile layouts through randomization.

    The algorithm is based upon fast-wfc, a C++ library for the same purpose: https://github.com/math-fehr/fast-wfc

These algorithms are designed to be generic in a way that will work with *any* regular grid. They should work just as well with square cells and hexagon cells and 3D grids. The user specifies the coordinate system that is used to identify cells and which cells are neighbors to which other cells.
