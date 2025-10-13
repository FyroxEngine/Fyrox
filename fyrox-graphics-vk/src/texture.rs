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

use crate::memory::{VkImage, VkMemoryManager};
use crate::ToVkType;
use ash::vk;
use fyrox_graphics::{
    error::FrameworkError,
    gpu_texture::{GpuTextureDescriptor, GpuTextureKind, GpuTextureTrait, PixelKind},
};
use gpu_allocator::MemoryLocation;
use std::rc::Rc;
use std::sync::Arc;

impl ToVkType<vk::Format> for PixelKind {
    fn to_vk(self) -> vk::Format {
        match self {
            PixelKind::R8 => vk::Format::R8_UNORM,
            PixelKind::RG8 => vk::Format::R8G8_UNORM,
            PixelKind::RGB8 => vk::Format::R8G8B8_UNORM,
            PixelKind::RGBA8 => vk::Format::R8G8B8A8_UNORM,
            PixelKind::R16 => vk::Format::R16_UNORM,
            PixelKind::RG16 => vk::Format::R16G16_UNORM,
            PixelKind::RGB16 => vk::Format::R16G16B16_UNORM,
            PixelKind::RGBA16 => vk::Format::R16G16B16A16_UNORM,
            PixelKind::R32F => vk::Format::R32_SFLOAT,
            PixelKind::RGB32F => vk::Format::R32G32B32_SFLOAT,
            PixelKind::RGBA32F => vk::Format::R32G32B32A32_SFLOAT,
            PixelKind::D32F => vk::Format::D32_SFLOAT,
            PixelKind::D16 => vk::Format::D16_UNORM,
            PixelKind::D24S8 => vk::Format::D24_UNORM_S8_UINT,
            PixelKind::R8UI => vk::Format::R8_UINT,
            PixelKind::R32UI => vk::Format::R32_UINT,
            _ => vk::Format::UNDEFINED, // Handle other variants as needed
        }
    }
}

/// Vulkan texture implementation.
pub struct VkGpuTexture {
    /// The underlying Vulkan image.
    image: VkImage,
    /// Image view for sampling.
    image_view: vk::ImageView,
    /// Texture kind.
    kind: GpuTextureKind,
    /// Pixel kind.
    pixel_kind: PixelKind,
    /// Memory manager reference.
    memory_manager: Arc<VkMemoryManager>,
}

impl VkGpuTexture {
    /// Creates a new Vulkan GPU texture.
    pub fn new(
        memory_manager: Arc<VkMemoryManager>,
        descriptor: GpuTextureDescriptor,
    ) -> Result<Self, FrameworkError> {
        let format = descriptor.pixel_kind.to_vk();

        let (image_type, extent, array_layers) = match &descriptor.kind {
            GpuTextureKind::Line { length } => (
                vk::ImageType::TYPE_1D,
                vk::Extent3D {
                    width: *length as u32,
                    height: 1,
                    depth: 1,
                },
                1,
            ),
            GpuTextureKind::Rectangle { width, height } => (
                vk::ImageType::TYPE_2D,
                vk::Extent3D {
                    width: *width as u32,
                    height: *height as u32,
                    depth: 1,
                },
                1,
            ),
            GpuTextureKind::Cube { size } => (
                vk::ImageType::TYPE_2D,
                vk::Extent3D {
                    width: *size as u32,
                    height: *size as u32,
                    depth: 1,
                },
                6,
            ),
            GpuTextureKind::Volume {
                width,
                height,
                depth,
            } => (
                vk::ImageType::TYPE_3D,
                vk::Extent3D {
                    width: *width as u32,
                    height: *height as u32,
                    depth: *depth as u32,
                },
                1,
            ),
        };

        let mip_levels = descriptor.mip_count.max(1) as u32;

        let usage = if descriptor.data.is_some() {
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED
        } else if Self::is_depth_format(format) {
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED
        } else {
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED
        };

        let image = memory_manager
            .create_image(
                image_type,
                format,
                extent,
                mip_levels,
                array_layers,
                vk::SampleCountFlags::TYPE_1,
                vk::ImageTiling::OPTIMAL,
                usage,
                MemoryLocation::GpuOnly,
                &descriptor.name,
            )
            .map_err(|e| FrameworkError::Custom(format!("Failed to create image: {}", e)))?;

        let view_type = match &descriptor.kind {
            GpuTextureKind::Line { .. } => vk::ImageViewType::TYPE_1D,
            GpuTextureKind::Rectangle { .. } => vk::ImageViewType::TYPE_2D,
            GpuTextureKind::Cube { .. } => vk::ImageViewType::CUBE,
            GpuTextureKind::Volume { .. } => vk::ImageViewType::TYPE_3D,
        };

        let aspect_mask = if Self::is_depth_format(format) {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let image_view = image
            .create_image_view(view_type, aspect_mask, 0, mip_levels, 0, array_layers)
            .map_err(|e| FrameworkError::Custom(format!("Failed to create image view: {:?}", e)))?;

        Ok(Self {
            image,
            image_view,
            kind: descriptor.kind,
            pixel_kind: descriptor.pixel_kind,
            memory_manager,
        })
    }

    /// Checks if the given format is a depth format.
    fn is_depth_format(format: vk::Format) -> bool {
        matches!(
            format,
            vk::Format::D16_UNORM
                | vk::Format::D32_SFLOAT
                | vk::Format::D24_UNORM_S8_UINT
                | vk::Format::D32_SFLOAT_S8_UINT
        )
    }

    /// Gets the Vulkan image handle.
    pub fn vk_image(&self) -> vk::Image {
        self.image.image
    }

    /// Gets the Vulkan image view handle.
    pub fn vk_image_view(&self) -> vk::ImageView {
        self.image_view
    }

    /// Gets the Vulkan format.
    pub fn vk_format(&self) -> vk::Format {
        self.image.format
    }
}

impl GpuTextureTrait for VkGpuTexture {
    fn set_data(
        &self,
        _kind: GpuTextureKind,
        _pixel_kind: PixelKind,
        _mip_count: usize,
        _data: Option<&[u8]>,
    ) -> Result<usize, FrameworkError> {
        // For now, return an error indicating it's not implemented
        // In a full implementation, this would update the texture data
        Err(FrameworkError::Custom(
            "set_data not implemented yet".to_string(),
        ))
    }

    fn kind(&self) -> GpuTextureKind {
        self.kind.clone()
    }

    fn pixel_kind(&self) -> PixelKind {
        self.pixel_kind
    }
}

impl Drop for VkGpuTexture {
    fn drop(&mut self) {
        unsafe {
            self.memory_manager
                .device()
                .device
                .destroy_image_view(self.image_view, None);
        }
    }
}

/// Creates a Vulkan GPU texture.
pub fn create_texture(
    memory_manager: Arc<VkMemoryManager>,
    descriptor: GpuTextureDescriptor,
) -> Result<Rc<dyn GpuTextureTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuTexture::new(memory_manager, descriptor)?))
}
