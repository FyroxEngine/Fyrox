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
use fyrox_sound::engine::SoundEngine;
use fyrox_sound::{
    buffer::{DataSource, SoundBufferResource},
    context::SoundContext,
    source::{SoundSourceBuilder, Status},
};
use std::{thread, time::Duration};

fn main() {
    // Initialize sound engine with default output device.
    let engine = SoundEngine::new().unwrap();

    // Initialize new sound context.
    let context = SoundContext::new();

    engine.state().add_context(context.clone());

    // Create sine wave.
    let sample_rate = 44100;
    let sine_wave = DataSource::Raw {
        sample_rate,
        channel_count: 1,
        samples: {
            let frequency = 440.0;
            let amplitude = 0.75;
            (0..44100)
                .map(|i| {
                    amplitude
                        * ((2.0 * std::f32::consts::PI * i as f32 * frequency) / sample_rate as f32)
                            .sin()
                })
                .collect()
        },
    };

    let sine_wave_buffer = SoundBufferResource::new_generic(sine_wave).unwrap();

    // Create generic source (without spatial effects) using that buffer.
    let source = SoundSourceBuilder::new()
        .with_buffer(sine_wave_buffer)
        .with_status(Status::Playing)
        .with_looping(true)
        .build()
        .unwrap();

    context.state().add_source(source);

    // Play sound for some time.
    thread::sleep(Duration::from_secs(10));
}
