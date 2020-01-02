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
};
use crate::buffer::{
    streaming::StreamingBuffer,
    generic::GenericBuffer,
};
use std::sync::{Arc, Mutex};

pub mod generic;
pub mod streaming;

#[derive(Debug)]
pub enum DataSource {
    File {
        path: PathBuf,
        data: BufReader<File>,
    },
    Memory(Cursor<Vec<u8>>),
}

impl DataSource {
    pub fn from_file<P>(path: P) -> Result<Self, std::io::Error> where P: AsRef<Path> {
        Ok(DataSource::File {
            path: path.as_ref().to_path_buf(),
            data: BufReader::new(File::open(path)?),
        })
    }

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
    pub fn new_streaming(data_source: DataSource) -> Result<Arc<Mutex<Self>>, DataSource> {
        Ok(Arc::new(Mutex::new(SoundBuffer::Streaming(StreamingBuffer::new(data_source)?))))
    }

    pub fn new_generic(data_source: DataSource) -> Result<Arc<Mutex<Self>>, DataSource> {
        Ok(Arc::new(Mutex::new(SoundBuffer::Generic(GenericBuffer::new(data_source)?))))
    }

    pub fn raw_streaming(data_source: DataSource) -> Result<Self, DataSource> {
        Ok(SoundBuffer::Streaming(StreamingBuffer::new(data_source)?))
    }

    pub fn raw_generic(data_source: DataSource) -> Result<Self, DataSource> {
        Ok(SoundBuffer::Generic(GenericBuffer::new(data_source)?))
    }

    pub fn generic(&self) -> &GenericBuffer {
        match self {
            SoundBuffer::Generic(generic) => generic,
            SoundBuffer::Streaming(spatial) => spatial.generic(),
        }
    }

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

