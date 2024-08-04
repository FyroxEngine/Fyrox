//! 3D/2D Game Engine.
//!
//! Tutorials can be found [here](https://fyrox-book.github.io/tutorials/tutorials.html)

#![doc(
    html_logo_url = "https://fyrox.rs/assets/logos/logo.png",
    html_favicon_url = "https://fyrox.rs/assets/logos/logo.png"
)]

#[cfg(not(feature = "dylib"))]
#[doc(inline)]
pub use fyrox_impl::*;

#[cfg(feature = "dylib")]
#[doc(inline)]
pub use fyrox_dylib::*;
