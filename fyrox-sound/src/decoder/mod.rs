use crate::{
    buffer::DataSource,
    decoder::{vorbis::OggDecoder, wav::WavDecoder},
    error::SoundError,
};
use std::time::Duration;

mod vorbis;
mod wav;

#[derive(Debug)]
pub(crate) enum Decoder {
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

    pub fn time_seek(&mut self, location: Duration) {
        match self {
            Decoder::Wav(wav) => wav.time_seek(location),
            Decoder::Ogg(ogg) => ogg.time_seek(location),
        }
    }

    pub fn get_channel_count(&self) -> usize {
        match self {
            Decoder::Wav(wav) => wav.channel_count(),
            Decoder::Ogg(ogg) => ogg.channel_count,
        }
    }

    pub fn get_sample_rate(&self) -> usize {
        match self {
            Decoder::Wav(wav) => wav.sample_rate(),
            Decoder::Ogg(ogg) => ogg.sample_rate,
        }
    }

    pub fn into_samples(self) -> Vec<f32> {
        self.collect()
    }

    pub fn channel_duration_in_samples(&self) -> usize {
        match self {
            Decoder::Wav(wav) => wav.channel_duration_in_samples(),
            Decoder::Ogg(ogg) => ogg.channel_duration_in_samples(),
        }
    }
}
