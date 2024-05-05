//! Generic sound source.
//!
//! # Overview
//!
//! Sound source is responsible for sound playback.
//!
//! # Usage
//!
//! Generic sound source can be constructed using GenericSourceBuilder like this:
//!
//! ```no_run
//! use std::sync::{Arc, Mutex};
//! use fyrox_sound::buffer::SoundBufferResource;
//! use fyrox_sound::pool::Handle;
//! use fyrox_sound::source::{SoundSource, Status};
//! use fyrox_sound::source::SoundSourceBuilder;
//! use fyrox_sound::context::SoundContext;
//!
//! fn make_source(context: &mut SoundContext, buffer: SoundBufferResource) -> Handle<SoundSource> {
//!     let source = SoundSourceBuilder::new()
//!        .with_buffer(buffer)
//!        .with_status(Status::Playing)
//!        .build()
//!        .unwrap();
//!     context.state().add_source(source)
//! }
//! ```

#![allow(clippy::float_cmp)]

use crate::{
    buffer::{streaming::StreamingBuffer, SoundBuffer, SoundBufferResource},
    bus::AudioBusGraph,
    context::DistanceModel,
    error::SoundError,
    listener::Listener,
};
use fyrox_core::{
    algebra::Vector3,
    reflect::prelude::*,
    uuid_provider,
    visitor::{Visit, VisitResult, Visitor},
};
use std::time::Duration;

/// Status (state) of sound source.
#[derive(Eq, PartialEq, Copy, Clone, Debug, Reflect, Visit)]
#[repr(u32)]
pub enum Status {
    /// Sound is stopped - it won't produces any sample and won't load mixer. This is default
    /// state of all sound sources.
    Stopped = 0,

    /// Sound is playing.
    Playing = 1,

    /// Sound is paused, it can stay in this state any amount if time. Playback can be continued by
    /// setting `Playing` status.
    Paused = 2,
}

uuid_provider!(Status = "1980bded-86cd-4eff-a5db-bab729bdb3ad");

/// See module info.
#[derive(Debug, Clone, Reflect, Visit)]
pub struct SoundSource {
    name: String,
    #[reflect(hidden)]
    buffer: Option<SoundBufferResource>,
    // Read position in the buffer in samples. Differs from `playback_pos` if buffer is streaming.
    // In case of streaming buffer its maximum value will be some fixed value which is
    // implementation defined. It can be less than zero, this happens when we are in the process
    // of reading next block in streaming buffer (see also prev_buffer_sample).
    #[reflect(hidden)]
    buf_read_pos: f64,
    // Real playback position in samples.
    #[reflect(hidden)]
    playback_pos: f64,
    #[reflect(min_value = 0.0, step = 0.05)]
    panning: f32,
    #[reflect(min_value = 0.0, step = 0.05)]
    pitch: f64,
    #[reflect(min_value = 0.0, step = 0.05)]
    gain: f32,
    looping: bool,
    #[reflect(min_value = 0.0, max_value = 1.0, step = 0.05)]
    spatial_blend: f32,
    // Important coefficient for runtime resampling. It is used to modify playback speed
    // of a source in order to match output device sampling rate. PCM data can be stored
    // in various sampling rates (22050 Hz, 44100 Hz, 88200 Hz, etc.) but output device
    // is running at fixed sampling rate (usually 44100 Hz). For example if we we'll feed
    // data to device with rate of 22050 Hz but device is running at 44100 Hz then we'll
    // hear that sound will have high pitch (2.0), to fix that we'll just pre-multiply
    // playback speed by 0.5.
    // However such auto-resampling has poor quality, but it is fast.
    #[reflect(read_only)]
    resampling_multiplier: f64,
    status: Status,
    #[visit(optional)]
    pub(crate) bus: String,
    play_once: bool,
    // Here we use Option because when source is just created it has no info about it
    // previous left and right channel gains. We can't set it to 1.0 for example
    // because it would give incorrect results: a sound would just start as loud as it
    // can be with no respect to real distance attenuation (or what else affects channel
    // gain). So if these are None engine will set correct values first and only then it
    // will start interpolation of gain.
    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) last_left_gain: Option<f32>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) last_right_gain: Option<f32>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) frame_samples: Vec<(f32, f32)>,
    // This sample is used when doing linear interpolation between two blocks of streaming buffer.
    #[reflect(hidden)]
    #[visit(skip)]
    prev_buffer_sample: (f32, f32),
    #[reflect(min_value = 0.0, step = 0.05)]
    radius: f32,
    position: Vector3<f32>,
    #[reflect(min_value = 0.0, step = 0.05)]
    max_distance: f32,
    #[reflect(min_value = 0.0, step = 0.05)]
    rolloff_factor: f32,
    // Some data that needed for iterative overlap-save convolution.
    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) prev_left_samples: Vec<f32>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) prev_right_samples: Vec<f32>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) prev_sampling_vector: Vector3<f32>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) prev_distance_gain: Option<f32>,
}

