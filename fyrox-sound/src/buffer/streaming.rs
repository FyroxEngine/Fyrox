//! Streaming buffer.
//!
//! # Overview
//!
//! Streaming buffers are used for long sounds (usually longer than 15 seconds) to reduce memory usage.
//! Some sounds in games are very long - music, ambient sounds, voice, etc. and it is too inefficient
//! to load and decode them directly into memory all at once - it will just take enormous amount of memory
//! that could be used to something more useful.
//!
//! # Usage
//!
//! There are almost no difference with generic buffers:
//!
//! ```no_run
//! use std::sync::{Mutex, Arc};
//! use fyrox_sound::buffer::{SoundBufferResource, DataSource, SoundBufferResourceExtension};
//!
//! async fn make_streaming_buffer() -> SoundBufferResource {
//!     let data_source = DataSource::from_file("some_long_sound.ogg").await.unwrap();
//!     SoundBufferResource::new_streaming(data_source).unwrap()
//! }
//! ```
//!
//! # Notes
//!
//! Streaming buffer cannot be shared across multiple source. On attempt to create a source with a streaming
//! buffer that already in use you'll get error.

use crate::{
    buffer::{generic::GenericBuffer, DataSource, RawStreamingDataSource},
    decoder::Decoder,
    error::SoundError,
};
use fyrox_core::{reflect::prelude::*, visitor::prelude::*};
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

/// Streaming buffer for long sounds. Does not support random access.
#[derive(Debug, Default, Visit, Reflect)]
pub struct StreamingBuffer {
    pub(crate) generic: GenericBuffer,
    /// Count of sources that share this buffer, it is important to keep only one
    /// user of streaming buffer, because streaming buffer does not allow random
    /// access.
    #[visit(skip)]
    pub(crate) use_count: usize,
    #[visit(skip)]
    #[reflect(hidden)]
    streaming_source: StreamingSource,
}

#[derive(Debug)]
enum StreamingSource {
    Null,
    Decoder(Decoder),
    Raw(Box<dyn RawStreamingDataSource>),
}

impl Default for StreamingSource {
    fn default() -> Self {
        Self::Null
    }
}

impl StreamingSource {
    #[inline]
    fn new(data_source: DataSource) -> Result<Self, DataSource> {
        Ok(match data_source {
            DataSource::File { .. } | DataSource::Memory(_) => {
                Self::Decoder(Decoder::new(data_source)?)
            }
            DataSource::RawStreaming(raw) => Self::Raw(raw),
            // It makes no sense to stream raw data which is already loaded into memory.
            _ => return Err(data_source),
        })
    }

    #[inline]
    fn sample_rate(&self) -> usize {
        match self {
            Self::Decoder(decoder) => decoder.get_sample_rate(),
            Self::Raw(raw) => raw.sample_rate(),
            Self::Null => 0,
        }
    }

    #[inline]
    fn channel_count(&self) -> usize {
        match self {
            Self::Decoder(decoder) => decoder.get_channel_count(),
            Self::Raw(raw) => raw.channel_count(),
            Self::Null => 0,
        }
    }

    fn channel_duration_in_samples(&self) -> usize {
        match self {
            Self::Null => 0,
            Self::Decoder(decoder) => decoder.channel_duration_in_samples(),
            Self::Raw(raw) => raw.channel_duration_in_samples(),
        }
    }

    fn rewind(&mut self) -> Result<(), SoundError> {
        match self {
            Self::Null => Ok(()),
            Self::Decoder(decoder) => decoder.rewind(),
            Self::Raw(raw) => raw.rewind(),
        }
    }

    fn time_seek(&mut self, location: Duration) {
        match self {
            Self::Null => {}
            Self::Decoder(decoder) => decoder.time_seek(location),
            Self::Raw(raw) => raw.time_seek(location),
        }
    }

    #[inline]
    fn read_next_samples_block_into(&mut self, buffer: &mut Vec<f32>) -> usize {
        buffer.clear();
        let count = StreamingBuffer::STREAM_SAMPLE_COUNT * self.channel_count();
        match self {
            Self::Decoder(decoder) => {
                for _ in 0..count {
                    if let Some(sample) = decoder.next() {
                        buffer.push(sample)
                    } else {
                        break;
                    }
                }
            }
            Self::Raw(raw_streaming) => {
                for _ in 0..count {
                    if let Some(sample) = raw_streaming.next() {
                        buffer.push(sample)
                    } else {
                        break;
                    }
                }
            }
            Self::Null => (),
        }

        buffer.len()
    }
}

impl StreamingBuffer {
    /// Defines amount of samples `per channel` which each streaming buffer will use for internal buffer.
    pub const STREAM_SAMPLE_COUNT: usize = 44100;

    /// Creates new streaming buffer using given data source. May fail if data source has unsupported format
    /// or it has corrupted data. Length of internal generic buffer cannot be changed but can be fetched from
    /// `StreamingBuffer::STREAM_SAMPLE_COUNT`
    ///
    /// # Notes
    ///
    /// This function will return Err if data source is `Raw`. It makes no sense to stream raw data which
    /// is already loaded into memory. Use Generic source instead!
    pub fn new(source: DataSource) -> Result<Self, DataSource> {
        let external_source_path = if let DataSource::File { path, .. } = &source {
            path.clone()
        } else {
            Default::default()
        };

        let mut streaming_source = StreamingSource::new(source)?;

        let mut samples = vec![];
        let channel_count = streaming_source.channel_count();
        streaming_source.read_next_samples_block_into(&mut samples);
        debug_assert_eq!(samples.len() % channel_count, 0);

        Ok(Self {
            generic: GenericBuffer {
                samples,
                sample_rate: streaming_source.sample_rate(),
                channel_count: streaming_source.channel_count(),
                channel_duration_in_samples: streaming_source.channel_duration_in_samples(),
                external_source_path,
            },
            use_count: 0,
            streaming_source,
        })
    }

    #[inline]
    pub(crate) fn read_next_block(&mut self) {
        self.streaming_source
            .read_next_samples_block_into(&mut self.generic.samples);
    }

    #[inline]
    pub(crate) fn rewind(&mut self) -> Result<(), SoundError> {
        self.streaming_source.rewind()
    }

    #[inline]
    pub(crate) fn time_seek(&mut self, location: Duration) {
        self.streaming_source.time_seek(location);
    }
}

impl Deref for StreamingBuffer {
    type Target = GenericBuffer;

    /// Returns shared reference to internal generic buffer. Can be useful to get some info (sample rate,
    /// channel count).
    fn deref(&self) -> &Self::Target {
        &self.generic
    }
}

impl DerefMut for StreamingBuffer {
    /// Returns mutable reference to internal generic buffer. Can be used to modify it.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.generic
    }
}
