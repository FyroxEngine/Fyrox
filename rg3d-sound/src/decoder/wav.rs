use crate::rg3d_core::byteorder::{LittleEndian, ReadBytesExt};
use crate::{buffer::DataSource, error::SoundError};
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

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

/// Wav decoder for uncompressed PCM data.
/// TODO: Add support for RLE-compressed samples.
#[derive(Debug)]
pub(in crate) struct WavDecoder {
    pub channel_count: usize,
    pub sample_rate: usize,
    byte_per_sample: usize,
    samples_left: usize,
    total_samples: usize,
    source: DataSource,
}

impl WavDecoder {
    const HEADER_SIZE: u64 = 44;

    fn read_header(source: &mut DataSource) -> Result<WavHeader, SoundError> {
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

    pub fn new(mut source: DataSource) -> Result<Self, DataSource> {
        let pos = source.seek(SeekFrom::Current(0)).unwrap();
        let header = match Self::read_header(&mut source) {
            Ok(h) => h,
            Err(_) => {
                source.seek(SeekFrom::Start(pos)).unwrap();
                return Err(source);
            }
        };
        let byte_per_sample = (header.bits_per_sample / 8) as usize;
        let total_samples = (header.data_chunk_size / byte_per_sample as u32) as usize;
        Ok(Self {
            channel_count: header.num_channels as usize,
            sample_rate: header.sample_rate as usize,
            byte_per_sample,
            total_samples,
            samples_left: total_samples,
            source,
        })
    }

    pub fn rewind(&mut self) -> Result<(), SoundError> {
        self.source.seek(SeekFrom::Start(Self::HEADER_SIZE))?;
        self.samples_left = self.total_samples;
        Ok(())
    }

    pub fn time_seek(&mut self, location: Duration) {
        let byte_index = self.channel_count as f64
            * location.as_secs_f64()
            * self.sample_rate as f64
            * self.byte_per_sample as f64;
        let _ = self
            .source
            .seek(SeekFrom::Start(Self::HEADER_SIZE + byte_index as u64));
    }

    pub fn duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            (self.total_samples / (self.sample_rate * self.channel_count)) as f64,
        ))
    }
}

impl Iterator for WavDecoder {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_left > 0 {
            self.samples_left -= 1;
            if self.byte_per_sample == 1 {
                Some(f32::from(self.source.read_i8().ok()?) / 127.0)
            } else if self.byte_per_sample == 2 {
                Some(f32::from(self.source.read_i16::<LittleEndian>().ok()?) / 32767.0)
            } else if self.byte_per_sample == 3 {
                let a = self.source.read_u8().ok()? as i32;
                let b = self.source.read_u8().ok()? as i32;
                let c = self.source.read_u8().ok()? as i32;
                let ival = a | (b << 8) | (c << 16);
                let sign = if ival & 0x0080_0000 != 0 { -1.0 } else { 1.0 };
                Some(sign * ival as f32 / 8_388_608.0)
            } else if self.byte_per_sample == 4 {
                Some(self.source.read_f32::<LittleEndian>().ok()?)
            } else {
                None
            }
        } else {
            None
        }
    }
}
