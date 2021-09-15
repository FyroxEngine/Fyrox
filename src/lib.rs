//! 3D and 2D Game Engine.

#![doc(
    html_logo_url = "https://rg3d.rs/assets/logos/logo2.png",
    html_favicon_url = "https://rg3d.rs/assets/logos/logo2.png"
)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::from_over_into)]

pub mod animation;
pub mod engine;
pub mod material;
pub mod renderer;
pub mod resource;
pub mod scene;
pub mod scene2d;
pub mod utils;

pub use crate::core::rand;
#[cfg(not(target_arch = "wasm32"))]
pub use glutin::*;
pub use lazy_static;
pub use tbc;
pub use walkdir;
#[cfg(target_arch = "wasm32")]
pub use winit::*;

pub use rg3d_core as core;
pub use rg3d_physics2d as physics2d;
pub use rg3d_physics3d as physics3d;
pub use rg3d_resource as asset;
pub use rg3d_sound as sound;
pub use rg3d_ui as gui;
