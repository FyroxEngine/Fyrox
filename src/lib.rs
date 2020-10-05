//! 3D Game Engine.
//!
//! Features:
//! - Scene graph with pivot, camera, mesh, light, particle system, sprite nodes.
//! - FBX Loader - both ASCII and binary. Note: Only 7100 - 7400 versions are supported!
//! - Advanced node-based UI with these widgets:
//!     - Border
//!     - Button
//!     - Canvas (layout panel)
//!     - Grid (layout panel)
//!     - Stack panel
//!     - Scroll bar
//!     - Scroll viewer
//!     - Scroll content presenter
//!     - Text
//!     - Text box
//!     - List box
//!     - Window
//! - Fonts - TTF Loader (compound characters are not supported yet)
//! - Built-in save/load using object visitor - save/load state of engine in one call.
//! - Skinning
//! - Animation blending - allows you to blend your animations as you want to, i.e. idle animation can be blended with walk.
//! - Animation retargetting - allows you to remap animation from one model to another.
//! - Automatic resource management
//!     - Texture
//!     - Models
//!     - Sound buffers
//! - Deferred shading
//!     - Point light
//!     - Spot light
//!     - Bump mapping
//! - Particle systems with soft particles.
//! - Sounds
//! - Physics
//!
//! # Demos
//!
//! For now there is one big project written using rg3d engine:
//!  https://github.com/mrDIMAS/rusty-shooter
//!

extern crate byteorder;
extern crate glutin;
extern crate image;
extern crate inflate;
extern crate lexical;
#[cfg(feature = "serde_integration")]
extern crate serde;
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate imageproc;

pub mod animation;
pub mod engine;
pub mod renderer;
pub mod resource;
pub mod scene;
pub mod utils;

pub use glutin::*;
pub use rand;

pub use rg3d_core as core;
pub use rg3d_physics as physics;
pub use rg3d_sound as sound;
pub use rg3d_ui as gui;
