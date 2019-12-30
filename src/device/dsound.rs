#![allow(non_snake_case)]

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