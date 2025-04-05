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

//! This module provides all needed types and methods to create/load sound buffers from different sources.
//!
//! # Overview
//!
//! Buffer is data source for sound sources in the engine. Each sound sound will fetch samples it needs
//! from a buffer, process them and send to output device. Buffer can be shared across multiple sources,
//! this is why each instance wrapped into `Arc<Mutex<>>`. Why not just load a buffer per source? This
//! is just inefficient memory-wise. Sound samples are very heavy: for example a mono sound that lasts
//! just 1 second will take ~172 Kb of memory (with 44100 Hz sampling rate and float sample representation).

use crate::{
    buffer::{generic::GenericBuffer, streaming::StreamingBuffer},
    error::SoundError,
};
use fyrox_core::{
    io::FileError, reflect::prelude::*, uuid::Uuid, visitor::prelude::*, TypeUuidProvider,
};
use fyrox_resource::untyped::ResourceKind;
use fyrox_resource::{
    io::{FileReader, ResourceIo},
    Resource, ResourceData, SOUND_BUFFER_RESOURCE_UUID,
};
use std::{
    error::Error,
    fmt::Debug,
    io::{Cursor, Read, Seek, SeekFrom},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::Duration,
};
use symphonia::core::io::MediaSource;

pub mod generic;
pub mod loader;
pub mod streaming;

/// Data source enumeration. Provides unified way of selecting data source for sound buffers. It can be either
/// a file or memory block.
#[derive(Debug)]
pub enum DataSource {
    /// Data source is a file of any supported format.
    File {
        /// Path to file.
        path: PathBuf,

        /// Reader for reading from the source
        data: Box<dyn FileReader>,
    },

    /// Data source is a memory block. Memory block must be in valid format (wav or vorbis/ogg). This variant can
    /// be used together with virtual file system.
    Memory(Cursor<Vec<u8>>),

    /// Raw samples in interleaved format with specified sample rate and channel count. Can be used for procedural
    /// sounds.
    ///
    /// # Notes
    ///
    /// Cannot be used with streaming buffers - it makes no sense to stream data that is already loaded into memory.
    Raw {
        /// Sample rate, typical values 22050, 44100, 48000, etc.
        sample_rate: usize,

        /// Total amount of channels.
        channel_count: usize,

        /// Raw samples in interleaved format. Count of samples must be multiple to channel count, otherwise you'll
        /// get error at attempt to use such buffer.
        samples: Vec<f32>,
    },

    /// Raw streaming source.
    RawStreaming(Box<dyn RawStreamingDataSource>),
}

/// A samples generator.
///
/// # Notes
///
/// Iterator implementation (the `next()` method) must produce samples in interleaved format, this
/// means that samples emitted by the method should be in `LRLRLR..` order, where `L` and `R` are
/// samples from left and right channels respectively. The sound engine supports both mono and
/// stereo sample sources.
pub trait RawStreamingDataSource: Iterator<Item = f32> + Send + Sync + Debug {
    /// Should return sample rate of the source.
    fn sample_rate(&self) -> usize;

    /// Should return total channel count.
    fn channel_count(&self) -> usize;

    /// Tells whether the provider should restart.
    ///
    /// Default implementation calls [`Self::time_seek`] with a zero duration
    fn rewind(&mut self) -> Result<(), SoundError> {
        self.time_seek(Duration::from_secs(0))
    }

    /// Allows you to start playback from given duration.
    fn time_seek(&mut self, _duration: Duration) -> Result<(), SoundError> {
        Ok(())
    }

    /// Returns total duration of the data.
    fn channel_duration_in_samples(&self) -> usize {
        0
    }
}

impl DataSource {
    /// Tries to create new `File` data source from given path. May fail if file does not exists.
    pub async fn from_file<P>(path: P, io: &dyn ResourceIo) -> Result<Self, FileError>
    where
        P: AsRef<Path>,
    {
        Ok(DataSource::File {
            path: path.as_ref().to_path_buf(),
            data: io.file_reader(path.as_ref()).await?,
        })
    }

    /// Creates new data source from given memory block. This function does not checks if this is valid source or
    /// not. Data source validity will be checked on first use.
    pub fn from_memory(data: Vec<u8>) -> Self {
        DataSource::Memory(Cursor::new(data))
    }

    /// Tries to get a path to external data source.
    pub fn path(&self) -> Option<&Path> {
        match self {
            DataSource::File { path, .. } => Some(path),
            _ => None,
        }
    }

    /// Tries to get a path to external data source.
    pub fn path_owned(&self) -> Option<PathBuf> {
        match self {
            DataSource::File { path, .. } => Some(path.clone()),
            _ => None,
        }
    }
}

impl Read for DataSource {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        match self {
            DataSource::File { data, .. } => data.read(buf),
            DataSource::Memory(b) => b.read(buf),
            DataSource::Raw { .. } => unreachable!("Raw data source does not supports Read trait!"),
            DataSource::RawStreaming { .. } => {
                unreachable!("Raw data source does not supports Read trait!")
            }
        }
    }
}

impl Seek for DataSource {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        match self {
            DataSource::File { data, .. } => data.seek(pos),
            DataSource::Memory(b) => b.seek(pos),
            DataSource::Raw { .. } => unreachable!("Raw data source does not supports Seek trait!"),
            DataSource::RawStreaming { .. } => {
                unreachable!("Raw data source does not supports Seek trait!")
            }
        }
    }
}

impl MediaSource for DataSource {
    fn is_seekable(&self) -> bool {
        match self {
            DataSource::File { .. } | DataSource::Memory(_) => true,
            DataSource::Raw { .. } | DataSource::RawStreaming(_) => false,
        }
    }

