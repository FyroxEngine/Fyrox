// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Contains various filter effects. For example, lowpass filter could be used to muffle sounds.

use crate::{
    context::SAMPLE_RATE,
    dsp::{filters::Biquad, filters::BiquadKind},
    effects::EffectRenderTrait,
};
use fyrox_core::{reflect::prelude::*, visitor::prelude::*};

macro_rules! define_filter_effect {
    ($(#[$attr:meta])* $name:ident, $kind:expr) => {
        $(#[$attr])*
        #[derive(Clone, Reflect, Visit, Debug, PartialEq)]
        pub struct $name {
            /// Cutoff frequency in Hertz.
            #[reflect(setter = "set_cutoff_frequency_hz")]
            cutoff_frequency_hz: f32,

            /// Gain of the effect.
            #[reflect(setter = "set_gain")]
            gain: f32,

            /// Band width at the cutoff frequency, the higher the value the wider the band.
            #[reflect(setter = "set_quality")]
            quality: f32,

            #[reflect(hidden)]
            left: Biquad,
            #[reflect(hidden)]
            right: Biquad,
        }

        impl Default for $name {
            fn default() -> Self {
                let mut filter = Self {
                    cutoff_frequency_hz: 2200.0,
                    gain: 1.0,
                    quality: 0.5,
                    left: Default::default(),
                    right: Default::default(),
                };
                filter.update();
                filter
            }
        }

        impl EffectRenderTrait for $name {
            fn render(&mut self, input: &[(f32, f32)], output: &mut [(f32, f32)]) {
                for ((input_left, input_right), (output_left, output_right)) in input.iter().zip(output) {
                    *output_left = self.left.feed(*input_left);
                    *output_right = self.right.feed(*input_right);
                }
            }
        }

        impl $name {
            /// Sets the gain of the filter. The value is usually in `[0.0..1.0]` range.
            #[inline]
            pub fn set_gain(&mut self, gain: f32) {
                self.gain = gain;
                self.update();
            }

            /// Returns filter's gain coefficient.
            #[inline]
            pub fn gain(&self) -> f32 {
                self.gain
            }

            /// Sets a cutoff frequency of the filter in Hertz. Its exact meaning depends on an actual filter type, but
            /// in general it defines a frequency at which the sound starts to decay.
            #[inline]
            pub fn set_cutoff_frequency_hz(&mut self, freq: f32) {
                self.cutoff_frequency_hz = freq;
                self.update();
            }

            /// Returns a cutoff frequency of the filter in Hertz.
            #[inline]
            pub fn cutoff_frequency_hz(&self) -> f32 {
                self.cutoff_frequency_hz
            }

            /// Quality defines a band width at which amplitude decays by half (or by 3 db in log scale), the lower it will
            /// be, the wider band will be and vice versa. See more info [here](https://ccrma.stanford.edu/~jos/filters/Quality_Factor_Q.html)
            #[inline]
            pub fn set_quality(&mut self, quality: f32) {
                self.quality = quality;
                self.update();
            }

            /// Returns the quality of the filter.
            #[inline]
            pub fn quality(&self) -> f32 {
                self.quality
            }

            fn update(&mut self) {
                self.left.tune(
                    $kind,
                    self.cutoff_frequency_hz / SAMPLE_RATE as f32,
                    self.gain,
                    self.quality
                );

                self.right.tune(
                    $kind,
                    self.cutoff_frequency_hz / SAMPLE_RATE as f32,
                    self.gain,
                    self.quality
                )
            }
        }
    };
}

define_filter_effect!(
    /// Lowpass filter defines a filter that passes through every frequency below the cutoff frequency.
    LowPassFilterEffect,
    BiquadKind::LowPass
);
define_filter_effect!(
    /// Highpass filter defines a filter that passes through every frequency upper the cutoff frequency.
    HighPassFilterEffect,
    BiquadKind::HighPass
);
define_filter_effect!(
    /// Bandpass filter defines a filter that passes a band of frequencies surrounding the cutoff frequency.
    BandPassFilterEffect,
    BiquadKind::BandPass
);
define_filter_effect!(
    /// Equally passes through each frequency, but shifts the phase of the signal by 90 degrees at the cutoff frequency.
    AllPassFilterEffect,
    BiquadKind::AllPass
);
define_filter_effect!(
    /// Reduces amplitude of frequencies in a shape like this ̅ \_ at the cutoff frequency.
    LowShelfFilterEffect,
    BiquadKind::LowShelf
);
define_filter_effect!(
    /// Reduces amplitude of frequencies in a shape like this _/̅  at the cutoff frequency.
    HighShelfFilterEffect,
    BiquadKind::HighShelf
);
