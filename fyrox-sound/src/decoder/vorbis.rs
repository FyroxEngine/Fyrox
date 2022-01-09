use crate::{buffer::DataSource, error::SoundError};
use lewton::{inside_ogg::OggStreamReader, samples::InterleavedSamples};
use std::fmt::{Debug, Formatter};
use std::{
    io::{Read, Seek, SeekFrom},
    time::Duration,
    vec,
};

pub struct OggDecoder {
    // Option here is because we need to bypass a bug in lewton by replacing
    // the whole OggStreamReader on rewind by extracting data source and
    // create new OggStreamReader from it. Its ugly.
    reader: Option<Box<OggStreamReader<DataSource>>>,
    samples: vec::IntoIter<f32>,
    pub channel_count: usize,
    pub sample_rate: usize,
}

impl Debug for OggDecoder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "OggDecoder")
    }
}

impl Iterator for OggDecoder {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.samples.next() {
            Some(sample)
        } else {
            if let Some(reader) = self.reader.as_mut() {
                if let Ok(Some(samples)) =
                    reader.read_dec_packet_generic::<InterleavedSamples<f32>>()
                {
                    self.samples = samples.samples.into_iter();
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

            let samples = if let Ok(Some(samples)) =
                reader.read_dec_packet_generic::<InterleavedSamples<f32>>()
            {
                samples.samples.into_iter()
            } else {
                Vec::new().into_iter()
            };

            Ok(Self {
                samples,
                channel_count: reader.ident_hdr.audio_channels as usize,
                sample_rate: reader.ident_hdr.audio_sample_rate as usize,
                reader: Some(Box::new(reader)),
            })
        } else {
            Err(source)
        }
    }

    pub fn rewind(&mut self) -> Result<(), SoundError> {
        // We have to create completely new instance of decoder because of bug in seek_absgp_pg
        // For more info see - https://github.com/RustAudio/lewton/issues/73
        let mut source = self.reader.take().unwrap().into_inner().into_inner();
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

    pub fn time_seek(&mut self, location: Duration) {
        // seek_absgp_pg seems to be bugged - it fails at seeking when all packets were read already.
        // For more info see - https://github.com/RustAudio/lewton/issues/73
        let sample_index =
            self.channel_count as f64 * location.as_secs_f64() * self.sample_rate as f64;
        if self
            .reader
            .as_mut()
            .unwrap()
            .seek_absgp_pg(sample_index as u64)
            .is_err()
        {
            println!("Failed to seek vorbis/ogg, see https://github.com/RustAudio/lewton/issues/73")
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        None
    }
}
