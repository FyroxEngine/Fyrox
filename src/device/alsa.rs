use crate::{
    error::SoundError,
    device::{
        Device,
        FeedCallback,
        SAMPLE_RATE,
        NativeSample,
        MixContext
    }
};
use alsa_sys::*;
use std::{
    ffi::{CStr, CString},
    os::raw::c_int,
    mem::size_of,
    sync::atomic::{AtomicPtr, Ordering},
};

pub struct AlsaSoundDevice {
    playback_device: AtomicPtr<snd_pcm_t>,
    frame_count: u32,
    callback: Box<FeedCallback>,
    out_data: Vec<NativeSample>,
    mix_buffer: Vec<(f32, f32)>,
}

pub fn err_code_to_string(err_code: c_int) -> String {
    unsafe {
        let message = CStr::from_ptr(snd_strerror(err_code) as *const _)
            .to_bytes()
            .to_vec();
        String::from_utf8(message).unwrap()
    }
}

pub fn check(err_code: c_int) -> Result<(), SoundError> {
    if err_code < 0 {
        Err(SoundError::FailedToInitializeDevice(err_code_to_string(err_code)))
    } else {
        Ok(())
    }
}

impl AlsaSoundDevice {
    pub fn new(buffer_len_bytes: u32, callback: Box<FeedCallback>) -> Result<Self, SoundError> {
        unsafe {
            let frame_count = buffer_len_bytes / 4; /* 16-bit stereo is 4 bytes, so frame count is bufferHalfSize / 4 */
            let mut playback_device = std::ptr::null_mut();
            check(snd_pcm_open(&mut playback_device, CString::new("default").unwrap().as_ptr() as *const _, SND_PCM_STREAM_PLAYBACK, 0))?;
            let mut hw_params = std::ptr::null_mut();
            check(snd_pcm_hw_params_malloc(&mut hw_params))?;
            check(snd_pcm_hw_params_any(playback_device, hw_params))?;
            let access = SND_PCM_ACCESS_RW_INTERLEAVED;
            check(snd_pcm_hw_params_set_access(playback_device, hw_params, access))?;
            check(snd_pcm_hw_params_set_format(playback_device, hw_params, SND_PCM_FORMAT_S16_LE))?;
            let mut exact_rate = SAMPLE_RATE;
            check(snd_pcm_hw_params_set_rate_near(playback_device, hw_params, &mut exact_rate, std::ptr::null_mut()))?;
            check(snd_pcm_hw_params_set_channels(playback_device, hw_params, 2))?;
            let mut exact_size = (frame_count * 2) as u64;
            check(snd_pcm_hw_params_set_buffer_size_near(playback_device, hw_params, &mut exact_size))?;
            check(snd_pcm_hw_params(playback_device, hw_params))?;
            snd_pcm_hw_params_free(hw_params);
            let mut sw_params = std::ptr::null_mut();
            check(snd_pcm_sw_params_malloc(&mut sw_params))?;
            check(snd_pcm_sw_params_current(playback_device, sw_params))?;
            check(snd_pcm_sw_params_set_avail_min(playback_device, sw_params, frame_count.into()))?;
            check(snd_pcm_sw_params_set_start_threshold(playback_device, sw_params, frame_count.into()))?;
            check(snd_pcm_sw_params(playback_device, sw_params))?;
            check(snd_pcm_prepare(playback_device))?;

            let samples_per_channel = buffer_len_bytes as usize / size_of::<NativeSample>();
            Ok(Self {
                playback_device: AtomicPtr::new(playback_device),
                frame_count,
                callback,
                out_data: vec![Default::default(); samples_per_channel],
                mix_buffer: vec![(0.0, 0.0); samples_per_channel],
            })
        }
    }
}

impl Device for AlsaSoundDevice {
    fn get_mix_context(&mut self) -> MixContext {
        MixContext {
            mix_buffer: self.mix_buffer.as_mut_slice(),
            out_data: &mut self.out_data,
            callback: &mut self.callback,
        }
    }

    fn feed(&mut self) {
        self.mix();

        unsafe {
            let device = self.playback_device.load(Ordering::SeqCst);
            let err = snd_pcm_writei(device, self.out_data.as_ptr() as *const _, self.frame_count.into()) as i32;
            if err == -32 {
                // EPIPE error (buffer underrun)
                snd_pcm_recover(device, err, 0);
            }
        }
    }
}

impl Drop for AlsaSoundDevice {
    fn drop(&mut self) {
        unsafe {
            let device = self.playback_device.load(Ordering::SeqCst);

            snd_pcm_close(device);
        }
    }
}