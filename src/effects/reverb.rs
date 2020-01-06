//! Reverberation module
//!
//! # Overview
//!
//! This is implementation of [Freeverb reverb effect](https://ccrma.stanford.edu/~jos/pasp/Freeverb.html)
//! Reverberation gives you scene a "volume" and improves overall perception of sound.
//!
//! # Usage
//!
//! ```
//! use std::time::Duration;
//! use rg3d_sound::context::Context;
//! use rg3d_sound::effects::reverb::Reverb;
//! use rg3d_sound::effects::Effect;
//!
//! fn set_reverberator(context: &mut Context) {
//!     let mut reverb = Reverb::new();
//!     reverb.set_decay_time(Duration::from_secs_f32(10.0));
//!     context.lock()
//! 	    .unwrap()
//!     	.add_effect(Effect::Reverb(reverb));
//! }
//! ```
//!
//! # Known problems
//!
//! This reverberator has little "metallic" tone, but since this is one of the simplest reverberators this
//! is acceptable. To remove this effect, more complex reverberator should be implemented.

use crate::dsp::filters::{
    LpfComb,
    AllPass,
};
use std::time::Duration;

struct ChannelReverb {
    fc: f32,
    sample_rate: usize,
    stereo_spread: usize,
    lp_fb_comb_filters: Vec<LpfComb>,
    all_pass_filters: Vec<AllPass>,
}

/// 60 decibels
const DB60: f32 = 0.001;

/// Sample rate for which this reverb was designed.
const DESIGN_SAMPLE_RATE: usize = 44100;

fn calculate_decay(len: usize, sample_rate: usize, decay_time: Duration) -> f32 {
    let time_len = len as f32 / sample_rate as f32;
    // Asymptotically goes to 1.0 by exponential law
    DB60.powf(time_len / decay_time.as_secs_f32())
}

impl ChannelReverb {
    /// Filter lengths given in samples
    const COMB_LENGTHS: [usize; 8] = [1557, 1617, 1491, 1422, 1277, 1356, 1188, 1116];
    const ALLPASS_LENGTHS: [usize; 4] = [225, 556, 441, 341];

    fn new(stereo_spread: usize, fc: f32, feedback: f32) -> Self {
        Self {
            fc,
            stereo_spread,
            sample_rate: DESIGN_SAMPLE_RATE,
            lp_fb_comb_filters: Self::COMB_LENGTHS.iter()
                .map(|len| LpfComb::new(*len + stereo_spread, fc, feedback))
                .collect(),
            all_pass_filters: Self::ALLPASS_LENGTHS.iter()
                .map(|len| AllPass::new(*len + stereo_spread, 0.5))
                .collect()
        }
    }

    fn set_sample_rate(&mut self, sample_rate: usize) {
        let scale = sample_rate as f32 / DESIGN_SAMPLE_RATE as f32;

        let feedback = self.lp_fb_comb_filters[0].feedback();
        // TODO: According to many papers delay line lengths should be prime numbers to
        //       remove metallic ringing effect. But still not sure why then initial lengths
        //       are not 100% prime, for example 1422 is not prime number.
        self.lp_fb_comb_filters = Self::COMB_LENGTHS.iter()
            .map(|len| LpfComb::new((scale * (*len) as f32) as usize + self.stereo_spread, self.fc, feedback))
            .collect();
        self.all_pass_filters = Self::ALLPASS_LENGTHS.iter()
            .map(|len| AllPass::new((scale * (*len) as f32) as usize + self.stereo_spread, 0.5))
            .collect();
    }

    fn set_decay_time(&mut self, decay_time: Duration) {
        for comb in self.lp_fb_comb_filters.iter_mut() {
            comb.set_feedback(calculate_decay(comb.len(), self.sample_rate, decay_time));
        }
    }

    fn set_fc(&mut self, fc: f32) {
        self.fc = fc;
        for comb in self.lp_fb_comb_filters.iter_mut() {
            comb.set_fc(fc)
        }
    }

    fn feed(&mut self, sample: f32) -> f32 {
        let mut result = 0.0;
        for comb in self.lp_fb_comb_filters.iter_mut() {
            result += comb.feed(sample);
        }
        for allpass in self.all_pass_filters.iter_mut() {
            result = allpass.feed(result);
        }
        result
    }
}

/// See module docs.
pub struct Reverb {
    dry: f32,
    wet: f32,
    left: ChannelReverb,
    right: ChannelReverb,
}

impl Default for Reverb {
    fn default() -> Self {
        Self::new()
    }
}

impl Reverb {
    const GAIN: f32 = 0.015;

    /// Creates new instance of reberb effect with cutoff frequency of ~11.2 kHz and
    /// 5 seconds decay time.
    pub fn new() -> Self {
        let fc = 0.25615; // 11296 Hz at 44100 Hz sample rate
        let feedback = 0.84;

        Self {
            dry: 1.0,
            wet: 1.0,
            left: ChannelReverb::new(0, fc, feedback),
            right: ChannelReverb::new(23, fc, feedback),
        }
    }

    /// Sets how much of input signal should be passed to output without any processing.
    /// Default value is 1.0.
    pub fn set_dry(&mut self, dry: f32) {
        self.dry = dry.min(1.0).max(0.0);
    }

    /// Returns dry part.
    pub fn get_dry(&self) -> f32 {
        self.dry
    }

    /// Sets stereo mixing of processed signal.
    /// 0.0 - left is left, right is right
    /// 1.0 - right is left, left is right.
    /// 0.5 - left is (left + right) * 0.5, right is (left + right) * 0.5
    /// and so on.
    pub fn set_wet(&mut self, wet: f32) {
        self.wet = wet.min(1.0).max(0.0);
    }

    /// Returns stereo mixing coefficient.
    pub fn get_wet(&self) -> f32 {
        self.wet
    }

    /// Sets actual sample rate of effect. It was designed to 44100 Hz sampling rate.
    /// TODO: This shouldn't be in public API.
    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        self.left.set_sample_rate(sample_rate);
        self.right.set_sample_rate(sample_rate);
    }

    /// Sets desired duration of reverberation, the more size your environment has,
    /// the larger duration of reverberation should be.
    pub fn set_decay_time(&mut self, decay_time: Duration) {
        self.left.set_decay_time(decay_time);
        self.right.set_decay_time(decay_time)
    }

    /// Sets cutoff frequency for lowpass filter in comb filters. Basically this parameter defines
    /// "tone" of reflections, when frequency is higher - then more high frequencies will be in
    /// output signal, and vice versa. For example if you have environment with high absorption of
    /// high frequencies, then sound in reality will be muffled - to simulate this you could set
    /// frequency to 3-4 kHz.
    ///
    /// # Notes
    ///
    /// This method uses normalized frequency as input, this means that you should divide your desired
    /// frequency in hertz by sample rate of sound context. Context has `normalize_frequency` method
    /// exactly for this purpose.
    pub fn set_fc(&mut self, fc: f32) {
        self.left.set_fc(fc);
        self.right.set_fc(fc);
    }

    pub(in crate) fn feed(&mut self, left: f32, right: f32) -> (f32, f32) {
        let wet1 = self.wet;
        let wet2 = 1.0 - self.wet;

        let input = (left + right) * Self::GAIN;

        let processed_left = self.left.feed(input);
        let processed_right = self.right.feed(input);

        let out_left = processed_left * wet1 + processed_right * wet2 + self.dry * left;
        let out_right = processed_right * wet1 + processed_left * wet2 + self.dry * right;

        (out_left, out_right)
    }
}