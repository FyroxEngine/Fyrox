//! Filters module.
//!
//! # Overview
//!
//! This module contains some of common filters used in digital signal processing.
//! Since this is very specific theme with lots of background, every filter has link to source with good
//! description of each filter. There is no need to describe them all here.

use crate::dsp::DelayLine;
use fyrox_core::{
    reflect::prelude::*,
    uuid_provider,
    visitor::{Visit, VisitResult, Visitor},
};

/// One-pole Filter.
/// For details see - <https://www.earlevel.com/main/2012/12/15/a-one-pole-filter/>
#[derive(Debug, PartialEq, Clone, Visit)]
pub struct OnePole {
    a0: f32,
    b1: f32,
    last: f32,
}

impl Default for OnePole {
    fn default() -> Self {
        Self {
            a0: 1.0,
            b1: 0.0,
            last: 0.0,
        }
    }
}

fn get_b1(fc: f32) -> f32 {
    (-2.0 * std::f32::consts::PI * fc.clamp(0.0, 1.0)).exp()
}

impl OnePole {
    /// Creates new instance of one pole filter with given normalized frequency.
    pub fn new(fc: f32) -> Self {
        let b1 = get_b1(fc);
        Self {
            b1,
            a0: 1.0 - b1,
            last: 0.0,
        }
    }

    /// Sets normalized frequency of the filter.
    pub fn set_fc(&mut self, fc: f32) {
        self.b1 = get_b1(fc);
        self.a0 = 1.0 - self.b1;
    }

    /// Sets pole of filter directly.
    pub fn set_pole(&mut self, pole: f32) {
        self.b1 = pole.clamp(0.0, 1.0);
        self.a0 = 1.0 - self.b1;
    }

    /// Processes single sample.
    pub fn feed(&mut self, sample: f32) -> f32 {
        let result = sample * self.a0 + self.last * self.b1;
        self.last = result;
        result
    }
}

/// Lowpass-Feedback Comb Filter
/// For details see - <https://ccrma.stanford.edu/~jos/pasp/Lowpass_Feedback_Comb_Filter.html>
#[derive(Debug, PartialEq, Clone, Visit)]
pub struct LpfComb {
    low_pass: OnePole,
    delay_line: DelayLine,
    feedback: f32,
}

impl Default for LpfComb {
    fn default() -> Self {
        Self {
            low_pass: Default::default(),
            delay_line: Default::default(),
            feedback: 0.0,
        }
    }
}

impl LpfComb {
    /// Creates new instance of lowpass-feedback comb filter with given parameters.
    pub fn new(len: usize, fc: f32, feedback: f32) -> Self {
        Self {
            low_pass: OnePole::new(fc),
            delay_line: DelayLine::new(len),
            feedback,
        }
    }

    /// Sets feedback factor. For numeric stability factor should be in 0..1 range.
    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }

    /// Returns current feedback factor.
    pub fn feedback(&self) -> f32 {
        self.feedback
    }

    /// Sets normalized frequency of internal lowpass filter.
    pub fn set_fc(&mut self, fc: f32) {
        self.low_pass.set_fc(fc)
    }

    /// Returns total length of internal delay line (in samples)
    pub fn len(&self) -> usize {
        self.delay_line.len()
    }

    /// Processes single sample.
    pub fn feed(&mut self, sample: f32) -> f32 {
        let result = sample + self.feedback * self.low_pass.feed(self.delay_line.last());
        self.delay_line.feed(result);
        result
    }
}

/// Allpass Filter - <https://ccrma.stanford.edu/~jos/pasp/Allpass_Filters.html>
/// For details see - <https://ccrma.stanford.edu/~jos/pasp/Allpass_Two_Combs.html>
#[derive(Debug, Clone, PartialEq, Visit)]
pub struct AllPass {
    delay_line: DelayLine,
    gain: f32,
}

impl Default for AllPass {
    fn default() -> Self {
        Self {
            delay_line: Default::default(),
            gain: 1.0,
        }
    }
}

impl AllPass {
    /// Creates new instance of allpass filter.
    pub fn new(len: usize, gain: f32) -> Self {
        Self {
            delay_line: DelayLine::new(len),
            gain,
        }
    }

    /// Sets overall gain of feedback parts of filter. Should be in 0..1 range.
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    /// Returns length of internal delay line.
    pub fn len(&self) -> usize {
        self.delay_line.len()
    }

    /// Processes single sample.
    pub fn feed(&mut self, sample: f32) -> f32 {
        let delay_line_output = self.delay_line.last();
        let am_arm = -self.gain * delay_line_output;
        let sum_left = sample + am_arm;
        let b0_arm = sum_left * self.gain;
        self.delay_line.feed(sum_left);
        delay_line_output + b0_arm
    }
}

/// Exact kind of biquad filter - it defines coefficients of the filter.
/// More info here: <https://shepazu.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html>
pub enum BiquadKind {
    /// Reduces amplitude of frequencies higher F_center.
    LowPass,

    /// Reduces amplitude of frequencies lower F_center.
    HighPass,

    /// Reduces amplitude of all frequencies except in some band around F_center giving _/̅ \_ shape
    BandPass,

