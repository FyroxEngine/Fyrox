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
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod dummy;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NativeSample {
    pub left: i16,
    pub right: i16,
}

impl Default for NativeSample {
    fn default() -> Self {
        Self { left: 0, right: 0 }
    }
}

pub type FeedCallback = dyn FnMut(&mut [(f32, f32)]) + Send;

pub struct MixContext<'a> {
    mix_buffer: &'a mut [(f32, f32)],
    out_data: &'a mut [NativeSample],
    callback: &'a mut FeedCallback,
}

fn sample_to_i16(sample: f32) -> i16 {
    const SCALE: f32 = std::i16::MAX as f32;
    let clamped = if sample > 1.0 {
        1.0
    } else if sample < -1.0 {
        -1.0
    } else {
        sample
    };
    (clamped * SCALE) as i16
}

trait Device {
    fn get_mix_context(&mut self) -> MixContext;

    fn run(&mut self);

    fn mix(&mut self) {
        let context = self.get_mix_context();

        // Clear mixer buffer.
        for (left, right) in context.mix_buffer.iter_mut() {
            *left = 0.0;
            *right = 0.0;
        }

        // Fill it.
        (context.callback)(context.mix_buffer);

        // Convert to i16 - device expects samples in this format.
        assert_eq!(context.mix_buffer.len(), context.out_data.len());
        for ((left, right), ref mut out_sample) in context.mix_buffer.iter().zip(context.out_data) {
            out_sample.left = sample_to_i16(*left);
            out_sample.right = sample_to_i16(*right);
        }
    }
}

/// Transfer ownership of device to separate mixer thread. It will
/// call the callback with a specified rate to get data to send to a physical device.
pub(in crate) fn run_device(buffer_len_bytes: u32, callback: Box<FeedCallback>) {
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        let mut device = dsound::DirectSoundDevice::new(buffer_len_bytes, callback).unwrap();
        #[cfg(target_os = "linux")]
        let mut device = alsa::AlsaSoundDevice::new(buffer_len_bytes, callback).unwrap();
        #[cfg(target_os = "macos")]
        let mut device = coreaudio::CoreaudioSoundDevice::new(buffer_len_bytes, callback).unwrap();
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        let mut device = dummy::DummySoundDevice::new(buffer_len_bytes, callback).unwrap();
        device.run()
    });
}