impl Default for SoundSource {
    fn default() -> Self {
        Self {
            name: Default::default(),
            buffer: None,
            buf_read_pos: 0.0,
            playback_pos: 0.0,
            panning: 0.0,
            pitch: 1.0,
            gain: 1.0,
            spatial_blend: 1.0,
            looping: false,
            resampling_multiplier: 1.0,
            status: Status::Stopped,
            bus: "Master".to_string(),
            play_once: false,
            last_left_gain: None,
            last_right_gain: None,
            frame_samples: Default::default(),
            prev_buffer_sample: (0.0, 0.0),
            radius: 1.0,
            position: Vector3::new(0.0, 0.0, 0.0),
            max_distance: f32::MAX,
            rolloff_factor: 1.0,
            prev_left_samples: Default::default(),
            prev_right_samples: Default::default(),
            prev_sampling_vector: Vector3::new(0.0, 0.0, 1.0),
            prev_distance_gain: None,
        }
    }
}

impl SoundSource {
    /// Sets new name of the sound source.
    pub fn set_name<N: AsRef<str>>(&mut self, name: N) {
        name.as_ref().clone_into(&mut self.name);
    }

    /// Returns the name of the sound source.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the name of the sound source.
    pub fn name_owned(&self) -> String {
        self.name.to_owned()
    }

    /// Sets spatial blend factor. It defines how much the source will be 2D and 3D sound at the same
    /// time. Set it to 0.0 to make the sound fully 2D and 1.0 to make it fully 3D. Middle values
    /// will make sound proportionally 2D and 3D at the same time.
    pub fn set_spatial_blend(&mut self, k: f32) {
        self.spatial_blend = k.clamp(0.0, 1.0);
    }

    /// Returns spatial blend factor.
    pub fn spatial_blend(&self) -> f32 {
        self.spatial_blend
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
            if let Some(SoundBuffer::Streaming(streaming)) = buffer.state().data() {
                streaming.use_count = streaming.use_count.saturating_sub(1);
            }
        }

