//! Sound library for games and interactive applications.
//!
//! ## Features
//!
//! - Generic and spatial sounds.
//! - WAV and OGG/Vorbis formats support.
//! - Streaming.
//! - Head-related transfer function support ([HRTF](https://en.wikipedia.org/wiki/Head-related_transfer_function)).
//! - Reverb effect.
//!
//! ## Examples
//!
//! Here is an example of how to play a sound using rg3d-sound:
//!
//! ```no_run
//! use std::{
//!     thread,
//!     time::Duration
//! };
//! use rg3d_sound::{
//!     source::{
//!         generic::GenericSourceBuilder,
//!         SoundSource,
//!         Status
//!     },
//!     context::Context,
//!     buffer::{
//!         DataSource,
//!         SoundBuffer
//!     },
//! };
//!
//!  let context = Context::new().unwrap();
//!
//!  let sound_buffer = SoundBuffer::new_generic(DataSource::from_file("sound.wav").unwrap()).unwrap();
//!
//!  let source = GenericSourceBuilder::new(sound_buffer)
//!     .with_status(Status::Playing)
//!     .build_source()
//!     .unwrap();
//!
//!  context.lock()
//!     .unwrap()
//!     .add_source(source);
//!
//!  thread::sleep(Duration::from_secs(3));
//!
//! ```
//!
//! Other examples can be found in `./examples` folder. Make sure you run them with `--release` flag.
//!
//! ## Supported OS
//!
//! Currently only Windows and Linux are supported.
//!
//! ## HRTF
//!
//! Library uses special HRIR Spheres which were composed from IRCAM HRIR database. Since
//! HRTF is very specific to each person, you should try some of them to find best for you.
//! They can be found [here](https://github.com/mrDIMAS/hrir_sphere_builder/tree/master/hrtf_base/IRCAM).

#![warn(missing_docs)]

// Platform-dependent crates
#[macro_use]
#[cfg(target_os = "windows")]
extern crate winapi;

#[cfg(target_os = "linux")]
extern crate alsa_sys;

// Generic crates.
extern crate lewton;
extern crate rg3d_core;

pub mod buffer;
pub mod context;

pub mod dsp;
pub mod effects;
pub mod engine;
pub mod error;
pub mod listener;
pub mod renderer;
pub mod source;

// Reexport some modules because there some types of them in public API.
pub use hrtf;
pub use rg3d_core::algebra;
pub use rg3d_core::math;
pub use rg3d_core::pool;

mod decoder;
mod device;
