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

use std::ops::{Deref, Range};
use std::rc::Rc;
use crate::{
    core::sstorage::ImmutableString,
    renderer::framework::{
        error::FrameworkError,
        gpu_program::{GpuProgram, UniformLocation},
        server::GraphicsServer,
    },
};

pub struct LuminanceShader {
    pub program: Box<dyn GpuProgram>,
    pub frame_sampler: UniformLocation,
    pub uniform_buffer_binding: usize,
}

impl LuminanceShader {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_luminance_fs.glsl");
        let vertex_source = include_str!("../shaders/hdr_luminance_vs.glsl");

        let program = server.create_program("LuminanceShader", vertex_source, fragment_source)?;

        Ok(Self {
            uniform_buffer_binding: program
                .uniform_block_index(&ImmutableString::new("Uniforms"))?,
            frame_sampler: program.uniform_location(&ImmutableString::new("frameSampler"))?,
            program,
        })
    }
}

pub struct HistogramDeviationWidth {
    value: usize
}

impl HistogramDeviationWidth {
    pub(crate) fn new(value: usize, histogram: &LuminanceHistogram) -> Result<Self, FrameworkError> {

        let maximum_allowed_deviation = histogram.bins.len() / 2;

        if value > maximum_allowed_deviation {
            return Err(FrameworkError::Custom("Invalid Histogram Deviation Width - Deviation Must Not Exceed Histogram Width Devided by Two".to_string()))
        }

        Ok(Self { value })
    }
    fn value(&self) -> usize {
        self.value
    }

    fn default() -> Self {
        Self{value:1}
    }
}

pub struct LuminanceHistogram{
    bins: Vec<Vec<f32>>,
    bin_width: f64,
}

impl LuminanceHistogram {
    pub(crate) fn new(bin_count: usize, value_range: Range<f32>) -> Self {

        let bin_width = (value_range.end as f64 - value_range.start as f64) / bin_count as f64;

        let mut bins = Vec::with_capacity(bin_count);
        for i in 0..bin_count {
            bins.push(Vec::<f32>::new());
        }

        LuminanceHistogram {
            bins,
            bin_width,
        }
    }

    pub(crate) fn push_value(&mut self, value: f32) {

        let bin_index: usize = (value / self.bin_width as f32).floor() as usize;

        self.bins[bin_index].push(value);

    }

    pub(crate) fn get_average_value(self, deviation_width: HistogramDeviationWidth) -> f32 {

        let biggest_bins = self.get_biggest_bins(deviation_width.value());

        let mut element_count = 0;
        let mut sum = 0.0;

        for b in biggest_bins {
            for v in b {
                sum = sum + v;
                element_count = element_count + 1;
            }
        }

        sum / element_count as f32
    }

    fn get_biggest_bins(mut self, amount: usize) -> Vec<Vec<f32>>{

        let mut biggest_bins = Vec::<Vec<f32>>::with_capacity(amount);

        for _ in 0..amount {

            let mut index = 0;

            for j in 0..self.bins.len() {
                if self.bins[j].len() > self.bins[index].len() {
                    index = j;
                }
            }

            let biggest_bin = self.bins.swap_remove(index);
            biggest_bins.push(biggest_bin);

        }

        biggest_bins
    }
}