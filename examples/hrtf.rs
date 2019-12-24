extern crate rg3d_sound;
extern crate rg3d_core;

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{
        self,
        Duration,
    },
    thread,
};
use rg3d_core::{
    math::{
        mat4::Mat4,
        vec3::Vec3,
        quat::Quat,
    },
};
use rg3d_sound::{
    hrtf::{
        HrtfRenderer,
        HrtfSphere
    },
    context::Context,
    source::{
        Source,
        SourceKind,
    },
    buffer::{Buffer, BufferKind},
    renderer::Renderer
};

fn main() {
    let hrtf = HrtfSphere::new(Path::new("examples/data/hrir_base.bin")).unwrap();

    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    context.lock().unwrap().set_renderer(Renderer::HrtfRenderer(HrtfRenderer::new(hrtf)));

    let sound_path = Path::new("examples/data/door_open.wav");
    let sound_buffer = Buffer::new(sound_path, BufferKind::Normal).unwrap();
    let mut source = Source::new_spatial(Arc::new(Mutex::new(sound_buffer))).unwrap();
    source.play();
    //source.set_looping(true);
    context.lock().unwrap().add_source(source);

    let sound_path = Path::new("examples/data/helicopter.wav");
    let sound_buffer = Buffer::new(sound_path, BufferKind::Normal).unwrap();
    let mut source = Source::new_spatial(Arc::new(Mutex::new(sound_buffer))).unwrap();
    source.play();
    source.set_looping(true);
    let source_handle = context.lock().unwrap().add_source(source);

    // Move sound around listener for some time.
    let start_time = time::Instant::now();
    let mut angle = 0.0f32;
    while (time::Instant::now() - start_time).as_secs() < 360 {
        {
            let mut context = context.lock().unwrap();
            let sound = context.get_source_mut(source_handle);
            if let SourceKind::Spatial(spatial) = sound.get_kind_mut() {
                let axis = Vec3::new(0.0, 1.0, 0.0);
                let rotation_matrix = Mat4::from_quat(Quat::from_axis_angle(axis, angle.to_radians()));

                let position = rotation_matrix.transform_vector(Vec3::new(0.0, 0.0, 3.0));
                spatial.set_position(&position);
            }

            angle += 1.6;

            // It is very important to call update context, on each update tick context
            // updates sound sources so they will take new spatial properties. Also
            // it will perform streaming. Update should be performed at least 5-10
            // times per second, no need to call it more frequently because context
            // configured that it will send samples to output device with fixed rate
            // (usually 10 Hz), so more frequent changes won't make any effect but just
            // will consume precious CPU clocks.
            context.update().unwrap();
        }

        // Limit rate of context updates.
        thread::sleep(Duration::from_millis(100));
    }
}