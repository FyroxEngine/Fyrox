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

//! Generic buffer module.
//!
//! # Overview
//!
//! Generic buffer is just an array of raw samples in IEEE float format with some additional info (sample rate,
//! channel count, etc.). All samples stored in interleaved format, which means samples for each channel are
//! near - `LRLRLR...`.
//!
//! # Usage
//!
//! Generic source can be created like so:
//!
//! ```no_run
//! use std::sync::{Mutex, Arc};
//! use fyrox_sound::buffer::{SoundBufferResource, DataSource, SoundBufferResourceExtension};
//! use fyrox_resource::io::FsResourceIo;
//!
//! async fn make_buffer() -> SoundBufferResource {
//!     let data_source = DataSource::from_file("sound.wav", &FsResourceIo).await.unwrap();
//!     SoundBufferResource::new_generic(data_source).unwrap()
//! }
//! ```

#![allow(clippy::manual_range_contains)]

use crate::{buffer::DataSource, decoder::Decoder};
use fyrox_core::{reflect::prelude::*, visitor::prelude::*};
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use super::SoundBufferResourceLoadError;

/// Samples container.
#[derive(Debug, Default, Visit, Reflect)]
pub struct Samples(pub Vec<f32>);

impl Deref for Samples {
    type Target = Vec<f32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Samples {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Generic sound buffer that contains decoded samples and allows random access.
#[derive(Debug, Default, Visit, Reflect)]
pub struct GenericBuffer {
    /// Interleaved decoded samples (mono sounds: L..., stereo sounds: LR...)
    /// For streaming buffers it contains only small part of decoded data
    /// (usually something around 1 sec).
    #[visit(skip)]
    pub(crate) samples: Samples,
    #[visit(skip)]
    pub(crate) channel_count: usize,
    #[visit(skip)]
    pub(crate) sample_rate: usize,
    #[visit(skip)]
    pub(crate) channel_duration_in_samples: usize,
}

impl GenericBuffer {
    /// Creates new generic buffer from specified data source. May fail if data source has unsupported
    /// format, corrupted, etc.
    ///
    /// # Notes
    ///
    /// `DataSource::RawStreaming` is **not** supported with generic buffers, use streaming buffer
    /// instead!
    ///
    /// Data source with raw samples must have sample count multiple of channel count, otherwise this
    /// function will return `Err`.
    pub fn new(source: DataSource) -> Result<Self, SoundBufferResourceLoadError> {
        match source {
            DataSource::Raw {
                sample_rate,
                channel_count,
                samples,
            } => {
                if channel_count < 1 || channel_count > 2 || samples.len() % channel_count != 0 {
                    Err(SoundBufferResourceLoadError::DataSourceError)
                } else {
                    Ok(Self {
                        channel_duration_in_samples: samples.len() / channel_count,
                        samples: Samples(samples),
                        channel_count,
                        sample_rate,
                    })
                }
            }
            DataSource::RawStreaming(_) => Err(SoundBufferResourceLoadError::DataSourceError),
            _ => {
                let decoder = Decoder::new(source)?;
                if decoder.get_channel_count() < 1 || decoder.get_channel_count() > 2 {
                    return Err(SoundBufferResourceLoadError::DataSourceError);
                }

                Ok(Self {
                    sample_rate: decoder.get_sample_rate(),
                    channel_count: decoder.get_channel_count(),
                    channel_duration_in_samples: decoder.channel_duration_in_samples(),
                    samples: Samples(decoder.into_samples()),
                })
            }
        }
    }

    /// Checks if buffer is empty or not.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Returns shared reference to an array with samples.
    #[inline]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Returns mutable reference to an array with samples that could be modified.
    pub fn samples_mut(&mut self) -> &mut [f32] {
        &mut self.samples
    }

    /// Returns exact amount of channels in the buffer.
    #[inline]
    pub fn channel_count(&self) -> usize {
        self.channel_count
    }

    /// Returns sample rate of the buffer.
    #[inline]
    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    /// Returns exact time length of the buffer.
    #[inline]
    pub fn duration(&self) -> Duration {
        Duration::from_nanos(
            (self.channel_duration_in_samples as u64 * 1_000_000_000u64) / self.sample_rate as u64,
        )
    }

    /// Returns exact duration of each channel (in samples) of the buffer. The returned value represents the entire length
    /// of each channel in the buffer, even if it is streaming and its content is not yet fully decoded.
    #[inline]
    pub fn channel_duration_in_samples(&self) -> usize {
        self.channel_duration_in_samples
    }
}