    fn byte_len(&self) -> Option<u64> {
        match self {
            DataSource::File { path: _, data } => data.byte_len(),
            DataSource::Memory(cursor) => MediaSource::byte_len(cursor),
            DataSource::Raw { .. } | DataSource::RawStreaming(_) => None,
        }
    }
}

/// An error that can occur during loading of sound buffer.
#[derive(Debug)]
pub enum SoundBufferResourceLoadError {
    /// A format is not supported.
    UnsupportedFormat,
    /// File load error.
    Io(FileError),
    /// Errors involving the data source
    ///
    /// This could be, e.g., wrong number of channels, or attempting to use a
    /// [`DataSource::RawStreaming`] where it doesn't make sense
    DataSourceError,
    /// Underlying sound error
    SoundError(SoundError),
}

impl std::fmt::Display for SoundBufferResourceLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SoundBufferResourceLoadError::UnsupportedFormat => {
                write!(f, "unsupported file format")
            }
            SoundBufferResourceLoadError::Io(e) => write!(f, "{e:?}"),
            SoundBufferResourceLoadError::DataSourceError => {
                write!(f, "error in underlying data source")
            }
            SoundBufferResourceLoadError::SoundError(e) => write!(f, "{e:?}"),
        }
    }
}

impl std::error::Error for SoundBufferResourceLoadError {}

/// Sound buffer is a data source for sound sources. See module documentation for more info.
#[derive(Debug, Visit, Reflect)]
pub enum SoundBuffer {
    /// General-purpose buffer, usually contains all the data and allows random
    /// access to samples. It is also used to make streaming buffer via composition.
    Generic(GenericBuffer),

    /// Buffer that will be filled by small portions of data only when it is needed.
    /// Ideal for large sounds (music, ambient, etc.), because unpacked PCM data
    /// takes very large amount of RAM. Allows random access only to currently loaded
    /// block, so in general there is no *true* random access.
    Streaming(StreamingBuffer),
}

impl From<SoundError> for SoundBufferResourceLoadError {
    fn from(err: SoundError) -> Self {
        Self::SoundError(err)
    }
}

/// Type alias for sound buffer resource.
pub type SoundBufferResource = Resource<SoundBuffer>;

/// Extension trait for sound buffer resource.
pub trait SoundBufferResourceExtension {
    /// Tries to create new streaming sound buffer from a given data source.
    fn new_streaming(
        data_source: DataSource,
    ) -> Result<Resource<SoundBuffer>, SoundBufferResourceLoadError>;

    /// Tries to create new generic sound buffer from a given data source.
    fn new_generic(
        data_source: DataSource,
    ) -> Result<Resource<SoundBuffer>, SoundBufferResourceLoadError>;
}

impl SoundBufferResourceExtension for SoundBufferResource {
    fn new_streaming(
        data_source: DataSource,
    ) -> Result<Resource<SoundBuffer>, SoundBufferResourceLoadError> {
        Ok(Resource::new_ok(
            Uuid::new_v4(),
            ResourceKind::External,
            SoundBuffer::Streaming(StreamingBuffer::new(data_source)?),
        ))
    }

    fn new_generic(
        data_source: DataSource,
    ) -> Result<Resource<SoundBuffer>, SoundBufferResourceLoadError> {
        Ok(Resource::new_ok(
            Uuid::new_v4(),
            ResourceKind::External,
            SoundBuffer::Generic(GenericBuffer::new(data_source)?),
        ))
    }
}

impl TypeUuidProvider for SoundBuffer {
    fn type_uuid() -> Uuid {
        SOUND_BUFFER_RESOURCE_UUID
    }
}

impl SoundBuffer {
    /// Tries to create new streaming sound buffer from a given data source. It returns raw sound
    /// buffer that has to be wrapped into Arc<Mutex<>> for use with sound sources.
    pub fn raw_streaming(data_source: DataSource) -> Result<Self, SoundBufferResourceLoadError> {
        Ok(Self::Streaming(StreamingBuffer::new(data_source)?))
    }

    /// Tries to create new generic sound buffer from a given data source. It returns raw sound
    /// buffer that has to be wrapped into Arc<Mutex<>> for use with sound sources.
    pub fn raw_generic(data_source: DataSource) -> Result<Self, SoundBufferResourceLoadError> {
        Ok(Self::Generic(GenericBuffer::new(data_source)?))
    }
}

impl Default for SoundBuffer {
    fn default() -> Self {
        SoundBuffer::Generic(Default::default())
    }
}

impl Deref for SoundBuffer {
    type Target = GenericBuffer;

    /// Returns shared reference to generic buffer for any enum variant. It is possible because
    /// streaming sound buffers are built on top of generic buffers.
    fn deref(&self) -> &Self::Target {
        match self {
            SoundBuffer::Generic(v) => v,
            SoundBuffer::Streaming(v) => v,
        }
    }
}

impl DerefMut for SoundBuffer {
    /// Returns mutable reference to generic buffer for any enum variant. It is possible because
    /// streaming sound buffers are built on top of generic buffers.
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            SoundBuffer::Generic(v) => v,
            SoundBuffer::Streaming(v) => v,
        }
    }
}

impl ResourceData for SoundBuffer {
    fn type_uuid(&self) -> Uuid {
        SOUND_BUFFER_RESOURCE_UUID
    }

    fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
        Err("Saving is not supported!".to_string().into())
    }

    fn can_be_saved(&self) -> bool {
        false
    }
}
