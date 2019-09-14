#![allow(dead_code)]

extern crate image;
extern crate glutin;
extern crate lexical;
extern crate byteorder;
extern crate inflate;
extern crate rand;
#[macro_use]
extern crate lazy_static;
extern crate rg3d_core;
extern crate rg3d_sound;

pub mod utils;
pub mod scene;
pub mod renderer;
pub mod engine;
pub mod resource;
pub mod physics;
pub mod gui;