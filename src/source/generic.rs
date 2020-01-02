use std::{
    sync::{
        Arc,
        Mutex,
    },
    time::Duration,
};
use crate::{
    buffer::{
        SoundBuffer,
        streaming::StreamingBuffer,
    },
    source::{
        Status,
        SoundSource
    },
    error::SoundError,
};
use rg3d_core::visitor::{
    Visit,
    VisitResult,
    Visitor,
};

pub struct GenericSource {
    buffer: Option<Arc<Mutex<SoundBuffer>>>,
    // Read position in the buffer. Differs from `playback_pos` if buffer is streaming.
    // In case of streaming buffer its maximum value will be size o
    buf_read_pos: f64,
    // Real playback position.
    playback_pos: f64,
    panning: f32,
    pitch: f64,
    gain: f32,
    looping: bool,
    // Important coefficient for runtime resampling. It is used to modify playback speed
    // of a source in order to match output device sampling rate. PCM data can be stored
    // in various sampling rates (22050 Hz, 44100 Hz, 88200 Hz, etc.) but output device
    // is running at fixed sampling rate (usually 44100 Hz). For example if we we'll feed
    // data to device with rate of 22050 Hz but device is running at 44100 Hz then we'll
    // hear that sound will have high pitch (2.0), to fix that we'll just pre-multiply
    // playback speed by 0.5.
    resampling_multiplier: f64,
    status: Status,
    play_once: bool,
    pub(in crate) last_left_gain: f32,
    pub(in crate) last_right_gain: f32
}

impl Default for GenericSource {
    fn default() -> Self {
        Self {
            buffer: None,
            buf_read_pos: 0.0,
            playback_pos: 0.0,
            panning: 0.0,
            pitch: 1.0,
            gain: 1.0,
            looping: false,
            resampling_multiplier: 1.0,
            status: Status::Stopped,
            play_once: false,
            last_left_gain: 1.0,
            last_right_gain: 1.0
        }
    }
}

/// Returns index of sample aligned to first channel by given arbitrary position.
/// Buffers has samples in interleaved format, it means that for channel amount > 1
/// samples will have this layout: LRLRLR..., when we reading from buffer we want
/// to start reading from first channel in buffer, but since we using automatic
/// resampling and variable pitch, read pos can have fractional part and even be
/// unaligned to first channel. This function fixes that, it takes arbitrary
/// position and aligns it to first channel so we can start reading samples for
/// each channel by:
/// left = read(index)
/// right = read(index + 1)
fn position_to_index(position: f64, channel_count: usize) -> usize {
    let index = position as usize;

    let aligned = if channel_count == 1 {
        index
    } else {
        index - index % channel_count
    };

    debug_assert_eq!(aligned % channel_count, 0);

    aligned
}

impl GenericSource {
    pub fn set_buffer(&mut self, buffer: Arc<Mutex<SoundBuffer>>) -> Result<Arc<Mutex<SoundBuffer>>, SoundError> {
        self.buf_read_pos = 0.0;
        self.playback_pos = 0.0;

        // Check new buffer if streaming - it must not be used by anyone else.
        if let SoundBuffer::Streaming(ref mut streaming) = *buffer.lock()? {
            if streaming.use_count != 0 {
                return Err(SoundError::StreamingBufferAlreadyInUse);
            }
            streaming.use_count += 1;
        }

        // If we already have streaming buffer assigned make sure to decrease use count
        // so it can be reused later on if needed.
        if let Some(mut buffer) = self.buffer.as_ref().and_then(|b| b.lock().ok()) {
            if let SoundBuffer::Streaming(ref mut streaming) = *buffer {
                streaming.use_count -= 1;
            }
        }

        Ok(self.buffer.replace(buffer).unwrap())
    }

    pub fn buffer(&self) -> Option<Arc<Mutex<SoundBuffer>>> {
        self.buffer.clone()
    }

    pub fn set_play_once(&mut self, play_once: bool) {
        self.play_once = play_once;
    }

    pub fn is_play_once(&self) -> bool {
        self.play_once
    }

    pub fn set_gain(&mut self, gain: f32) -> &mut Self {
        self.gain = gain;
        self
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }

    pub fn set_panning(&mut self, panning: f32) -> &mut Self {
        self.panning = panning.max(-1.0).min(1.0);
        self
    }

