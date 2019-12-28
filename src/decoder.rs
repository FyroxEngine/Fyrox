use byteorder::{
    ReadBytesExt,
    LittleEndian,
};
use lewton::{
    inside_ogg::OggStreamReader,
    samples::InterleavedSamples
};
use std::{
    vec,
    io::{
        Read,
        Seek,
        SeekFrom,
    },
};
use crate::{
    error::SoundError,
    buffer::DataSource
};

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
///
/// TODO: Add support for RLE-compressed samples.
pub(in crate) struct WavDecoder {
    channel_count: usize,
    sample_rate: usize,
    byte_per_sample: usize,
    samples_left: usize,
    total_samples: usize,
    source: DataSource,
}

impl WavDecoder {
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

    fn rewind(&mut self) -> Result<(), SoundError> {
        // TODO: Ensure that this is true for all kinds of wav files.
        let wav_header_size = 44;
        self.source.seek(SeekFrom::Start(wav_header_size))?;
        self.samples_left = self.total_samples;
        Ok(())
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
                let sign = if ival & 0x800000 != 0 { -1.0 } else { 1.0 };
                Some(sign * ival as f32 / 8388608.0)
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

pub struct OggDecoder {
    reader: Option<OggStreamReader<DataSource>>,
    samples: vec::IntoIter<f32>,
    channel_count: usize,
    sample_rate: usize,
}

impl Iterator for OggDecoder {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.samples.next() {
            Some(sample)
        } else {
            if let Some(reader) = self.reader.as_mut() {
                if let Ok(samples) = reader.read_dec_packet_generic::<InterleavedSamples<f32>>() {
                    if let Some(samples) = samples {
                        self.samples = samples.samples.into_iter();
                    }
                }
            }
            self.samples.next()
        }
    }
}

fn is_vorbis_ogg(source: &mut DataSource) -> bool {
    let pos = source.seek(SeekFrom::Current(0)).unwrap();

    let is_vorbis = OggStreamReader::new(source.by_ref()).is_ok();

    source.seek(SeekFrom::Start(pos)).unwrap();

    is_vorbis
}

impl OggDecoder {
    pub fn new(mut source: DataSource) -> Result<Self, DataSource> {
        if is_vorbis_ogg(&mut source) {
            let mut reader = OggStreamReader::new(source).unwrap();

            let samples =
                if let Ok(samples) = reader.read_dec_packet_generic::<InterleavedSamples<f32>>() {
                    if let Some(samples) = samples {
                        samples.samples.into_iter()
                    } else {
                        Vec::new().into_iter()
                    }
                } else {
                    Vec::new().into_iter()
                };

            Ok(Self {
                samples,
                channel_count: reader.ident_hdr.audio_channels as usize,
                sample_rate: reader.ident_hdr.audio_sample_rate as usize,
                reader: Some(reader),
            })
        } else {
            Err(source)
        }
    }

    // https://github.com/RustAudio/lewton/issues/73
    fn rewind(&mut self) -> Result<(), SoundError> {
        let mut source = self.reader
            .take()
            .unwrap()
            .into_inner()
            .into_inner();
        source.seek(SeekFrom::Start(0))?;
        *self = match Self::new(source) {
            Ok(ogg_decoder) => ogg_decoder,
            // Drop source here, this will invalidate decoder and it can't produce any
            // samples anymore. This is unrecoverable error, but *should* never happen
            // in reality.
            Err(_) => return Err(SoundError::UnsupportedFormat),
        };
        Ok(())
    }
}

pub(in crate) enum Decoder {
    Wav(WavDecoder),
    Ogg(OggDecoder),
}

impl Iterator for Decoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Decoder::Wav(wav) => wav.next(),
            Decoder::Ogg(ogg) => ogg.next(),
        }
    }
}

impl Decoder {
    pub fn new(source: DataSource) -> Result<Self, DataSource> {
        // Try Wav
        let source = match WavDecoder::new(source) {
            Ok(wav_decoder) => return Ok(Decoder::Wav(wav_decoder)),
            Err(source) => source,
        };
        // Try Vorbis/Ogg
        let source = match OggDecoder::new(source) {
            Ok(ogg_decoder) => return Ok(Decoder::Ogg(ogg_decoder)),
            Err(source) => source,
        };
        Err(source)
    }

    pub fn rewind(&mut self) -> Result<(), SoundError> {
        match self {
            Decoder::Wav(wav) => wav.rewind(),
            Decoder::Ogg(ogg) => ogg.rewind(),
        }
    }

    pub fn get_channel_count(&self) -> usize {
        match self {
            Decoder::Wav(wav) => wav.channel_count,
            Decoder::Ogg(ogg) => ogg.channel_count
        }
    }

    pub fn get_sample_rate(&self) -> usize {
        match self {
            Decoder::Wav(wav) => wav.sample_rate,
            Decoder::Ogg(ogg) => ogg.sample_rate
        }
    }
}