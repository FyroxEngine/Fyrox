use std::io::{
    Read,
    Seek,
    SeekFrom
};
use byteorder::{
    ReadBytesExt,
    LittleEndian
};
use crate::error::SoundError;

struct WavHeader {
    chunk_id: [u8; 4],
    chunk_size: u32,
    format: [u8; 4],
    fmt_chunk_id: [u8; 4],
    fmt_chunk_size: u32,
    audio_format: u16,
    num_channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    data_chunk_id: [u8; 4],
    data_chunk_size: u32,
}

pub trait Reader: Read + Seek + Send + Sync {}

impl<T> Reader for T where T: Read + Seek + Send + Sync {}

/// Wav decoder for uncompressed PCM data.
///
/// TODO: Add support for RLE-compressed samples.
pub(in crate) struct WavDecoder {
    channel_count: usize,
    sample_rate: usize,
    sample_per_channel: usize,
    byte_per_sample: usize,
    samples_left: usize,
    source: Box<dyn Reader>,
}

impl WavDecoder {
    fn read_header(source: &mut dyn Read) -> Result<WavHeader, SoundError> {
        let mut header = WavHeader {
            chunk_id: [0, 0, 0, 0],
            chunk_size: 0,
            format: [0, 0, 0, 0],
            fmt_chunk_id: [0, 0, 0, 0],
            fmt_chunk_size: 0,
            audio_format: 0,
            num_channels: 0,
            sample_rate: 0,
            byte_rate: 0,
            block_align: 0,
            bits_per_sample: 0,
            data_chunk_id: [0, 0, 0, 0],
            data_chunk_size: 0,
        };
        source.read_exact(&mut header.chunk_id)?;
        if header.chunk_id.as_ref() != b"RIFF" {
            return Err(SoundError::InvalidHeader);
        }
        header.chunk_size = source.read_u32::<LittleEndian>()?;
        source.read_exact(&mut header.format)?;
        if header.format.as_ref() != b"WAVE" {
            return Err(SoundError::InvalidHeader);
        }
        source.read_exact(&mut header.fmt_chunk_id)?;
        if header.fmt_chunk_id.as_ref() != b"fmt " {
            return Err(SoundError::InvalidHeader);
        }
        header.fmt_chunk_size = source.read_u32::<LittleEndian>()?;
        header.audio_format = source.read_u16::<LittleEndian>()?;
        if header.audio_format != 1 {
            return Err(SoundError::InvalidHeader);
        }
        header.num_channels = source.read_u16::<LittleEndian>()?;
        header.sample_rate = source.read_u32::<LittleEndian>()?;
        header.byte_rate = source.read_u32::<LittleEndian>()?;
        header.block_align = source.read_u16::<LittleEndian>()?;
        header.bits_per_sample = source.read_u16::<LittleEndian>()?;
        source.read_exact(&mut header.data_chunk_id)?;
        header.data_chunk_size = source.read_u32::<LittleEndian>()?;
        Ok(header)
    }

    pub fn new(mut source: Box<dyn Reader>) -> Result<Self, SoundError> {
        let header = Self::read_header(&mut source)?;
        let sample_per_channel = (header.data_chunk_size / u32::from(header.block_align)) as usize;
        Ok(Self {
            channel_count: header.num_channels as usize,
            sample_rate: header.sample_rate as usize,
            sample_per_channel,
            byte_per_sample: (header.bits_per_sample / 8) as usize,
            samples_left: sample_per_channel,
            source,
        })
    }
}

pub trait Decoder: Send + Sync {
    /// Read specified `count` of samples *per channel* in given `data` buffer in non-interleaved
    /// format (i.e. for 2 channel it would look like this: ```LLLLLLLLLLLLLL RRRRRRRRRRRRRR```).
    ///
    /// Returns real amount of samples per channel that were stored in `data`. Real amount
    /// of samples can be different of requested `count` in two cases:
    /// 1) There was not enough samples left in source
    /// 2) `data` buffer capacity is less than requested amount of samples.
    ///
    /// `sample_per_channel` is the real amount of samples per channel that if multiplied by
    /// channel count can exactly fit into `data`. Example: `LLLLLRRRRR` here `sample_per_channel`
    /// is 5. In case if you need to read all the data from source pass value returned from
    /// `get_samples_per_channel()` as `sample_per_channel` and provide a buffer with proper size.
    /// This value is useful for streaming sources, where you have small buffer and you filling
    /// it from time to time by portions of data.
    fn read(&mut self, data: &mut [f32], sample_per_channel: usize, offset: usize, count: usize) -> Result<usize, SoundError>;

    /// Returns amount of sampler per one channel in source.
    fn get_samples_per_channel(&self) -> usize;

    /// Returns total amount of channels in source.
    fn get_channel_count(&self) -> usize;

    /// Returns amount of unread samples *per channel* left in buffer.
    fn get_samples_left(&self) -> usize;

    /// Sets internal read cursor to beginning of the data.
    fn rewind(&mut self) -> Result<(), SoundError>;

    /// Returns sample rate in source. This value will be used to calculate appropriate playback
    /// speed for sound source when playing.
    fn get_sample_rate(&self) -> usize;
}

impl Decoder for WavDecoder {
    fn read(&mut self, data: &mut [f32], sample_per_channel: usize, offset: usize, count: usize) -> Result<usize, SoundError> {
        // We *probably* can read at least N samples which can fit into buffer.
        let max_out_sample_count_per_channel = data.len() / self.channel_count;
        let mut cap = if count < max_out_sample_count_per_channel {
            count
        } else {
            max_out_sample_count_per_channel
        };

        // We can't read more data than left in source.
        if cap > self.samples_left {
            cap = self.samples_left;
        }

        let mut samples_read = 0;
        while samples_read < cap {
            for k in 0..self.channel_count {
                let channel_start = k * sample_per_channel;
                let position = channel_start + offset + samples_read;
                if self.byte_per_sample == 1 {
                    data[position] = f32::from(self.source.read_i8()?) / 127.0;
                } else if self.byte_per_sample == 2 {
                    data[position] = f32::from(self.source.read_i16::<LittleEndian>()?) / 32767.0;
                } else {
                    return Err(SoundError::UnsupportedFormat);
                }
            }
            samples_read += 1;
            self.samples_left -= 1;
        }

        Ok(samples_read)
    }

    fn get_samples_per_channel(&self) -> usize {
        self.sample_per_channel
    }

    fn get_channel_count(&self) -> usize {
        self.channel_count
    }

    fn get_samples_left(&self) -> usize {
        self.sample_per_channel
    }

    fn rewind(&mut self) -> Result<(), SoundError> {
        // TODO: Ensure that this is true for all kinds of wav files.
        let wav_header_size = 44;
        self.source.seek(SeekFrom::Start(wav_header_size))?;
        self.samples_left = self.sample_per_channel;
        Ok(())
    }

    fn get_sample_rate(&self) -> usize {
        self.sample_rate
    }
}