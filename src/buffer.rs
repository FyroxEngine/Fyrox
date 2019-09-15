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
    io::BufReader,
};
use rg3d_core::visitor::{Visit, VisitResult, Visitor, VisitError};

/// Sound samples buffer.
///
/// # Notes
///
/// Data in buffer is NON-INTERLEAVED, which for left (L) and right (R) channels means:
/// `LLLLLLLLLLLLLL RRRRRRRRRRRRRR`. So basically data split into chunks for each channel.
/// Why is that so? To simplify reading data from buffer at various playback speed. Since engine
/// performs "resampling" on the fly it is very important that we'll data from correct positions
/// in buffer. Lets see at example of *interleaved* buffer: `LRLRLRLRLRLRLRLR` and assume that
/// we reading data from it with any speed (0.82, 1.15, 1.23, etc. to imitate Doppler shift for
/// example), in case of interleaved buffer it is hard to tell from which position we should
/// read, because read cursor can contain fractional part (1.53, 42.1, etc) but to fetch data
/// we have to round read cursor to nearest integer. Let see at example: assume that we reading
/// from 1.53 position, it rounds to 1 (by just dropping fractional part) and we'll read from 1
/// `LRLRLRLRLRLRLRLR`
///   ^ here
/// but this is the sample for *right* channel, but we have to read left first and only then
/// right, to fix that we could just use modulo to put read cursor into correct position, like
/// this: `read_pos = computed_read_pos % channel_count`, but modulo is expensive operation to
/// perform very frequently in time-critical code like mixing sounds.
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
    upload_next_block: bool,
    read_cursor: usize,
    write_cursor: usize,
    sample_rate: usize,
    /// Count of sources that share this buffer, it is important to keep only one
    /// user of streaming buffer, because streaming buffer does not allow random
    /// access.
    pub(in crate) use_count: usize,
    decoder: Option<Box<dyn Decoder>>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            kind: BufferKind::Normal,
            samples: Vec::new(),
            channel_count: 0,
            sample_per_channel: 0,
            total_sample_per_channel: 0,
            upload_next_block: false,
            read_cursor: 0,
            write_cursor: 0,
            sample_rate: 0,
            use_count: 0,
            decoder: None
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

        self.kind.visit("Kind", visitor)?;

        visitor.leave_region()
    }
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
            BufferKind::Stream => 44100,
            _ => decoder.get_samples_per_channel()
        };

        let block_sample_count = sample_per_channel * decoder.get_channel_count();
        let buffer_sample_count = match kind {
            BufferKind::Stream => 2 * block_sample_count,
            _ => block_sample_count,
        };

        let mut samples = vec![0.0; buffer_sample_count];

        decoder.read(&mut samples, sample_per_channel, 0, sample_per_channel)?;
        if kind == BufferKind::Stream {
            // Fill second part of buffer in case if we'll stream data.
            decoder.read(&mut samples[block_sample_count..buffer_sample_count], sample_per_channel, 0, sample_per_channel)?;
        }

        Ok(Self {
            use_count: 0,
            total_sample_per_channel: decoder.get_samples_per_channel(),
            sample_rate: decoder.get_sample_rate(),
            sample_per_channel,
            samples,
            channel_count: decoder.get_channel_count(),
            read_cursor: 0,
            write_cursor: block_sample_count,
            upload_next_block: false,
            decoder: if kind == BufferKind::Stream {
                Some(Box::new(decoder))
            } else {
                None
            },
            kind,
        })
    }

    pub fn update(&mut self) -> Result<(), SoundError> {
        if self.upload_next_block {
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
            self.upload_next_block = false;
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

    pub fn get_sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub(in crate) fn prepare_read_next_block(&mut self) {
        std::mem::swap(&mut self.read_cursor, &mut self.write_cursor);
        self.upload_next_block = true;
    }

    pub(in crate) fn rewind(&mut self) -> Result<(), SoundError> {
        if self.kind == BufferKind::Stream {
            if let Some(decoder) = &mut self.decoder {
                decoder.rewind()?;
                // Reset read and write cursors and upload data into parts of buffer.
                self.read_cursor = 0;
                self.write_cursor = self.sample_per_channel * self.channel_count;
                decoder.read(&mut self.samples, self.sample_per_channel, 0, self.sample_per_channel)?;
                let write_buffer = &mut self.samples[self.write_cursor..(2 * self.write_cursor)];
                decoder.read(write_buffer, self.sample_per_channel, 0, self.sample_per_channel)?;
            }
        }
        Ok(())
    }
}