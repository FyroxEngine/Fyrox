use rg3d_sound::{
    context::Context,
    buffer::{
        DataSource,
        SoundBuffer,
    },
    pool::Handle,
    source::{
        generic::GenericSourceBuilder,
        SoundSource,
        Status,
    },
};
use std::{
    thread,
    time::Duration,
};

fn main() {
    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    // Load sound buffer. Buffer must be wrapped into Arc<Mutex<>> to be able to share buffer
    // between multiple sources.
    let waterfall_buffer = SoundBuffer::new_streaming(DataSource::from_file("examples/data/waterfall.ogg").unwrap()).unwrap();

    // Create flat source (without spatial effects) using that buffer.
    let source = GenericSourceBuilder::new(waterfall_buffer)
        .with_status(Status::Playing)
        .with_looping(true)
        .build_source()
        .unwrap();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    let _source_handle: Handle<SoundSource> = context.lock().unwrap().add_source(source);

    thread::sleep(Duration::from_secs(30))
}