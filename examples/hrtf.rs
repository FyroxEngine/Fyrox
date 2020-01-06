use std::{
    time::{
        self,
        Duration,
    },
    thread,
};
use rg3d_sound::{
    math::{
        mat4::Mat4,
        vec3::Vec3,
        quat::Quat,
    },
    hrtf::{
        HrtfRenderer,
        HrtfSphere,
    },
    context::Context,
    buffer::{
        DataSource,
        SoundBuffer,
    },
    renderer::Renderer,
    source::{
        generic::GenericSourceBuilder,
        SoundSource,
        spatial::SpatialSourceBuilder,
        Status,
    },
};

fn main() {
    let hrtf = HrtfSphere::new("examples/data/IRC_1002_C.bin").unwrap();

    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    // Set HRTF renderer instead of default.
    context.lock()
        .unwrap()
        .set_renderer(Renderer::HrtfRenderer(HrtfRenderer::new(hrtf)));

    // Create some sounds.
    let sound_buffer = SoundBuffer::new_generic(DataSource::from_file("examples/data/door_open.wav").unwrap()).unwrap();
    let source = SpatialSourceBuilder::new(
        GenericSourceBuilder::new(sound_buffer)
            .with_status(Status::Playing)
            .build()
            .unwrap())
        .build_source();
    context.lock()
        .unwrap()
        .add_source(source);

    let sound_buffer = SoundBuffer::new_generic(DataSource::from_file("examples/data/helicopter.wav").unwrap()).unwrap();
    let source = SpatialSourceBuilder::new(
        GenericSourceBuilder::new(sound_buffer)
            .with_status(Status::Playing)
            .with_looping(true)
            .build()
            .unwrap())
        .build_source();
    let source_handle = context.lock()
        .unwrap()
        .add_source(source);

    // Move source sound around listener for some time.
    let start_time = time::Instant::now();
    let mut angle = 0.0f32;
    while (time::Instant::now() - start_time).as_secs() < 360 {
        // Separate scope for update to make sure that mutex lock will be released before
        // thread::sleep will be called so context can actually work in background thread.
        {
            let mut context = context.lock().unwrap();
            let sound = context.source_mut(source_handle);
            if let SoundSource::Spatial(spatial) = sound {
                let axis = Vec3::new(0.0, 1.0, 0.0);
                let rotation_matrix = Mat4::from_quat(Quat::from_axis_angle(axis, angle.to_radians()));
                spatial.set_position(&rotation_matrix.transform_vector(Vec3::new(0.0, 0.0, 3.0)));
            }

            angle += 1.6;

            println!("Sound render time {:?}", context.full_render_duration());
        }

        // Limit rate of updates.
        thread::sleep(Duration::from_millis(100));
    }
}