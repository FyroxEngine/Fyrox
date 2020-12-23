//! Generic sound source.
//!
//! # Overview
//!
//! Generic sound source is base building block for each other types of sound sources. It holds state of buffer read
//! cursor, information about panning, pitch, looping, etc. It performs automatic resampling on the fly.
//!
//! # Usage
//!
//! Generic sound source can be constructed using GenericSourceBuilder like this:
//!
//! ```no_run
//! use std::sync::{Arc, Mutex};
//! use rg3d_sound::buffer::SoundBuffer;
//! use rg3d_sound::pool::Handle;
//! use rg3d_sound::source::{SoundSource, Status};
//! use rg3d_sound::source::generic::GenericSourceBuilder;
//! use rg3d_sound::context::Context;
//!
//! fn make_source(context: &mut Context, buffer: Arc<Mutex<SoundBuffer>>) -> Handle<SoundSource> {
//!     let source = GenericSourceBuilder::new(buffer)
//!        .with_status(Status::Playing)
//!        .build_source()
//!        .unwrap();
//!     context.add_source(source)
//! }
//!
//! ```

use crate::{
    buffer::{streaming::StreamingBuffer, SoundBuffer},
    error::SoundError,
    source::{SoundSource, Status},
};
use rg3d_core::visitor::{Visit, VisitResult, Visitor};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// See module info.
#[derive(Debug, Clone)]
pub struct GenericSource {
    buffer: Option<Arc<Mutex<SoundBuffer>>>,
    // Read position in the buffer. Differs from `playback_pos` if buffer is streaming.
    // In case of streaming buffer its maximum value will be some fixed value which is
    // implementation defined.
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
    // However such auto-resampling has poor quality, but it is fast.
    resampling_multiplier: f64,
    status: Status,
    play_once: bool,
    // Here we use Option because when source is just created it has no info about it
    // previous left and right channel gains. We can't set it to 1.0 for example
    // because it would give incorrect results: a sound would just start as loud as it
    // can be with no respect to real distance attenuation (or what else affects channel
    // gain). So if these are None engine will set correct values first and only then it
    // will start interpolation of gain.
    pub(in crate) last_left_gain: Option<f32>,
    pub(in crate) last_right_gain: Option<f32>,
    pub(in crate) frame_samples: Vec<(f32, f32)>,
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
            last_left_gain: None,
            last_right_gain: None,
            frame_samples: Default::default(),
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
    /// Changes buffer of source. Returns old buffer. Source will continue playing from beginning, old
    /// position will be discarded.
    pub fn set_buffer(
        &mut self,
        buffer: Arc<Mutex<SoundBuffer>>,
    ) -> Result<Option<Arc<Mutex<SoundBuffer>>>, SoundError> {
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

        Ok(self.buffer.replace(buffer))
    }

    /// Returns current buffer if any.
    pub fn buffer(&self) -> Option<Arc<Mutex<SoundBuffer>>> {
        self.buffer.clone()
    }

    /// Marks buffer for single play. It will be automatically destroyed when it will finish playing.
    ///
    /// # Notes
    ///
    /// Make sure you not using handles to "play once" sounds, attempt to get reference of "play once" sound
    /// may result in panic if source already deleted. Looping sources will never be automatically deleted
    /// because their playback never stops.
    pub fn set_play_once(&mut self, play_once: bool) {
        self.play_once = play_once;
    }

    /// Returns true if this source is marked for single play, false - otherwise.
    pub fn is_play_once(&self) -> bool {
        self.play_once
    }

    /// Sets new gain (volume) of sound. Value should be in 0..1 range, but it is not clamped
    /// and larger values can be used to "overdrive" sound.
    ///
    /// # Notes
    ///
    /// Physical volume has non-linear scale (logarithmic) so perception of sound at 0.25 gain
    /// will be different if logarithmic scale was used.
    pub fn set_gain(&mut self, gain: f32) -> &mut Self {
        self.gain = gain;
        self
    }

