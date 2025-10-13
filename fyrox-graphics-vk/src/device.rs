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

use crate::instance::VkInstance;
use ash::{vk, Device, Instance};
use std::collections::HashSet;
use std::ffi::CStr;

/// Queue family indices for different types of operations.
#[derive(Debug, Clone)]
pub struct QueueFamilyIndices {
    /// Graphics queue family index.
    pub graphics_family: Option<u32>,
    /// Presentation queue family index.
    pub present_family: Option<u32>,
    /// Transfer queue family index.
    pub transfer_family: Option<u32>,
    /// Compute queue family index.
    pub compute_family: Option<u32>,
}

impl QueueFamilyIndices {
    /// Creates a new instance with no queue families found.
    pub fn new() -> Self {
        Self {
            graphics_family: None,
            present_family: None,
            transfer_family: None,
            compute_family: None,
        }
    }

    /// Checks if all required queue families are found.
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }

    /// Gets unique queue families as a set.
    pub fn unique_families(&self) -> HashSet<u32> {
        let mut families = HashSet::new();
        if let Some(graphics) = self.graphics_family {
            families.insert(graphics);
        }
        if let Some(present) = self.present_family {
            families.insert(present);
        }
        if let Some(transfer) = self.transfer_family {
            families.insert(transfer);
        }
        if let Some(compute) = self.compute_family {
            families.insert(compute);
        }
        families
    }
}

/// Vulkan logical device wrapper.
pub struct VkDevice {
    /// The physical device.
    pub physical_device: vk::PhysicalDevice,
    /// The logical device.
    pub device: Device,
    /// Queue family indices.
    pub queue_families: QueueFamilyIndices,
    /// Graphics queue.
    pub graphics_queue: vk::Queue,
    /// Presentation queue.
    pub present_queue: vk::Queue,
    /// Transfer queue (may be the same as graphics queue).
    pub transfer_queue: vk::Queue,
    /// Compute queue (may be the same as graphics queue).
    pub compute_queue: vk::Queue,
    /// Physical device properties.
    pub properties: vk::PhysicalDeviceProperties,
    /// Physical device features.
    pub features: vk::PhysicalDeviceFeatures,
    /// Physical device memory properties.
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
}

impl VkDevice {
    /// Creates a new Vulkan device.
    pub fn new(
        vk_instance: &VkInstance,
        surface: Option<vk::SurfaceKHR>,
        surface_loader: Option<&ash::khr::surface::Instance>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let physical_device = Self::pick_physical_device(vk_instance, surface, surface_loader)?;
        let queue_families = Self::find_queue_families(
            &vk_instance.instance,
            physical_device,
            surface,
            surface_loader,
        );

        if !queue_families.is_complete() {
            return Err("Failed to find suitable queue families".into());
        }

        let unique_queue_families = queue_families.unique_families();
        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique_queue_families
            .iter()
            .map(|&queue_family| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(queue_family)
                    .queue_priorities(&[1.0])
            })
            .collect();

        // Required device extensions
        let device_extensions = [ash::khr::swapchain::NAME.as_ptr()];

