use rg3d_sound::{
    buffer::{DataSource, SoundBuffer},
    context::Context,
    pool::Handle,
    source::{generic::GenericSourceBuilder, SoundSource, Status},
};
use std::{thread, time::Duration};

fn main() {
    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    // Load sound buffer.
    let door_open_buffer =
        SoundBuffer::new_generic(DataSource::from_file("examples/data/door_open.wav").unwrap())
            .unwrap();

    // Create generic source (without spatial effects) using that buffer.
    let source = GenericSourceBuilder::new(door_open_buffer)
        .with_status(Status::Playing)
        .build_source()
        .unwrap();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    let _source_handle: Handle<SoundSource> = context.lock().unwrap().add_source(source);

    // Wait until sound will play completely.
    thread::sleep(Duration::from_secs(3));
}
