extern crate rg3d_sound;

use rg3d_sound::{
    context::Context,
    source::Source,
    buffer::{Buffer, BufferKind}
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration
};
use rg3d_core::pool::Handle;

fn main() {
    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    // Load sound buffer.
    let door_open_path = Path::new("examples/data/door_open.wav");
    let door_open_buffer = Buffer::new(door_open_path, BufferKind::Normal).unwrap();

    // Create flat source (without spatial effects) using that buffer.
    // Buffer must be wrapped into Arc<Mutex<>> to be able to share buffer
    // between multiple sources.
    let mut source = Source::new_flat(Arc::new(Mutex::new(door_open_buffer))).unwrap();

    // Play sound explicitly, by default sound created as stopped.
    source.play();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it.
    let _source_handle: Handle<Source> = context.lock().unwrap().add_source(source);

    // Wait until sound will play completely.
    thread::sleep(Duration::from_secs(3));
}