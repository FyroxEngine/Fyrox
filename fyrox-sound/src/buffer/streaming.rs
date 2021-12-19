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
//! use fyrox_sound::buffer::{SoundBufferResource, DataSource};
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

use crate::buffer::RawStreamingDataSource;
use crate::{
    buffer::{generic::GenericBuffer, DataSource},
    decoder::Decoder,
    error::SoundError,
};
use fyrox_core::visitor::{Visit, VisitResult, Visitor};
use std::ops::{Deref, DerefMut};
use std::time::Duration;

/// Streaming buffer for long sounds. Does not support random access.
#[derive(Debug, Default)]
pub struct StreamingBuffer {
    pub(in crate) generic: GenericBuffer,
    /// Count of sources that share this buffer, it is important to keep only one
    /// user of streaming buffer, because streaming buffer does not allow random
    /// access.
    pub(in crate) use_count: usize,
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
        match data_source {
            DataSource::File { .. } | DataSource::Memory(_) => {
                Ok(Self::Decoder(Decoder::new(data_source)?))
            }
            DataSource::RawStreaming(raw) => Ok(Self::Raw(raw)),
            // It makes no sense to stream raw data which is already loaded into memory.
            _ => Err(data_source),
        }
    }

    #[inline]
    fn sample_rate(&self) -> usize {
        match self {
            StreamingSource::Decoder(decoder) => decoder.get_sample_rate(),
            StreamingSource::Raw(raw) => raw.sample_rate(),
            StreamingSource::Null => 0,
        }
    }

    #[inline]
    fn channel_count(&self) -> usize {
        match self {
            StreamingSource::Decoder(decoder) => decoder.get_channel_count(),
            StreamingSource::Raw(raw) => raw.channel_count(),
            StreamingSource::Null => 0,
        }
    }

    fn duration(&self) -> Option<Duration> {
        match self {
            StreamingSource::Null => None,
            StreamingSource::Decoder(decoder) => decoder.duration(),
            StreamingSource::Raw(raw) => raw.duration(),
        }
    }

    fn rewind(&mut self) -> Result<(), SoundError> {
        match self {
            StreamingSource::Null => Ok(()),
            StreamingSource::Decoder(decoder) => decoder.rewind(),
            StreamingSource::Raw(raw) => raw.rewind(),
        }
    }

    fn time_seek(&mut self, location: Duration) {
        match self {
            StreamingSource::Null => {}
            StreamingSource::Decoder(decoder) => decoder.time_seek(location),
            StreamingSource::Raw(raw) => raw.time_seek(location),
        }
    }

    #[inline]
    fn read_next_samples_block_into(&mut self, buffer: &mut Vec<f32>) -> usize {
        buffer.clear();
        let count = StreamingBuffer::STREAM_SAMPLE_COUNT * self.channel_count();
        match self {
            StreamingSource::Decoder(decoder) => {
                for _ in 0..count {
                    if let Some(sample) = decoder.next() {
                        buffer.push(sample)
                    } else {
                        break;
                    }
                }
            }
            StreamingSource::Raw(raw_streaming) => {
                for _ in 0..count {
                    if let Some(sample) = raw_streaming.next() {
                        buffer.push(sample)
                    } else {
                        break;
                    }
                }
            }
            StreamingSource::Null => (),
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

        let mut samples = Vec::new();
        let channel_count = streaming_source.channel_count();
        streaming_source.read_next_samples_block_into(&mut samples);
        debug_assert_eq!(samples.len() % channel_count, 0);

        Ok(Self {
            generic: GenericBuffer {
                samples,
                sample_rate: streaming_source.sample_rate(),
                channel_count: streaming_source.channel_count(),
                external_source_path,
            },
            use_count: 0,
            streaming_source,
        })
    }

    /// Returns total duration of data. Can be `None` if internal decoder does not supports seeking.
    pub fn duration(&self) -> Option<Duration> {
        self.streaming_source.duration()
    }

    #[inline]
    pub(in crate) fn read_next_block(&mut self) {
        self.streaming_source
            .read_next_samples_block_into(&mut self.generic.samples);
    }

    #[inline]
    pub(in crate) fn rewind(&mut self) -> Result<(), SoundError> {
        self.streaming_source.rewind()
    }

    #[inline]
    pub(in crate) fn time_seek(&mut self, location: Duration) {
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

impl Visit for StreamingBuffer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.generic.visit("Generic", visitor)?;

        visitor.leave_region()
    }
}
