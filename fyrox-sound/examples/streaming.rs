use fyrox_resource::io::FsResourceIo;
use fyrox_sound::buffer::SoundBufferResourceExtension;
use fyrox_sound::{
    buffer::{DataSource, SoundBufferResource},
    context::SoundContext,
    engine::SoundEngine,
    futures::executor::block_on,
    pool::Handle,
    source::{SoundSource, SoundSourceBuilder, Status},
};
use std::{thread, time::Duration};

fn main() {
    // Initialize sound engine with default output device.
    let engine = SoundEngine::new().unwrap();

    // Initialize new sound context.
    let context = SoundContext::new();

    engine.state().add_context(context.clone());

    // Load sound buffer.
    let waterfall_buffer = SoundBufferResource::new_streaming(
        block_on(DataSource::from_file(
            "examples/data/waterfall.ogg",
            // Load from the default resource io (File system)
            &FsResourceIo,
        ))
        .unwrap(),
    )
    .unwrap();

    // Create flat source (without spatial effects) using that buffer.
    let source = SoundSourceBuilder::new()
        .with_buffer(waterfall_buffer)
        .with_status(Status::Playing)
        .with_looping(true)
        .build()
        .unwrap();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    let _source_handle: Handle<SoundSource> = context.state().add_source(source);

    thread::sleep(Duration::from_secs(30))
}
