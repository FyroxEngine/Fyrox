//! This module provides all needed types and methods to create/load sound buffers from different sources.
//!
//! # Overview
//!
//! Buffer is data source for sound sources in the engine. Each sound sound will fetch samples it needs
//! from a buffer, process them and send to output device. Buffer can be shared across multiple sources,
//! this is why each instance wrapped into `Arc<Mutex<>>`. Why not just load a buffer per source? This
//! is just inefficient memory-wise. Sound samples are very heavy: for example a mono sound that lasts
//! just 1 second will take ~172 Kb of memory (with 44100 Hz sampling rate and float sample representation).

use crate::buffer::{generic::GenericBuffer, streaming::StreamingBuffer};
use crate::error::SoundError;
use fyrox_core::{io::FileLoadError, reflect::Reflect, visitor::prelude::*};
use fyrox_resource::{define_new_resource, Resource, ResourceData, ResourceState};
use std::fmt::Debug;
use std::time::Duration;
use std::{
    borrow::Cow,
    io::{Cursor, Read, Seek, SeekFrom},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

pub mod generic;
pub mod streaming;

/// Data source enumeration. Provides unified way of selecting data source for sound buffers. It can be either
/// a file or memory block.
#[derive(Debug)]
pub enum DataSource {
    /// Data source is a file of any supported format.
    File {
        /// Path to file.
        path: PathBuf,

        /// Buffered file opened for read.
        #[cfg(not(target_arch = "wasm32"))]
        data: std::io::BufReader<std::fs::File>,

        /// TODO: In case of WASM load file entirely.
        #[cfg(target_arch = "wasm32")]
        data: Cursor<Vec<u8>>,
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
    fn rewind(&mut self) -> Result<(), SoundError> {
        Ok(())
    }

    /// Allows you to start playback from given duration.
    fn time_seek(&mut self, _duration: Duration) {}

    /// Returns total duration of data. Can be `None` if internal decoder does not supports seeking.
    fn duration(&self) -> Option<Duration> {
        None
    }
}

impl DataSource {
    /// Tries to create new `File` data source from given path. May fail if file does not exists.
    pub async fn from_file<P>(path: P) -> Result<Self, FileLoadError>
    where
        P: AsRef<Path>,
    {
        Ok(DataSource::File {
            path: path.as_ref().to_path_buf(),

            #[cfg(not(target_arch = "wasm32"))]
            data: std::io::BufReader::new(match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) => return Err(FileLoadError::Io(e)),
            }),

            #[cfg(target_arch = "wasm32")]
            data: Cursor::new(fyrox_core::io::load_file(path).await?),
        })
    }

    /// Creates new data source from given memory block. This function does not checks if this is valid source or
    /// not. Data source validity will be checked on first use.
    pub fn from_memory(data: Vec<u8>) -> Self {
        DataSource::Memory(Cursor::new(data))
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

/// An error that can occur during loading of sound buffer.
#[derive(Debug)]
pub enum SoundBufferResourceLoadError {
    /// A format is not supported.
    UnsupportedFormat,
    /// File load error.
    Io(FileLoadError),
}

define_new_resource!(
    /// A shared sound buffer resource.
    #[derive(Reflect)]
    #[reflect(hide_all)]
    SoundBufferResource<SoundBufferState, SoundBufferResourceLoadError>
);

impl SoundBufferResource {
    /// Tries to create new streaming sound buffer from a given data source. Returns sound source
    /// wrapped into Arc<Mutex<>> that can be directly used with sound sources.
    pub fn new_streaming(data_source: DataSource) -> Result<Self, DataSource> {
        Ok(Self(Resource::new(ResourceState::Ok(
            SoundBufferState::Streaming(StreamingBuffer::new(data_source)?),
        ))))
    }

    /// Tries to create new generic sound buffer from a given data source. Returns sound source
    /// wrapped into Arc<Mutex<>> that can be directly used with sound sources.
    pub fn new_generic(data_source: DataSource) -> Result<Self, DataSource> {
        Ok(Self(Resource::new(ResourceState::Ok(
            SoundBufferState::Generic(GenericBuffer::new(data_source)?),
        ))))
    }
}

/// Sound buffer is a data source for sound sources. See module documentation for more info.
#[derive(Debug, Visit)]
pub enum SoundBufferState {
    /// General-purpose buffer, usually contains all the data and allows random
    /// access to samples. It is also used to make streaming buffer via composition.
    Generic(GenericBuffer),

    /// Buffer that will be filled by small portions of data only when it is needed.
    /// Ideal for large sounds (music, ambient, etc.), because unpacked PCM data
    /// takes very large amount of RAM. Allows random access only to currently loaded
    /// block, so in general there is no *true* random access.
    Streaming(StreamingBuffer),
}

impl SoundBufferState {
    /// Tries to create new streaming sound buffer from a given data source. It returns raw sound
    /// buffer that has to be wrapped into Arc<Mutex<>> for use with sound sources.
    pub fn raw_streaming(data_source: DataSource) -> Result<Self, DataSource> {
        Ok(Self::Streaming(StreamingBuffer::new(data_source)?))
    }

    /// Tries to create new generic sound buffer from a given data source. It returns raw sound
    /// buffer that has to be wrapped into Arc<Mutex<>> for use with sound sources.
    pub fn raw_generic(data_source: DataSource) -> Result<Self, DataSource> {
        Ok(Self::Generic(GenericBuffer::new(data_source)?))
    }
}

impl Default for SoundBufferState {
    fn default() -> Self {
        SoundBufferState::Generic(Default::default())
    }
}

impl Deref for SoundBufferState {
    type Target = GenericBuffer;

    /// Returns shared reference to generic buffer for any enum variant. It is possible because
    /// streaming sound buffers are built on top of generic buffers.
    fn deref(&self) -> &Self::Target {
        match self {
            SoundBufferState::Generic(v) => v,
            SoundBufferState::Streaming(v) => v,
        }
    }
}

impl DerefMut for SoundBufferState {
    /// Returns mutable reference to generic buffer for any enum variant. It is possible because
    /// streaming sound buffers are built on top of generic buffers.
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            SoundBufferState::Generic(v) => v,
            SoundBufferState::Streaming(v) => v,
        }
    }
}

impl ResourceData for SoundBufferState {
    fn path(&self) -> Cow<Path> {
        Cow::from(&self.external_source_path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.external_source_path = path;
    }
}
