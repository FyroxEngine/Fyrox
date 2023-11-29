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
use std::time::Duration;

/// Generic sound buffer that contains decoded samples and allows random access.
#[derive(Debug, Default, Visit, Reflect)]
pub struct GenericBuffer {
    /// Interleaved decoded samples (mono sounds: L..., stereo sounds: LR...)
    /// For streaming buffers it contains only small part of decoded data
    /// (usually something around 1 sec).
    #[visit(skip)]
    pub(crate) samples: Vec<f32>,
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
    pub fn new(source: DataSource) -> Result<Self, DataSource> {
        match source {
            DataSource::Raw {
                sample_rate,
                channel_count,
                samples,
            } => {
                if channel_count < 1 || channel_count > 2 || samples.len() % channel_count != 0 {
                    Err(DataSource::Raw {
                        sample_rate,
                        channel_count,
                        samples,
                    })
                } else {
                    Ok(Self {
                        channel_duration_in_samples: samples.len() / channel_count,
                        samples,
                        channel_count,
                        sample_rate,
                    })
                }
            }
            DataSource::RawStreaming(_) => Err(source),
            _ => {
                // Store cursor to handle errors.
                let (is_memory, external_cursor) = if let DataSource::Memory(cursor) = &source {
                    (true, cursor.clone())
                } else {
                    (false, Default::default())
                };

                let decoder = Decoder::new(source)?;
                if decoder.get_channel_count() < 1 || decoder.get_channel_count() > 2 {
                    if is_memory {
                        return Err(DataSource::Memory(external_cursor));
                    } else {
                        // There is not much we can do here: if the user supplied DataSource::File,
                        // they probably do not want us to re-read the file again in
                        // DataSource::from_file.
                        return Err(DataSource::Raw {
                            sample_rate: decoder.get_sample_rate(),
                            channel_count: decoder.get_channel_count(),
                            samples: vec![],
                        });
                    }
                }

                Ok(Self {
                    sample_rate: decoder.get_sample_rate(),
                    channel_count: decoder.get_channel_count(),
                    channel_duration_in_samples: decoder.channel_duration_in_samples(),
                    samples: decoder.into_samples(),
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
