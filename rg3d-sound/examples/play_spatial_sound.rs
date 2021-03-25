use rg3d_sound::engine::SoundEngine;
use rg3d_sound::{
    algebra::{Point3, UnitQuaternion, Vector3},
    buffer::{DataSource, SoundBuffer},
    context::Context,
    pool::Handle,
    source::{generic::GenericSourceBuilder, spatial::SpatialSourceBuilder, SoundSource, Status},
};
use std::{
    thread,
    time::{self, Duration},
};

fn main() {
    // Initialize sound engine with default output device.
    let engine = SoundEngine::new();

    // Initialize new sound context.
    let context = Context::new();

    engine.lock().unwrap().add_context(context.clone());

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
    let source_handle: Handle<SoundSource> = context.state().add_source(source);

    // Move sound around listener for some time.
    let start_time = time::Instant::now();
    let mut angle = 0.0f32;
    while (time::Instant::now() - start_time).as_secs() < 11 {
        if let SoundSource::Spatial(spatial) = context.state().source_mut(source_handle) {
            let axis = Vector3::y_axis();
            let rotation_matrix =
                UnitQuaternion::from_axis_angle(&axis, angle.to_radians()).to_homogeneous();
            spatial.set_position(
                &rotation_matrix
                    .transform_point(&Point3::new(0.0, 0.0, 3.0))
                    .coords,
            );
        }
        angle += 3.6;

        // Limit rate of updates.
        thread::sleep(Duration::from_millis(100));
    }
}
