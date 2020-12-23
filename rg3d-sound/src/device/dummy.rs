use crate::{
    device::{Device, FeedCallback, MixContext, NativeSample},
    error::SoundError,
};
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct DummySoundDevice {
    callback: Box<FeedCallback>,
    out_data: Vec<NativeSample>,
    mix_buffer: Vec<(f32, f32)>,
}

impl DummySoundDevice {
    pub fn new(buffer_len_bytes: u32, callback: Box<FeedCallback>) -> Result<Self, SoundError> {
        let samples_per_channel = buffer_len_bytes as usize / size_of::<NativeSample>();
        Ok(Self {
            callback,
            out_data: vec![Default::default(); samples_per_channel],
            mix_buffer: vec![(0.0, 0.0); samples_per_channel],
        })
    }
}

impl Device for DummySoundDevice {
    fn get_mix_context(&mut self) -> MixContext {
        MixContext {
            mix_buffer: self.mix_buffer.as_mut_slice(),
            out_data: &mut self.out_data,
            callback: &mut self.callback,
        }
    }

    fn run(&mut self, stop_token: Arc<AtomicBool>) {
        loop {
            if stop_token.load(Ordering::SeqCst) {
                break;
            }

            self.mix();
        }
    }
}
