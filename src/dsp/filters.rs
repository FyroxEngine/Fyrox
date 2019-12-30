use crate::dsp::DelayLine;

/// One-pole Filter.
/// For details see - https://www.earlevel.com/main/2012/12/15/a-one-pole-filter/
pub struct OnePole {
    a0: f32,
    b1: f32,
    last: f32
}

fn get_b1(fc: f32) -> f32 {
    (-2.0 * std::f32::consts::PI * fc.min(1.0).max(0.0)).exp()
}

impl OnePole {
    pub fn new(fc: f32) -> Self {
        let b1 = get_b1(fc);
        Self {
            b1,
            a0: 1.0 - b1,
            last: 0.0
        }
    }

    pub fn set_fc(&mut self, fc: f32) {
        self.b1 = get_b1(fc);
        self.a0 = 1.0 - self.b1;
    }

    pub fn set_pole(&mut self, pole: f32) {
        self.b1 = pole.min(1.0).max(0.0);
        self.a0 = 1.0 - self.b1;
    }

    pub fn feed(&mut self, sample: f32) -> f32 {
        let result = sample * self.a0 + self.last * self.b1;
        self.last = result;
        result
    }
}

/// Lowpass-Feedback Comb Filter
/// For details see - https://ccrma.stanford.edu/~jos/pasp/Lowpass_Feedback_Comb_Filter.html
pub struct LpfComb {
    low_pass: OnePole,
    delay_line: DelayLine,
    feedback: f32
}

impl LpfComb {
    pub fn new(len: usize, fc: f32, feedback: f32) -> Self {
        Self {
            low_pass: OnePole::new(fc),
            delay_line: DelayLine::new(len),
            feedback
        }
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }

    pub fn get_feedback(&self) -> f32 {
        self.feedback
    }

    pub fn set_fc(&mut self, fc: f32) {
        self.low_pass.set_fc(fc)
    }

    pub fn len(&self) -> usize {
        self.delay_line.len()
    }

    pub fn feed(&mut self, sample: f32) -> f32 {
        let result = sample + self.feedback * self.low_pass.feed(self.delay_line.last());
        self.delay_line.feed(result);
        result
    }
}

/// Allpass Filter - https://ccrma.stanford.edu/~jos/pasp/Allpass_Filters.html
/// For details see - https://ccrma.stanford.edu/~jos/pasp/Allpass_Two_Combs.html
pub struct AllPass {
    delay_line: DelayLine,
    gain: f32
}

impl AllPass {
    pub fn new(len: usize, gain: f32) -> Self {
        Self {
            delay_line: DelayLine::new(len),
            gain
        }
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    pub fn len(&self) -> usize {
        self.delay_line.len()
    }

    pub fn feed(&mut self, sample: f32) -> f32 {
        let delay_line_output = self.delay_line.last();
        let am_arm = -self.gain * delay_line_output;
        let sum_left = sample + am_arm;
        let b0_arm = sum_left * self.gain;
        self.delay_line.feed(sum_left);
        let sum_right = delay_line_output + b0_arm;
        sum_right
    }
}