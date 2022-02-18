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
pub mod game;
pub mod material;
pub mod renderer;
pub mod resource;
pub mod scene;
pub mod utils;

pub use crate::core::rand;
#[cfg(not(target_arch = "wasm32"))]
pub use glutin::*;
pub use lazy_static;
pub use tbc;
pub use walkdir;
#[cfg(target_arch = "wasm32")]
pub use winit::*;

#[doc(inline)]
pub use fyrox_core as core;

#[doc(inline)]
pub use fyrox_resource as asset;

#[doc(inline)]
pub use fyrox_ui as gui;

/// Defines a builder's `with_xxx` method.
#[macro_export]
macro_rules! define_with {
    ($(#[$attr:meta])* fn $name:ident($field:ident: $ty:ty)) => {
        $(#[$attr])*
        pub fn $name(mut self, value: $ty) -> Self {
            self.$field = value;
            self
        }
    };
}
