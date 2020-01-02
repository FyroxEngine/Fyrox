// Clippy is being stupid here again, filters cannot be empty and there is no
// need to define is_empty() method.
#![allow(clippy::len_without_is_empty)]

/// Digital signal processing module.
///
/// # Abbreviations
///
/// `fc` - normalized frequency, i.e. `fc = 0.2` with `sample rate = 44100 Hz` will be `f = 8820 Hz`

pub mod filters;

pub struct DelayLine {
    samples: Vec<f32>,
    last: f32,
    pos: usize
}

impl DelayLine {
    pub fn new(len: usize) -> Self {
        Self {
            samples: vec![0.0; len],
            last: 0.0,
            pos: 0
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn feed(&mut self, sample: f32) -> f32 {
        self.last = self.samples[self.pos];
        self.samples[self.pos] = sample;
        self.pos += 1;
        if self.pos >= self.samples.len() {
            self.pos -= self.samples.len()
        }
        self.last
    }

    pub fn last(&self) -> f32 {
        self.last
    }
}

pub fn hamming_window(i: usize, sample_count: usize) -> f32 {
    0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (sample_count - 1) as f32).cos()
}

pub fn hann_window(i: usize, sample_count: usize) -> f32 {
    0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (sample_count - 1) as f32).cos()
}

pub fn make_window<W: Fn(usize, usize) -> f32>(sample_count: usize, func: W) -> Vec<f32> {
    (0..sample_count).map(|i| func(i, sample_count)).collect()
}