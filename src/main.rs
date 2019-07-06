#![allow(dead_code)]

// Textures
extern crate image;
// Window
extern crate glutin;
// Fast string -> number conversion
extern crate lexical;

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

use crate::game::{Game};

fn main() {
    Game::new().run();
}