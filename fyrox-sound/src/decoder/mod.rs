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
use std::time::Duration;

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
    samples: std::vec::IntoIter<f32>,
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

        let mut hint = Hint::new();

        // If pulling audio from a file, give the probe the extension
        if let DataSource::File { ref path, data: _ } = source {
            if let Some(osstr) = path.extension() {
                if let Some(ext) = osstr.to_str() {
                    hint.with_extension(ext);
                }
            }
        };

        let media_source_stream =
            MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());

        let res = probe.format(
            &hint,
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
        let mut buffer: AudioBuffer<f32> = decoded.make_equivalent();
        decoded.convert(&mut buffer);
        let samples = Self::interleaved(&mut buffer);

        Ok(samples.into_iter())
    }

    fn interleaved(buffer: &mut AudioBuffer<f32>) -> Vec<f32> {
        let channel_count = buffer.spec().channels.count();
        let frames = buffer.frames();

        let mut channels = Vec::with_capacity(channel_count);
        for i in 0..channel_count {
            channels.push(buffer.chan(i));
        }

        let mut res = Vec::with_capacity(channel_count * frames);
        for i in 0..frames {
            for channel in channels.iter() {
                res.push(channel[i]);
            }
        }

        res
    }

    pub fn rewind(&mut self) -> Result<(), SoundError> {
        self.time_seek(Duration::from_secs(0))
    }

    pub fn time_seek(&mut self, location: Duration) -> Result<(), SoundError> {
        self.reader.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: location.into(),
                track_id: None,
            },
        )?;

        Ok(())
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
