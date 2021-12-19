// Clippy is being stupid here again, filters cannot be empty and there is no
// need to define is_empty() method.
#![allow(clippy::len_without_is_empty)]

//! Digital signal processing module. Provides basic elements to process signal sample-by-sample.
//!
//! # Abbreviations
//!
//! `fc` - normalized frequency, i.e. `fc = 0.2` with `sample rate = 44100 Hz` will be `f = 8820 Hz`

use fyrox_core::visitor::{Visit, VisitResult, Visitor};

pub mod filters;

/// See more info here <https://ccrma.stanford.edu/~jos/pasp/Delay_Lines.html>
#[derive(Debug, Clone)]
pub struct DelayLine {
    samples: Vec<f32>,
    last: f32,
    pos: u32,
}

impl DelayLine {
    /// Creates new instance of delay line of given length in samples.
    pub fn new(len: usize) -> Self {
        Self {
            samples: vec![0.0; len],
            last: 0.0,
            pos: 0,
        }
    }

    /// Returns length of delay line in samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Processes single sample.
    pub fn feed(&mut self, sample: f32) -> f32 {
        self.last = self.samples[self.pos as usize];
        self.samples[self.pos as usize] = sample;
        self.pos += 1;
        if self.pos >= self.samples.len() as u32 {
            self.pos -= self.samples.len() as u32
        }
        self.last
    }

    /// Returns last processed sample.
    pub fn last(&self) -> f32 {
        self.last
    }
}

impl Default for DelayLine {
    fn default() -> Self {
        Self {
            samples: vec![0.0],
            last: 0.0,
            pos: 0,
        }
    }
}

impl Visit for DelayLine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.last.visit("Last", visitor)?;
        self.pos.visit("Pos", visitor)?;
        self.samples.visit("Samples", visitor)?;

        visitor.leave_region()
    }
}

/// Calculates single coefficient of Hamming window.
/// <https://en.wikipedia.org/wiki/Window_function#Hamming_window>
pub fn hamming_window(i: usize, sample_count: usize) -> f32 {
    0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (sample_count - 1) as f32).cos()
}

/// Calculates single coefficient of Hann window.
/// <https://en.wikipedia.org/wiki/Hann_function>
pub fn hann_window(i: usize, sample_count: usize) -> f32 {
    0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (sample_count - 1) as f32).cos()
}

/// Creates new window using specified window function.
/// <https://en.wikipedia.org/wiki/Window_function>
pub fn make_window<W: Fn(usize, usize) -> f32>(sample_count: usize, func: W) -> Vec<f32> {
    (0..sample_count).map(|i| func(i, sample_count)).collect()
}
