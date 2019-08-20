#![allow(dead_code)]

// Textures
extern crate image;
// Window
extern crate glutin;
// Fast string -> number conversion
extern crate lexical;

extern crate byteorder;
extern crate base64;

// Serialization
extern crate serde;
extern crate serde_json;

mod utils;
mod math;
mod scene;
mod renderer;
mod engine;
mod resource;
mod physics;
mod game;
mod gui;

use crate::game::{Game};

fn main() {
    Game::new().run();
}