    /// Passes all frequencies but gives 90* phase shift to a signal at F_center.
    AllPass,

    /// Reduces amplitude of frequencies in a shape like this ̅ \_ where location of center of \
    /// defined by F_center.
    LowShelf,

    /// Reduces amplitude of frequencies in a shape like this _/̅  where location of center of /
    /// defined by F_center.
    HighShelf,
}

/// Generic second order digital filter.
/// More info here: <https://ccrma.stanford.edu/~jos/filters/BiQuad_Section.html>
#[derive(Clone, Debug, Reflect, Visit, PartialEq)]
pub struct Biquad {
    /// B0 Coefficient of the equation.
    pub b0: f32,
    /// B1 Coefficient of the equation.
    pub b1: f32,
    /// B2 Coefficient of the equation.
    pub b2: f32,
    /// A1 Coefficient of the equation.
    pub a1: f32,
    /// A2 Coefficient of the equation.
    pub a2: f32,
    #[reflect(hidden)]
    prev1: f32,
    #[reflect(hidden)]
    prev2: f32,
}

uuid_provider!(Biquad = "4560a1b3-74fc-4b0e-802f-ff1f253bf565");

impl Biquad {
    /// Creates new filter of given kind with specified parameters, where:
    /// `fc` - normalized frequency
    /// `gain` - desired gain at `fc`
    /// `quality` - defines band width at which amplitude decays by half (or by 3 db in log scale), the lower it will
    /// be, the wider band will be and vice versa. See more info [here](https://ccrma.stanford.edu/~jos/filters/Quality_Factor_Q.html)
    pub fn new(kind: BiquadKind, fc: f32, gain: f32, quality: f32) -> Self {
        let mut filter = Self::default();

        filter.tune(kind, fc, gain, quality);

        filter
    }

    /// Creates new instance of filter with given coefficients.
    pub fn from_coefficients(b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            prev1: 0.0,
            prev2: 0.0,
        }
    }

    /// Tunes filter using specified parameters.
    /// `kind` - new kind of filter.
    /// `fc` - normalized frequency
    /// `gain` - desired gain at `fc`
    /// `quality` - defines band width at which amplitude decays by half (or by 3 db in log scale), the lower it will
    /// be, the wider band will be and vice versa. See more info [here](https://ccrma.stanford.edu/~jos/filters/Quality_Factor_Q.html)
    pub fn tune(&mut self, kind: BiquadKind, fc: f32, gain: f32, quality: f32) {
        let w0 = 2.0 * std::f32::consts::PI * fc;
        let w0_cos = w0.cos();
        let w0_sin = w0.sin();
        let alpha = w0_sin / (2.0 * quality);

        let (b0, b1, b2, a0, a1, a2) = match kind {
            BiquadKind::LowPass => {
                let b0 = (1.0 - w0_cos) / 2.0;
                let b1 = 1.0 - w0_cos;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * w0_cos;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadKind::HighPass => {
                let b0 = (1.0 + w0_cos) / 2.0;
                let b1 = -(1.0 + w0_cos);
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * w0_cos;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadKind::BandPass => {
                let b0 = w0_sin / 2.0;
                let b1 = 0.0;
                let b2 = -b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * w0_cos;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadKind::AllPass => {
                let b0 = 1.0 - alpha;
                let b1 = -2.0 * w0_cos;
                let b2 = 1.0 + alpha;
                let a0 = b2;
                let a1 = -2.0 * w0_cos;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadKind::LowShelf => {
                let sq = 2.0 * gain.sqrt() * alpha;
                let b0 = gain * ((gain + 1.0) - (gain - 1.0) * w0_cos + sq);
                let b1 = 2.0 * gain * ((gain - 1.0) - (gain + 1.0) * w0_cos);
                let b2 = gain * ((gain + 1.0) - (gain - 1.0) * w0_cos - sq);
                let a0 = (gain + 1.0) + (gain - 1.0) * w0_cos + sq;
                let a1 = -2.0 * ((gain - 1.0) + (gain + 1.0) * w0_cos);
                let a2 = (gain + 1.0) + (gain - 1.0) * w0_cos - sq;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadKind::HighShelf => {
                let sq = 2.0 * gain.sqrt() * alpha;
                let b0 = gain * ((gain + 1.0) + (gain - 1.0) * w0_cos + sq);
                let b1 = -2.0 * gain * ((gain - 1.0) + (gain + 1.0) * w0_cos);
                let b2 = gain * ((gain + 1.0) + (gain - 1.0) * w0_cos - sq);
                let a0 = (gain + 1.0) - (gain - 1.0) * w0_cos + sq;
                let a1 = 2.0 * ((gain - 1.0) - (gain + 1.0) * w0_cos);
                let a2 = (gain + 1.0) - (gain - 1.0) * w0_cos - sq;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// Processes single sample.
    pub fn feed(&mut self, sample: f32) -> f32 {
        let result = sample * self.b0 + self.prev1;
        self.prev1 = sample * self.b1 - result * self.a1 + self.prev2;
        self.prev2 = sample * self.b2 - result * self.a2;
        result
    }
}

impl Default for Biquad {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            prev1: 0.0,
            prev2: 0.0,
        }
    }
}