    pub fn panning(&self) -> f32 {
        self.panning
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn play(&mut self) -> &mut Self {
        self.status = Status::Playing;
        self
    }

    pub fn pause(&mut self) -> &mut Self {
        self.status = Status::Paused;
        self
    }

    pub fn set_looping(&mut self, looping: bool) -> &mut Self {
        self.looping = looping;
        self
    }

    pub fn is_looping(&self) -> bool {
        self.looping
    }

    pub fn set_pitch(&mut self, pitch: f64) -> &mut Self {
        self.pitch = pitch;
        self
    }

    pub fn pitch(&self) -> f64 {
        self.pitch
    }

    pub fn stop(&mut self) -> Result<(), SoundError> {
        self.status = Status::Stopped;

        self.buf_read_pos = 0.0;
        self.playback_pos = 0.0;

        if let Some(mut buffer) = self.buffer.as_ref().and_then(|b| b.lock().ok()) {
            if let SoundBuffer::Streaming(ref mut streaming) = *buffer {
                streaming.rewind()?;
            }
        }

        Ok(())
    }

    pub fn playback_time(&self) -> Duration {
        if let Some(buffer) = self.buffer.as_ref().and_then(|b| b.lock().ok()) {
            let i = position_to_index(self.playback_pos, buffer.generic().channel_count());
            Duration::from_secs_f64((i / buffer.generic().sample_rate()) as f64)
        } else {
            Duration::from_secs(0)
        }
    }

    pub fn set_playback_time(&mut self, time: Duration) {
        if let Some(mut buffer) = self.buffer.as_mut().and_then(|b| b.lock().ok()) {
            if let SoundBuffer::Streaming(ref mut streaming) = *buffer {
                // Make sure decoder is at right position.
                streaming.time_seek(time);
            }
            // Set absolute position first.
            self.playback_pos = (time.as_secs_f64() * buffer.generic().channel_count() as f64)
                .min(buffer.generic().index_of_last_sample() as f64);
            // Then adjust buffer read position.
            self.buf_read_pos =
                match *buffer {
                    SoundBuffer::Streaming(ref mut streaming) => {
                        // Make sure to load correct data into buffer from decoder.
                        streaming.read_next_block();
                        // Streaming sources has different buffer read position because
                        // buffer contains only small portion of data.
                        self.playback_pos % streaming.generic.samples.len() as f64
                    }
                    SoundBuffer::Generic(_) => {
                        self.playback_pos
                    }
                };
            assert!(position_to_index(self.buf_read_pos, buffer.generic().channel_count()) < buffer.generic().samples().len());
        }
    }

    pub(in crate) fn next_sample_pair(&mut self, buffer: &mut SoundBuffer) -> (f32, f32) {
        let step = self.pitch * self.resampling_multiplier;

        self.buf_read_pos += step;
        self.playback_pos += step;

        let channel_count = buffer.generic().channel_count();
        let mut i = position_to_index(self.buf_read_pos, channel_count);

        let len = buffer.generic().samples().len();
        if i > buffer.generic().index_of_last_sample() {
            let mut end_reached = true;
            if let SoundBuffer::Streaming(streaming) = buffer {
                // Means that this is the last available block.
                if len != channel_count * StreamingBuffer::STREAM_SAMPLE_COUNT {
                    let _ = streaming.rewind();
                } else {
                    end_reached = false;
                }
                streaming.read_next_block();
            }
            if end_reached {
                if !self.looping {
                    self.status = Status::Stopped;
                }
                self.playback_pos = 0.0;
            }
            self.buf_read_pos = 0.0;
            i = 0;
        }

        let samples = buffer.generic().samples();
        if channel_count == 2 {
            let left = samples[i];
            let right = samples[i + 1];
            (left, right)
        } else {
            let sample = samples[i];
            (sample, sample)
        }
    }
}

impl Drop for GenericSource {
    fn drop(&mut self) {
        if let Some(mut buffer) = self.buffer.as_ref().and_then(|b| b.lock().ok()) {
            if let SoundBuffer::Streaming(ref mut streaming) = *buffer {
                streaming.use_count -= 1;
            }
        }
    }
}

impl Visit for GenericSource {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.buffer.visit("Buffer", visitor)?;
        self.buf_read_pos.visit("BufReadPos", visitor)?;
        self.playback_pos.visit("PlaybackPos", visitor)?;
        self.panning.visit("Pan", visitor)?;
        self.pitch.visit("Pitch", visitor)?;
        self.gain.visit("Gain", visitor)?;
        self.looping.visit("Looping", visitor)?;
        self.resampling_multiplier.visit("ResamplingMultiplier", visitor)?;
        self.status.visit("Status", visitor)?;
        self.play_once.visit("PlayOnce", visitor)?;

        visitor.leave_region()
    }
}

pub struct GenericSourceBuilder {
    buffer: Arc<Mutex<SoundBuffer>>,
    gain: f32,
    pitch: f32,
    panning: f32,
    looping: bool,
    status: Status,
    play_once: bool,
}

impl GenericSourceBuilder {
    pub fn new(buffer: Arc<Mutex<SoundBuffer>>) -> Self {
        Self {
            buffer,
            gain: 1.0,
            pitch: 1.0,
            panning: 0.0,
            looping: false,
            status: Status::Stopped,
            play_once: false,
        }
    }

    pub fn with_gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }

    pub fn with_pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch;
        self
    }

    pub fn with_panning(mut self, panning: f32) -> Self {
        self.panning = panning;
        self
    }

    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    pub fn with_play_once(mut self, play_once: bool) -> Self {
        self.play_once = play_once;
        self
    }

    pub fn build(self) -> Result<GenericSource, SoundError> {
        let device_sample_rate = f64::from(crate::device::SAMPLE_RATE);
        let mut locked_buffer = self.buffer.lock()?;
        if let SoundBuffer::Streaming(ref mut streaming) = *locked_buffer {
            if streaming.use_count != 0 {
                return Err(SoundError::StreamingBufferAlreadyInUse);
            }
            streaming.use_count += 1;
        }
        let sample_rate = locked_buffer.generic().sample_rate() as f64;
        let channel_count = locked_buffer.generic().channel_count() as f64;
        let resampling_multiplier = sample_rate / device_sample_rate * channel_count;
        Ok(GenericSource {
            resampling_multiplier,
            buffer: Some(self.buffer.clone()),
            gain: self.gain,
            pitch: self.pitch as f64,
            play_once: self.play_once,
            panning: self.panning,
            status: self.status,
            looping: self.looping,
            ..Default::default()
        })
    }

    pub fn build_source(self) -> Result<SoundSource, SoundError> {
        Ok(SoundSource::Generic(self.build()?))
    }
}