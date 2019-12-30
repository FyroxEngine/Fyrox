extern crate rg3d_sound;

use rg3d_sound::{
    context::Context,
    source::Source,
    buffer::{
        Buffer,
        BufferKind,
        DataSource
    }
};
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
    time
};
use rg3d_core::pool::Handle;

fn main() {
    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    // Load sound buffer.
    let waterfall_buffer = Buffer::new(DataSource::from_file("examples/data/waterfall.ogg").unwrap(), BufferKind::Stream).unwrap();

    // Create flat source (without spatial effects) using that buffer.
    // Buffer must be wrapped into Arc<Mutex<>> to be able to share buffer
    // between multiple sources.
    let mut source = Source::new_flat(Arc::new(Mutex::new(waterfall_buffer))).unwrap();

    // Play sound explicitly, by default sound created as stopped.
    source.play();

    source.set_looping(true);

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it.
    let _source_handle: Handle<Source> = context.lock().unwrap().add_source(source);

    let start_time = time::Instant::now();
    while (time::Instant::now() - start_time).as_secs() < 600 {
        {
            // It is very important to call update context, on each update tick context
            // updates will check if it is the time to upload next piece of data into buffer
            // (perform streaming).
            context.lock().unwrap().update();
        }

        // Limit rate of context updates.
        thread::sleep(Duration::from_millis(100));
    }
}