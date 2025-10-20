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

use crate::device::{SwapchainSupportDetails, VkDevice};
use ash::{vk, Device};
use std::cmp::{max, min};
use std::sync::{Arc, Weak};

/// Vulkan swapchain wrapper.
pub struct VkSwapchain {
    /// The swapchain loader.
    pub loader: ash::khr::swapchain::Device,
    /// The swapchain handle.
    pub swapchain: vk::SwapchainKHR,
    /// Swapchain images.
    pub images: Vec<vk::Image>,
    /// Swapchain image views.
    pub image_views: Vec<vk::ImageView>,
    /// Swapchain image format.
    pub format: vk::Format,
    /// Swapchain extent.
    pub extent: vk::Extent2D,
    /// Current frame index.
    pub current_frame: usize,
    /// Maximum frames in flight.
    pub max_frames_in_flight: usize,
    /// Weak device reference for cleanup (avoids circular reference)
    device: Weak<VkDevice>,
}

impl VkSwapchain {
    /// Creates a new swapchain.
    pub fn new(
        instance: &ash::Instance,
        device: &Arc<VkDevice>,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::khr::surface::Instance,
        width: u32,
        height: u32,
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let support =
            SwapchainSupportDetails::query(surface_loader, device.physical_device, surface);

        let surface_format = Self::choose_swap_surface_format(&support.formats);
        let present_mode = Self::choose_swap_present_mode(&support.present_modes);
        let extent = Self::choose_swap_extent(&support.capabilities, width, height);

        let mut image_count = support.capabilities.min_image_count + 1;
        if support.capabilities.max_image_count > 0
            && image_count > support.capabilities.max_image_count
        {
            image_count = support.capabilities.max_image_count;
        }

        let queue_family_indices = [
            device.queue_families.graphics_family.unwrap(),
            device.queue_families.present_family.unwrap(),
        ];

        let (image_sharing_mode, queue_family_index_count) =
            if device.queue_families.graphics_family != device.queue_families.present_family {
                (vk::SharingMode::CONCURRENT, 2)
            } else {
                (vk::SharingMode::EXCLUSIVE, 0)
            };

        let mut create_info = vk::SwapchainCreateInfoKHR::default();
        create_info.surface = surface;
        create_info.min_image_count = image_count;
        create_info.image_format = surface_format.format;
        create_info.image_color_space = surface_format.color_space;
        create_info.image_extent = extent;
        create_info.image_array_layers = 1;
        create_info.image_usage = vk::ImageUsageFlags::COLOR_ATTACHMENT;
        create_info.image_sharing_mode = image_sharing_mode;
        create_info.queue_family_index_count = queue_family_index_count;
        create_info.p_queue_family_indices = if queue_family_index_count > 0 {
            queue_family_indices.as_ptr()
        } else {
            std::ptr::null()
        };
        create_info.pre_transform = support.capabilities.current_transform;
        create_info.composite_alpha = vk::CompositeAlphaFlagsKHR::OPAQUE;
        create_info.present_mode = present_mode;
        create_info.clipped = vk::TRUE;
        create_info.old_swapchain = old_swapchain.unwrap_or(vk::SwapchainKHR::null());

        let swapchain_loader = ash::khr::swapchain::Device::new(instance, &device.device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None)? };

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

        let image_views = Self::create_image_views(&device.device, &images, surface_format.format)?;

