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
//! use rg3d_sound::buffer::SoundBufferResource;
//! use rg3d_sound::pool::Handle;
//! use rg3d_sound::source::{SoundSource, Status};
//! use rg3d_sound::source::generic::GenericSourceBuilder;
//! use rg3d_sound::context::SoundContext;
//!
//! fn make_source(context: &mut SoundContext, buffer: SoundBufferResource) -> Handle<SoundSource> {
//!     let source = GenericSourceBuilder::new()
//!        .with_buffer(buffer)
//!        .with_status(Status::Playing)
//!        .build_source()
//!        .unwrap();
//!     context.state().add_source(source)
//! }
//!
//! ```

use crate::{
    buffer::{streaming::StreamingBuffer, SoundBufferResource, SoundBufferState},
    error::SoundError,
    source::{SoundSource, Status},
};
use rg3d_core::{
    inspect::{Inspect, PropertyInfo},
    visitor::{Visit, VisitResult, Visitor},
};
use rg3d_resource::ResourceState;
use std::time::Duration;

/// See module info.
#[derive(Debug, Clone, Inspect)]
pub struct GenericSource {
    name: String,
    buffer: Option<SoundBufferResource>,
    // Read position in the buffer in samples. Differs from `playback_pos` if buffer is streaming.
    // In case of streaming buffer its maximum value will be some fixed value which is
    // implementation defined. It can be less than zero, this happens when we are in the process
    // of reading next block in streaming buffer (see also prev_buffer_sample).
    #[inspect(skip)]
    buf_read_pos: f64,
    // Real playback position in samples.
    #[inspect(skip)]
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
    #[inspect(read_only)]
    resampling_multiplier: f64,
    status: Status,
    play_once: bool,
    // Here we use Option because when source is just created it has no info about it
    // previous left and right channel gains. We can't set it to 1.0 for example
    // because it would give incorrect results: a sound would just start as loud as it
    // can be with no respect to real distance attenuation (or what else affects channel
    // gain). So if these are None engine will set correct values first and only then it
    // will start interpolation of gain.
    #[inspect(skip)]
    pub(in crate) last_left_gain: Option<f32>,
    #[inspect(skip)]
    pub(in crate) last_right_gain: Option<f32>,
    #[inspect(skip)]
    pub(in crate) frame_samples: Vec<(f32, f32)>,
    // This sample is used when doing linear interpolation between two blocks of streaming buffer.
    #[inspect(skip)]
    prev_buffer_sample: (f32, f32),
}

impl Default for GenericSource {
    fn default() -> Self {
        Self {
            name: Default::default(),
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
            prev_buffer_sample: (0.0, 0.0),
        }
    }
}

impl GenericSource {
    /// Sets new name of the sound source.
    pub fn set_name<N: AsRef<str>>(&mut self, name: N) {
        self.name = name.as_ref().to_owned();
    }

    /// Returns the name of the sound source.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the name of the sound source.
    pub fn name_owned(&self) -> String {
        self.name.to_owned()
    }

    /// Changes buffer of source. Returns old buffer. Source will continue playing from beginning, old
    /// position will be discarded.
    pub fn set_buffer(
        &mut self,
        buffer: Option<SoundBufferResource>,
    ) -> Result<Option<SoundBufferResource>, SoundError> {
        self.buf_read_pos = 0.0;
        self.playback_pos = 0.0;

        // If we already have streaming buffer assigned make sure to decrease use count
        // so it can be reused later on if needed.
        if let Some(buffer) = self.buffer.clone() {
            if let SoundBufferState::Streaming(ref mut streaming) = *buffer.data_ref() {
                streaming.use_count = streaming.use_count.saturating_sub(1);
            }
        }

        if let Some(buffer) = buffer.clone() {
            match *buffer.state() {
                ResourceState::LoadError { .. } => return Err(SoundError::BufferFailedToLoad),
                ResourceState::Ok(ref mut locked_buffer) => {
                    // Check new buffer if streaming - it must not be used by anyone else.
                    if let SoundBufferState::Streaming(ref mut streaming) = *locked_buffer {
                        if streaming.use_count != 0 {
                            return Err(SoundError::StreamingBufferAlreadyInUse);
                        }
                        streaming.use_count += 1;
                    }

                    // Make sure to recalculate resampling multiplier, otherwise sound will play incorrectly.
                    let device_sample_rate = f64::from(crate::context::SAMPLE_RATE);
                    let sample_rate = locked_buffer.sample_rate() as f64;
                    self.resampling_multiplier = sample_rate / device_sample_rate;
                }
                ResourceState::Pending { .. } => unreachable!(),
            }
        }

        Ok(std::mem::replace(&mut self.buffer, buffer))
    }

