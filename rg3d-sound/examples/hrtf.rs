use rg3d_core::algebra::Point3;
use rg3d_sound::engine::SoundEngine;
use rg3d_sound::{
    algebra::{UnitQuaternion, Vector3},
    buffer::{DataSource, SoundBuffer},
    context::{self, Context},
    hrtf::HrirSphere,
    renderer::{hrtf::HrtfRenderer, Renderer},
    source::{generic::GenericSourceBuilder, spatial::SpatialSourceBuilder, SoundSource, Status},
};
use std::{
    thread,
    time::{self, Duration},
};

fn main() {
    // Initialize sound engine with default output device.
    let engine = SoundEngine::new();

    let hrir_sphere =
        HrirSphere::from_file("examples/data/IRC_1002_C.bin", context::SAMPLE_RATE).unwrap();

    // Initialize new sound context with default output device.
    let context = Context::new();

    engine.lock().unwrap().add_context(context.clone());

    // Set HRTF renderer instead of default.
    context
        .state()
        .set_renderer(Renderer::HrtfRenderer(HrtfRenderer::new(hrir_sphere)));

    // Create some sounds.
    let sound_buffer =
        SoundBuffer::new_generic(DataSource::from_file("examples/data/door_open.wav").unwrap())
            .unwrap();
    let source = SpatialSourceBuilder::new(
        GenericSourceBuilder::new(sound_buffer)
            .with_status(Status::Playing)
            .build()
            .unwrap(),
    )
    .build_source();
    context.state().add_source(source);

    let sound_buffer =
        SoundBuffer::new_generic(DataSource::from_file("examples/data/helicopter.wav").unwrap())
            .unwrap();
    let source = SpatialSourceBuilder::new(
        GenericSourceBuilder::new(sound_buffer)
            .with_status(Status::Playing)
            .with_looping(true)
            .build()
            .unwrap(),
    )
    .build_source();
    let source_handle = context.state().add_source(source);

    // Move source sound around listener for some time.
    let start_time = time::Instant::now();
    let mut angle = 0.0f32;
    while (time::Instant::now() - start_time).as_secs() < 360 {
        // Separate scope for update to make sure that mutex lock will be released before
        // thread::sleep will be called so context can actually work in background thread.
        {
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

            angle += 1.6;

            println!(
                "Sound render time {:?}",
                context.state().full_render_duration()
            );
        }

        // Limit rate of updates.
        thread::sleep(Duration::from_millis(100));
    }
}