        if let Some(buffer) = buffer.clone() {
            match buffer.state().data() {
                None => return Err(SoundError::BufferFailedToLoad),
                Some(locked_buffer) => {
                    // Check new buffer if streaming - it must not be used by anyone else.
                    if let SoundBuffer::Streaming(ref mut streaming) = *locked_buffer {
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
        self.panning = panning.clamp(-1.0, 1.0);
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
            if let Some(SoundBuffer::Streaming(streaming)) = buffer.state().data() {
                streaming.rewind()?;
            }
        }

        Ok(())
    }
    /// Sets position of source in world space.
    pub fn set_position(&mut self, position: Vector3<f32>) -> &mut Self {
        self.position = position;
        self
    }

    /// Returns positions of source.
    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    /// Sets radius of imaginable sphere around source in which no distance attenuation is applied.
    pub fn set_radius(&mut self, radius: f32) -> &mut Self {
        self.radius = radius;
        self
    }

    /// Returns radius of source.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets rolloff factor. Rolloff factor is used in distance attenuation and has different meaning
    /// in various distance models. It is applicable only for InverseDistance and ExponentDistance
    /// distance models. See DistanceModel docs for formulae.
    pub fn set_rolloff_factor(&mut self, rolloff_factor: f32) -> &mut Self {
        self.rolloff_factor = rolloff_factor;
        self
    }

    /// Returns rolloff factor.
    pub fn rolloff_factor(&self) -> f32 {
        self.rolloff_factor
    }

    /// Sets maximum distance until which distance gain will be applicable. Basically it doing this
    /// min(max(distance, radius), max_distance) which clamps distance in radius..max_distance range.
    /// From listener's perspective this will sound like source has stopped decreasing its volume even
    /// if distance continue to grow.
    pub fn set_max_distance(&mut self, max_distance: f32) -> &mut Self {
        self.max_distance = max_distance;
        self
    }

    /// Returns max distance.
    pub fn max_distance(&self) -> f32 {
        self.max_distance
    }

    /// Sets new name of the target audio bus. The name must be valid, otherwise the sound won't play!
    /// Default is [`AudioBusGraph::PRIMARY_BUS`].
    pub fn set_bus<S: AsRef<str>>(&mut self, bus: S) {
        bus.as_ref().clone_into(&mut self.bus);
    }

    /// Return the name of the target audio bus.
    pub fn bus(&self) -> &str {
        &self.bus
    }

    // Distance models were taken from OpenAL Specification because it looks like they're
    // standard in industry and there is no need to reinvent it.
    // https://www.openal.org/documentation/openal-1.1-specification.pdf
    pub(crate) fn calculate_distance_gain(
        &self,
        listener: &Listener,
        distance_model: DistanceModel,
    ) -> f32 {
        let distance = self
            .position
            .metric_distance(&listener.position())
            .clamp(self.radius, self.max_distance);
        match distance_model {
            DistanceModel::None => 1.0,
            DistanceModel::InverseDistance => {
                self.radius / (self.radius + self.rolloff_factor * (distance - self.radius))
            }
            DistanceModel::LinearDistance => {
                1.0 - self.radius * (distance - self.radius) / (self.max_distance - self.radius)
            }
            DistanceModel::ExponentDistance => (distance / self.radius).powf(-self.rolloff_factor),
        }
    }

    pub(crate) fn calculate_panning(&self, listener: &Listener) -> f32 {
        (listener.position() - self.position)
            .try_normalize(f32::EPSILON)
            // Fallback to look axis will give zero panning which will result in even
            // gain in each channels (as if there was no panning at all).
            .unwrap_or_else(|| listener.look_axis())
            .dot(&listener.ear_axis())
    }

    pub(crate) fn calculate_sampling_vector(&self, listener: &Listener) -> Vector3<f32> {
        let to_self = listener.position() - self.position;

        (listener.basis() * to_self)
            .try_normalize(f32::EPSILON)
            // This is ok to fallback to (0, 0, 1) vector because it's given
            // in listener coordinate system.
            .unwrap_or_else(|| Vector3::new(0.0, 0.0, 1.0))
    }

    /// Returns playback duration.
    pub fn playback_time(&self) -> Duration {
        if let Some(buffer) = self.buffer.as_ref() {
            if let Some(buffer) = buffer.state().data() {
                return Duration::from_secs_f64(self.playback_pos / (buffer.sample_rate() as f64));
            }
        }

        Duration::from_secs(0)
    }

    /// Sets playback duration.
    pub fn set_playback_time(&mut self, time: Duration) {
        if let Some(buffer) = self.buffer.as_ref() {
            if let Some(buffer) = buffer.state().data() {
                if let SoundBuffer::Streaming(ref mut streaming) = *buffer {
                    // Make sure decoder is at right position.
                    streaming.time_seek(time.clamp(Duration::from_secs(0), streaming.duration()));
                }
                // Set absolute position first.
                self.playback_pos = (time.as_secs_f64() * buffer.sample_rate as f64)
                    .clamp(0.0, buffer.duration().as_secs_f64());
                // Then adjust buffer read position.
                self.buf_read_pos = match *buffer {
                    SoundBuffer::Streaming(ref mut streaming) => {
                        // Make sure to load correct data into buffer from decoder.
                        streaming.read_next_block();
                        // Streaming sources has different buffer read position because
                        // buffer contains only small portion of data.
                        self.playback_pos % (StreamingBuffer::STREAM_SAMPLE_COUNT as f64)
                    }
                    SoundBuffer::Generic(_) => self.playback_pos,
                };
                assert!(
                    self.buf_read_pos * (buffer.channel_count() as f64)
                        < buffer.samples().len() as f64
                );
            }
        }
    }

    pub(crate) fn render(&mut self, amount: usize) {
        if self.frame_samples.capacity() < amount {
            self.frame_samples = Vec::with_capacity(amount);
        }

        self.frame_samples.clear();

        if let Some(buffer) = self.buffer.clone() {
            let mut state = buffer.state();
            if let Some(buffer) = state.data() {
                if self.status == Status::Playing && !buffer.is_empty() {
                    self.render_playing(buffer, amount);
                }
            }
        }
        // Fill the remaining part of frame_samples.
        self.frame_samples.resize(amount, (0.0, 0.0));
    }

    fn render_playing(&mut self, buffer: &mut SoundBuffer, amount: usize) {
        let mut count = 0;
        loop {
            count += self.render_until_block_end(buffer, amount - count);
            if count == amount {
                break;
            }

            let channel_count = buffer.channel_count();
            let len = buffer.samples().len();
            let mut end_reached = true;
            if let SoundBuffer::Streaming(streaming) = buffer {
                // Means that this is the last available block.
                if len != channel_count * StreamingBuffer::STREAM_SAMPLE_COUNT {
                    let _ = streaming.rewind();
                } else {
                    end_reached = false;
                }
                self.prev_buffer_sample = get_last_sample(streaming);
                streaming.read_next_block();
            }
            if end_reached {
                self.buf_read_pos = 0.0;
                self.playback_pos = 0.0;
                if !self.looping {
                    self.status = Status::Stopped;
                    return;
                }
            } else {
                self.buf_read_pos -= len as f64 / channel_count as f64;
            }
        }
    }

    // Renders until the end of the block or until amount samples is written and returns
    // the number of written samples.
    fn render_until_block_end(&mut self, buffer: &mut SoundBuffer, mut amount: usize) -> usize {
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
                    self.frame_samples
                        .push((buffer.samples[i * 2], buffer.samples[i * 2 + 1]))
                }
            } else {
                for i in from..from + rendered {
                    self.frame_samples
                        .push((buffer.samples[i], buffer.samples[i]))
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
        buffer: &mut SoundBuffer,
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
                let l = buffer.samples[idx * 2] * (1.0 - w) + buffer.samples[idx * 2 + 2] * w;
                let r = buffer.samples[idx * 2 + 1] * (1.0 - w) + buffer.samples[idx * 2 + 3] * w;
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

    pub(crate) fn frame_samples(&self) -> &[(f32, f32)] {
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

impl Drop for SoundSource {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.as_ref() {
            if let Some(SoundBuffer::Streaming(streaming)) = buffer.state().data() {
                streaming.use_count = streaming.use_count.saturating_sub(1);
            }
        }
    }
}

/// Allows you to construct generic sound source with desired state.
///
/// # Usage
///
/// ```no_run
/// use std::sync::{Arc, Mutex};
/// use fyrox_sound::buffer::SoundBufferResource;
/// use fyrox_sound::source::{SoundSourceBuilder};
/// use fyrox_sound::source::{Status, SoundSource};
///
/// fn make_sound_source(buffer: SoundBufferResource) -> SoundSource {
///     SoundSourceBuilder::new()
///         .with_buffer(buffer)
///         .with_status(Status::Playing)
///         .with_gain(0.5)
///         .with_looping(true)
///         .with_pitch(1.25)
///         .build()
///         .unwrap()
/// }
/// ```
pub struct SoundSourceBuilder {
    buffer: Option<SoundBufferResource>,
    gain: f32,
    pitch: f64,
    name: String,
    panning: f32,
    looping: bool,
    status: Status,
    play_once: bool,
    playback_time: Duration,
    radius: f32,
    position: Vector3<f32>,
    max_distance: f32,
    rolloff_factor: f32,
    spatial_blend: f32,
    bus: String,
}

impl Default for SoundSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundSourceBuilder {
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
            playback_time: Default::default(),
            radius: 1.0,
            position: Vector3::new(0.0, 0.0, 0.0),
            max_distance: f32::MAX,
            rolloff_factor: 1.0,
            spatial_blend: 1.0,
            bus: AudioBusGraph::PRIMARY_BUS.to_string(),
        }
    }

    /// Sets desired sound buffer to play.
    pub fn with_buffer(mut self, buffer: SoundBufferResource) -> Self {
        self.buffer = Some(buffer);
        self
    }

    /// Sets desired sound buffer to play.
    pub fn with_opt_buffer(mut self, buffer: Option<SoundBufferResource>) -> Self {
        self.buffer = buffer;
        self
    }

    /// See [`SoundSource::set_gain`]
    pub fn with_gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }

    /// See [`SoundSource::set_spatial_blend`]
    pub fn with_spatial_blend_factor(mut self, k: f32) -> Self {
        self.spatial_blend = k.clamp(0.0, 1.0);
        self
    }

    /// See [`SoundSource::set_pitch`]
    pub fn with_pitch(mut self, pitch: f64) -> Self {
        self.pitch = pitch;
        self
    }

    /// See [`SoundSource::set_panning`]
    pub fn with_panning(mut self, panning: f32) -> Self {
        self.panning = panning;
        self
    }

    /// See [`SoundSource::set_looping`]
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Sets desired status of source.
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// See `set_play_once` of SoundSource
    pub fn with_play_once(mut self, play_once: bool) -> Self {
        self.play_once = play_once;
        self
    }

    /// Sets desired name of the source.
    pub fn with_name<N: AsRef<str>>(mut self, name: N) -> Self {
        name.as_ref().clone_into(&mut self.name);
        self
    }

    /// Sets desired starting playback time.
    pub fn with_playback_time(mut self, time: Duration) -> Self {
        self.playback_time = time;
        self
    }

    /// See `set_position` of SpatialSource.
    pub fn with_position(mut self, position: Vector3<f32>) -> Self {
        self.position = position;
        self
    }

    /// See `set_radius` of SpatialSource.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// See `set_max_distance` of SpatialSource.
    pub fn with_max_distance(mut self, max_distance: f32) -> Self {
        self.max_distance = max_distance;
        self
    }

    /// See `set_rolloff_factor` of SpatialSource.
    pub fn with_rolloff_factor(mut self, rolloff_factor: f32) -> Self {
        self.rolloff_factor = rolloff_factor;
        self
    }

    /// Sets desired output bus for the sound source.
    pub fn with_bus<S: AsRef<str>>(mut self, bus: S) -> Self {
        self.bus = bus.as_ref().to_string();
        self
    }

    /// Creates new instance of generic sound source. May fail if buffer is invalid.
    pub fn build(self) -> Result<SoundSource, SoundError> {
        let mut source = SoundSource {
            buffer: self.buffer.clone(),
            gain: self.gain,
            pitch: self.pitch,
            play_once: self.play_once,
            panning: self.panning,
            status: self.status,
            looping: self.looping,
            name: self.name,
            frame_samples: Default::default(),
            radius: self.radius,
            position: self.position,
            max_distance: self.max_distance,
            rolloff_factor: self.rolloff_factor,
            spatial_blend: self.spatial_blend,
            prev_left_samples: Default::default(),
            prev_right_samples: Default::default(),
            bus: self.bus,
            ..Default::default()
        };

        source.set_buffer(self.buffer)?;
        source.set_playback_time(self.playback_time);

        Ok(source)
    }
}
