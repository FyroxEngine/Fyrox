// Platform-dependent crates
#[macro_use]
#[cfg(target_os="windows")]
extern crate winapi;

#[cfg(target_os="linux")]
extern crate alsa_sys;

// Generic crates.
extern crate byteorder;
extern crate rg3d_core;

pub mod error;
pub mod decoder;
pub mod buffer;
pub mod source;
pub mod device;
pub mod context;
pub mod listener;
