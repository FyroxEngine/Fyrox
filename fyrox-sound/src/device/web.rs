use crate::{
    context::SAMPLE_RATE,
    device::{Device, MixContext, NativeSample},
};
use fyrox_core::{
    parking_lot::{Mutex, RwLock},
    wasm_bindgen::{self, prelude::*, JsCast},
    web_sys::{AudioContext, AudioContextOptions},
};
use std::{mem::size_of, sync::Arc};

pub struct WebAudioDevice {
    ctx: Arc<AudioContext>,
    onended: Vec<Arc<RwLock<Option<Closure<dyn FnMut()>>>>>,
    buffer_duration_secs: f32,
}

impl WebAudioDevice {
    pub fn new<F: FnMut(&mut [(f32, f32)]) + Send + 'static>(
        buffer_len_bytes: u32,
        callback: F,
    ) -> Self {
        let callback = Arc::new(Mutex::new(callback));

        let mut options = AudioContextOptions::new();
        options.sample_rate(SAMPLE_RATE as f32);
        let ctx = Arc::new(AudioContext::new_with_context_options(&options).unwrap());
        let samples_per_channel = buffer_len_bytes as usize / size_of::<NativeSample>();
        let buffer_duration_secs = samples_per_channel as f32 / (SAMPLE_RATE as f32);
        let mut onended: Vec<Arc<RwLock<Option<Closure<dyn FnMut()>>>>> = Vec::new();

        let time = Arc::new(RwLock::new(0.0f32));

        for _ in 0..2 {
            let buffer = ctx
                .create_buffer(2, samples_per_channel as u32, SAMPLE_RATE as f32)
                .unwrap();

            let onended_closure: Arc<RwLock<Option<Closure<dyn FnMut()>>>> =
                Arc::new(RwLock::new(None));

            let ctx_clone = ctx.clone();
            let onended_closure_clone = onended_closure.clone();
            let time = time.clone();
            let callback = callback.clone();
            let mut mix_buffer = vec![(0.0f32, 0.0f32); samples_per_channel];
            let mut temp_samples = vec![0.0f32; samples_per_channel];
            onended_closure
                .write()
                .replace(Closure::wrap(Box::new(move || {
                    for (l, r) in mix_buffer.iter_mut() {
                        *r = 0.0;
                        *l = 0.0;
                    }

                    let current_time = ctx_clone.current_time() as f32;
                    let raw_time = *time.read();
                    let start_time = if raw_time >= current_time {
                        raw_time
                    } else {
                        current_time
                    };

                    callback.lock()(&mut mix_buffer);

                    // Fill left channel.
                    for ((l, _), sample) in mix_buffer.iter().zip(temp_samples.iter_mut()) {
                        *sample = *l;
                    }
                    buffer.copy_to_channel(&temp_samples, 0).unwrap();

                    // Fill right channel.
                    for ((_, r), sample) in mix_buffer.iter().zip(temp_samples.iter_mut()) {
                        *sample = *r;
                    }
                    buffer.copy_to_channel(&temp_samples, 1).unwrap();

                    // Create source.
                    let source = ctx_clone.create_buffer_source().unwrap();
                    source.set_buffer(Some(&buffer));
                    source
                        .connect_with_audio_node(&ctx_clone.destination())
                        .unwrap();
                    source.set_onended(Some(
                        onended_closure_clone
                            .read()
                            .as_ref()
                            .unwrap()
                            .as_ref()
                            .unchecked_ref(),
                    ));
                    source.start_with_when(start_time as f64).unwrap();

                    *time.write() = start_time + buffer_duration_secs;
                })));

            onended.push(onended_closure);
        }

        Self {
            ctx,
            onended,
            buffer_duration_secs,
        }
    }
}

impl Drop for WebAudioDevice {
    fn drop(&mut self) {
        let _ = self.ctx.close().unwrap();
    }
}

impl Device for WebAudioDevice {
    fn get_mix_context(&mut self) -> Option<MixContext<'_>> {
        None
    }

    fn run(&mut self) {
        let window = fyrox_core::web_sys::window().unwrap();
        let mut offset_ms = 0;
        let time_step_ms = (self.buffer_duration_secs * 1_000.0) as i32;
        for on_ended_closure in self.onended.iter() {
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    on_ended_closure
                        .read()
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unchecked_ref(),
                    offset_ms,
                )
                .unwrap();
            offset_ms += time_step_ms;
        }
    }
}
