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

use crate::core::sstorage::ImmutableString;
use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::GlGraphicsServer,
};

pub struct MapShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub hdr_sampler: UniformLocation,
    pub lum_sampler: UniformLocation,
    pub bloom_sampler: UniformLocation,
    pub color_map_sampler: UniformLocation,
    pub use_color_grading: UniformLocation,
    pub key_value: UniformLocation,
    pub min_luminance: UniformLocation,
    pub max_luminance: UniformLocation,
    pub auto_exposure: UniformLocation,
    pub fixed_exposure: UniformLocation,
}

impl MapShader {
    pub fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_map.glsl");
        let vertex_source = include_str!("../shaders/simple_vs.glsl");

        let program =
            GpuProgram::from_source(server, "HdrToLdrShader", vertex_source, fragment_source)?;

        Ok(Self {
            wvp_matrix: program
                .uniform_location(server, &ImmutableString::new("worldViewProjection"))?,
            hdr_sampler: program.uniform_location(server, &ImmutableString::new("hdrSampler"))?,
            lum_sampler: program.uniform_location(server, &ImmutableString::new("lumSampler"))?,
            bloom_sampler: program
                .uniform_location(server, &ImmutableString::new("bloomSampler"))?,
            color_map_sampler: program
                .uniform_location(server, &ImmutableString::new("colorMapSampler"))?,
            use_color_grading: program
                .uniform_location(server, &ImmutableString::new("useColorGrading"))?,
            key_value: program.uniform_location(server, &ImmutableString::new("keyValue"))?,
            min_luminance: program
                .uniform_location(server, &ImmutableString::new("minLuminance"))?,
            max_luminance: program
                .uniform_location(server, &ImmutableString::new("maxLuminance"))?,
            auto_exposure: program
                .uniform_location(server, &ImmutableString::new("autoExposure"))?,
            fixed_exposure: program
                .uniform_location(server, &ImmutableString::new("fixedExposure"))?,
            program,
        })
    }
}
