use rg3d_sound::{
    buffer::DataSource,
    buffer::SoundBuffer,
    context::Context,
    math::{mat4::Mat4, quat::Quat, vec3::Vec3},
    source::{generic::GenericSourceBuilder, spatial::SpatialSourceBuilder, Status},
};
use std::{
    thread,
    time::{self, Duration},
};

fn main() {
    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    // Load sound buffer.
    let drop_buffer =
        SoundBuffer::new_generic(DataSource::from_file("examples/data/drop.wav").unwrap()).unwrap();

    // Create spatial source - spatial sources can be positioned in space.
    let source = SpatialSourceBuilder::new(
        GenericSourceBuilder::new(drop_buffer)
            .with_looping(true)
            .with_status(Status::Playing)
            .build()
            .unwrap(),
    )
    .build_source();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    context.lock().unwrap().add_source(source);

    // Rotate listener for some time.
    let start_time = time::Instant::now();
    let mut angle = 0.0f32;
    while (time::Instant::now() - start_time).as_secs() < 20 {
        // Separate scope for update to make sure that mutex lock will be released before
        // thread::sleep will be called so context can actually work in background thread.
        {
            let mut context = context.lock().unwrap();

            let listener = context.listener_mut();

            // Define up-axis of listener.
            let up = Vec3::new(0.0, 1.0, 0.0);

            // And rotate look axis.
            let rotation_matrix = Mat4::from_quat(Quat::from_axis_angle(up, angle.to_radians()));
            let look = rotation_matrix.transform_vector(Vec3::new(0.0, 0.0, 1.0));

            // Finally combine axes. _lh suffix here means that we using left-handed coordinate system.
            // there is also _rh (right handed) version. Also basis can be set directly by using `set_basis`
            listener.set_orientation_lh(look, up);

            // Move listener a bit back from sound source.
            listener.set_position(Vec3::new(0.0, 0.0, -2.0));

            // Continue rotation.
            angle += 2.0;
        }

        // Limit rate of updates.
        thread::sleep(Duration::from_millis(100));
    }
}
