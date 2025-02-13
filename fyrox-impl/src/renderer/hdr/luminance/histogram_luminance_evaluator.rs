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

use std::fmt::Debug;
use std::ops::Range;
use fyrox_core::info;
use crate::renderer::hdr::luminance::luminance_evaluator::LuminanceEvaluator;

pub struct HistogramLuminanceEvaluator {
  bin_count: usize,
  luminance_value_range: Range<f32>,
  sample_count: usize,
}

impl LuminanceEvaluator for HistogramLuminanceEvaluator {
  fn average_luminance(self, data: &[f32]) -> f32 {
    let mut histogram = LuminanceHistogram::new(self.bin_count, self.luminance_value_range);

    for value in data {
      // pixel value to bin index
      histogram.push_value(*value);
    }

    let h = histogram
      .reduce_to_biggest_samples(self.sample_count);

    info!("{:?}", h);

    h.average_histogram_value()

  }
}

impl Default for HistogramLuminanceEvaluator {
  fn default() -> Self {
    Self {
      bin_count: 128,
      luminance_value_range: 0.0f32..1.0f32,
      sample_count: 5,
    }
  }
}


struct LuminanceHistogram{
  bins: Vec<Vec<f32>>,
  bin_width: f64,
}


impl LuminanceHistogram {
  pub(crate) fn new(bin_count: usize, value_range: Range<f32>) -> Self {

    let bin_width = (value_range.end as f64 - value_range.start as f64) / bin_count as f64;

    let mut bins = Vec::with_capacity(bin_count);
    for _ in 0..bin_count {
      bins.push(Vec::<f32>::new());
    }

    LuminanceHistogram {
      bins,
      bin_width,
    }
  }

  pub(crate) fn push_value(&mut self, value: f32) {

    let bin_index: usize = (value.abs() / self.bin_width as f32).floor() as usize;
    // info!("value: {}; index: {}", value, bin_index);
    self.bins[bin_index].push(value as f32);

  }

  fn reduce_to_biggest_samples(self, sample_count: usize) -> Self {

    let mut biggest_bins = Vec::<Vec<f32>>::with_capacity(sample_count);

    let mut bins = self.bins;

    for _ in 0..sample_count {

      let mut index = 0;

      for j in 0..bins.len() {
        if bins[j].len() > bins[index].len() {
          index = j;
        }
      }

      let biggest_bin = bins.swap_remove(index);
      biggest_bins.push(biggest_bin);

    }

    Self {
      bins: biggest_bins,
      bin_width: self.bin_width,
    }
  }

  fn average_histogram_value(&self) -> f32 {
    let value_count = self.bins.iter().map(|b| b.len()).sum::<usize>();

    let sum = self.bins.iter().map(|bin| bin.iter().sum::<f32>()).sum::<f32>();

    sum / value_count as f32
  }

}

impl Debug for LuminanceHistogram {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

    let mut lengths = Vec::with_capacity(self.bins.len());

    for b in &self.bins{
      lengths.push(b.len());
    }

    write!(f, "LuminanceHistogram {:?}", lengths)
  }
}