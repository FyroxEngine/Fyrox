use std::{
    sync::{
        Arc,
        Mutex,
    },
    time::Duration,
};
use crate::{
    buffer::{Buffer, BufferKind},
    error::SoundError,
    listener::Listener,
};
use rg3d_core::{
    math::vec3::Vec3,
    visitor::{Visit, VisitResult, Visitor, VisitError},
};
use rustfft::num_complex::Complex;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Status {
    Stopped,
    Playing,
    Paused,
}

impl Visit for Status {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut kind: u8 = match self {
            Status::Stopped => 0,
            Status::Playing => 1,
            Status::Paused => 2,
        };

        kind.visit(name, visitor)?;

        if visitor.is_reading() {
            *self = match kind {
                0 => Status::Stopped,
                1 => Status::Playing,
                2 => Status::Paused,
                _ => return Err(VisitError::User("invalid status".to_string()))
            }
        }

        Ok(())
    }
}

pub struct SpatialSource {
    /// Radius of sphere around sound source at which sound volume is half of initial.
    radius: f32,
    position: Vec3,
}

impl SpatialSource {
    pub fn set_position(&mut self, position: &Vec3) {
        self.position = *position;
    }

    pub fn get_position(&self) -> Vec3 {
        self.position
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    pub fn get_radius(&self) -> f32 {
        self.radius
    }
}

impl Visit for SpatialSource {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.position.visit("Position", visitor)?;

        visitor.leave_region()
    }
}

impl Default for SpatialSource {
    fn default() -> Self {
        Self {
            radius: 10.0,
            position: Vec3::ZERO,
        }
    }
}

pub enum SourceKind {
    Flat,
    Spatial(SpatialSource),
}

impl Visit for SourceKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind: u8 = match self {
            SourceKind::Flat => 0,
            SourceKind::Spatial(_) => 1,
        };

        kind.visit("Id", visitor)?;

        if visitor.is_reading() {
            *self = match kind {
                0 => SourceKind::Flat,
                1 => SourceKind::Spatial(SpatialSource::default()),
                _ => return Err(VisitError::User("invalid source kind".to_string()))
            }
        }

        match self {
            SourceKind::Flat => (),
            SourceKind::Spatial(spatial) => spatial.visit("Content", visitor)?,
        }

        visitor.leave_region()
    }
}

pub struct Source {
    kind: SourceKind,
    buffer: Option<Arc<Mutex<Buffer>>>,
    // Read position in the buffer. Differs from `playback_pos` if buffer is streaming.
    // In case of streaming buffer its maximum value will be size o
    buf_read_pos: f64,
    // Real playback position.
    playback_pos: f64,
    pan: f32,
    pitch: f64,
    gain: f32,
    looping: bool,
    pub(in crate) left_gain: f32,
    pub(in crate) right_gain: f32,
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
    pub(in crate) hrtf_sampling_vector: Vec3,
    // Rest of samples from previous frame that has to be added to output signal.
    pub(in crate) last_frame_left_samples: Vec<Complex<f32>>,
    pub(in crate) last_frame_right_samples: Vec<Complex<f32>>,
    pub(in crate) distance_gain: f32,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            kind: SourceKind::Flat,
            buffer: None,
            buf_read_pos: 0.0,
            playback_pos: 0.0,
            pan: 0.0,
            pitch: 1.0,
            gain: 1.0,
            looping: false,
            left_gain: 1.0,
            right_gain: 1.0,
            resampling_multiplier: 1.0,
            status: Status::Stopped,
            play_once: false,
            hrtf_sampling_vector: Default::default(),
            last_frame_left_samples: Default::default(),
            last_frame_right_samples: Default::default(),
            distance_gain: 1.0,
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

impl Source {
    pub fn new_spatial(buffer: Arc<Mutex<Buffer>>) -> Result<Self, SoundError> {
        Self::new(SourceKind::Spatial(SpatialSource::default()), buffer)
    }

    pub fn new_flat(buffer: Arc<Mutex<Buffer>>) -> Result<Self, SoundError> {
        Self::new(SourceKind::Flat, buffer)
    }

    pub fn new(kind: SourceKind, buffer: Arc<Mutex<Buffer>>) -> Result<Self, SoundError> {
        let device_sample_rate = f64::from(crate::device::SAMPLE_RATE);
        let mut locked_buffer = buffer.lock()?;
        if locked_buffer.get_kind() == BufferKind::Stream && locked_buffer.use_count != 0 {
            return Err(SoundError::StreamingBufferAlreadyInUse);
        }
        let buffer_sample_rate = locked_buffer.get_sample_rate() as f64;
        locked_buffer.use_count += 1;
        Ok(Self {
            kind,
            buffer: Some(buffer.clone()),
            resampling_multiplier: buffer_sample_rate / device_sample_rate,
            last_frame_left_samples: Default::default(),
            last_frame_right_samples: Default::default(),
            ..Default::default()
        })
    }

    pub fn get_buffer(&self) -> Option<Arc<Mutex<Buffer>>> {
        if let Some(buffer) = &self.buffer {
            Some(buffer.clone())
        } else {
            None
        }
    }

    pub fn set_play_once(&mut self, play_once: bool) {
        self.play_once = play_once;
    }

