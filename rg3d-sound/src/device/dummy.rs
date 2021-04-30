#![allow(unused)]

use crate::{
    device::{Device, FeedCallback, MixContext, NativeSample},
    error::SoundError,
};
use std::mem::size_of;

pub struct DummySoundDevice {}

impl DummySoundDevice<F: FnMut(&mut [(f32, f32)]) + Send + 'static> {
    pub fn new(_buffer_len_bytes: u32, _callback: F) -> Result<Self, SoundError> {
        Ok(Self)
    }
}

impl Device for DummySoundDevice {
    fn get_mix_context(&mut self) -> Option<MixContext> {
        None
    }

    fn run(&mut self) {
        loop {
            self.mix();

            //std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
