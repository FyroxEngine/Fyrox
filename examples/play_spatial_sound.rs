use std::{
    time::{
        self,
        Duration,
    },
    thread,
};
use rg3d_sound::{
    source::{
        generic::GenericSourceBuilder,
        spatial::SpatialSourceBuilder,
        SoundSource,
        Status
    },
    context::Context,
    buffer::{
        DataSource,
    },
    buffer::SoundBuffer,
    math::{
        mat4::Mat4,
        vec3::Vec3,
        quat::Quat,
    },
    pool::Handle,
};

fn main() {
    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    // Load sound buffer. Buffer must be wrapped into Arc<Mutex<>> to be able to share buffer
    // between multiple sources.
    let drop_buffer = SoundBuffer::new_generic(DataSource::from_file("examples/data/drop.wav").unwrap()).unwrap();

    // Create spatial source - spatial sources can be positioned in space.
    let source = SpatialSourceBuilder::new(
        GenericSourceBuilder::new(drop_buffer)
            .with_looping(true)
            .with_status(Status::Playing)
            .build()
            .unwrap())
        .build_source();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    let source_handle: Handle<SoundSource> = context.lock().unwrap().add_source(source);

    // Move sound around listener for some time.
    let start_time = time::Instant::now();
    let mut angle = 0.0f32;
    while (time::Instant::now() - start_time).as_secs() < 11 {
        {
            let mut context = context.lock().unwrap();
            let sound = context.get_source_mut(source_handle);
            if let SoundSource::Spatial(spatial) = sound {
                let axis = Vec3::new(0.0, 1.0, 0.0);
                let rotation_matrix = Mat4::from_quat(Quat::from_axis_angle(axis, angle.to_radians()));
                spatial.set_position(&rotation_matrix.transform_vector(Vec3::new(0.0, 0.0, 3.0)));
            }
            angle += 3.6;
        }

        // Limit rate of updates.
        thread::sleep(Duration::from_millis(100));
    }
}