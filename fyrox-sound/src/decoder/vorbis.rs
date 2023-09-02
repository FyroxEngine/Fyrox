use crate::{buffer::DataSource, error::SoundError};
use lewton::{inside_ogg::read_headers, inside_ogg::OggStreamReader, samples::InterleavedSamples};
use ogg::PacketReader;
use std::{
    fmt::{Debug, Formatter},
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
    pub channel_duration_in_samples: usize,
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
    let pos = source.stream_position().unwrap();

    let is_vorbis = OggStreamReader::new(source.by_ref()).is_ok();

    source.seek(SeekFrom::Start(pos)).unwrap();

    is_vorbis
}

// God bless `stb_vorbis` - https://github.com/nothings/stb/blob/master/stb_vorbis.c#L4946
// lewton::audio::get_decoded_sample_count is bugged and does not work correctly. So instead of using it,
// we use `stb_vorbis` approach - find last packet, take its position and return it. This function is still
// unideal, because we read all packets one-by-one, instead of just jumping to the last one.
fn total_duration_in_samples(source: &mut DataSource) -> usize {
    let initial_stream_position = source.stream_position().unwrap();

    let mut reader = PacketReader::new(source.by_ref());
    if read_headers(&mut reader).is_ok() {
        let mut last_packet = None;
        while let Ok(Some(packet)) = reader.read_packet() {
            last_packet = Some(packet);
        }

        source
            .seek(SeekFrom::Start(initial_stream_position))
            .unwrap();

        last_packet
            .map(|p| p.absgp_page() as usize)
            .unwrap_or_default()
    } else {
        0
    }
}

impl OggDecoder {
    pub fn new(mut source: DataSource) -> Result<Self, DataSource> {
        if is_vorbis_ogg(&mut source) {
            let channel_duration_in_samples = total_duration_in_samples(&mut source);

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
                channel_duration_in_samples,
            })
        } else {
            Err(source)
        }
    }

    pub fn rewind(&mut self) -> Result<(), SoundError> {
        // We have to create completely new instance of decoder because of bug in seek_absgp_pg
        // For more info see - https://github.com/RustAudio/lewton/issues/73
        let mut source = self.reader.take().unwrap().into_inner().into_inner();
        source.rewind()?;
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
        let sample_index = location.as_secs_f64() * self.sample_rate as f64;
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

    pub fn channel_duration_in_samples(&self) -> usize {
        self.channel_duration_in_samples
    }
}
