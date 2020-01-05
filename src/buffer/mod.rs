//! This module provides all needed types and methods to create/load sound buffers from different sources.
//!
//! # Overview
//!
//! Buffer is data source for sound sources in the engine. Each sound sound will fetch samples it needs
//! from a buffer, process them and send to output device. Buffer can be shared across multiple sources,
//! this is why each instance wrapped into `Arc<Mutex<>>`. Why not just load a buffer per source? This
//! is just inefficient memory-wise. Sound samples are very heavy: for example a mono sound that lasts
//! just 1 second will take ~172 Kb of memory (with 44100 Hz sampling rate and float sample representation).

use rg3d_core::visitor::{
    Visit,
    VisitResult,
    Visitor,
    VisitError,
};
use std::{
    io::{
        BufReader,
        Cursor,
        SeekFrom,
        Seek,
        Read,
    },
    path::{
        PathBuf,
        Path,
    },
    fs::File,
    sync::{
        Arc,
        Mutex
    }
};
use crate::buffer::{
    streaming::StreamingBuffer,
    generic::GenericBuffer,
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
        data: BufReader<File>,
    },

    /// Data source is a memory block. Memory block must be in valid format (wav or vorbis/ogg). This variant can
    /// be used together with virtual file system.
    Memory(Cursor<Vec<u8>>),
}

impl DataSource {
    /// Tries to create new `File` data source from given path. May fail if file does not exists.
    pub fn from_file<P>(path: P) -> Result<Self, std::io::Error> where P: AsRef<Path> {
        Ok(DataSource::File {
            path: path.as_ref().to_path_buf(),
            data: BufReader::new(File::open(path)?),
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
        }
    }
}

impl Seek for DataSource {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        match self {
            DataSource::File { data, .. } => data.seek(pos),
            DataSource::Memory(b) => b.seek(pos),
        }
    }
}

/// Sound buffer is a data source for sound sources. See module documentation for more info.
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

impl Default for SoundBuffer {
    fn default() -> Self {
        SoundBuffer::Generic(Default::default())
    }
}

impl SoundBuffer {
    /// Tries to create new streaming sound buffer from a given data source. Returns sound source
    /// wrapped into Arc<Mutex<>> that can be directly used with sound sources.
    pub fn new_streaming(data_source: DataSource) -> Result<Arc<Mutex<Self>>, DataSource> {
        Ok(Arc::new(Mutex::new(SoundBuffer::Streaming(StreamingBuffer::new(data_source)?))))
    }

    /// Tries to create new generic sound buffer from a given data source. Returns sound source
    /// wrapped into Arc<Mutex<>> that can be directly used with sound sources.
    pub fn new_generic(data_source: DataSource) -> Result<Arc<Mutex<Self>>, DataSource> {
        Ok(Arc::new(Mutex::new(SoundBuffer::Generic(GenericBuffer::new(data_source)?))))
    }

    /// Tries to create new streaming sound buffer from a given data source. It returns raw sound
    /// buffer that has to be wrapped into Arc<Mutex<>> for use with sound sources.
    pub fn raw_streaming(data_source: DataSource) -> Result<Self, DataSource> {
        Ok(SoundBuffer::Streaming(StreamingBuffer::new(data_source)?))
    }

    /// Tries to create new generic sound buffer from a given data source. It returns raw sound
    /// buffer that has to be wrapped into Arc<Mutex<>> for use with sound sources.
    pub fn raw_generic(data_source: DataSource) -> Result<Self, DataSource> {
        Ok(SoundBuffer::Generic(GenericBuffer::new(data_source)?))
    }

    /// Returns shared reference to generic buffer for any enum variant. It is possible because
    /// streaming sound buffers are built on top of generic buffers.
    pub fn generic(&self) -> &GenericBuffer {
        match self {
            SoundBuffer::Generic(generic) => generic,
            SoundBuffer::Streaming(spatial) => spatial.generic(),
        }
    }

    /// Returns mutable reference to generic buffer for any enum variant. It is possible because
    /// streaming sound buffers are built on top of generic buffers.
    pub fn generic_mut(&mut self) -> &mut GenericBuffer {
        match self {
            SoundBuffer::Generic(generic) => generic,
            SoundBuffer::Streaming(spatial) => spatial.generic_mut(),
        }
    }
}

impl Visit for SoundBuffer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind: u8 = match self {
            SoundBuffer::Generic(_) => 0,
            SoundBuffer::Streaming(_) => 1,
        };

        kind.visit("Id", visitor)?;

        if visitor.is_reading() {
            *self = match kind {
                0 => SoundBuffer::Generic(Default::default()),
                1 => SoundBuffer::Streaming(Default::default()),
                _ => return Err(VisitError::User("invalid buffer kind".to_string()))
            }
        }

        match self {
            SoundBuffer::Generic(generic) => generic.visit("Data", visitor)?,
            SoundBuffer::Streaming(streaming) => streaming.visit("Data", visitor)?,
        }

        visitor.leave_region()
    }
}

