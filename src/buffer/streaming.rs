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
//! use rg3d_sound::buffer::{SoundBuffer, DataSource};
//!
//! fn make_streaming_buffer() -> Arc<Mutex<SoundBuffer>> {
//!     let data_source = DataSource::from_file("some_long_sound.ogg").unwrap();
//!     SoundBuffer::new_streaming(data_source).unwrap()
//! }
//! ```
//!
//! # Notes
//!
//! Streaming buffer cannot be shared across multiple source. On attempt to create a source with a streaming
//! buffer that already in use you'll get error.

use crate::{
    buffer::{generic::GenericBuffer, DataSource},
    decoder::Decoder,
    error::SoundError,
};
use rg3d_core::visitor::{Visit, VisitResult, Visitor};
use std::ops::{Deref, DerefMut};
use std::time::Duration;

/// Streaming buffer for long sounds. Does not support random access.
#[derive(Debug)]
pub struct StreamingBuffer {
    pub(in crate) generic: GenericBuffer,
    /// Count of sources that share this buffer, it is important to keep only one
    /// user of streaming buffer, because streaming buffer does not allow random
    /// access.
    pub(in crate) use_count: usize,
    decoder: Decoder,
}

impl Default for StreamingBuffer {
    fn default() -> Self {
        Self {
            generic: Default::default(),
            decoder: Decoder::Null,
            use_count: 0,
        }
    }
}

#[inline]
fn read_samples(buffer: &mut Vec<f32>, decoder: &mut Decoder, count: usize) -> usize {
    buffer.clear();
    for _ in 0..count {
        if let Some(sample) = decoder.next() {
            buffer.push(sample)
        } else {
            break;
        }
    }
    buffer.len()
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
        if let DataSource::Raw { .. } = source {
            return Err(source);
        };

        let external_source_path = if let DataSource::File { path, .. } = &source {
            Some(path.clone())
        } else {
            None
        };

        let mut decoder = Decoder::new(source)?;

        let mut samples = Vec::new();
        let channel_count = decoder.get_channel_count();
        read_samples(
            &mut samples,
            &mut decoder,
            Self::STREAM_SAMPLE_COUNT * channel_count,
        );
        debug_assert_eq!(samples.len() % channel_count, 0);

        Ok(Self {
            generic: GenericBuffer {
                samples,
                sample_rate: decoder.get_sample_rate(),
                channel_count: decoder.get_channel_count(),
                external_source_path,
            },
            use_count: 0,
            decoder,
        })
    }

    /// Returns total duration of data. Can be `None` if internal decoder does not supports seeking.
    pub fn duration(&self) -> Option<Duration> {
        self.decoder.duration()
    }

    #[inline]
    pub(in crate) fn read_next_block(&mut self) {
        read_samples(
            &mut self.generic.samples,
            &mut self.decoder,
            self.generic.channel_count * Self::STREAM_SAMPLE_COUNT,
        );
    }

    #[inline]
    pub(in crate) fn rewind(&mut self) -> Result<(), SoundError> {
        self.decoder.rewind()
    }

    #[inline]
    pub(in crate) fn time_seek(&mut self, location: Duration) {
        self.decoder.time_seek(location);
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
