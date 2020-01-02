// Platform-dependent crates
#[macro_use]
#[cfg(target_os="windows")]
extern crate winapi;

#[cfg(target_os="linux")]
extern crate alsa_sys;

// Generic crates.
extern crate byteorder;
extern crate rg3d_core;
extern crate rustfft;
extern crate lewton;

pub mod error;
pub mod buffer;
pub mod source;
pub mod context;
pub mod listener;
pub mod hrtf;
pub mod renderer;
pub mod effects;
pub mod dsp;
pub mod decoder;
pub mod device;

// Reexport some modules because there some types of them in public API.
pub use rg3d_core::math as math;
pub use rg3d_core::pool as pool;