// WASM does not use some portions of the code, so compiler will complain about this,
// here we just suppress the warning.
#![allow(dead_code)]

//! Device module.
//!
//! # Overview
//!
//! Device is an abstraction over output device which provides unified way of communication with
//! output device.

#[cfg(target_os = "windows")]
mod dsound;

#[cfg(target_os = "linux")]
mod alsa;

#[cfg(target_os = "macos")]
mod coreaudio;

// The dummy target works on all platforms
mod dummy;

#[cfg(target_arch = "wasm32")]
mod web;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct NativeSample {
    pub left: i16,
    pub right: i16,
}

pub type FeedCallback = dyn FnMut(&mut [(f32, f32)]) + Send;

pub struct MixContext<'a> {
    mix_buffer: &'a mut [(f32, f32)],
    out_data: &'a mut [NativeSample],
    callback: &'a mut FeedCallback,
}

trait Device {
    fn get_mix_context(&mut self) -> Option<MixContext>;

    fn run(&mut self);

    fn mix(&mut self) {
        if let Some(context) = self.get_mix_context() {
            // Clear mixer buffer.
            for (left, right) in context.mix_buffer.iter_mut() {
                *left = 0.0;
                *right = 0.0;
            }

            // Fill it.
            (context.callback)(context.mix_buffer);

            // Convert to i16 - device expects samples in this format.
            assert_eq!(context.mix_buffer.len(), context.out_data.len());
            for ((left, right), ref mut out_sample) in
                context.mix_buffer.iter().zip(context.out_data)
            {
                fn sample_to_i16(sample: f32) -> i16 {
                    const SCALE: f32 = i16::MAX as f32;
                    let clamped = sample.clamp(-1.0, 1.0);
                    (clamped * SCALE) as i16
                }

                out_sample.left = sample_to_i16(*left);
                out_sample.right = sample_to_i16(*right);
            }
        }
    }
}

/// Transfer ownership of device to separate mixer thread. It will
/// call the callback with a specified rate to get data to send to a physical device.
#[allow(unused_variables)]
pub(crate) fn run_device<F>(headless: bool, buffer_len_bytes: u32, callback: F)
where
    F: FnMut(&mut [(f32, f32)]) + Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        if headless {
            let mut device = dummy::DummySoundDevice::new(buffer_len_bytes, callback).unwrap();
            device.run();
        } else {
            #[cfg(target_os = "windows")]
            let mut device = dsound::DirectSoundDevice::new(buffer_len_bytes, callback).unwrap();
            #[cfg(target_os = "linux")]
            let mut device = alsa::AlsaSoundDevice::new(buffer_len_bytes, callback).unwrap();
            #[cfg(target_os = "macos")]
            let mut device =
                coreaudio::CoreaudioSoundDevice::new(buffer_len_bytes, callback).unwrap();
            #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
            let mut device = dummy::DummySoundDevice::new(buffer_len_bytes, callback).unwrap();
            device.run()
        }
    });

    #[cfg(target_arch = "wasm32")]
    {
        if headless {
            let mut device = dummy::DummySoundDevice::new(buffer_len_bytes, callback);
            device.run();
            std::mem::forget(device);
        } else {
            let mut device = web::WebAudioDevice::new(buffer_len_bytes, callback);
            device.run();
            std::mem::forget(device);
        }
    }
}
