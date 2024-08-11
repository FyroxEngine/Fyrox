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

use fyrox_sound::buffer::SoundBufferResourceExtension;
use fyrox_sound::{
    buffer::{DataSource, RawStreamingDataSource, SoundBufferResource},
    context::SoundContext,
    engine::SoundEngine,
    source::{SoundSourceBuilder, Status},
};
use std::{thread, time::Duration};

#[derive(Debug)]
struct SamplesGenerator {
    sample_rate: usize,
    frequency: f32,
    amplitude: f32,
    index: usize,
}

impl SamplesGenerator {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100,
            frequency: 440.0,
            amplitude: 0.75,
            index: 0,
        }
    }
}

impl Iterator for SamplesGenerator {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.amplitude
            * ((2.0 * std::f32::consts::PI * self.index as f32 * self.frequency)
                / self.sample_rate as f32)
                .sin();

        self.index += 1;

        Some(sample)
    }
}

impl RawStreamingDataSource for SamplesGenerator {
    fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    fn channel_count(&self) -> usize {
        1
    }
}

fn main() {
    // Initialize sound engine with default output device.
    let engine = SoundEngine::new().unwrap();

    // Initialize new sound context.
    let context = SoundContext::new();

    engine.state().add_context(context.clone());

    // Create sine wave generator
    let sine_wave = DataSource::RawStreaming(Box::new(SamplesGenerator::new()));

    let sine_wave_buffer = SoundBufferResource::new_streaming(sine_wave).unwrap();

    // Create generic source (without spatial effects) using that buffer.
    let source = SoundSourceBuilder::new()
        .with_buffer(sine_wave_buffer)
        .with_status(Status::Playing)
        .build()
        .unwrap();

    context.state().add_source(source);

    // Play sound for some time.
    thread::sleep(Duration::from_secs(10));
}
