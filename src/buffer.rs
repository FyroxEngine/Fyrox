use crate::{
    error::SoundError,
    decoder::Decoder
};
use rg3d_core::visitor::{
    Visit,
    VisitResult,
    Visitor,
    VisitError,
};
use std::{
    io::{BufReader, Cursor, SeekFrom, Seek, Read},
    path::{PathBuf, Path},
    fs::File
};

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

pub struct Buffer {
    kind: BufferKind,
    /// Interleaved decoded samples (mono sounds: L..., stereo sounds: LR...)
    /// For streaming buffers it contains only small part of decoded data
    /// (usually something around 1 sec).
    samples: Vec<f32>,
    channel_count: usize,
    sample_rate: usize,
    /// Count of sources that share this buffer, it is important to keep only one
    /// user of streaming buffer, because streaming buffer does not allow random
    /// access.
    pub(in crate) use_count: usize,
    decoder: Option<Decoder>,
    external_source_path: Option<PathBuf>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            kind: BufferKind::Normal,
            samples: Vec::new(),
            channel_count: 0,
            sample_rate: 0,
            use_count: 0,
            decoder: None,
            external_source_path: None,
        }
    }
}

impl Visit for BufferKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut kind: u8 = match self {
            BufferKind::Normal => 0,
            BufferKind::Stream => 1,
        };

        kind.visit(name, visitor)?;

        if visitor.is_reading() {
            *self = match kind {
                0 => BufferKind::Normal,
                1 => BufferKind::Stream,
                _ => return Err(VisitError::User("invalid buffer kind".to_string()))
            }
        }

        Ok(())
    }
}

impl Visit for Buffer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        // Only save path and kind (streaming or not), it will be enough to
        // correctly reload resource on load.
        self.external_source_path.visit("Path", visitor)?;
        self.kind.visit("Kind", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum BufferKind {
    /// Buffer that contains all the data.
    Normal,

    /// Buffer that will be filled by small portions of data only when it is needed.
    /// Ideal for large sounds (music, ambient, etc.), because unpacked PCM data
    /// takes very large amount of RAM.
    Stream,
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
    return buffer.len();
}

impl Buffer {
    pub const STREAM_SAMPLE_COUNT: usize = 44100;

    pub fn new(source: DataSource, kind: BufferKind) -> Result<Self, DataSource> {
        let external_source_path = if let DataSource::File { path, .. } = &source {
            Some(path.clone())
        } else {
            None
        };

        let mut decoder = Decoder::new(source)?;

        let samples = match kind {
            BufferKind::Normal => {
                let mut samples = Vec::new();
                while let Some(sample) = decoder.next() {
                    samples.push(sample);
                }
                samples
            }
            BufferKind::Stream => {
                let mut samples = Vec::new();
                let channel_count = decoder.get_channel_count();
                read_samples(&mut samples, &mut decoder, Self::STREAM_SAMPLE_COUNT * channel_count);
                debug_assert_eq!(samples.len() % channel_count, 0);
                samples
            }
        };

        Ok(Self {
            kind,
            samples,
            use_count: 0,
            sample_rate: decoder.get_sample_rate(),
            channel_count: decoder.get_channel_count(),
            external_source_path,
            decoder: if kind == BufferKind::Stream { Some(decoder) } else { None },
        })
    }

    pub fn get_external_data_path(&self) -> Option<PathBuf> {
        self.external_source_path.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn get_kind(&self) -> BufferKind {
        self.kind
    }

    pub fn get_samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn get_channel_count(&self) -> usize {
        self.channel_count
    }

    pub fn get_sample_rate(&self) -> usize {
        self.sample_rate
    }

    #[inline]
    pub(in crate) fn read_at(&self, offset: usize) -> f32 {
        self.samples[offset]
    }

    #[inline]
    pub(in crate) fn read_next_block(&mut self) {
        if let Some(decoder) = self.decoder.as_mut() {
            read_samples(&mut self.samples, decoder, self.channel_count * Self::STREAM_SAMPLE_COUNT);
        }
    }

    #[inline]
    pub(in crate) fn rewind(&mut self) -> Result<(), SoundError> {
        if let Some(decoder) = self.decoder.as_mut() {
            decoder.rewind()?;
        }
        Ok(())
    }
}