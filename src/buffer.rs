use crate::{
    decoder::{
        WavDecoder,
        Decoder,
    },
    error::SoundError,
};
use std::{
    fs::File,
    path::Path,
    sync::{
        atomic::{
            Ordering,
            AtomicBool,
        }
    },
};
use std::io::BufReader;

pub struct Buffer {
    kind: BufferKind,
    samples: Vec<f32>,
    channel_count: usize,
    sample_per_channel: usize,
    total_sample_per_channel: usize,
    upload_next_block: AtomicBool,
    read_cursor: usize,
    write_cursor: usize,
    decoder: Option<Box<dyn Decoder>>,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum BufferKind {
    Normal,
    Stream,
}

impl Buffer {
    pub fn new(path: &Path, kind: BufferKind) -> Result<Self, SoundError> {
        let file = File::open(path)?;

        let ext = path.extension().ok_or(SoundError::UnsupportedFormat)?
            .to_str().ok_or(SoundError::UnsupportedFormat)?;

        let mut decoder = match ext {
            "wav" => {
                WavDecoder::new(Box::new(BufReader::new(file)))?
            }
            _ => return Err(SoundError::UnsupportedFormat)
        };

        let sample_per_channel = match kind {
            BufferKind::Normal => {
                decoder.get_sample_per_channel()
            }
            BufferKind::Stream => {
                2 * 44100
            }
        };

        let block_sample_count = sample_per_channel * decoder.get_channel_count();
        let buffer_sample_count = match kind {
            BufferKind::Normal => block_sample_count,
            BufferKind::Stream => 2 * block_sample_count,
        };

        let mut samples = vec![0.0; buffer_sample_count];

        decoder.read(&mut samples, sample_per_channel, 0, sample_per_channel)?;
        Ok(Self {
            total_sample_per_channel: decoder.get_sample_per_channel(),
            sample_per_channel,
            samples,
            channel_count: decoder.get_channel_count(),
            read_cursor: 0,
            write_cursor: block_sample_count,
            upload_next_block: AtomicBool::new(false),
            decoder: if kind == BufferKind::Stream {
                Some(Box::new(decoder))
            } else {
                None
            },
            kind,
        })
    }

    pub fn update(&mut self) {
        if self.upload_next_block.load(Ordering::SeqCst) {
            // TODO Streaming
        }
    }

    pub fn get_kind(&self) -> BufferKind {
        self.kind
    }

    pub fn get_sample_per_channel(&self) -> usize {
        self.sample_per_channel
    }

    pub fn get_total_sample_per_channel(&self) -> usize {
        self.total_sample_per_channel
    }

    pub fn get_samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn read(&self, offset: usize) -> f32 {
        self.samples[self.read_cursor + offset]
    }

    pub fn write(&mut self, offset: usize, sample: f32) {
        self.samples[self.write_cursor + offset] = sample;
    }

    pub fn get_channel_count(&self) -> usize {
        self.channel_count
    }

    pub(in crate) fn prepare_read_next_block(&mut self) {
        std::mem::swap(&mut self.read_cursor, &mut self.write_cursor);
        self.upload_next_block.store(true, Ordering::SeqCst);
    }
}