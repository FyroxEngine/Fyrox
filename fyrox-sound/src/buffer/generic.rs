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
//! use fyrox_sound::buffer::{SoundBufferResource, DataSource};
//!
//! async fn make_buffer() -> SoundBufferResource {
//!     let data_source = DataSource::from_file("sound.wav").await.unwrap();
//!     SoundBufferResource::new_generic(data_source).unwrap()
//! }
//! ```

#![allow(clippy::manual_range_contains)]

use crate::{buffer::DataSource, decoder::Decoder};
use fyrox_core::visitor::{Visit, VisitResult, Visitor};
use std::path::Path;
use std::{path::PathBuf, time::Duration};

/// Generic sound buffer that contains decoded samples and allows random access.
#[derive(Debug, Default)]
pub struct GenericBuffer {
    /// Interleaved decoded samples (mono sounds: L..., stereo sounds: LR...)
    /// For streaming buffers it contains only small part of decoded data
    /// (usually something around 1 sec).
    pub(in crate) samples: Vec<f32>,
    pub(in crate) channel_count: usize,
    pub(in crate) sample_rate: usize,
    pub(in crate) external_source_path: PathBuf,
}

impl Visit for GenericBuffer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.external_source_path.visit("Path", visitor)?;

        visitor.leave_region()
    }
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
                        samples,
                        channel_count,
                        sample_rate,
                        external_source_path: Default::default(),
                    })
                }
            }
            DataSource::RawStreaming(_) => Err(source),
            _ => {
                let external_source_path = if let DataSource::File { path, .. } = &source {
                    path.clone()
                } else {
                    Default::default()
                };

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
                    samples: decoder.into_samples(),
                    external_source_path,
                })
            }
        }
    }

    /// In case if buffer was created from file, this method returns file name. Can be useful for
    /// serialization needs where you just need to know which file needs to be reloaded from disk
    /// when you loading a saved game.
    #[inline]
    pub fn external_data_path(&self) -> &Path {
        &self.external_source_path
    }

    /// Sets new path for external data source.
    pub fn set_external_data_path(&mut self, path: PathBuf) -> PathBuf {
        std::mem::replace(&mut self.external_source_path, path)
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

    /// Returns exact duration of the buffer.
    #[inline]
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(
            (self.samples.len() / (self.channel_count * self.sample_rate)) as f64,
        )
    }
}
