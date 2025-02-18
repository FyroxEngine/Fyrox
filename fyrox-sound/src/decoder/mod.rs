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

use std::io::Seek;
use std::{time::Duration, vec};

use symphonia::core::audio::{AudioBuffer, Signal};
use symphonia::core::codecs::{Decoder as SymphoniaDecoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;
use symphonia::default;

use crate::{buffer::DataSource, error::SoundError};

pub(crate) struct Decoder {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn SymphoniaDecoder>,
    samples: vec::IntoIter<f32>,
    pub channel_count: usize,
    pub sample_rate: usize,
    pub channel_duration_in_samples: usize,
}

impl std::fmt::Debug for Decoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Decoder")
            .field("channel_count", &self.channel_count)
            .field("sample_rate", &self.sample_rate)
            .field(
                "channel_duration_in_samples",
                &self.channel_duration_in_samples,
            )
            .finish()
    }
}

impl Iterator for Decoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.samples.next() {
            Some(sample)
        } else {
            if let Ok(packet) = self.reader.next_packet() {
                if let Ok(decoded) = self.decoder.decode(&packet) {
                    let buffer: AudioBuffer<Self::Item> = decoded.make_equivalent();
                    let samples = buffer.chan(0);

                    let vec: Vec<f32> = samples.to_vec();
                    self.samples = vec.into_iter();
                }
            }
            self.samples.next()
        }
    }
}

impl Decoder {
    pub fn new(mut source: DataSource) -> Result<Self, DataSource> {
        // TODO: add custom error type and replace unwraps with `?`

        let initial_stream_position = source.stream_position().unwrap();

        let codec_registry = default::get_codecs();
        let probe = default::get_probe();
        let media_source_stream =
            MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());

        // TODO: add better hint if, e.g., we know the file extension
        let res = probe
            .format(
                &Hint::new(),
                media_source_stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .unwrap();

        let mut reader = res.format;
        let tracks = reader.tracks();
        let first_track = tracks.first().unwrap();
        let codec_params = &first_track.codec_params;
        let mut decoder = codec_registry
            .make(codec_params, &DecoderOptions::default())
            .unwrap();

        // Get duration
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

        let channel_duration_in_samples = last_packet
            .map(|p| p.ts.try_into().unwrap())
            .unwrap_or_default();

        // Get samples
        let mut vec: Vec<f32> = Vec::new();
        if let Ok(packet) = reader.next_packet() {
            if let Ok(decoded) = decoder.decode(&packet) {
                let buffer: AudioBuffer<f32> = decoded.make_equivalent();
                let samples = buffer.chan(0);

                vec = samples.to_vec();
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
    }

    pub fn rewind(&mut self) -> Result<(), SoundError> {
        if self
            .reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::default(),
                    track_id: None,
                },
            )
            .is_ok()
        {
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

    pub fn get_channel_count(&self) -> usize {
        self.channel_count
    }

    pub fn get_sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub fn into_samples(self) -> Vec<f32> {
        self.collect()
    }

    pub fn channel_duration_in_samples(&self) -> usize {
        self.channel_duration_in_samples
    }
}
