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
//! use fyrox_sound::context::SoundContext;
//! use fyrox_sound::effects::reverb::Reverb;
//! use fyrox_sound::effects::Effect;
//!
//! fn set_reverberator(context: &mut SoundContext) {
//!     let mut reverb = Reverb::new();
//!     reverb.set_decay_time(10.0);
//!     context.state().bus_graph_mut().primary_bus_mut().add_effect(Effect::Reverb(reverb));
//! }
//! ```
//!
//! # Known problems
//!
//! This reverberator has little "metallic" tone, but since this is one of the simplest reverberators this
//! is acceptable. To remove this effect, more complex reverberator should be implemented.

use crate::{
    dsp::filters::{AllPass, LpfComb},
    effects::EffectRenderTrait,
};
use fyrox_core::{reflect::prelude::*, visitor::prelude::*};

#[derive(Default, Debug, Clone, PartialEq, Visit)]
struct ChannelReverb {
    fc: f32,
    sample_rate: u32,
    stereo_spread: u32,
    lp_fb_comb_filters: Vec<LpfComb>,
    all_pass_filters: Vec<AllPass>,
}

/// 60 decibels
const DB60: f32 = 0.001;

/// Sample rate for which this reverb was designed.
const DESIGN_SAMPLE_RATE: u32 = 44100;

fn calculate_decay(len: usize, sample_rate: u32, decay_time: f32) -> f32 {
    let time_len = len as f32 / sample_rate as f32;
    // Asymptotically goes to 1.0 by exponential law
    DB60.powf(time_len / decay_time)
}

impl ChannelReverb {
    /// Filter lengths given in samples
    const COMB_LENGTHS: [usize; 8] = [1557, 1617, 1491, 1422, 1277, 1356, 1188, 1116];
    const ALLPASS_LENGTHS: [usize; 4] = [225, 556, 441, 341];

    fn new(stereo_spread: u32, fc: f32, feedback: f32, decay_time: f32) -> Self {
        let mut reverb = Self {
            fc,
            stereo_spread,
            sample_rate: DESIGN_SAMPLE_RATE,
            lp_fb_comb_filters: Self::COMB_LENGTHS
                .iter()
                .map(|len| LpfComb::new(*len + stereo_spread as usize, fc, feedback))
                .collect(),
            all_pass_filters: Self::ALLPASS_LENGTHS
                .iter()
                .map(|len| AllPass::new(*len + stereo_spread as usize, 0.5))
                .collect(),
        };

        reverb.set_decay_time(decay_time);

        reverb
    }

    fn set_sample_rate(&mut self, sample_rate: usize) {
        let scale = sample_rate as f32 / DESIGN_SAMPLE_RATE as f32;

        let feedback = self.lp_fb_comb_filters[0].feedback();
        // TODO: According to many papers delay line lengths should be prime numbers to
        //       remove metallic ringing effect. But still not sure why then initial lengths
        //       are not 100% prime, for example 1422 is not prime number.
        self.lp_fb_comb_filters = Self::COMB_LENGTHS
            .iter()
            .map(|len| {
                LpfComb::new(
                    (scale * (*len) as f32) as usize + self.stereo_spread as usize,
                    self.fc,
                    feedback,
                )
            })
            .collect();
        self.all_pass_filters = Self::ALLPASS_LENGTHS
            .iter()
            .map(|len| {
                AllPass::new(
                    (scale * (*len) as f32) as usize + self.stereo_spread as usize,
                    0.5,
                )
            })
            .collect();
    }

    fn set_decay_time(&mut self, decay_time: f32) {
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
#[derive(Debug, Clone, Reflect, PartialEq)]
pub struct Reverb {
    dry: f32,
    wet: f32,
    #[reflect(setter = "set_decay_time", min_value = 0.0)]
    decay_time: f32,
    #[reflect(setter = "set_fc", min_value = 0.0, max_value = 1.0)]
    fc: f32,
    #[reflect(hidden)]
    left: ChannelReverb,
    #[reflect(hidden)]
    right: ChannelReverb,
}

impl Visit for Reverb {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.dry.visit("Dry", &mut region)?;
        self.wet.visit("Wet", &mut region)?;
        self.decay_time.visit("DecayTime", &mut region)?;
        self.fc.visit("Fc", &mut region)?;

        if region.is_reading() {
            self.left = ChannelReverb::new(0, self.fc, Reverb::FEEDBACK, self.decay_time);
            self.right = ChannelReverb::new(23, self.fc, Reverb::FEEDBACK, self.decay_time);
        }

        Ok(())
    }
}

impl Default for Reverb {
    fn default() -> Self {
        Self::new()
    }
}

impl Reverb {
    /// Total amount of allpass + comb filters per channel
    const TOTAL_FILTERS_COUNT: f32 = 4.0 + 8.0;

    const FEEDBACK: f32 = 0.84;

    /// Accumulated gain of all sources of signal (all filters, delay lines, etc.)
    /// it is used to scale input samples and prevent multiplication of signal gain
    /// too much which will cause signal overflow.
    ///
    /// 2.0 here because left and right signals will be mixed together.
    const GAIN: f32 = 1.0 / (2.0 * Self::TOTAL_FILTERS_COUNT);

    /// Creates new instance of reverb effect with cutoff frequency of ~11.2 kHz and
    /// 5 seconds decay time.
    pub fn new() -> Self {
        let fc = 0.25615; // 11296 Hz at 44100 Hz sample rate

        let decay_time = 2.0;

        Self {
            dry: 1.0,
            wet: 1.0,
            decay_time: 2.0,
            fc,
            left: ChannelReverb::new(0, fc, Reverb::FEEDBACK, decay_time),
            right: ChannelReverb::new(23, fc, Reverb::FEEDBACK, decay_time),
        }
    }

    /// Sets how much of input signal should be passed to output without any processing.
    /// Default value is 1.0.
    pub fn set_dry(&mut self, dry: f32) {
        self.dry = dry.clamp(0.0, 1.0);
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
        self.wet = wet.clamp(0.0, 1.0);
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
    pub fn set_decay_time(&mut self, decay_time: f32) {
        self.decay_time = decay_time;
        self.left.set_decay_time(decay_time);
        self.right.set_decay_time(decay_time)
    }

    /// Returns current decay time.
    pub fn decay_time(&self) -> f32 {
        self.decay_time
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
        self.fc = fc;
        self.left.set_fc(fc);
        self.right.set_fc(fc);
    }

    /// Returns current cutoff frequency.
    pub fn fc(&self) -> f32 {
        self.fc
    }
}

impl EffectRenderTrait for Reverb {
    fn render(&mut self, input: &[(f32, f32)], mix_buf: &mut [(f32, f32)]) {
        let wet = self.wet;
        let dry = 1.0 - self.wet;

        for ((out_left, out_right), &(left, right)) in mix_buf.iter_mut().zip(input.iter()) {
            let mid = (left + right) * 0.5;
            let input = mid * Self::GAIN;

            let processed_left = self.left.feed(input);
            let processed_right = self.right.feed(input);

            *out_left = processed_left * wet + processed_right * dry + self.dry * left;
            *out_right = processed_right * wet + processed_left * dry + self.dry * right;
        }
    }
}
