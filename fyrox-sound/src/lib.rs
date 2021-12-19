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
//! Here is an example of how to play a sound using fyrox-sound:
//!
//! ```no_run
//! use std::{
//!     thread,
//!     time::Duration
//! };
//! use fyrox_sound::{
//!     source::{
//!         generic::GenericSourceBuilder,
//!         SoundSource,
//!         Status
//!     },
//!     context::SoundContext,
//!     buffer::{
//!         DataSource,
//!         SoundBufferResource
//!     },
//! };
//!
//!  let context = SoundContext::new();
//!
//!  let sound_buffer = SoundBufferResource::new_generic(fyrox_sound::futures::executor::block_on(DataSource::from_file("sound.wav")).unwrap()).unwrap();
//!
//!  let source = GenericSourceBuilder::new()
//!     .with_buffer(sound_buffer)
//!     .with_status(Status::Playing)
//!     .build_source()
//!     .unwrap();
//!
//!  context.state()
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
pub use fyrox_core::algebra;
pub use fyrox_core::futures;
pub use fyrox_core::math;
pub use fyrox_core::pool;

mod decoder;
mod device;
