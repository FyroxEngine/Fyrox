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

use crate::renderer::cache::TemporaryCache;
use fyrox_core::err_once;
use fyrox_graphics::{
    error::FrameworkError,
    sampler::{
        GpuSampler, GpuSamplerDescriptor, MagnificationFilter, MinificationFilter, WrapMode,
    },
    server::GraphicsServer,
};
use fyrox_texture::{
    sampler::{TextureSampler, TextureSamplerResource},
    TextureMagnificationFilter, TextureMinificationFilter, TextureWrapMode,
};

pub fn convert_magnification_filter(v: TextureMagnificationFilter) -> MagnificationFilter {
    match v {
        TextureMagnificationFilter::Nearest => MagnificationFilter::Nearest,
        TextureMagnificationFilter::Linear => MagnificationFilter::Linear,
    }
}

pub fn convert_minification_filter(v: TextureMinificationFilter) -> MinificationFilter {
    match v {
        TextureMinificationFilter::Nearest => MinificationFilter::Nearest,
        TextureMinificationFilter::NearestMipMapNearest => MinificationFilter::NearestMipMapNearest,
        TextureMinificationFilter::NearestMipMapLinear => MinificationFilter::NearestMipMapLinear,
        TextureMinificationFilter::Linear => MinificationFilter::Linear,
        TextureMinificationFilter::LinearMipMapNearest => MinificationFilter::LinearMipMapNearest,
        TextureMinificationFilter::LinearMipMapLinear => MinificationFilter::LinearMipMapLinear,
    }
}

pub fn convert_wrap_mode(v: TextureWrapMode) -> WrapMode {
    match v {
        TextureWrapMode::Repeat => WrapMode::Repeat,
        TextureWrapMode::ClampToEdge => WrapMode::ClampToEdge,
        TextureWrapMode::ClampToBorder => WrapMode::ClampToBorder,
        TextureWrapMode::MirroredRepeat => WrapMode::MirroredRepeat,
        TextureWrapMode::MirrorClampToEdge => WrapMode::MirrorClampToEdge,
    }
}

fn create_sampler(
    server: &dyn GraphicsServer,
    sampler: &TextureSampler,
) -> Result<SamplerRenderData, FrameworkError> {
    Ok(SamplerRenderData {
        gpu_sampler: server.create_sampler(GpuSamplerDescriptor {
            mag_filter: convert_magnification_filter(sampler.magnification_filter()),
            min_filter: convert_minification_filter(sampler.minification_filter()),
            s_wrap_mode: convert_wrap_mode(sampler.s_wrap_mode()),
            t_wrap_mode: convert_wrap_mode(sampler.t_wrap_mode()),
            r_wrap_mode: convert_wrap_mode(sampler.r_wrap_mode()),
            anisotropy: sampler.anisotropy_level(),
            min_lod: sampler.min_lod(),
            max_lod: sampler.max_lod(),
            lod_bias: sampler.lod_bias(),
        })?,
        sampler_modifications_counter: sampler.modification_count,
    })
}

#[derive(Clone)]
pub struct SamplerRenderData {
    pub gpu_sampler: GpuSampler,
    sampler_modifications_counter: u64,
}

#[derive(Default)]
pub struct SamplerCache {
    cache: TemporaryCache<SamplerRenderData>,
}

impl SamplerCache {
    /// Unconditionally uploads requested texture into GPU memory, previous GPU texture will be automatically
    /// destroyed.
    pub fn upload(
        &mut self,
        server: &dyn GraphicsServer,
        sampler_resource: &TextureSamplerResource,
    ) -> Result<(), FrameworkError> {
        let sampler = sampler_resource.state();
        if let Some(sampler) = sampler.data_ref() {
            self.cache.get_entry_mut_or_insert_with(
                &sampler.cache_index,
                Default::default(),
                || create_sampler(server, sampler),
            )?;
            Ok(())
        } else {
            Err(FrameworkError::Custom(
                "Sampler is not loaded yet!".to_string(),
            ))
        }
    }

    pub fn get(
        &mut self,
        server: &dyn GraphicsServer,
        sampler_resource: &TextureSamplerResource,
    ) -> Option<&GpuSampler> {
        let sampler_data_guard = sampler_resource.state();
        if let Some(sampler) = sampler_data_guard.data_ref() {
            match self.cache.get_mut_or_insert_with(
                &sampler.cache_index,
                Default::default(),
                || create_sampler(server, sampler),
            ) {
                Ok(entry) => {
                    // Check if some value has changed in resource.

                    if entry.sampler_modifications_counter != sampler.sampler_modifications_count()
                    {
                        entry.gpu_sampler = create_sampler(server, sampler).unwrap();
                    }

                    return Some(entry);
                }
                Err(e) => {
                    drop(sampler_data_guard);
                    err_once!(
                        sampler_resource.key() as usize,
                        "Failed to create GPU sampler from sampler. Reason: {:?}",
                        e,
                    );
                }
            }
        }
        None
    }

    pub fn update(&mut self, dt: f32) {
        self.cache.update(dt)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn unload(&mut self, sampler: &TextureSamplerResource) {
        if let Some(sampler) = sampler.state().data() {
            self.cache.remove(&sampler.cache_index);
        }
    }

    pub fn alive_count(&self) -> usize {
        self.cache.alive_count()
    }
}
