use fyrox_resource::io::FsResourceIo;
use fyrox_sound::buffer::SoundBufferResourceExtension;
use fyrox_sound::renderer::hrtf::{HrirSphereResource, HrirSphereResourceExt};
use fyrox_sound::{
    algebra::{Point3, UnitQuaternion, Vector3},
    buffer::{DataSource, SoundBufferResource},
    context::{self, SoundContext},
    effects::{reverb::Reverb, Effect},
    engine::SoundEngine,
    futures::executor::block_on,
    hrtf::HrirSphere,
    renderer::{hrtf::HrtfRenderer, Renderer},
    source::{SoundSourceBuilder, Status},
};
use std::path::PathBuf;
use std::{
    thread,
    time::{self, Duration},
};

fn main() {
    let hrir_path = PathBuf::from("examples/data/IRC_1002_C.bin");
    let hrir_sphere = HrirSphere::from_file(&hrir_path, context::SAMPLE_RATE).unwrap();

    // Initialize sound engine with default output device.
    let engine = SoundEngine::new().unwrap();

    // Initialize new sound context.
    let context = SoundContext::new();

    engine.state().add_context(context.clone());

    // Set HRTF renderer instead of default for binaural sound.
    context
        .state()
        .set_renderer(Renderer::HrtfRenderer(HrtfRenderer::new(
            HrirSphereResource::from_hrir_sphere(hrir_sphere, hrir_path.into()),
        )));

    {
        // Create reverb effect and set its decay time.
        let mut reverb = Reverb::new();
        reverb.set_decay_time(10.0);

        // Add the reverb to the primary bus.
        let mut state = context.state();
        state
            .bus_graph_mut()
            .primary_bus_mut()
            .add_effect(Effect::Reverb(reverb));
    }

    // Create some sounds.
    let sound_buffer = SoundBufferResource::new_generic(
        block_on(DataSource::from_file(
            "examples/data/door_open.wav", // Load from the default resource io (File system)
            &FsResourceIo,
        ))
        .unwrap(),
    )
    .unwrap();
    let source = SoundSourceBuilder::new()
        // Each sound must specify the bus to which it will output the samples. By default it is "Primary" bus.
        .with_bus("Primary")
        .with_buffer(sound_buffer)
        .with_status(Status::Playing)
        .build()
        .unwrap();
    context.state().add_source(source);

    let sound_buffer = SoundBufferResource::new_generic(
        block_on(DataSource::from_file(
            "examples/data/drop.wav",
            // Load from the default resource io (File system)
            &FsResourceIo,
        ))
        .unwrap(),
    )
    .unwrap();
    let source = SoundSourceBuilder::new()
        .with_buffer(sound_buffer)
        .with_status(Status::Playing)
        .with_looping(true)
        .build()
        .unwrap();
    let drop_sound_handle = context.state().add_source(source);

    // Move sound around listener for some time.
    let start_time = time::Instant::now();
    let mut angle = 0.0f32;
    while (time::Instant::now() - start_time).as_secs() < 360 {
        let axis = Vector3::y_axis();
        let rotation_matrix =
            UnitQuaternion::from_axis_angle(&axis, angle.to_radians()).to_homogeneous();
        context.state().source_mut(drop_sound_handle).set_position(
            rotation_matrix
                .transform_point(&Point3::new(0.0, 0.0, 1.0))
                .coords,
        );

        angle += 1.6;

        println!(
            "Sound render time {:?}",
            context.state().full_render_duration()
        );

        // Limit rate of context updates.
        thread::sleep(Duration::from_millis(100));
    }
}
