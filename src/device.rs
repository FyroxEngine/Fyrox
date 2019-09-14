use crate::error::SoundError;

// TODO: Make this configurable, for now its set to most commonly used sample rate of 44100 Hz.
pub const SAMPLE_RATE: u32 = 44100;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Sample {
    pub left: i16,
    pub right: i16,
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
                IDirectSound,
                DirectSoundCreate,
                DSBCAPS_CTRLPOSITIONNOTIFY,
                DSSCL_PRIORITY,
                DS_OK,
                DSBCAPS_GLOBALFOCUS,
                DSBUFFERDESC,
                IDirectSoundBuffer,
                IID_IDirectSoundNotify,
                DSBPLAY_LOOPING,
            },
            winuser::GetForegroundWindow,
            synchapi::{
                CreateEventA,
                WaitForMultipleObjects,
            },
            unknwnbase::{
                IUnknownVtbl,
                IUnknown,
            },
            winbase::{INFINITE, WAIT_OBJECT_0},
        },
        shared::{
            guiddef::IID_NULL,
            mmreg::{WAVEFORMATEX, WAVE_FORMAT_PCM},
            ntdef::{
                HANDLE,
                PVOID,
            },
            minwindef::DWORD,
            winerror::HRESULT,
        },
        ctypes::c_void,
    };
    use crate::{
        device::{Sample, FeedCallback, SAMPLE_RATE},
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
        out_data: Vec<Sample>,
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
                let bits_per_sample = 8 * size_of::<i16>() as u16;
                let block_align = (bits_per_sample / 8) * channels_count;

                let mut buffer_format = WAVEFORMATEX {
                    wFormatTag: WAVE_FORMAT_PCM,
                    nChannels: channels_count,
                    nSamplesPerSec: SAMPLE_RATE,
                    nAvgBytesPerSec: SAMPLE_RATE * u32::from(block_align),
                    nBlockAlign: block_align,
                    wBitsPerSample: bits_per_sample,
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

                let samples_per_channel = buffer_len_bytes as usize / size_of::<Sample>();
                Ok(Self {
                    direct_sound: AtomicPtr::new(direct_sound),
                    buffer: AtomicPtr::new(buffer),
                    out_data: Vec::with_capacity(samples_per_channel),
                    mix_buffer: vec![(0.0, 0.0); samples_per_channel],
                    notify_points: [a, b],
                    buffer_len_bytes,
                    callback,
                })
            }
        }

        pub fn feed(&mut self) {
            // Clear mixer buffer.
            for (left, right) in self.mix_buffer.iter_mut() {
                *left = 0.0;
                *right = 0.0;
            }

            // Fill it.
            (self.callback)(self.mix_buffer.as_mut_slice());

            let scale = f32::from(std::i16::MAX);
            // Convert to i16 - device expects samples in this format.
            self.out_data.clear();
            for (left, right) in self.mix_buffer.iter() {
                let left_clamped = if *left > 1.0 {
                    0.0
                } else if *left < -1.0 {
                    -1.0
                } else {
                    *left
                };

                let right_clamped = if *right > 1.0 {
                    0.0
                } else if *right < -1.0 {
                    -1.0
                } else {
                    *right
                };

                self.out_data.push(Sample {
                    left: (left_clamped * scale) as i16,
                    right: (right_clamped * scale) as i16,
                })
            }

            let notify_points = [
                self.notify_points[0].load(Ordering::SeqCst),
                self.notify_points[1].load(Ordering::SeqCst)
            ];
            let buffer = self.buffer.load(Ordering::SeqCst);
            let mut output_data: *mut c_void = std::ptr::null_mut();
            let mut size: DWORD = 0;

            // Wait and send.
            unsafe {
                let result = WaitForMultipleObjects(2,
                                                    notify_points.as_ptr(),
                                                    0,
                                                    INFINITE);
                if result == WAIT_OBJECT_0 {
                    (*buffer).Lock(self.buffer_len_bytes,
                                   self.buffer_len_bytes,
                                   &mut output_data,
                                   &mut size,
                                   std::ptr::null_mut(),
                                   std::ptr::null_mut(),
                                   0);
                    std::ptr::copy_nonoverlapping(self.out_data.as_ptr() as *mut u8,
                                                  output_data as *mut u8,
                                                  size as usize);
                    (*buffer).Unlock(output_data, size, std::ptr::null_mut(), 0);
                } else if result == WAIT_OBJECT_0 + 1 {
                    (*buffer).Lock(0,
                                   self.buffer_len_bytes,
                                   &mut output_data,
                                   &mut size,
                                   std::ptr::null_mut(),
                                   std::ptr::null_mut(),
                                   0);
                    std::ptr::copy_nonoverlapping(self.out_data.as_ptr() as *mut u8,
                                                  output_data as *mut u8,
                                                  size as usize);
                    (*buffer).Unlock(output_data, size, std::ptr::null_mut(), 0);
                }
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
}

pub type FeedCallback = dyn FnMut(&mut [(f32, f32)]) + Send;

pub fn run_device(buffer_len_bytes: u32, callback: Box<FeedCallback>) -> Result<(), SoundError> {
    if cfg!(windows) {
        // Transfer ownership of device to separate mixer thread. It will
        // call the callback with a specified rate to get data to send to a physical device.
        let mut device = windows::DirectSoundDevice::new(buffer_len_bytes, callback)?;
        std::thread::spawn(move || {
            loop { device.feed() }
        });
        Ok(())
    } else {
        Err(SoundError::NoBackend)
    }
}