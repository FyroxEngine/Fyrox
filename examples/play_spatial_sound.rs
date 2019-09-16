extern crate rg3d_sound;
extern crate rg3d_core;

use rg3d_sound::{
    context::Context,
    source::Source,
    buffer::{Buffer, BufferKind},
    source::SourceKind
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time,
    thread,
    time::Duration
};
use rg3d_core::{
    math::mat4::Mat4,
    pool::Handle,
    math::vec3::Vec3,
    math::quat::Quat
};

fn main() {
    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    // Load sound buffer.
    let drop_path = Path::new("examples/data/drop.wav");
    let drop_buffer = Buffer::new(drop_path, BufferKind::Normal).unwrap();

    // Create spatial source - spatial sources can be positioned in space.
    // Buffer must be wrapped into Arc<Mutex<>> to be able to share buffer
    // between multiple sources.
    let mut source = Source::new_spatial(Arc::new(Mutex::new(drop_buffer))).unwrap();

    // Play sound explicitly, by default sound created as stopped.
    source.play();

    // Make sure that sound will play infinitely.
    source.set_looping(true);

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it.
    let source_handle: Handle<Source> = context.lock().unwrap().add_source(source);

    // Move sound around listener for some time.
    let start_time = time::Instant::now();
    let mut angle = 0.0f32;
    while (time::Instant::now() - start_time).as_secs() < 11 {
        if let Some(sound) = context.lock().unwrap().get_source_mut(source_handle) {
            if let SourceKind::Spatial(spatial) = sound.get_kind_mut() {
                let axis = Vec3::make(0.0, 1.0, 0.0);
                let rotation_matrix = Mat4::from_quat(Quat::from_axis_angle(axis, angle.to_radians()));

                let position = rotation_matrix.transform_vector(Vec3::make(0.0, 0.0, 3.0));
                spatial.set_position(&position);
            }
        }
        angle += 3.6;

        // It is very important to call update context, on each update tick context
        // updates sound sources so they will take new spatial properties. Also
        // it will perform streaming. Update should be performed at least 5-10
        // times per second, no need to call it more frequently because context
        // configured that it will send samples to output device with fixed rate
        // (usually 10 Hz), so more frequent changes won't make any effect but just
        // will consume precious CPU clocks.
        context.lock().unwrap().update();

        // Limit rate of context updates.
        thread::sleep(Duration::from_millis(100));
    }
}