    /// Returns current gain (volume) of sound. Value is in 0..1 range.
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Sets panning coefficient. Value must be in -1..+1 range. Where -1 - only left channel will be audible,
    /// 0 - both, +1 - only right.
    pub fn set_panning(&mut self, panning: f32) -> &mut Self {
        self.panning = panning.max(-1.0).min(1.0);
        self
    }

    /// Returns current panning coefficient in -1..+1 range. For more info see `set_panning`. Default value is 0.
    pub fn panning(&self) -> f32 {
        self.panning
    }

    /// Returns status of sound source.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Changes status to `Playing`.
    pub fn play(&mut self) -> &mut Self {
        self.status = Status::Playing;
        self
    }

    /// Changes status to `Paused`
    pub fn pause(&mut self) -> &mut Self {
        self.status = Status::Paused;
        self
    }

    /// Enabled or disables sound looping. Looping sound will never stop by itself, but can be stopped or paused
    /// by calling `stop` or `pause` methods. Useful for music, ambient sounds, etc.
    pub fn set_looping(&mut self, looping: bool) -> &mut Self {
        self.looping = looping;
        self
    }

    /// Returns looping status.
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Sets sound pitch. Defines "tone" of sounds. Default value is 1.0
    pub fn set_pitch(&mut self, pitch: f64) -> &mut Self {
        self.pitch = pitch.abs();
        self
    }

    /// Returns pitch of sound source.
    pub fn pitch(&self) -> f64 {
        self.pitch
    }

    /// Stops sound source. Automatically rewinds streaming buffers.
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

    /// Returns playback duration.
    pub fn playback_time(&self) -> Duration {
        if let Some(buffer) = self.buffer.as_ref().and_then(|b| b.lock().ok()) {
            let i = position_to_index(self.playback_pos, buffer.channel_count());
            Duration::from_secs_f64((i / buffer.sample_rate()) as f64)
        } else {
            Duration::from_secs(0)
        }
    }

    /// Sets playback duration.
    pub fn set_playback_time(&mut self, time: Duration) {
        if let Some(mut buffer) = self.buffer.as_mut().and_then(|b| b.lock().ok()) {
            if let SoundBuffer::Streaming(ref mut streaming) = *buffer {
                // Make sure decoder is at right position.
                streaming.time_seek(time);
            }
            // Set absolute position first.
            self.playback_pos = (time.as_secs_f64() * buffer.channel_count() as f64)
                .min(buffer.index_of_last_sample() as f64);
            // Then adjust buffer read position.
            self.buf_read_pos = match *buffer {
                SoundBuffer::Streaming(ref mut streaming) => {
                    // Make sure to load correct data into buffer from decoder.
                    streaming.read_next_block();
                    // Streaming sources has different buffer read position because
                    // buffer contains only small portion of data.
                    self.playback_pos % streaming.generic.samples.len() as f64
                }
                SoundBuffer::Generic(_) => self.playback_pos,
            };
            assert!(
                position_to_index(self.buf_read_pos, buffer.channel_count())
                    < buffer.samples().len()
            );
        }
    }

