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
    sync::atomic::{
        Ordering,
        AtomicBool,
    },
    io::BufReader,
};

/// Sound samples buffer.
///
/// # Notes
///
/// Data in buffer is NON-INTERLEAVED, which for left (L) and right (R) channels means:
/// LLLLLLLLLLLLLL RRRRRRRRRRRRRR
/// So basically data split into chunks for each channel.
///
/// # Important notes about streaming
///
/// When buffer is streaming, data size doubles.
/// |LLLLLLLLLLLLLL RRRRRRRRRRRRRR|LLLLLLLLLLLLLL RRRRRRRRRRRRRR|
/// ^                             ^
/// `read_cursor`                 `write_cursor`
///
/// So while you read data from buffer by `read_cursor`, other part will be filled with new
/// portion of data. When `read_cursor` will reach `write_cursor`, they'll be swapped and
/// reading will be performed from new loaded data, while old data will be filled with
/// new portion of data, this process will continue until end of file and when eof is
/// reached, streaming will be started from beginning of a file.
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
    /// Buffer that contains all data the data.
    Normal,

    /// Buffer that will be filled by small portions of data only when it is needed.
    /// Ideal for large sounds (music, ambient, etc.), because unpacked PCM data
    /// takes very large amount of RAM.
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
                decoder.get_samples_per_channel()
            }
            BufferKind::Stream => {
                44100
            }
        };

        let block_sample_count = sample_per_channel * decoder.get_channel_count();
        let buffer_sample_count = match kind {
            BufferKind::Normal => block_sample_count,
            BufferKind::Stream => 2 * block_sample_count,
        };

        let mut samples = vec![0.0; buffer_sample_count];

        decoder.read(&mut samples, sample_per_channel, 0, sample_per_channel)?;
        if kind == BufferKind::Stream {
            // Fill second part of buffer in case if we'll stream data.
            decoder.read(&mut samples[block_sample_count..buffer_sample_count], sample_per_channel, 0,sample_per_channel)?;
        }

        Ok(Self {
            total_sample_per_channel: decoder.get_samples_per_channel(),
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

    pub fn update(&mut self) -> Result<(), SoundError> {
        if self.upload_next_block.load(Ordering::SeqCst) {
            if let Some(decoder) = &mut self.decoder {
                let data = &mut self.samples[self.write_cursor..(self.write_cursor + self.sample_per_channel * self.channel_count)];
                let read = decoder.read(data, self.sample_per_channel, 0, self.sample_per_channel)?;
                if read < self.sample_per_channel {
                    // Make sure to read rest of block from begin of source file.
                    decoder.rewind()?;
                    let second_read = decoder.read(data, self.sample_per_channel, read, self.sample_per_channel - read)?;
                    assert_eq!(second_read + read, self.sample_per_channel);
                }
            }
            self.upload_next_block.store(false, Ordering::SeqCst);
        }
        Ok(())
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