        let device_features = vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true)
            .fill_mode_non_solid(true)
            .geometry_shader(true)
            .tessellation_shader(true);

        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions)
            .enabled_features(&device_features);

        let device = unsafe {
            vk_instance
                .instance
                .create_device(physical_device, &create_info, None)?
        };

        let graphics_queue =
            unsafe { device.get_device_queue(queue_families.graphics_family.unwrap(), 0) };
        let present_queue =
            unsafe { device.get_device_queue(queue_families.present_family.unwrap(), 0) };
        let transfer_queue = unsafe {
            device.get_device_queue(
                queue_families
                    .transfer_family
                    .unwrap_or(queue_families.graphics_family.unwrap()),
                0,
            )
        };
        let compute_queue = unsafe {
            device.get_device_queue(
                queue_families
                    .compute_family
                    .unwrap_or(queue_families.graphics_family.unwrap()),
                0,
            )
        };

        let properties = unsafe {
            vk_instance
                .instance
                .get_physical_device_properties(physical_device)
        };
        let features = unsafe {
            vk_instance
                .instance
                .get_physical_device_features(physical_device)
        };
        let memory_properties = unsafe {
            vk_instance
                .instance
                .get_physical_device_memory_properties(physical_device)
        };

        Ok(Self {
            physical_device,
            device,
            queue_families,
            graphics_queue,
            present_queue,
            transfer_queue,
            compute_queue,
            properties,
            features,
            memory_properties,
        })
    }

    /// Picks a suitable physical device.
    fn pick_physical_device(
        vk_instance: &VkInstance,
        surface: Option<vk::SurfaceKHR>,
        surface_loader: Option<&ash::khr::surface::Instance>,
    ) -> Result<vk::PhysicalDevice, Box<dyn std::error::Error>> {
        let devices = vk_instance.enumerate_physical_devices()?;
        if devices.is_empty() {
            return Err("No Vulkan-capable devices found".into());
        }

        // Find the best device (prefer discrete GPU)
        let mut chosen_device = None;
        let mut device_score = 0;

        for &device in &devices {
            let score = Self::rate_device_suitability(
                &vk_instance.instance,
                device,
                surface,
                surface_loader,
            );
            if score > device_score {
                device_score = score;
                chosen_device = Some(device);
            }
        }

        chosen_device.ok_or_else(|| "No suitable Vulkan device found".into())
    }

    /// Rates a physical device's suitability.
    fn rate_device_suitability(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: Option<vk::SurfaceKHR>,
        surface_loader: Option<&ash::khr::surface::Instance>,
    ) -> u32 {
        let properties = unsafe { instance.get_physical_device_properties(device) };
        let features = unsafe { instance.get_physical_device_features(device) };

        let mut score = 0;

        // Prefer discrete GPUs
        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            score += 1000;
        }

        // Maximum possible size of textures affects graphics quality
        score += properties.limits.max_image_dimension2_d;

        // Check if device supports required features
        if features.geometry_shader == 0 {
            return 0; // Must support geometry shaders
        }

        // Check queue families
        let queue_families = Self::find_queue_families(instance, device, surface, surface_loader);
        if !queue_families.is_complete() {
            return 0;
        }

        // Check device extensions
        if !Self::check_device_extension_support(instance, device) {
            return 0;
        }

        // Check swapchain support if surface is provided
        if let (Some(surface), Some(surface_loader)) = (surface, surface_loader) {
            let swapchain_support = SwapchainSupportDetails::query(surface_loader, device, surface);
            if swapchain_support.formats.is_empty() || swapchain_support.present_modes.is_empty() {
                return 0;
            }
        }

        score
    }

    /// Finds queue families for a physical device.
    fn find_queue_families(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: Option<vk::SurfaceKHR>,
        surface_loader: Option<&ash::khr::surface::Instance>,
    ) -> QueueFamilyIndices {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        let mut indices = QueueFamilyIndices::new();

        for (index, &queue_family) in queue_families.iter().enumerate() {
            let index = index as u32;

            // Graphics queue
            if indices.graphics_family.is_none()
                && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            {
                indices.graphics_family = Some(index);
            }

            // Transfer queue (prefer dedicated transfer queue)
            if queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                if indices.transfer_family.is_none() {
                    indices.transfer_family = Some(index);
                } else if !queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    // Prefer dedicated transfer queue
                    indices.transfer_family = Some(index);
                }
            }

            // Compute queue (prefer dedicated compute queue)
            if queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                if indices.compute_family.is_none() {
                    indices.compute_family = Some(index);
                } else if !queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    // Prefer dedicated compute queue
                    indices.compute_family = Some(index);
                }
            }

            // Present queue
            if let (Some(surface), Some(surface_loader)) = (surface, surface_loader) {
                let present_support = unsafe {
                    surface_loader
                        .get_physical_device_surface_support(device, index, surface)
                        .unwrap_or(false)
                };
                if indices.present_family.is_none() && present_support {
                    indices.present_family = Some(index);
                }
            } else if indices.present_family.is_none() {
                // If no surface, assume graphics queue can present
                indices.present_family = indices.graphics_family;
            }
        }

        // Fallback for transfer and compute queues
        if indices.transfer_family.is_none() {
            indices.transfer_family = indices.graphics_family;
        }
        if indices.compute_family.is_none() {
            indices.compute_family = indices.graphics_family;
        }

        indices
    }

    /// Checks if the device supports required extensions.
    fn check_device_extension_support(instance: &Instance, device: vk::PhysicalDevice) -> bool {
        let available_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .unwrap_or_default()
        };

        let required_extensions = [ash::khr::swapchain::NAME];

        for required in &required_extensions {
            let required_name = unsafe { CStr::from_ptr(required.as_ptr()) };
            let found = available_extensions.iter().any(|available| {
                let available_name =
                    unsafe { CStr::from_ptr(available.extension_name.as_ptr() as *const i8) };
                available_name == required_name
            });

            if !found {
                return false;
            }
        }

        true
    }

    /// Finds a suitable memory type for the given requirements.
    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        for i in 0..self.memory_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && self.memory_properties.memory_types[i as usize]
                    .property_flags
                    .contains(properties)
            {
                return Some(i);
            }
        }
        None
    }
}

impl Drop for VkDevice {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_device(None);
        }
    }
}

/// Swapchain support details for a physical device.
#[derive(Debug, Clone)]
pub struct SwapchainSupportDetails {
    /// Surface capabilities.
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    /// Available surface formats.
    pub formats: Vec<vk::SurfaceFormatKHR>,
    /// Available present modes.
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupportDetails {
    /// Queries swapchain support for a physical device.
    pub fn query(
        surface_loader: &ash::khr::surface::Instance,
        device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Self {
        let capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(device, surface)
                .unwrap_or_default()
        };

        let formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(device, surface)
                .unwrap_or_default()
        };

        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(device, surface)
                .unwrap_or_default()
        };

        Self {
            capabilities,
            formats,
            present_modes,
        }
    }
}
