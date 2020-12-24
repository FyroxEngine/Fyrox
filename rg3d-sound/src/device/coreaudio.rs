use crate::{
    device::{Device, FeedCallback, MixContext, NativeSample},
    error::SoundError,
};
use coreaudio_sys::*;
use std::{ffi::c_void, mem::size_of};

pub struct CoreaudioSoundDevice {
    /// Give fixed memory location
    inner: Box<Inner>,
}

unsafe impl Send for CoreaudioSoundDevice {}

struct Inner {
    // MixContext
    fill_callback: Box<FeedCallback>,
    out_data: Vec<NativeSample>,
    mix_buffer: Vec<(f32, f32)>,
    // else
    queue: AudioQueueRef,
    bufs: [AudioQueueBufferRef; 2],
    buffer_len_bytes: usize,
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe {
            AudioQueueStop(self.queue, true as u8);
            // dispose audio queue and all of its resources, including its buffers
            AudioQueueDispose(self.queue, false as u8);
        }
    }
}

/// Handles error codes of coreaudio functions
fn check(error: OSStatus, msg: &str) -> Result<(), SoundError> {
    if error == noErr as i32 {
        Ok(())
    } else {
        let msg = format!("{}. Error code {}", msg, error);
        Err(SoundError::FailedToInitializeDevice(msg))
    }
}

/// Callback function set on `AufioQueueNewOutput`
unsafe extern "C" fn audio_queue_callback(
    user_data: *mut c_void,
    queue: AudioQueueRef,
    buf: AudioQueueBufferRef,
) {
    let inner: &mut Inner = &mut *(user_data as *mut Inner);
    inner.mix(); // Device::mix

    // set the buffer data
    let src = inner.out_data.as_mut_ptr() as *mut u8;
    let dst = (*buf).mAudioData as *const u8 as *mut u8;
    let len = inner.buffer_len_bytes as usize;
    std::ptr::copy_nonoverlapping(src, dst, len);

    AudioQueueEnqueueBuffer(queue, buf, 0, std::ptr::null_mut());
}

impl CoreaudioSoundDevice {
    pub fn new(
        buffer_len_bytes: u32,
        fill_callback: Box<FeedCallback>,
    ) -> Result<Self, SoundError> {
        // 16-bit linear PCM
        let desc = AudioStreamBasicDescription {
            mSampleRate: crate::context::SAMPLE_RATE as f64,
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kLinearPCMFormatFlagIsSignedInteger | kLinearPCMFormatFlagIsPacked,
            mBitsPerChannel: 16,
            mFramesPerPacket: 1,
            mChannelsPerFrame: 2,
            mBytesPerFrame: 4,
            mBytesPerPacket: 4,
            mReserved: 0,
        };

        // create data at fixed memory location
        let samples_per_channel = buffer_len_bytes as usize / size_of::<NativeSample>();
        let mut inner = Box::new(Inner {
            fill_callback,
            out_data: vec![Default::default(); samples_per_channel],
            mix_buffer: vec![(0.0, 0.0); samples_per_channel],
            queue: std::ptr::null_mut(),
            bufs: [std::ptr::null_mut(); 2],
            buffer_len_bytes: buffer_len_bytes as usize,
        });

        inner.queue = {
            let mut queue = std::ptr::null_mut();
            let res = unsafe {
                AudioQueueNewOutput(
                    &desc,
                    Some(self::audio_queue_callback),
                    // `user_data` passed to ^ (`self::audio_queue_callback`)
                    (&mut *inner) as *const Inner as *const c_void as *mut c_void,
                    // run the callback in this thread
                    core_foundation_sys::runloop::CFRunLoopGetCurrent() as *mut _,
                    kCFRunLoopCommonModes,
                    0,
                    &mut queue,
                )
            };

            self::check(res, "Failed to `AudioQueueNewOutput`")?;
            if queue == std::ptr::null_mut() {
                return Err(SoundError::FailedToInitializeDevice(
                    "Succeeded in `AudioQueueNewOutput` but the queue is null".into(),
                ));
            }

            queue
        };

        // create two audio buffers
        for i in 0..2 {
            inner.bufs[i] = {
                let mut buf: AudioQueueBufferRef = std::ptr::null_mut();
                let res =
                    unsafe { AudioQueueAllocateBuffer(inner.queue, buffer_len_bytes, &mut buf) };

                check(res, "Failed to `AudioQueueAllocateBuffer`")?;
                if buf == std::ptr::null_mut() {
                    return Err(SoundError::FailedToInitializeDevice(
                        "Succeeded in `AudioQueueAllocateBuffer` but the buffer is null".into(),
                    ))?;
                }

                // fill the buffer with zeroes
                unsafe {
                    (*buf).mAudioDataByteSize = buffer_len_bytes;

                    let data_ptr = (*buf).mAudioData;
                    std::ptr::write_bytes(
                        data_ptr as *const u8 as *mut u8,
                        0u8,
                        buffer_len_bytes as usize,
                    );

                    AudioQueueEnqueueBuffer(inner.queue, buf, 0, std::ptr::null_mut());
                }

                buf
            };
        }

        let res = unsafe { AudioQueueStart(inner.queue, std::ptr::null_mut()) };
        check(res, "Failed to `AudioQueueStart`")?;

        Ok(Self { inner })
    }
}

impl Device for CoreaudioSoundDevice {
    fn get_mix_context(&mut self) -> MixContext {
        MixContext {
            mix_buffer: &mut self.inner.mix_buffer,
            out_data: &mut self.inner.out_data,
            callback: &mut self.inner.fill_callback,
        }
    }

    fn run(&mut self) {
        unsafe {
            CFRunLoopRun(); // blocking
        }

        unreachable!();
    }
}

impl Device for Inner {
    fn get_mix_context(&mut self) -> MixContext {
        MixContext {
            mix_buffer: &mut self.mix_buffer,
            out_data: &mut self.out_data,
            callback: &mut self.fill_callback,
        }
    }

    fn run(&mut self) {
        unreachable!("`impl Device for Inner` is only for `mix`");
    }
}