    /// Returns current buffer if any.
    pub fn buffer(&self) -> Option<SoundBufferResource> {
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

        if let Some(buffer) = self.buffer.as_ref() {
            let mut buffer = buffer.data_ref();
            if let SoundBufferState::Streaming(ref mut streaming) = *buffer {
                streaming.rewind()?;
            }
        }

        Ok(())
    }

    /// Returns playback duration.
    pub fn playback_time(&self) -> Duration {
        if let Some(buffer) = self.buffer.as_ref() {
            let buffer = buffer.data_ref();
            Duration::from_secs_f64(self.playback_pos / (buffer.sample_rate() as f64))
        } else {
            Duration::from_secs(0)
        }
    }

    /// Sets playback duration.
    pub fn set_playback_time(&mut self, time: Duration) {
        if let Some(buffer) = self.buffer.as_ref() {
            let mut buffer = buffer.data_ref();
            if let SoundBufferState::Streaming(ref mut streaming) = *buffer {
                // Make sure decoder is at right position.
                streaming.time_seek(time);
            }
            // Set absolute position first.
            self.playback_pos = time.as_secs_f64() * buffer.sample_rate as f64;
            // Then adjust buffer read position.
            self.buf_read_pos = match *buffer {
                SoundBufferState::Streaming(ref mut streaming) => {
                    // Make sure to load correct data into buffer from decoder.
                    streaming.read_next_block();
                    // Streaming sources has different buffer read position because
                    // buffer contains only small portion of data.
                    self.playback_pos % (StreamingBuffer::STREAM_SAMPLE_COUNT as f64)
                }
                SoundBufferState::Generic(_) => self.playback_pos,
            };
            assert!(
                self.buf_read_pos * (buffer.channel_count() as f64) < buffer.samples().len() as f64
            );
        }
    }

    pub(in crate) fn render(&mut self, amount: usize) {
        if self.frame_samples.capacity() < amount {
            self.frame_samples = Vec::with_capacity(amount);
        }

        self.frame_samples.clear();

        if let Some(buffer) = self.buffer.clone() {
            let mut state = buffer.state();
            if let ResourceState::Ok(ref mut buffer) = *state {
                if self.status == Status::Playing && !buffer.is_empty() {
                    self.render_playing(buffer, amount);
                }
            }
        }
        // Fill the remaining part of frame_samples.
        self.frame_samples.resize(amount, (0.0, 0.0));
    }

    fn render_playing(&mut self, buffer: &mut SoundBufferState, amount: usize) {
        let mut count = 0;
        loop {
            count += self.render_until_block_end(buffer, amount - count);
            if count == amount {
                break;
            }

            let channel_count = buffer.channel_count();
            let len = buffer.samples().len();
            let mut end_reached = true;
            if let SoundBufferState::Streaming(streaming) = buffer {
                // Means that this is the last available block.
                if len != channel_count * StreamingBuffer::STREAM_SAMPLE_COUNT {
                    let _ = streaming.rewind();
                } else {
                    end_reached = false;
                }
                self.prev_buffer_sample = get_last_sample(&streaming);
                streaming.read_next_block();
            }
            if end_reached {
                if !self.looping {
                    self.status = Status::Stopped;
                    return;
                }
                self.buf_read_pos = 0.0;
                self.playback_pos = 0.0;
            } else {
                self.buf_read_pos -= len as f64 / channel_count as f64;
            }
        }
    }