    pub fn is_play_once(&self) -> bool {
        self.play_once
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    pub fn get_gain(&self) -> f32 {
        self.gain
    }

    pub fn get_status(&self) -> Status {
        self.status
    }

    pub fn play(&mut self) {
        self.status = Status::Playing;
    }

    pub fn pause(&mut self) {
        self.status = Status::Paused;
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    pub fn is_looping(&self) -> bool {
        self.looping
    }

    pub fn set_pitch(&mut self, pitch: f64) {
        self.pitch = pitch;
    }

    pub fn stop(&mut self) -> Result<(), SoundError> {
        self.status = Status::Stopped;

        self.playback_pos = 0.0;

        if let Some(buffer) = &self.buffer {
            buffer.lock()?.rewind()?;
        }

        Ok(())
    }

    pub(in crate) fn update(&mut self, listener: &Listener) {
        let mut dist_gain = 1.0;
        if let SourceKind::Spatial(spatial) = &self.kind {
            let dir = spatial.position - listener.position;
            let sqr_distance = dir.sqr_len();
            if sqr_distance < 0.0001 {
                self.pan = 0.0;
                self.hrtf_sampling_vector = Vec3::new(0.0, 0.0, 1.0);
            } else {
                let norm_dir = dir.normalized().unwrap_or(Vec3::new(0.0, 0.0, 1.0));
                self.pan = norm_dir.dot(&listener.ear_axis);
                self.hrtf_sampling_vector = listener.view_matrix
                    .transform_vector_normal(spatial.position - listener.position)
                    .normalized()
                    .unwrap_or(Vec3::new(0.0, 0.0, 1.0));
            }
            dist_gain = 1.0 / (1.0 + (sqr_distance / (spatial.radius * spatial.radius)));
        }
        self.distance_gain = dist_gain;
        self.left_gain = self.gain * dist_gain * (1.0 + self.pan);
        self.right_gain = self.gain * dist_gain * (1.0 - self.pan);
    }

    pub fn get_kind(&self) -> &SourceKind {
        &self.kind
    }

    pub fn get_kind_mut(&mut self) -> &mut SourceKind {
        &mut self.kind
    }

    pub fn as_spatial(&self) -> &SpatialSource {
        match self.kind {
            SourceKind::Flat => panic!("Cast as spatial sound failed!"),
            SourceKind::Spatial(ref spatial) => spatial,
        }
    }

    pub fn as_spatial_mut(&mut self) -> &mut SpatialSource {
        match self.kind {
            SourceKind::Flat => panic!("Cast as spatial sound failed!"),
            SourceKind::Spatial(ref mut spatial) => spatial,
        }
    }

    pub fn get_playback_time(&self) -> Duration {
        if let Some(buffer) = self.buffer.as_ref().and_then(|b| b.lock().ok()) {
            let i = position_to_index(self.playback_pos, buffer.get_channel_count());
            return Duration::from_secs_f64((i / buffer.get_sample_rate()) as f64);
        }
        Duration::from_secs(0)
    }

    pub fn set_playback_time(&mut self, time: Duration) {
        if let Some(mut buffer) = self.buffer.as_mut().and_then(|b| b.lock().ok()) {
            // Make sure decoder is at right position.
            buffer.time_seek(time);
            // Set absolute position first.
            self.playback_pos = (time.as_secs_f64() * buffer.get_channel_count() as f64)
                .min(buffer.index_of_last_sample() as f64);
            // Then adjust buffer read position.
            self.buf_read_pos =
                if buffer.get_kind() == BufferKind::Stream {
                    // Make sure to load correct data into buffer from decoder.
                    buffer.read_next_block();
                    // Streaming sources has different buffer read position because
                    // buffer contains only small portion of data.
                    self.playback_pos % buffer.get_samples().len() as f64
                } else {
                    self.playback_pos
                };
            assert!(position_to_index(self.buf_read_pos, buffer.get_channel_count()) < buffer.get_samples().len());
        }
    }

    pub(in crate) fn next_sample_pair(&mut self, buffer: &mut Buffer) -> (f32, f32) {
        let step = self.pitch * self.resampling_multiplier * buffer.get_channel_count() as f64;

        self.buf_read_pos += step;
        self.playback_pos += step;

        let mut i = position_to_index(self.buf_read_pos, buffer.get_channel_count());

        let len = buffer.get_samples().len();
        if i > buffer.index_of_last_sample() {
            let mut end_reached = true;
            if buffer.get_kind() == BufferKind::Stream {
                // Means that this is the last available block.
                if len != buffer.get_channel_count() * Buffer::STREAM_SAMPLE_COUNT {
                    let _ = buffer.rewind();
                } else {
                    end_reached = false;
                }
                buffer.read_next_block();
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

        if buffer.get_channel_count() == 2 {
            let left = buffer.read_at(i);
            let right = buffer.read_at(i + 1);
            (left, right)
        } else {
            let sample = buffer.read_at(i);
            (sample, sample)
        }
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        if let Some(rc_buffer) = &self.buffer {
            if let Ok(mut buffer) = rc_buffer.lock() {
                buffer.use_count -= 1;
            }
        }
    }
}

impl Visit for Source {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.kind.visit("Kind", visitor)?;
        self.buffer.visit("Buffer", visitor)?;
        self.buf_read_pos.visit("BufReadPos", visitor)?;
        self.playback_pos.visit("PlaybackPos", visitor)?;
        self.pan.visit("Pan", visitor)?;
        self.pitch.visit("Pitch", visitor)?;
        self.gain.visit("Gain", visitor)?;
        self.looping.visit("Looping", visitor)?;
        self.left_gain.visit("LeftGain", visitor)?;
        self.right_gain.visit("RightGain", visitor)?;
        self.resampling_multiplier.visit("ResamplingMultiplier", visitor)?;
        self.status.visit("Status", visitor)?;
        self.play_once.visit("PlayOnce", visitor)?;

        visitor.leave_region()
    }
}