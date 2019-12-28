use crate::error::SoundError;

// TODO: Make this configurable, for now its set to most commonly used sample rate of 44100 Hz.
pub const SAMPLE_RATE: u32 = 44100;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NativeSample {
    pub left: i16,
    pub right: i16,
}

impl Default for NativeSample {
    fn default() -> Self {
        Self {
            left: 0,
            right: 0,
        }
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

    fn feed(&mut self);

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

#[cfg(target_os = "windows")]
#[allow(non_snake_case)]
mod windows {
    use std::{
        mem::size_of,
        sync::atomic::{Ordering, AtomicPtr},
    };
    use winapi::{
        um::{
            dsound::{
                IDirectSound, DirectSoundCreate,
                DSBCAPS_CTRLPOSITIONNOTIFY, DSSCL_PRIORITY,
                DS_OK, DSBCAPS_GLOBALFOCUS,
                DSBUFFERDESC, IDirectSoundBuffer,
                IID_IDirectSoundNotify, DSBPLAY_LOOPING,
            },
            winuser::GetForegroundWindow,
            synchapi::{CreateEventA, WaitForMultipleObjects},
            unknwnbase::{IUnknownVtbl, IUnknown},
            winbase::{INFINITE, WAIT_OBJECT_0},
        },
        shared::{
            guiddef::IID_NULL, mmreg::{WAVEFORMATEX, WAVE_FORMAT_PCM},
            ntdef::{HANDLE, PVOID}, minwindef::DWORD, winerror::HRESULT,
        },
        ctypes::c_void,
    };
    use crate::{
        device::{NativeSample, FeedCallback, SAMPLE_RATE, Device, MixContext},
        error::SoundError,
    };

    // Declare missing structs and interfaces.
    STRUCT! {struct DSBPOSITIONNOTIFY {
        dwOffset: DWORD,
        hEventNotify: HANDLE,
    }}

    RIDL! {#[uuid(0xb021_0783, 0x89cd, 0x11d0, 0xaf, 0x8, 0x0, 0xa0, 0xc9, 0x25, 0xcd, 0x16)]
    interface IDirectSoundNotify(IDirectSoundNotifyVtbl): IUnknown(IUnknownVtbl) {
        fn SetNotificationPositions(
            dwPositionNotifies : DWORD,
            pcPositionNotifies : PVOID,
            ) -> HRESULT,
    }}

    pub struct DirectSoundDevice {
        direct_sound: AtomicPtr<IDirectSound>,
        buffer: AtomicPtr<IDirectSoundBuffer>,
        notify_points: [AtomicPtr<c_void>; 2],
        buffer_len_bytes: u32,
        out_data: Vec<NativeSample>,
        mix_buffer: Vec<(f32, f32)>,
        callback: Box<FeedCallback>,
    }

    impl DirectSoundDevice {
        pub fn new(buffer_len_bytes: u32, callback: Box<FeedCallback>) -> Result<Self, SoundError> {
            unsafe {
                let mut direct_sound = std::ptr::null_mut();
                if DirectSoundCreate(std::ptr::null(), &mut direct_sound, std::ptr::null_mut()) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice("Failed to initialize DirectSound".to_string()));
                }

                if (*direct_sound).SetCooperativeLevel(GetForegroundWindow(), DSSCL_PRIORITY) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice("Failed to set cooperative level".to_string()));
                }

                let channels_count = 2;
                let byte_per_sample = size_of::<i16>() as u16;
                let block_align = byte_per_sample * channels_count;

                let mut buffer_format = WAVEFORMATEX {
                    wFormatTag: WAVE_FORMAT_PCM,
                    nChannels: channels_count,
                    nSamplesPerSec: SAMPLE_RATE,
                    nAvgBytesPerSec: SAMPLE_RATE * u32::from(block_align),
                    nBlockAlign: block_align,
                    wBitsPerSample: 8 * byte_per_sample,
                    cbSize: size_of::<WAVEFORMATEX>() as u16,
                };

                let buffer_desc = DSBUFFERDESC {
                    dwSize: size_of::<DSBUFFERDESC>() as u32,
                    dwFlags: DSBCAPS_CTRLPOSITIONNOTIFY | DSBCAPS_GLOBALFOCUS,
                    dwBufferBytes: 2 * buffer_len_bytes,
                    dwReserved: 0,
                    lpwfxFormat: &mut buffer_format,
                    guid3DAlgorithm: IID_NULL,
                };

                let mut buffer: *mut IDirectSoundBuffer = std::ptr::null_mut();
                if (*direct_sound).CreateSoundBuffer(&buffer_desc, &mut buffer, std::ptr::null_mut()) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice("Failed to create back buffer.".to_string()));
                }

                let mut notify: *mut IDirectSoundNotify = std::ptr::null_mut();
                if (*buffer).QueryInterface(&IID_IDirectSoundNotify, ((&mut notify) as *mut *mut IDirectSoundNotify) as *mut *mut c_void) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice("Failed to obtain IDirectSoundNotify interface.".to_string()));
                }

                let notify_points = [
                    CreateEventA(std::ptr::null_mut(), 0, 0, std::ptr::null()),
                    CreateEventA(std::ptr::null_mut(), 0, 0, std::ptr::null())
                ];

                let mut pos = [
                    DSBPOSITIONNOTIFY {
                        dwOffset: 0,
                        hEventNotify: notify_points[0],
                    },
                    DSBPOSITIONNOTIFY {
                        dwOffset: buffer_desc.dwBufferBytes / 2,
                        hEventNotify: notify_points[1],
                    }
                ];

                let pos_ptr = &mut pos as *mut [DSBPOSITIONNOTIFY; 2];
                if (*notify).SetNotificationPositions(2, pos_ptr as *mut c_void) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice("Failed to set notification positions.".to_string()));
                }

                if (*buffer).Play(0, 0, DSBPLAY_LOOPING) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice("Failed to begin playing back buffer.".to_string()));
                }

                let a = AtomicPtr::new(notify_points[0]);
                let b = AtomicPtr::new(notify_points[1]);

                let samples_per_channel = buffer_len_bytes as usize / size_of::<NativeSample>();

                Ok(Self {
                    direct_sound: AtomicPtr::new(direct_sound),
                    buffer: AtomicPtr::new(buffer),
                    out_data: vec![Default::default(); samples_per_channel],
                    mix_buffer: vec![(0.0, 0.0); samples_per_channel],
                    notify_points: [a, b],
                    buffer_len_bytes,
                    callback,
                })
            }
        }
    }

    impl Drop for DirectSoundDevice {
        fn drop(&mut self) {
            unsafe {
                let direct_sound = self.direct_sound.load(Ordering::SeqCst);
                assert_eq!((*direct_sound).Release(), 0);
            }
        }
    }

    unsafe fn write(ds_buffer: *mut IDirectSoundBuffer, offset_bytes: u32, len_bytes: u32, data: &[NativeSample]) {
        let mut size = 0;
        let mut device_buffer = std::ptr::null_mut();
        (*ds_buffer).Lock(offset_bytes, len_bytes, &mut device_buffer, &mut size, std::ptr::null_mut(), std::ptr::null_mut(), 0);
        std::ptr::copy_nonoverlapping(data.as_ptr() as *mut u8, device_buffer as *mut u8, size as usize);
        (*ds_buffer).Unlock(device_buffer, size, std::ptr::null_mut(), 0);
    }

    impl Device for DirectSoundDevice {
        fn get_mix_context(&mut self) -> MixContext {
            MixContext {
                mix_buffer: self.mix_buffer.as_mut_slice(),
                out_data: &mut self.out_data,
                callback: &mut self.callback,
            }
        }

        fn feed(&mut self) {
            self.mix();

            let notify_points = [
                self.notify_points[0].load(Ordering::SeqCst),
                self.notify_points[1].load(Ordering::SeqCst)
            ];
            let buffer = self.buffer.load(Ordering::SeqCst);

            // Wait and send.
            unsafe {
                const WAIT_OBJECT_1: u32 = WAIT_OBJECT_0 + 1;
                match WaitForMultipleObjects(2, notify_points.as_ptr(), 0, INFINITE) {
                    WAIT_OBJECT_0 => write(buffer, self.buffer_len_bytes, self.buffer_len_bytes, &self.out_data),
                    WAIT_OBJECT_1 => write(buffer, 0, self.buffer_len_bytes, &self.out_data),
                    _ => panic!("Unknown buffer point!")
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use crate::{
        device::{FeedCallback, SAMPLE_RATE, NativeSample},
        error::SoundError,
    };
    use alsa_sys::*;
    use std::{
        ffi::{CStr, CString},
        os::raw::c_int,
        mem::size_of,
        sync::atomic::{AtomicPtr, Ordering},
    };
    use crate::device::{Device, MixContext};

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

    pub fn check_result(err_code: c_int) -> Result<(), SoundError> {
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
                check_result(snd_pcm_open(&mut playback_device, CString::new("default").unwrap().as_ptr() as *const _, SND_PCM_STREAM_PLAYBACK, 0))?;
                let mut hw_params = std::ptr::null_mut();
                check_result(snd_pcm_hw_params_malloc(&mut hw_params))?;
                check_result(snd_pcm_hw_params_any(playback_device, hw_params))?;
                let access = SND_PCM_ACCESS_RW_INTERLEAVED;
                check_result(snd_pcm_hw_params_set_access(playback_device, hw_params, access))?;
                check_result(snd_pcm_hw_params_set_format(playback_device, hw_params, SND_PCM_FORMAT_S16_LE))?;
                let mut exact_rate = SAMPLE_RATE;
                check_result(snd_pcm_hw_params_set_rate_near(playback_device, hw_params, &mut exact_rate, std::ptr::null_mut()))?;
                check_result(snd_pcm_hw_params_set_channels(playback_device, hw_params, 2))?;
                let mut exact_size = (frame_count * 2) as u64;
                check_result(snd_pcm_hw_params_set_buffer_size_near(playback_device, hw_params, &mut exact_size))?;
                check_result(snd_pcm_hw_params(playback_device, hw_params))?;
                snd_pcm_hw_params_free(hw_params);
                let mut sw_params = std::ptr::null_mut();
                check_result(snd_pcm_sw_params_malloc(&mut sw_params))?;
                check_result(snd_pcm_sw_params_current(playback_device, sw_params))?;
                check_result(snd_pcm_sw_params_set_avail_min(playback_device, sw_params, frame_count.into()))?;
                check_result(snd_pcm_sw_params_set_start_threshold(playback_device, sw_params, frame_count.into()))?;
                check_result(snd_pcm_sw_params(playback_device, sw_params))?;
                check_result(snd_pcm_prepare(playback_device))?;

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
}

#[cfg(target_os = "linux")]
pub fn run_device_internal(buffer_len_bytes: u32, callback: Box<FeedCallback>) -> Result<(), SoundError> {
    let mut device = linux::AlsaSoundDevice::new(buffer_len_bytes, callback)?;
    std::thread::spawn(move || {
        loop { device.feed() }
    });
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn run_device_internal(buffer_len_bytes: u32, callback: Box<FeedCallback>) -> Result<(), SoundError> {
    let mut device = windows::DirectSoundDevice::new(buffer_len_bytes, callback)?;
    std::thread::spawn(move || {
        loop { device.feed() }
    });
    Ok(())
}

// Transfer ownership of device to separate mixer thread. It will
// call the callback with a specified rate to get data to send to a physical device.
pub fn run_device(buffer_len_bytes: u32, callback: Box<FeedCallback>) -> Result<(), SoundError> {
    run_device_internal(buffer_len_bytes, callback)
}