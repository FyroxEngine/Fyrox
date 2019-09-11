use crate::error::SoundError;

#[repr(C)]
#[derive(Debug)]
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
    use crate::device::{Sample, FeedCallback};
    use crate::error::SoundError;

    // Declare missing structs and interfaces.
    STRUCT! {struct DSBPOSITIONNOTIFY {
        dwOffset: DWORD,
        hEventNotify: HANDLE,
    }}

    RIDL! {#[uuid(0xb0210783, 0x89cd, 0x11d0, 0xaf, 0x8, 0x0, 0xa0, 0xc9, 0x25, 0xcd, 0x16)]
    interface IDirectSoundNotify(IDirectSoundNotifyVtbl): IUnknown(IUnknownVtbl) {
        fn SetNotificationPositions(
            dwPositionNotifies : DWORD,
            pcPositionNotifies : PVOID,
            ) -> HRESULT,
    }}

    pub struct DirectSoundDevice {
        dsound: AtomicPtr<IDirectSound>,
        buffer: AtomicPtr<IDirectSoundBuffer>,
        notify_points: [AtomicPtr<c_void>; 2],
        buffer_len_bytes: u32,
        sample_rate: u32,
        out_data: Vec<Sample>,
        callback: Box<FeedCallback>,
    }

    #[cfg(target_os = "windows")]
    impl DirectSoundDevice {
        pub fn new(buffer_len_bytes: u32, callback: Box<FeedCallback>) -> Result<Self, SoundError> {
            unsafe {
                let mut dsound = std::ptr::null_mut();
                if DirectSoundCreate(std::ptr::null(), &mut dsound, std::ptr::null_mut()) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice);
                }

                if (*dsound).SetCooperativeLevel(GetForegroundWindow(), DSSCL_PRIORITY) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice);
                }

                let sample_rate = 44100;
                let channels_count = 2;
                let bits_per_sample = 8 * size_of::<i16>() as u16;
                let block_align = (bits_per_sample / 8) * channels_count;

                let mut buffer_format = WAVEFORMATEX {
                    wFormatTag: WAVE_FORMAT_PCM,
                    nChannels: channels_count,
                    nSamplesPerSec: sample_rate,
                    nAvgBytesPerSec: sample_rate * (block_align as u32),
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
                if (*dsound).CreateSoundBuffer(&buffer_desc, &mut buffer, std::ptr::null_mut()) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice);
                }

                let mut void_notify: *mut c_void = std::ptr::null_mut();
                if (*buffer).QueryInterface(&IID_IDirectSoundNotify, &mut void_notify) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice);
                }
                let notify = void_notify as *mut IDirectSoundNotify;

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
                    return Err(SoundError::FailedToInitializeDevice);
                }

                let total_bytes = buffer_len_bytes as usize;
                let mut samples_buffer = Vec::with_capacity(total_bytes);
                for _ in 0..total_bytes {
                    samples_buffer.push(Sample { left: 0, right: 0 });
                }

                if (*buffer).Play(0, 0, DSBPLAY_LOOPING) != DS_OK {
                    return Err(SoundError::FailedToInitializeDevice);
                }

                let a = AtomicPtr::new(notify_points[0]);
                let b = AtomicPtr::new(notify_points[1]);

                Ok(Self {
                    dsound: AtomicPtr::new(dsound),
                    buffer: AtomicPtr::new(buffer),
                    out_data: samples_buffer,
                    sample_rate,
                    notify_points: [a, b],
                    buffer_len_bytes,
                    callback,
                })
            }
        }

        pub fn feed(&mut self) {
            unsafe {
                let notify_points = [
                    self.notify_points[0].load(Ordering::SeqCst),
                    self.notify_points[1].load(Ordering::SeqCst)
                ];
                let buffer = self.buffer.load(Ordering::SeqCst);
                let mut output_data: *mut c_void = std::ptr::null_mut();
                let mut size: DWORD = 0;
                let result = WaitForMultipleObjects(2,
                                                    notify_points.as_ptr(),
                                                    0,
                                                    INFINITE);
                if result == WAIT_OBJECT_0 {
                    // Upload samples in the second half of a buffer.
                    (self.callback)(self.out_data.as_mut_slice());
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
                    // Upload samples in the first half of a buffer.
                    (self.callback)(self.out_data.as_mut_slice());
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

    #[cfg(target_os = "windows")]
    impl Drop for DirectSoundDevice {
        fn drop(&mut self) {
            unsafe {
                let dsound = self.dsound.load(Ordering::SeqCst);
                assert_eq!((*dsound).Release(), 0);
            }
        }
    }
}

pub type FeedCallback = dyn FnMut(&mut [Sample]) + Send;

pub fn run_device(buffer_len_bytes: u32, callback: Box<FeedCallback>) -> Result<(), SoundError> {
    if cfg!(windows) {
        // Transfer ownership of device to separate mixer thread. It will
        // call the callback with a specified rate to get data to send to a physical device.
        let mut device = windows::DirectSoundDevice::new(buffer_len_bytes, callback)?;
        std::thread::spawn(move || {
            loop {
                device.feed()
            }
        });
        Ok(())
    } else {
        panic!("not implemented");
    }
}