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

use crate::device::VkDevice;
use crate::ToVkType;
use ash::vk;
use fyrox_graphics::{
    error::FrameworkError,
    sampler::{GpuSamplerDescriptor, GpuSamplerTrait},
    CompareFunc,
};
use std::rc::Rc;
use std::sync::Arc;

// Filter and wrap mode implementations would go here when the enums are available

// SamplerMipMapMode implementation would go here when the enum is available

impl ToVkType<vk::CompareOp> for CompareFunc {
    fn to_vk(self) -> vk::CompareOp {
        match self {
            CompareFunc::Never => vk::CompareOp::NEVER,
            CompareFunc::Less => vk::CompareOp::LESS,
            CompareFunc::Equal => vk::CompareOp::EQUAL,
            CompareFunc::LessOrEqual => vk::CompareOp::LESS_OR_EQUAL,
            CompareFunc::Greater => vk::CompareOp::GREATER,
            CompareFunc::NotEqual => vk::CompareOp::NOT_EQUAL,
            CompareFunc::GreaterOrEqual => vk::CompareOp::GREATER_OR_EQUAL,
            CompareFunc::Always => vk::CompareOp::ALWAYS,
        }
    }
}

/// Vulkan sampler implementation.
pub struct VkGpuSampler {
    /// The sampler handle.
    sampler: vk::Sampler,
    /// Device reference.
    device: Arc<VkDevice>,
}

impl std::fmt::Debug for VkGpuSampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VkGpuSampler")
            .field("sampler", &self.sampler)
            .finish()
    }
}

impl VkGpuSampler {
    /// Creates a new Vulkan sampler.
    pub fn new(
        device: Arc<VkDevice>,
        descriptor: GpuSamplerDescriptor,
    ) -> Result<Self, FrameworkError> {
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(false)
            .max_anisotropy(1.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(descriptor.max_lod);

        let sampler = unsafe {
            device
                .device
                .create_sampler(&sampler_info, None)
                .map_err(|e| FrameworkError::Custom(format!("Failed to create sampler: {:?}", e)))?
        };

        Ok(Self { sampler, device })
    }

    /// Gets the Vulkan sampler handle.
    pub fn vk_sampler(&self) -> vk::Sampler {
        self.sampler
    }
}

impl GpuSamplerTrait for VkGpuSampler {
    // Empty trait implementation - the trait is a marker trait in current Fyrox version
}

impl Drop for VkGpuSampler {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_sampler(self.sampler, None);
        }
    }
}

/// Creates a Vulkan GPU sampler.
pub fn create_sampler(
    device: Arc<VkDevice>,
    descriptor: GpuSamplerDescriptor,
) -> Result<Rc<dyn GpuSamplerTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuSampler::new(device, descriptor)?))
}
