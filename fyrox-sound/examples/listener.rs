use fyrox_core::algebra::Point3;
use fyrox_resource::io::FsResourceIo;
use fyrox_sound::buffer::SoundBufferResourceExtension;
use fyrox_sound::{
    algebra::{UnitQuaternion, Vector3},
    buffer::{DataSource, SoundBufferResource},
    context::SoundContext,
    engine::SoundEngine,
    futures::executor::block_on,
    source::{SoundSourceBuilder, Status},
};
use std::{
    thread,
    time::{self, Duration},
};

fn main() {
    // Initialize sound engine with default output device.
    let engine = SoundEngine::new().unwrap();

    // Initialize new sound context.
    let context = SoundContext::new();

    engine.state().add_context(context.clone());

    // Load sound buffer.
    let drop_buffer = SoundBufferResource::new_generic(
        block_on(DataSource::from_file(
            "examples/data/drop.wav", // Load from the default resource io (File system)
            &FsResourceIo,
        ))
        .unwrap(),
    )
    .unwrap();

    // Create spatial source - spatial sources can be positioned in space.
    let source = SoundSourceBuilder::new()
        .with_buffer(drop_buffer)
        .with_looping(true)
        .with_status(Status::Playing)
        .build()
        .unwrap();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    context.state().add_source(source);

    // Rotate listener for some time.
    let start_time = time::Instant::now();
    let mut angle = 0.0f32;
    while (time::Instant::now() - start_time).as_secs() < 20 {
        // Separate scope for update to make sure that mutex lock will be released before
        // thread::sleep will be called so context can actually work in background thread.
        {
            let mut context = context.state();

            let listener = context.listener_mut();

            // Define up-axis of listener.
            let up = Vector3::y_axis();

            // And rotate look axis.
            let rotation_matrix =
                UnitQuaternion::from_axis_angle(&up, angle.to_radians()).to_homogeneous();
            let look = rotation_matrix
                .transform_point(&Point3::new(0.0, 0.0, 1.0))
                .coords;

            // Finally combine axes. _lh suffix here means that we using left-handed coordinate system.
            // there is also _rh (right handed) version. Also basis can be set directly by using `set_basis`
            listener.set_orientation_lh(look, *up);

            // Move listener a bit back from sound source.
            listener.set_position(Vector3::new(0.0, 0.0, -2.0));

            // Continue rotation.
            angle += 2.0;
        }

        // Limit rate of updates.
        thread::sleep(Duration::from_millis(100));
    }
}
