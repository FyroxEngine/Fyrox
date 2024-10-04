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
    gl::server::GlGraphicsServer,
    gpu_program::{GpuProgram, UniformLocation},
};

pub struct MapShader {
    pub program: GpuProgram,
    pub hdr_sampler: UniformLocation,
    pub lum_sampler: UniformLocation,
    pub bloom_sampler: UniformLocation,
    pub color_map_sampler: UniformLocation,
    pub uniform_buffer_binding: usize,
}

impl MapShader {
    pub fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_map.glsl");
        let vertex_source = include_str!("../shaders/hdr_map_vs.glsl");

        let program =
            GpuProgram::from_source(server, "HdrToLdrShader", vertex_source, fragment_source)?;

        Ok(Self {
            hdr_sampler: program.uniform_location(server, &ImmutableString::new("hdrSampler"))?,
            lum_sampler: program.uniform_location(server, &ImmutableString::new("lumSampler"))?,
            bloom_sampler: program
                .uniform_location(server, &ImmutableString::new("bloomSampler"))?,
            color_map_sampler: program
                .uniform_location(server, &ImmutableString::new("colorMapSampler"))?,
            uniform_buffer_binding: program
                .uniform_block_index(server, &ImmutableString::new("Uniforms"))?,
            program,
        })
    }
}
