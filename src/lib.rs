//! 3D and 2D Game Engine.

#![doc(
    html_logo_url = "https://fyrox.rs/assets/logos/logo.png",
    html_favicon_url = "https://fyrox.rs/assets/logos/logo.png"
)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::from_over_into)]
#![allow(clippy::approx_constant)]

pub use fyrox_animation as generic_animation;
pub mod engine;
pub mod material;
pub mod plugin;
pub mod renderer;
pub mod resource;
pub mod scene;
pub mod script;
pub mod utils;

pub use crate::core::rand;
pub use fxhash;
pub use lazy_static;
pub use tbc;
pub use walkdir;
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
