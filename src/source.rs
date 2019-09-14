#![allow(clippy::or_fun_call)]

use std::sync::{Arc, Mutex};
use crate::{
    buffer::{Buffer, BufferKind},
    error::SoundError,
    listener::Listener
};
use rg3d_core::{
    math::vec3::Vec3,
    visitor::{Visit, VisitResult, Visitor}
};
use rg3d_core::visitor::VisitError;

#[derive(Eq, PartialEq, Copy, Clone)]
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
            radius: 0.0,
            position: Vec3::zero()
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
    /// Read position in the buffer. Differs from `playback_pos` if buffer is streaming.
    /// In case of streaming buffer its maximum value will be size o
    buf_read_pos: f64,
    /// Real playback position.
    playback_pos: f64,
    pan: f32,
    pitch: f64,
    gain: f32,
    looping: bool,
    left_gain: f32,
    right_gain: f32,
    /// Important coefficient for runtime resampling. It is used to modify playback speed
    /// of a source in order to match output device sampling rate. PCM data can be stored
    /// in various sampling rates (22050 Hz, 44100 Hz, 88200 Hz, etc.) but output device
    /// is running at fixed sampling rate (usually 44100 Hz). For example if we we'll feed
    /// data to device with rate of 22050 Hz but device is running at 44100 Hz then we'll
    /// hear that sound will have high pitch (2.0), to fix that we'll just pre-multiply
    /// playback speed by 0.5.
    resampling_multiplier: f64,
    status: Status,
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
            status: Status::Stopped
        }
    }
}

impl Source {
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
            buf_read_pos: 0.0,
            playback_pos: 0.0,
            resampling_multiplier: buffer_sample_rate / device_sample_rate,
            pan: 0.0,
            pitch: 1.0,
            gain: 1.0,
            looping: false,
            left_gain: 1.0,
            right_gain: 1.0,
            status: Status::Playing,
        })
    }

    pub fn get_buffer(&self) -> Option<Arc<Mutex<Buffer>>> {
        if let Some(buffer) = &self.buffer {
            Some(buffer.clone())
        } else {
            None
        }
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

    pub fn stop(&mut self) -> Result<(), SoundError> {
        self.status = Status::Stopped;

        self.buf_read_pos = 0.0;
        self.playback_pos = 0.0;

        if let Some(buffer) = &self.buffer {
            buffer.lock()?.rewind()?;
        }

        Ok(())
    }

    pub(in crate) fn update(&mut self, listener: &Listener) -> Result<(), SoundError>{
        if let Some(buffer) = &self.buffer {
            buffer.lock()?.update()?;
        }
        let mut dist_gain = 1.0;
        if let SourceKind::Spatial(spatial) = &self.kind {
            let dir = spatial.position - listener.position;
            let sqr_distance = dir.sqr_len();
            if sqr_distance < 0.0001 {
                self.pan = 0.0;
            } else {
                let norm_dir = dir.normalized().ok_or(SoundError::MathError("|v| == 0.0".to_string()))?;
                self.pan = norm_dir.dot(&listener.ear_axis);
            }
            dist_gain = 1.0 / (1.0 + (sqr_distance / (spatial.radius * spatial.radius)));
        }
        self.left_gain = self.gain * dist_gain * (1.0 + self.pan);
        self.right_gain = self.gain * dist_gain * (1.0 - self.pan);
        Ok(())
    }

    pub(in crate) fn sample_into(&mut self, mix_buffer: &mut [(f32, f32)]) {
        if self.status != Status::Playing {
            return;
        }

        let step = self.pitch * self.resampling_multiplier;

        if let Some(rc_buffer) = &self.buffer {
            if let Ok(mut buffer) = rc_buffer.lock() {
                for (left, right) in mix_buffer {
                    self.buf_read_pos += step;
                    self.playback_pos += step;

                    let mut i = self.buf_read_pos as usize;

                    if i >= buffer.get_sample_per_channel() {
                        if buffer.get_kind() == BufferKind::Stream {
                            buffer.prepare_read_next_block();
                        }
                        self.buf_read_pos = 0.0;
                        i = 0;
                    }

                    if self.playback_pos >= buffer.get_total_sample_per_channel() as f64 {
                        self.playback_pos = 0.0;
                        if self.looping && buffer.get_kind() == BufferKind::Stream {
                            if buffer.get_sample_per_channel() != 0 {
                                self.buf_read_pos = (i % buffer.get_sample_per_channel()) as f64;
                            } else {
                                self.buf_read_pos = 0.0;
                            }
                        } else {
                            self.buf_read_pos = 0.0;
                        }
                        self.status = if self.looping {
                            Status::Playing
                        } else {
                            Status::Stopped
                        };
                    }

                    if buffer.get_channel_count() == 2 {
                        *left += self.left_gain * buffer.read(i);
                        *right += self.right_gain * buffer.read(i + buffer.get_sample_per_channel());
                    } else {
                        let sample = buffer.read(i);
                        *left += self.left_gain * sample;
                        *right += self.right_gain * sample;
                    }
                }
            }
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

        visitor.leave_region()
    }
}