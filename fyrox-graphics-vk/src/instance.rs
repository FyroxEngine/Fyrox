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

use ash::{vk, Entry, Instance};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::CString;

/// Vulkan instance wrapper that manages the Vulkan instance creation and validation layers.
pub struct VkInstance {
    /// The Ash entry point.
    pub entry: Entry,
    /// The Vulkan instance.
    pub instance: Instance,
    /// Debug messenger for validation layers (if enabled).
    pub debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    /// Debug utils loader for validation layers.
    pub debug_utils: Option<ash::ext::debug_utils::Instance>,
}

impl VkInstance {
    /// Creates a new Vulkan instance with optional validation layers.
    pub fn new(
        _window: Option<&dyn HasWindowHandle>,
        display: Option<&dyn HasDisplayHandle>,
        enable_validation: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { Entry::load()? };

        let app_name = CString::new("Fyrox Engine")?;
        let engine_name = CString::new("Fyrox")?;

        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_2)
            .build();

        // Get required extensions
        let display_handle = display
            .map(|d| d.display_handle().unwrap().as_raw())
            .unwrap_or(raw_window_handle::RawDisplayHandle::Android(
                raw_window_handle::AndroidDisplayHandle::new(),
            ));
        let mut extensions = ash_window::enumerate_required_extensions(display_handle)
            .map_err(|e| format!("Failed to enumerate required extensions: {}", e))?
            .to_vec();

        // Add debug utils extension if validation is enabled
        if enable_validation {
            extensions.push(ash::ext::debug_utils::NAME.as_ptr());
        }

        // Validation layers
        let layer_names = if enable_validation {
            vec![CString::new("VK_LAYER_KHRONOS_validation")?]
        } else {
            vec![]
        };

        let layer_names_raw: Vec<*const i8> =
            layer_names.iter().map(|name| name.as_ptr()).collect();

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_names_raw)
            .enabled_extension_names(&extensions)
            .build();

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        // Setup debug messenger if validation is enabled
        let (debug_utils, debug_messenger) = if enable_validation {
            let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);

            let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback))
                .build();
            debug_info.message_severity = vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO;

            let debug_messenger =
                unsafe { debug_utils.create_debug_utils_messenger(&debug_info, None)? };

            (Some(debug_utils), Some(debug_messenger))
        } else {
            (None, None)
        };

        Ok(Self {
            entry,
            instance,
            debug_messenger,
            debug_utils,
        })
    }

    /// Gets available physical devices.
    pub fn enumerate_physical_devices(&self) -> Result<Vec<vk::PhysicalDevice>, vk::Result> {
        unsafe { self.instance.enumerate_physical_devices() }
    }
}

impl Drop for VkInstance {
    fn drop(&mut self) {
        unsafe {
            if let (Some(debug_utils), Some(debug_messenger)) =
                (&self.debug_utils, self.debug_messenger)
            {
                debug_utils.destroy_debug_utils_messenger(debug_messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

/// Debug callback for Vulkan validation layers.
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        std::borrow::Cow::from("")
    } else {
        std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "VERBOSE",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "INFO",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "WARNING",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "ERROR",
        _ => "UNKNOWN",
    };

    let type_str = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "GENERAL",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "VALIDATION",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "PERFORMANCE",
        _ => "UNKNOWN",
    };

    log::debug!(
        "[Vulkan {}][{}] ID: {} Name: {}\n{}",
        severity,
        type_str,
        message_id_number,
        message_id_name,
        message
    );

    vk::FALSE
}
