//! Filters module.
//!
//! # Overview
//!
//! This module contains some of common filters used in digital signal processing.
//! Since this is very specific theme with lots of background, every filter has link to source with good
//! description of each filter. There is no need to describe them all here.

use crate::dsp::DelayLine;
use rg3d_core::visitor::{Visit, VisitResult, Visitor};

/// One-pole Filter.
/// For details see - https://www.earlevel.com/main/2012/12/15/a-one-pole-filter/
#[derive(Debug, Clone)]
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
    (-2.0 * std::f32::consts::PI * fc.min(1.0).max(0.0)).exp()
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
        self.b1 = pole.min(1.0).max(0.0);
        self.a0 = 1.0 - self.b1;
    }

    /// Processes single sample.
    pub fn feed(&mut self, sample: f32) -> f32 {
        let result = sample * self.a0 + self.last * self.b1;
        self.last = result;
        result
    }
}

impl Visit for OnePole {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.a0.visit("A0", visitor)?;
        self.b1.visit("B1", visitor)?;
        self.last.visit("Last", visitor)?;

        visitor.leave_region()
    }
}

/// Lowpass-Feedback Comb Filter
/// For details see - https://ccrma.stanford.edu/~jos/pasp/Lowpass_Feedback_Comb_Filter.html
#[derive(Debug, Clone)]
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

impl Visit for LpfComb {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.delay_line.visit("DelayLine", visitor)?;
        self.feedback.visit("Feedback", visitor)?;
        self.low_pass.visit("LowPass", visitor)?;

        visitor.leave_region()
    }
}

/// Allpass Filter - https://ccrma.stanford.edu/~jos/pasp/Allpass_Filters.html
/// For details see - https://ccrma.stanford.edu/~jos/pasp/Allpass_Two_Combs.html
#[derive(Debug, Clone)]
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

impl Visit for AllPass {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.delay_line.visit("DelayLine", visitor)?;
        self.gain.visit("Gain", visitor)?;

        visitor.leave_region()
    }
}

/// Exact kind of biquad filter - it defines coefficients of the filter.
/// More info here: https://shepazu.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html
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
/// More info here: https://ccrma.stanford.edu/~jos/filters/BiQuad_Section.html
#[derive(Clone, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    prev1: f32,
    prev2: f32,
}

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

impl Visit for Biquad {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.b0.visit("b0", visitor)?;
        self.b1.visit("b1", visitor)?;
        self.b2.visit("b2", visitor)?;
        self.a1.visit("a1", visitor)?;
        self.a2.visit("a2", visitor)?;
        self.prev1.visit("prev1", visitor)?;
        self.prev2.visit("prev2", visitor)?;

        visitor.leave_region()
    }
}
