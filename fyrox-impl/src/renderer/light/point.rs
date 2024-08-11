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
    state::PipelineState,
};

pub struct PointLightShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub depth_sampler: UniformLocation,
    pub color_sampler: UniformLocation,
    pub normal_sampler: UniformLocation,
    pub material_sampler: UniformLocation,
    pub point_shadow_texture: UniformLocation,
    pub shadows_enabled: UniformLocation,
    pub soft_shadows: UniformLocation,
    pub light_position: UniformLocation,
    pub light_radius: UniformLocation,
    pub light_color: UniformLocation,
    pub inv_view_proj_matrix: UniformLocation,
    pub camera_position: UniformLocation,
    pub shadow_bias: UniformLocation,
    pub light_intensity: UniformLocation,
    pub shadow_alpha: UniformLocation,
}

impl PointLightShader {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/deferred_point_light_fs.glsl");
        let vertex_source = include_str!("../shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source(state, "PointLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            depth_sampler: program
                .uniform_location(state, &ImmutableString::new("depthTexture"))?,
            color_sampler: program
                .uniform_location(state, &ImmutableString::new("colorTexture"))?,
            normal_sampler: program
                .uniform_location(state, &ImmutableString::new("normalTexture"))?,
            material_sampler: program
                .uniform_location(state, &ImmutableString::new("materialTexture"))?,
            point_shadow_texture: program
                .uniform_location(state, &ImmutableString::new("pointShadowTexture"))?,
            shadows_enabled: program
                .uniform_location(state, &ImmutableString::new("shadowsEnabled"))?,
            soft_shadows: program.uniform_location(state, &ImmutableString::new("softShadows"))?,
            light_position: program.uniform_location(state, &ImmutableString::new("lightPos"))?,
            light_radius: program.uniform_location(state, &ImmutableString::new("lightRadius"))?,
            light_color: program.uniform_location(state, &ImmutableString::new("lightColor"))?,
            inv_view_proj_matrix: program
                .uniform_location(state, &ImmutableString::new("invViewProj"))?,
            camera_position: program
                .uniform_location(state, &ImmutableString::new("cameraPosition"))?,
            shadow_bias: program.uniform_location(state, &ImmutableString::new("shadowBias"))?,
            light_intensity: program
                .uniform_location(state, &ImmutableString::new("lightIntensity"))?,
            shadow_alpha: program.uniform_location(state, &ImmutableString::new("shadowAlpha"))?,
            program,
        })
    }
}
