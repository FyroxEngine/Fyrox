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
    dsp::{filters::Biquad, filters::BiquadKind},
    effects::EffectRenderTrait,
};
use fyrox_core::{reflect::prelude::*, visitor::prelude::*};

macro_rules! define_filter_effect {
    ($(#[$attr:meta])* $name:ident, $kind:expr) => {
        #[derive(Clone, Reflect, Visit, Debug, PartialEq)]
         $(#[$attr])*
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
            #[visit(skip)]
            sample_rate: u32,

            #[reflect(hidden)]
            left: Biquad,
            #[reflect(hidden)]
            right: Biquad,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    cutoff_frequency_hz: 2200.0,
                    gain: 1.0,
                    quality: 0.5,
                    left: Default::default(),
                    right: Default::default(),
                    sample_rate: 0
                }
            }
        }

        impl EffectRenderTrait for $name {
            fn render(&mut self, sample_rate: u32, input: &[(f32, f32)], output: &mut [(f32, f32)]) {
                if self.sample_rate != sample_rate {
                    self.sample_rate = sample_rate;

                    self.left.tune(
                        $kind,
                        self.cutoff_frequency_hz / self.sample_rate as f32,
                        self.gain,
                        self.quality
                    );

                    self.right.tune(
                        $kind,
                        self.cutoff_frequency_hz / self.sample_rate as f32,
                        self.gain,
                        self.quality
                    );
                }

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
                self.sample_rate = 0;
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
                self.sample_rate = 0;
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
                self.sample_rate = 0;
            }

            /// Returns the quality of the filter.
            #[inline]
            pub fn quality(&self) -> f32 {
                self.quality
            }
        }
    };
}

define_filter_effect!(
    /// Lowpass filter defines a filter that passes through every frequency below the cutoff frequency.
    #[reflect(type_uuid = "88d111d2-50a7-4dd5-bb66-73bede961885")]
    LowPassFilterEffect,
    BiquadKind::LowPass
);
define_filter_effect!(
    /// Highpass filter defines a filter that passes through every frequency upper the cutoff frequency.
    #[reflect(type_uuid = "48362041-466b-494c-9cee-706368f5b924")]
    HighPassFilterEffect,
    BiquadKind::HighPass
);
define_filter_effect!(
    /// Bandpass filter defines a filter that passes a band of frequencies surrounding the cutoff frequency.
    #[reflect(type_uuid = "7e07703c-3202-4a75-8268-9bf704eb59f1")]
    BandPassFilterEffect,
    BiquadKind::BandPass
);
define_filter_effect!(
    /// Equally passes through each frequency, but shifts the phase of the signal by 90 degrees at the cutoff frequency.
    #[reflect(type_uuid = "47f0e07c-e030-4166-a2d0-de123d3b1bda")]
    AllPassFilterEffect,
    BiquadKind::AllPass
);
define_filter_effect!(
    /// Reduces amplitude of frequencies in a shape like this ̅ \_ at the cutoff frequency.
    #[reflect(type_uuid = "79a4a2af-a544-4852-a360-d05d94474c5c")]
    LowShelfFilterEffect,
    BiquadKind::LowShelf
);
define_filter_effect!(
    /// Reduces amplitude of frequencies in a shape like this _/̅  at the cutoff frequency.
    #[reflect(type_uuid = "a80fb390-5e3c-4dc4-8d97-22c7aebc5d2b")]
    HighShelfFilterEffect,
    BiquadKind::HighShelf
);
