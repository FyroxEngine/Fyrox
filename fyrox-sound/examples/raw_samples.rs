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
