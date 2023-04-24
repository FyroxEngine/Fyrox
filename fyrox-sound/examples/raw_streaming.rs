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
