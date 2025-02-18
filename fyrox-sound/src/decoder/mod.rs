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

use fyrox_core::err;
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
            self.samples = Self::samples(&mut self.reader, &mut self.decoder).ok()?;
            self.samples.next()
        }
    }
}

impl Decoder {
    pub fn new(mut source: DataSource) -> Result<Self, SoundError> {
        let initial_stream_position = source.stream_position()?;

        let codec_registry = default::get_codecs();
        let probe = default::get_probe();
        let media_source_stream =
            MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());

        // TODO: add better hint if, e.g., we know the file extension
        let res = probe.format(
            &Hint::new(),
            media_source_stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let mut reader = res.format;
        let tracks = reader.tracks();
        let first_track = tracks.first().ok_or(SoundError::InvalidHeader)?;
        let codec_params = &first_track.codec_params;
        let mut decoder = codec_registry.make(codec_params, &DecoderOptions::default())?;

        // Get duration
        let mut last_packet = None;
        while let Ok(packet) = reader.next_packet() {
            last_packet = Some(packet);
        }
        reader.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: Time::new(initial_stream_position, 0.0),
                track_id: None,
            },
        )?;

        let channel_duration_in_samples = last_packet.map(|p| p.ts as usize).unwrap_or_default();

        let samples = Self::samples(&mut reader, &mut decoder)?;

        let params = &reader
            .tracks()
            .first()
            .ok_or(SoundError::InvalidHeader)?
            .codec_params;

        Ok(Self {
            samples,
            channel_count: params.channels.unwrap_or_default().count(),
            sample_rate: params.sample_rate.ok_or(SoundError::InvalidHeader)? as usize,
            reader,
            decoder,
            channel_duration_in_samples,
        })
    }

    fn samples(
        reader: &mut Box<dyn FormatReader>,
        decoder: &mut Box<dyn SymphoniaDecoder>,
    ) -> Result<std::vec::IntoIter<f32>, SoundError> {
        let packet = reader.next_packet()?;
        let decoded = decoder.decode(&packet)?;
        let buffer: AudioBuffer<f32> = decoded.make_equivalent();
        let samples = buffer.chan(0);

        Ok(samples.to_vec().into_iter())
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
            err!("Failed to seek in track")
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
