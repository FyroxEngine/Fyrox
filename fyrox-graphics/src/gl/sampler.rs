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

use crate::sampler::{Coordinate, MagnificationFilter, MinificationFilter, WrapMode};
use crate::{
    error::FrameworkError,
    gl::{server::GlGraphicsServer, ToGlConstant},
    sampler::{GpuSamplerDescriptor, GpuSamplerTrait},
};
use glow::HasContext;
use std::rc::Weak;

impl ToGlConstant for MinificationFilter {
    fn into_gl(self) -> u32 {
        match self {
            Self::Nearest => glow::NEAREST,
            Self::NearestMipMapNearest => glow::NEAREST_MIPMAP_NEAREST,
            Self::NearestMipMapLinear => glow::NEAREST_MIPMAP_LINEAR,
            Self::Linear => glow::LINEAR,
            Self::LinearMipMapNearest => glow::LINEAR_MIPMAP_NEAREST,
            Self::LinearMipMapLinear => glow::LINEAR_MIPMAP_LINEAR,
        }
    }
}

impl ToGlConstant for MagnificationFilter {
    fn into_gl(self) -> u32 {
        match self {
            Self::Nearest => glow::NEAREST,
            Self::Linear => glow::LINEAR,
        }
    }
}

impl ToGlConstant for WrapMode {
    fn into_gl(self) -> u32 {
        match self {
            Self::Repeat => glow::REPEAT,
            Self::ClampToEdge => glow::CLAMP_TO_EDGE,
            Self::ClampToBorder => glow::CLAMP_TO_BORDER,
            Self::MirroredRepeat => glow::MIRRORED_REPEAT,
            Self::MirrorClampToEdge => glow::MIRROR_CLAMP_TO_EDGE,
        }
    }
}

impl ToGlConstant for Coordinate {
    fn into_gl(self) -> u32 {
        match self {
            Self::S => glow::TEXTURE_WRAP_S,
            Self::T => glow::TEXTURE_WRAP_T,
            Self::R => glow::TEXTURE_WRAP_R,
        }
    }
}

#[derive(Debug)]
pub struct GlSampler {
    state: Weak<GlGraphicsServer>,
    pub(crate) id: glow::Sampler,
}

impl GpuSamplerTrait for GlSampler {}

impl GlSampler {
    pub fn new(
        server: &GlGraphicsServer,
        desc: GpuSamplerDescriptor,
    ) -> Result<Self, FrameworkError> {
        unsafe {
            let gl = &server.gl;
            let id = gl.create_sampler()?;
            let GpuSamplerDescriptor {
                min_filter,
                mag_filter,
                s_wrap_mode,
                t_wrap_mode,
                r_wrap_mode,
                anisotropy,
                min_lod,
                max_lod,
                lod_bias,
            } = desc;
            gl.bind_sampler(0, Some(id));
            gl.sampler_parameter_i32(id, glow::TEXTURE_MAG_FILTER, mag_filter.into_gl() as i32);
            gl.sampler_parameter_i32(id, glow::TEXTURE_MIN_FILTER, min_filter.into_gl() as i32);
            gl.sampler_parameter_f32(id, glow::TEXTURE_LOD_BIAS, lod_bias);
            gl.sampler_parameter_f32(id, glow::TEXTURE_MIN_LOD, min_lod);
            gl.sampler_parameter_f32(id, glow::TEXTURE_MAX_LOD, max_lod);
            gl.sampler_parameter_f32(id, glow::TEXTURE_MAX_ANISOTROPY, anisotropy);
            gl.sampler_parameter_i32(id, glow::TEXTURE_WRAP_S, s_wrap_mode.into_gl() as i32);
            gl.sampler_parameter_i32(id, glow::TEXTURE_WRAP_T, t_wrap_mode.into_gl() as i32);
            gl.sampler_parameter_i32(id, glow::TEXTURE_WRAP_R, r_wrap_mode.into_gl() as i32);
            gl.bind_sampler(0, None);

            Ok(Self {
                state: server.weak(),
                id,
            })
        }
    }
}

impl Drop for GlSampler {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            unsafe {
                state.gl.delete_sampler(self.id);
            }
        }
    }
}
