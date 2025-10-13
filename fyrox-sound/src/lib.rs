// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
//!         SoundSourceBuilder,
//!         SoundSource,
//!         Status
//!     },
//!     context::SoundContext,
//!     buffer::{
//!         DataSource,
//!         SoundBufferResource
//!     },
//! };
//! use fyrox_sound::buffer::SoundBufferResourceExtension;
//! use fyrox_resource::io::FsResourceIo;
//!
//!  let context = SoundContext::new();
//!
//!  let sound_buffer = SoundBufferResource::new_generic(fyrox_sound::futures::executor::block_on(DataSource::from_file("sound.wav", &FsResourceIo)).unwrap()).unwrap();
//!
//!  let source = SoundSourceBuilder::new()
//!     .with_buffer(sound_buffer)
//!     .with_status(Status::Playing)
//!     .build()
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
//! - Windows (DirectSound)
//! - Linux (alsa)
//! - macOS (CoreAudio)
//! - WebAssembly (WebAudio)
//! - Android (AAudio, API Level 26+)
//!
//! ## HRTF
//!
//! Library uses special HRIR Spheres which were composed from IRCAM HRIR database. Since
//! HRTF is very specific to each person, you should try some of them to find best for you.
//! They can be found [here](https://github.com/mrDIMAS/hrir_sphere_builder/tree/master/hrtf_base/IRCAM).

#![warn(missing_docs)]

pub mod buffer;
pub mod context;

pub mod bus;
pub mod dsp;
pub mod effects;
pub mod engine;
pub mod error;
pub mod listener;
pub mod renderer;
pub mod source;

// Reexport some modules because there some types of them in public API.
pub use fyrox_core::algebra;
pub use fyrox_core::futures;
pub use fyrox_core::math;
pub use fyrox_core::pool;
pub use hrtf;

mod decoder;
