//! 3D and 2D Game Engine.

#![doc(
    html_logo_url = "https://rg3d.rs/assets/logos/logo2.png",
    html_favicon_url = "https://rg3d.rs/assets/logos/logo2.png"
)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::from_over_into)]
#![allow(clippy::approx_constant)]

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

/// Core data structures and algorithms used throughout rg3d. Re-exported from the [rg3d_core](https://docs.rs/rg3d_core/*/rg3d_core/) crate.
pub mod core {
    pub use ::rg3d_core::*;
}
/// Physics for 2D scenes using the Rapier physics engine. Re-exported from the [rg3d_physics2d](https://docs.rs/rg3d_physics2d/*/rg3d_physics2d/) crate.
pub mod physics2d {
    pub use rg3d_physics2d::*;
}
/// Physics for 3D scenes using the Rapier physics engine. Re-exported from the [rg3d_physics3d](https://docs.rs/rg3d_physics3d/*/rg3d_physics3d/) crate.
pub mod physics3d {
    pub use rg3d_physics3d::*;
}
/// Resource management. Re-exported from the [rg3d_resource](https://docs.rs/rg3d_resource/*/rg3d_resource/) crate.
pub mod asset {
    pub use rg3d_resource::*;
}
/// Sound library for games and interactive applications. Re-exported from the [rg3d_sound](https://docs.rs/rg3d_sound/*/rg3d_sound/) crate.
pub mod sound {
    pub use rg3d_sound::*;
}
/// Extendable, retained mode, graphics API agnostic UI library. Re-exported from the [rg3d_ui](https://docs.rs/rg3d_ui/*/rg3d_ui/) crate.
pub mod gui {
    pub use rg3d_ui::*;
}