    // Renders until the end of the block or until amount samples is written and returns
    // the number of written samples.
    fn render_until_block_end(&mut self, buffer: &mut SoundBufferState, mut amount: usize) -> usize {
        let step = self.pitch * self.resampling_multiplier;
        if step == 1.0 {
            if self.buf_read_pos < 0.0 {
                // This can theoretically happen if we change pitch on the fly.
                self.frame_samples.push(self.prev_buffer_sample);
                self.buf_read_pos = 0.0;
                amount -= 1;
            }
            // Fast-path for common case when there is no resampling and no pitch change.
            let from = self.buf_read_pos as usize;
            let buffer_len = buffer.samples.len() / buffer.channel_count;
            let rendered = (buffer_len - from).min(amount);
            if buffer.channel_count == 2 {
                for i in from..from + rendered {
                    self.frame_samples.push((buffer.samples[i * 2], buffer.samples[i * 2 + 1]))
                }
            } else {
                for i in from..from + rendered {
                    self.frame_samples.push((buffer.samples[i], buffer.samples[i]))
                }
            }
            self.buf_read_pos += rendered as f64;
            self.playback_pos += rendered as f64;
            rendered
        } else {
            self.render_until_block_end_resample(buffer, amount, step)
        }
    }

    // Does linear resampling while rendering until the end of the block.
    fn render_until_block_end_resample(
        &mut self,
        buffer: &mut SoundBufferState,
        amount: usize,
        step: f64,
    ) -> usize {
        let mut rendered = 0;

        while self.buf_read_pos < 0.0 {
            // Interpolate between last sample of previous buffer and first sample of current
            // buffer. This is important, otherwise there will be quiet but audible pops
            // in the output.
            let w = (self.buf_read_pos - self.buf_read_pos.floor()) as f32;
            let cur_first_sample = if buffer.channel_count == 2 {
                (buffer.samples[0], buffer.samples[1])
            } else {
                (buffer.samples[0], buffer.samples[0])
            };
            let l = self.prev_buffer_sample.0 * (1.0 - w) + cur_first_sample.0 * w;
            let r = self.prev_buffer_sample.1 * (1.0 - w) + cur_first_sample.1 * w;
            self.frame_samples.push((l, r));
            self.buf_read_pos += step;
            self.playback_pos += step;
            rendered += 1;
        }

        // We want to keep global positions in f64, but use f32 in inner loops (this improves
        // code generation and performance at least on some systems), so we split the buf_read_pos
        // into integer and f32 part.
        let buffer_base_idx = self.buf_read_pos as usize;
        let mut buffer_rel_pos = (self.buf_read_pos - buffer_base_idx as f64) as f32;
        let start_buffer_rel_pos = buffer_rel_pos;
        let rel_step = step as f32;
        // We skip one last element because the hot loop resampling between current and next
        // element. Last elements are appended after the hot loop.
        let buffer_last = buffer.samples.len() / buffer.channel_count - 1;
        if buffer.channel_count == 2 {
            while rendered < amount {
                let (idx, w) = {
                    let idx = buffer_rel_pos as usize;
                    // This looks a bit complicated but fract() is quite a bit slower on x86,
                    // because it turns into a function call on targets < SSE4.1, unlike aarch64)
                    (idx + buffer_base_idx, buffer_rel_pos - idx as f32)
                };
                if idx >= buffer_last {
                    break;
                }
                let l = buffer.samples[idx * 2] * (1.0 - w)
                    + buffer.samples[idx * 2 + 2] * w;
                let r = buffer.samples[idx * 2 + 1] * (1.0 - w)
                    + buffer.samples[idx * 2 + 3] * w;
                self.frame_samples.push((l, r));
                buffer_rel_pos += rel_step;
                rendered += 1;
            }
        } else {
            while rendered < amount {
                let (idx, w) = {
                    let idx = buffer_rel_pos as usize;
                    // See comment above.
                    (idx + buffer_base_idx, buffer_rel_pos - idx as f32)
                };
                if idx >= buffer_last {
                    break;
                }
                let v = buffer.samples[idx] * (1.0 - w) + buffer.samples[idx + 1] * w;
                self.frame_samples.push((v, v));
                buffer_rel_pos += rel_step;
                rendered += 1;
            }
        }

        self.buf_read_pos += (buffer_rel_pos - start_buffer_rel_pos) as f64;
        self.playback_pos += (buffer_rel_pos - start_buffer_rel_pos) as f64;
        rendered
    }

