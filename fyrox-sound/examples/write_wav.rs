use rg3d_sound::engine::SoundEngine;
use rg3d_sound::{
    buffer::{DataSource, SoundBufferResource},
    context::SoundContext,
    pool::Handle,
    source::{generic::GenericSourceBuilder, SoundSource, Status},
};

fn main() {
    // Initialize sound engine without output device.
    let engine = SoundEngine::without_device();

    // Create new context.
    let context = SoundContext::new();

    // Register context in the engine.
    engine.lock().unwrap().add_context(context.clone());

    // Load sound buffer.
    let door_open_buffer = SoundBufferResource::new_generic(
        rg3d_sound::futures::executor::block_on(DataSource::from_file(
            "examples/data/door_open.wav",
        ))
        .unwrap(),
    )
    .unwrap();

    // Create generic source (without spatial effects) using that buffer.
    let source = GenericSourceBuilder::new()
        .with_buffer(door_open_buffer)
        .with_status(Status::Playing)
        .build_source()
        .unwrap();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    let _source_handle: Handle<SoundSource> = context.state().add_source(source);

    // Create output wav file. The sample rate is currently fixed.
    let wav_spec = hound::WavSpec {
        channels: 2,
        sample_rate: rg3d_sound::context::SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut wav_writer = hound::WavWriter::create("output.wav", wav_spec).unwrap();

    // Create an output buffer.
    let buf_len = SoundEngine::render_buffer_len();
    let mut buf = vec![(0.0f32, 0.0f32); buf_len];
    let mut samples_written = 0;

    // Wait until sound will play completely.
    while samples_written < 3 * rg3d_sound::context::SAMPLE_RATE {
        engine.lock().unwrap().render(&mut buf);
        for &(l, r) in buf.iter() {
            wav_writer.write_sample(l).unwrap();
            wav_writer.write_sample(r).unwrap();
        }
        samples_written += buf_len as u32;
    }

    wav_writer.finalize().unwrap();
}
