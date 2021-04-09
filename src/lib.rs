//! 3D Game Engine.

#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::from_over_into)]

extern crate ddsfile;
extern crate glutin;
extern crate image;
extern crate inflate;
extern crate lexical;
extern crate rayon;
#[cfg(feature = "serde_integration")]
extern crate serde;

#[cfg(test)]
extern crate imageproc;

pub mod animation;
pub mod engine;
pub mod renderer;
pub mod resource;
pub mod scene;
pub mod utils;

pub use crate::core::rand;
pub use glutin::*;
pub use lazy_static;
pub use tbc;

pub use futures;
pub use rapier3d as physics;
pub use rg3d_core as core;
pub use rg3d_sound as sound;
pub use rg3d_ui as gui;
