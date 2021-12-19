use rg3d_sound::engine::SoundEngine;
use rg3d_sound::futures::executor::block_on;
use rg3d_sound::{
    buffer::{DataSource, SoundBufferResource},
    context::SoundContext,
    pool::Handle,
    source::{generic::GenericSourceBuilder, SoundSource, Status},
};
use std::{thread, time::Duration};

fn main() {
    // Initialize sound engine with default output device.
    let engine = SoundEngine::new();

    // Initialize new sound context.
    let context = SoundContext::new();

    engine.lock().unwrap().add_context(context.clone());

    // Load sound buffer.
    let waterfall_buffer = SoundBufferResource::new_streaming(
        block_on(DataSource::from_file("examples/data/waterfall.ogg")).unwrap(),
    )
    .unwrap();

    // Create flat source (without spatial effects) using that buffer.
    let source = GenericSourceBuilder::new()
        .with_buffer(waterfall_buffer)
        .with_status(Status::Playing)
        .with_looping(true)
        .build_source()
        .unwrap();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    let _source_handle: Handle<SoundSource> = context.state().add_source(source);

    thread::sleep(Duration::from_secs(30))
}
