use std::{
    io::Read
};
use byteorder::{ReadBytesExt, LittleEndian};
use std::io::Seek;
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

pub struct WavDecoder {
    channel_count: usize,
    sample_rate: usize,
    sample_per_channel: usize,
    byte_per_sample: usize,
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
        Ok(Self {
            channel_count: header.num_channels as usize,
            sample_rate: header.sample_rate as usize,
            sample_per_channel: (header.data_chunk_size / u32::from(header.block_align)) as usize,
            byte_per_sample: (header.bits_per_sample / 8) as usize,
            source,
        })
    }
}

pub trait Decoder: Send + Sync {
    fn read(&mut self, data: &mut [f32], sample_per_channel: usize, offset: usize, count: usize) -> Result<usize, SoundError>;

    fn get_sample_per_channel(&self) -> usize;

    fn get_channel_count(&self) -> usize;
}

impl Decoder for WavDecoder {
    fn read(&mut self, data: &mut [f32], sample_per_channel: usize, offset: usize, count: usize) -> Result<usize, SoundError> {
        let mut i = 0;
        while i < count {
            for k in 0..self.channel_count {
                let channel_start = k * sample_per_channel;
                let position = channel_start + offset + i;
                if self.byte_per_sample == 1 {
                    data[position] = f32::from(self.source.read_i8()?) / 127.0;
                } else if self.byte_per_sample == 2 {
                    data[position] = f32::from(self.source.read_i16::<LittleEndian>()?) / 32767.0;
                } else {
                    return Err(SoundError::UnsupportedFormat);
                }
            }
            i += 1;
        }
        Ok(i)
    }

    fn get_sample_per_channel(&self) -> usize {
        self.sample_per_channel
    }

    fn get_channel_count(&self) -> usize {
        self.channel_count
    }
}