    fn next_sample_pair(&mut self, buffer: &mut SoundBuffer) -> (f32, f32) {
        let step = self.pitch * self.resampling_multiplier;

        self.buf_read_pos += step;
        self.playback_pos += step;

        let channel_count = buffer.channel_count();
        let mut i = position_to_index(self.buf_read_pos, channel_count);

        let len = buffer.samples().len();
        if i > buffer.index_of_last_sample() {
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

        let samples = buffer.samples();
        if channel_count == 2 {
            let left = samples[i];
            let right = samples[i + 1];
            (left, right)
        } else {
            let sample = samples[i];
            (sample, sample)
        }
    }

    pub(in crate) fn render(&mut self, amount: usize) {
        if self.frame_samples.capacity() < amount {
            self.frame_samples = Vec::with_capacity(amount);
        }

        self.frame_samples.clear();

        if let Some(mut buffer) = self.buffer.clone().as_ref().and_then(|b| {
            b.lock()
                .ok()
                .and_then(|b| if b.is_empty() { None } else { Some(b) })
        }) {
            for _ in 0..amount {
                if self.status == Status::Playing {
                    let pair = self.next_sample_pair(&mut buffer);
                    self.frame_samples.push(pair);
                } else {
                    self.frame_samples.push((0.0, 0.0));
                }
            }
        } else {
            for _ in 0..amount {
                self.frame_samples.push((0.0, 0.0));
            }
        }
    }

    pub(in crate) fn frame_samples(&self) -> &[(f32, f32)] {
        &self.frame_samples
    }
}

impl Drop for GenericSource {
    fn drop(&mut self) {
        if let Some(mut buffer) = self.buffer.as_ref().and_then(|b| b.lock().ok()) {
            if let SoundBuffer::Streaming(ref mut streaming) = *buffer {
                streaming.use_count = streaming.use_count.saturating_sub(1);
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
        self.resampling_multiplier
            .visit("ResamplingMultiplier", visitor)?;
        self.status.visit("Status", visitor)?;
        self.play_once.visit("PlayOnce", visitor)?;

        visitor.leave_region()
    }
}

/// Allows you to construct generic sound source with desired state.
///
/// # Usage
///
/// ```no_run
/// use std::sync::{Arc, Mutex};
/// use rg3d_sound::buffer::SoundBuffer;
/// use rg3d_sound::source::generic::{GenericSource, GenericSourceBuilder};
/// use rg3d_sound::source::{Status, SoundSource};
///
/// fn make_generic_source(buffer: Arc<Mutex<SoundBuffer>>) -> GenericSource {
///     GenericSourceBuilder::new(buffer)
///         .with_status(Status::Playing)
///         .with_gain(0.5)
///         .with_looping(true)
///         .with_pitch(1.25)
///         .build()
///         .unwrap()
/// }
///
/// fn make_source(buffer: Arc<Mutex<SoundBuffer>>) -> SoundSource {
///     GenericSourceBuilder::new(buffer)
///         .with_status(Status::Playing)
///         .with_gain(0.5)
///         .with_looping(true)
///         .with_pitch(1.25)
///         .build_source() // build_source creates SoundSource::Generic directly
///         .unwrap()
/// }
/// ```
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
    /// Creates new generic source builder with specified buffer.
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

    /// See `set_gain` of GenericSource
    pub fn with_gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }

    /// See `set_pitch` of GenericSource
    pub fn with_pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch;
        self
    }

    /// See `set_panning` of GenericSource
    pub fn with_panning(mut self, panning: f32) -> Self {
        self.panning = panning;
        self
    }

    /// See `set_looping` of GenericSource
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Sets desired status of source.
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// See `set_play_once` of GenericSource
    pub fn with_play_once(mut self, play_once: bool) -> Self {
        self.play_once = play_once;
        self
    }

    /// Creates new instance of generic sound source. May fail if buffer is invalid.
    pub fn build(self) -> Result<GenericSource, SoundError> {
        let device_sample_rate = f64::from(crate::context::SAMPLE_RATE);
        let mut locked_buffer = self.buffer.lock()?;
        if let SoundBuffer::Streaming(ref mut streaming) = *locked_buffer {
            if streaming.use_count != 0 {
                return Err(SoundError::StreamingBufferAlreadyInUse);
            }
            streaming.use_count += 1;
        }
        let sample_rate = locked_buffer.sample_rate() as f64;
        let channel_count = locked_buffer.channel_count() as f64;
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
            frame_samples: Default::default(),
            ..Default::default()
        })
    }

    /// Creates new instance of sound source of `Generic` variant.
    pub fn build_source(self) -> Result<SoundSource, SoundError> {
        Ok(SoundSource::Generic(self.build()?))
    }
}