    pub(in crate) fn frame_samples(&self) -> &[(f32, f32)] {
        &self.frame_samples
    }
}

fn get_last_sample(buffer: &StreamingBuffer) -> (f32, f32) {
    let len = buffer.samples.len();
    if len == 0 {
        return (0.0, 0.0);
    }
    if buffer.channel_count == 2 {
        (buffer.samples[len - 2], buffer.samples[len - 1])
    } else {
        (buffer.samples[len - 1], buffer.samples[len - 1])
    }
}

impl Drop for GenericSource {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.as_ref() {
            let mut buffer = buffer.data_ref();
            if let SoundBufferState::Streaming(ref mut streaming) = *buffer {
                streaming.use_count = streaming.use_count.saturating_sub(1);
            }
        }
    }
}

impl Visit for GenericSource {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let _ = self.name.visit("Name", visitor);
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
/// use rg3d_sound::buffer::SoundBufferResource;
/// use rg3d_sound::source::generic::{GenericSource, GenericSourceBuilder};
/// use rg3d_sound::source::{Status, SoundSource};
///
/// fn make_generic_source(buffer: SoundBufferResource) -> GenericSource {
///     GenericSourceBuilder::new()
///         .with_buffer(buffer)
///         .with_status(Status::Playing)
///         .with_gain(0.5)
///         .with_looping(true)
///         .with_pitch(1.25)
///         .build()
///         .unwrap()
/// }
///
/// fn make_source(buffer: SoundBufferResource) -> SoundSource {
///     GenericSourceBuilder::new()
///         .with_buffer(buffer)
///         .with_status(Status::Playing)
///         .with_gain(0.5)
///         .with_looping(true)
///         .with_pitch(1.25)
///         .build_source() // build_source creates SoundSource::Generic directly
///         .unwrap()
/// }
/// ```
pub struct GenericSourceBuilder {
    buffer: Option<SoundBufferResource>,
    gain: f32,
    pitch: f32,
    name: String,
    panning: f32,
    looping: bool,
    status: Status,
    play_once: bool,
}

impl Default for GenericSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericSourceBuilder {
    /// Creates new generic source builder with specified buffer.
    pub fn new() -> Self {
        Self {
            buffer: None,
            gain: 1.0,
            pitch: 1.0,
            name: Default::default(),
            panning: 0.0,
            looping: false,
            status: Status::Stopped,
            play_once: false,
        }
    }

    /// Sets desired sound buffer to play.
    pub fn with_buffer(mut self, buffer: SoundBufferResource) -> Self {
        self.buffer = Some(buffer);
        self
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

    /// Sets desired name of the source.
    pub fn with_name<N: AsRef<str>>(mut self, name: N) -> Self {
        self.name = name.as_ref().to_owned();
        self
    }

    /// Creates new instance of generic sound source. May fail if buffer is invalid.
    pub fn build(self) -> Result<GenericSource, SoundError> {
        let mut source = GenericSource {
            buffer: self.buffer.clone(),
            gain: self.gain,
            pitch: self.pitch as f64,
            play_once: self.play_once,
            panning: self.panning,
            status: self.status,
            looping: self.looping,
            name: self.name,
            frame_samples: Default::default(),
            ..Default::default()
        };

        source.set_buffer(self.buffer)?;

        Ok(source)
    }

    /// Creates new instance of sound source of `Generic` variant.
    pub fn build_source(self) -> Result<SoundSource, SoundError> {
        Ok(SoundSource::Generic(self.build()?))
    }
}