        Ok(Self {
            loader: swapchain_loader,
            swapchain,
            images,
            image_views,
            format: surface_format.format,
            extent,
            current_frame: 0,
            max_frames_in_flight: image_count as usize,
            device: Arc::downgrade(device),
        })
    }

    /// Creates image views for swapchain images.
    fn create_image_views(
        device: &Device,
        images: &[vk::Image],
        format: vk::Format,
    ) -> Result<Vec<vk::ImageView>, vk::Result> {
        let mut image_views = Vec::with_capacity(images.len());

        for &image in images {
            let components = vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            };
            let subresource_range = vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            };
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(components)
                .subresource_range(subresource_range)
                .build();

            let image_view = unsafe { device.create_image_view(&create_info, None)? };
            image_views.push(image_view);
        }

        Ok(image_views)
    }

    /// Chooses the best surface format.
    fn choose_swap_surface_format(
        available_formats: &[vk::SurfaceFormatKHR],
    ) -> vk::SurfaceFormatKHR {
        // Prefer sRGB color space with BGRA format
        for &format in available_formats {
            if format.format == vk::Format::B8G8R8A8_SRGB
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                return format;
            }
        }

        // Fallback to first available format
        available_formats[0]
    }

    /// Chooses the best present mode.
    fn choose_swap_present_mode(
        available_present_modes: &[vk::PresentModeKHR],
    ) -> vk::PresentModeKHR {
        // Prefer mailbox mode for low latency
        for &present_mode in available_present_modes {
            if present_mode == vk::PresentModeKHR::MAILBOX {
                return present_mode;
            }
        }

        // FIFO is guaranteed to be available
        vk::PresentModeKHR::FIFO
    }

    /// Chooses the swap extent (resolution).
    fn choose_swap_extent(
        capabilities: &vk::SurfaceCapabilitiesKHR,
        window_width: u32,
        window_height: u32,
    ) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: max(
                    capabilities.min_image_extent.width,
                    min(capabilities.max_image_extent.width, window_width),
                ),
                height: max(
                    capabilities.min_image_extent.height,
                    min(capabilities.max_image_extent.height, window_height),
                ),
            }
        }
    }

    /// Acquires the next image from the swapchain.
    pub fn acquire_next_image(
        &mut self,
        timeout: u64,
        image_available_semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> Result<(u32, bool), vk::Result> {
        unsafe {
            self.loader.acquire_next_image(
                self.swapchain,
                timeout,
                image_available_semaphore,
                fence,
            )
        }
    }

    /// Presents an image to the swapchain.
    pub fn present(
        &self,
        wait_semaphores: &[vk::Semaphore],
        image_index: u32,
        present_queue: vk::Queue,
    ) -> Result<bool, vk::Result> {
        let swapchains = [self.swapchain];
        let image_indices = [image_index];

        let mut present_info = vk::PresentInfoKHR::default();
        present_info.wait_semaphore_count = wait_semaphores.len() as u32;
        present_info.p_wait_semaphores = if wait_semaphores.is_empty() {
            std::ptr::null()
        } else {
            wait_semaphores.as_ptr()
        };
        present_info.swapchain_count = 1;
        present_info.p_swapchains = swapchains.as_ptr();
        present_info.p_image_indices = image_indices.as_ptr();
        present_info.p_results = std::ptr::null_mut();

        unsafe { self.loader.queue_present(present_queue, &present_info) }
    }

    /// Recreates the swapchain with new dimensions.
    pub fn recreate(
        &mut self,
        instance: &ash::Instance,
        device: &Arc<VkDevice>,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::khr::surface::Instance,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            device.device.device_wait_idle()?;
        }

        // Clean up old swapchain
        self.cleanup(&device.device);

        // Create new swapchain
        let new_swapchain = Self::new(
            instance,
            device,
            surface,
            surface_loader,
            width,
            height,
            Some(self.swapchain),
        )?;

        // Update self with new swapchain data
        self.swapchain = new_swapchain.swapchain;
        self.images = new_swapchain.images.clone();
        self.image_views = new_swapchain.image_views.clone();
        self.format = new_swapchain.format;
        self.extent = new_swapchain.extent;

        Ok(())
    }

    /// Cleans up swapchain resources.
    fn cleanup(&mut self, device: &Device) {
        unsafe {
            for &image_view in &self.image_views {
                device.destroy_image_view(image_view, None);
            }
            self.image_views.clear();
        }
    }
}

impl Drop for VkSwapchain {
    fn drop(&mut self) {
        unsafe {
            // Destroy image views first (child objects)
            if let Some(device) = self.device.upgrade() {
                for &image_view in &self.image_views {
                    device.device.destroy_image_view(image_view, None);
                }
                self.image_views.clear();
            }

            // Then destroy the swapchain (parent object)
            // Note: swapchain images are owned by the swapchain and will be destroyed with it
            if self.swapchain != vk::SwapchainKHR::null() {
                self.loader.destroy_swapchain(self.swapchain, None);
            }
        }
    }
}
