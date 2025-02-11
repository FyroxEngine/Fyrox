// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{buffer::DataSource, error::SoundError};

use symphonia::core::audio::{AudioBuffer, Signal};
use symphonia::core::codecs::{CodecParameters, Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::units::Time;
use symphonia::default::codecs::VorbisDecoder;
use symphonia::default::formats::OggReader;

use std::{
    fmt::{Debug, Formatter},
    io::Seek,
    time::Duration,
    vec,
};

pub struct OggDecoder {
    reader: OggReader,
    decoder: VorbisDecoder,
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
            if let Ok(packet) = self.reader.next_packet() {
                if let Ok(decoded) = self.decoder.decode(&packet) {
                    let buffer: AudioBuffer<Self::Item> = decoded.make_equivalent();
                    let samples = buffer.chan(0);

                    let vec: Vec<f32> = samples.into_iter().cloned().collect();
                    self.samples = vec.into_iter();
                }
            }
            self.samples.next()
        }
    }
}

fn is_vorbis_ogg(mut source: DataSource) -> bool {
    let pos = source.stream_position().unwrap();

    let media_source =
        MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());

    let res = OggReader::try_new(media_source, &FormatOptions::default());

    if let Ok(mut reader) = res {
        reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::new(pos, 0.0),
                    track_id: None,
                },
            )
            .unwrap();

        true
    } else {
        false
    }
}

// God bless `stb_vorbis` - https://github.com/nothings/stb/blob/master/stb_vorbis.c#L4946
// lewton::audio::get_decoded_sample_count is bugged and does not work correctly. So instead of using it,
// we use `stb_vorbis` approach - find last packet, take its position and return it. This function is still
// unideal, because we read all packets one-by-one, instead of just jumping to the last one.
fn total_duration_in_samples(mut source: DataSource) -> usize {
    let initial_stream_position = source.stream_position().unwrap();

    let media_source =
        MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());

    if let Ok(mut reader) = OggReader::try_new(media_source, &FormatOptions::default()) {
        let mut last_packet = None;
        while let Ok(packet) = reader.next_packet() {
            last_packet = Some(packet);
        }

        reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::new(initial_stream_position, 0.0),
                    track_id: None,
                },
            )
            .unwrap();

        last_packet
            .map(|p| p.ts.try_into().unwrap())
            .unwrap_or_default()
    } else {
        0
    }
}

impl OggDecoder {
    // TODO: fix return type
    pub fn new(source: DataSource) -> Result<Self, ()> {
        if is_vorbis_ogg(source) {
            let channel_duration_in_samples = total_duration_in_samples(source);

            let media_source =
                MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());

            let mut reader = OggReader::try_new(media_source, &FormatOptions::default()).unwrap();
            let mut decoder =
                VorbisDecoder::try_new(&CodecParameters::default(), &DecoderOptions::default())
                    .unwrap();

            let mut vec: Vec<f32> = Vec::new();
            if let Ok(packet) = reader.next_packet() {
                if let Ok(decoded) = decoder.decode(&packet) {
                    let buffer: AudioBuffer<f32> = decoded.make_equivalent();
                    let samples = buffer.chan(0);

                    vec = samples.into_iter().cloned().collect();
                }
            }

            let samples = vec.into_iter();

            let params = &reader.tracks().first().unwrap().codec_params;

            Ok(Self {
                samples,
                channel_count: params.channels.unwrap_or_default().count(),
                sample_rate: params.sample_rate.unwrap() as usize,
                reader,
                decoder,
                channel_duration_in_samples,
            })
        } else {
            Err(())
        }
    }

    pub fn rewind(&mut self) -> Result<(), SoundError> {
        if let Ok(_) = self.reader.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: Time::default(),
                track_id: None,
            },
        ) {
            Ok(())
        } else {
            Err(SoundError::UnsupportedFormat)
        }
    }

    pub fn time_seek(&mut self, location: Duration) {
        if self
            .reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: location.into(),
                    track_id: None,
                },
            )
            .is_err()
        {
            println!("Failed to seek vorbis/ogg?")
        }
    }

    pub fn channel_duration_in_samples(&self) -> usize {
        self.channel_duration_in_samples
    }
}
