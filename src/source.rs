use std::sync::{Arc, Mutex};
use crate::buffer::{Buffer, BufferKind};
use crate::device::Sample;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Status {
    Stopped,
    Playing,
    Paused,
}

pub struct Source {
    buffer: Arc<Mutex<Buffer>>,
    /// Read position in the buffer. Differs from @playback_pos if buffer is streaming.
    buf_read_pos: f64,
    /// Real playback position.
    playback_pos: f64,
    current_sample_rate: f64,
    pan: f32,
    pitch: f32,
    gain: f32,
    looping: bool,
    /// Radius of sphere around sound source at which sound volume is half of initial.
    radius: f32,
    left_gain: f32,
    right_gain: f32,
    status: Status,
}

impl Source {
    pub fn new(buffer: Arc<Mutex<Buffer>>) -> Self {
        Self {
            buffer,
            buf_read_pos: 0.0,
            playback_pos: 0.0,
            current_sample_rate: 1.0,
            pan: 0.0,
            pitch: 1.0,
            gain: 1.0,
            looping: false,
            radius: 10.0,
            left_gain: 1.0,
            right_gain: 1.0,
            status: Status::Playing,
        }
    }

    pub fn sample(&mut self) -> (f32, f32) {
        if let Ok(mut buffer) = self.buffer.lock() {
            self.buf_read_pos += self.current_sample_rate;
            self.playback_pos += self.current_sample_rate;

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
                let left = self.left_gain * buffer.read(i);
                let right = self.right_gain * buffer.read(i + buffer.get_sample_per_channel());
                (left, right)
            } else {
                let sample = buffer.read(i);
                (self.left_gain * sample, self.right_gain * sample)
            }
        } else {
            (0.0, 0.0)
        }
    }
}