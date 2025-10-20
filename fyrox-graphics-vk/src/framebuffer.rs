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
use crate::texture::VkGpuTexture;
use ash::vk;
use fyrox_graphics::{
    error::FrameworkError,
    framebuffer::{Attachment, AttachmentKind, GpuFrameBufferTrait},
    gpu_texture::GpuTextureKind,
};
use std::rc::Rc;
use std::sync::Arc;

/// Vulkan framebuffer implementation.
pub struct VkGpuFrameBuffer {
    /// The render pass.
    render_pass: vk::RenderPass,
    /// The framebuffer.
    framebuffer: vk::Framebuffer,
    /// Color attachments.
    color_attachments: Vec<Attachment>,
    /// Depth attachment.
    depth_attachment: Option<Attachment>,
    /// Framebuffer width.
    #[allow(dead_code)]
    width: u32,
    /// Framebuffer height.
    #[allow(dead_code)]
    height: u32,
    /// Device reference.
    device: Arc<VkDevice>,
    /// Dummy image for backbuffer placeholder (if this is a backbuffer).
    dummy_image: Option<vk::Image>,
    /// Dummy image view for backbuffer placeholder (if this is a backbuffer).
    dummy_image_view: Option<vk::ImageView>,
    /// Dummy image memory for backbuffer placeholder (if this is a backbuffer).
    dummy_image_memory: Option<vk::DeviceMemory>,
}

impl VkGpuFrameBuffer {
    /// Creates a new Vulkan framebuffer.
    pub fn new(
        device: Arc<VkDevice>,
        depth_attachment: Option<Attachment>,
        color_attachments: Vec<Attachment>,
    ) -> Result<Self, FrameworkError> {
        // Validate that we have at least one attachment
        if color_attachments.is_empty() && depth_attachment.is_none() {
            return Err(FrameworkError::Custom(
                "At least one attachment (color or depth) is required".to_string(),
            ));
        }

        // Get dimensions from the first available attachment
        let (width, height) = if !color_attachments.is_empty() {
            let first_attachment = &color_attachments[0];
            Self::get_attachment_dimensions(first_attachment)?
        } else if let Some(ref depth) = depth_attachment {
            Self::get_attachment_dimensions(depth)?
        } else {
            return Err(FrameworkError::Custom(
                "Cannot determine framebuffer dimensions without attachments".to_string(),
            ));
        };

        // Create render pass
        let render_pass = Self::create_render_pass(&device, &depth_attachment, &color_attachments)?;

        // Collect image views for framebuffer creation
        let mut image_views = Vec::new();

        // Add color attachment views
        for attachment in &color_attachments {
            let image_view = Self::get_attachment_image_view(attachment)?;
            image_views.push(image_view);
        }

        // Add depth attachment view if present
        if let Some(depth_attachment) = &depth_attachment {
            let image_view = Self::get_attachment_image_view(depth_attachment)?;
            image_views.push(image_view);
        }

        // Create framebuffer
        let framebuffer_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(&image_views)
            .width(width)
            .height(height)
            .layers(1);

        let framebuffer = unsafe {
            device
                .device
                .create_framebuffer(&framebuffer_info, None)
                .map_err(|e| {
                    FrameworkError::Custom(format!("Failed to create framebuffer: {:?}", e))
                })?
        };

        Ok(Self {
            render_pass,
            framebuffer,
            color_attachments,
            depth_attachment,
            width,
            height,
            device,
            dummy_image: None,
            dummy_image_view: None,
            dummy_image_memory: None,
        })
    }

    /// Creates a placeholder backbuffer framebuffer for Vulkan.
    /// This represents the swapchain framebuffer and is used as a placeholder
    /// that will be properly handled during rendering operations.
    pub fn backbuffer(device: Arc<VkDevice>) -> Self {
        // Create a minimal 1x1 dummy image for the backbuffer placeholder
        let image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::B8G8R8A8_UNORM)
            .extent(vk::Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let dummy_image = unsafe {
            device
                .device
                .create_image(&image_create_info, None)
                .expect("Failed to create backbuffer dummy image")
        };

        // Allocate memory for the dummy image
        let mem_requirements = unsafe { device.device.get_image_memory_requirements(dummy_image) };

        let memory_type_index = device
            .find_memory_type(
                mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .expect("Failed to find suitable memory type");

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);

        let dummy_image_memory = unsafe {
            device
                .device
                .allocate_memory(&alloc_info, None)
                .expect("Failed to allocate backbuffer dummy image memory")
        };

        unsafe {
            device
                .device
                .bind_image_memory(dummy_image, dummy_image_memory, 0)
                .expect("Failed to bind backbuffer dummy image memory");
        }

        // Create image view
        let view_create_info = vk::ImageViewCreateInfo::default()
            .image(dummy_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::B8G8R8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let dummy_image_view = unsafe {
            device
                .device
                .create_image_view(&view_create_info, None)
                .expect("Failed to create backbuffer dummy image view")
        };

        // Create render pass
        let attachment = vk::AttachmentDescription::default()
            .format(vk::Format::B8G8R8A8_UNORM)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_attachment_ref));

        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(std::slice::from_ref(&attachment))
            .subpasses(std::slice::from_ref(&subpass));

        let render_pass = unsafe {
            device
                .device
                .create_render_pass(&render_pass_info, None)
                .expect("Failed to create backbuffer render pass")
        };

        // Create framebuffer with the dummy image view
        let attachments = [dummy_image_view];
        let framebuffer_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(1)
            .height(1)
            .layers(1);

        let framebuffer = unsafe {
            device
                .device
                .create_framebuffer(&framebuffer_info, None)
                .expect("Failed to create backbuffer framebuffer")
        };

        Self {
            render_pass,
            framebuffer,
            color_attachments: vec![],
            depth_attachment: None,
            width: 1,
            height: 1,
            device,
            dummy_image: Some(dummy_image),
            dummy_image_view: Some(dummy_image_view),
            dummy_image_memory: Some(dummy_image_memory),
        }
    }

    /// Creates a render pass for the given attachments.
    fn create_render_pass(
        device: &VkDevice,
        depth_attachment: &Option<Attachment>,
        color_attachments: &[Attachment],
    ) -> Result<vk::RenderPass, FrameworkError> {
        let mut attachments = Vec::new();
        let mut color_attachment_refs = Vec::new();
        let mut depth_attachment_ref = None;

        // Add color attachments
        for (index, attachment) in color_attachments.iter().enumerate() {
            let format = Self::get_attachment_format(attachment)?;

            let attachment_desc = vk::AttachmentDescription::default()
                .format(format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

            attachments.push(attachment_desc);

            let attachment_ref = vk::AttachmentReference::default()
                .attachment(index as u32)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

            color_attachment_refs.push(attachment_ref);
        }

        // Add depth attachment if present
        if let Some(depth_attachment) = depth_attachment {
            let format = Self::get_attachment_format(depth_attachment)?;

            let attachment_desc = vk::AttachmentDescription::default()
                .format(format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

            attachments.push(attachment_desc);

            depth_attachment_ref = Some(
                vk::AttachmentReference::default()
                    .attachment(attachments.len() as u32 - 1)
                    .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
            );
        }

        // Create subpass
        let mut subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_refs);

        if let Some(depth_ref) = &depth_attachment_ref {
            subpass = subpass.depth_stencil_attachment(depth_ref);
        }

        let subpasses = [subpass];

        // Create render pass
        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses);

        unsafe {
            device
                .device
                .create_render_pass(&render_pass_info, None)
                .map_err(|e| {
                    FrameworkError::Custom(format!("Failed to create render pass: {:?}", e))
                })
        }
    }

    /// Gets the format of an attachment.
    fn get_attachment_format(attachment: &Attachment) -> Result<vk::Format, FrameworkError> {
        match &attachment.kind {
            AttachmentKind::Color => {
                if let Some(vk_texture) = attachment.texture.as_any().downcast_ref::<VkGpuTexture>()
                {
                    Ok(vk_texture.vk_format())
                } else {
                    Ok(vk::Format::R8G8B8A8_UNORM) // Default format
                }
            }
            AttachmentKind::Depth => {
                if let Some(vk_texture) = attachment.texture.as_any().downcast_ref::<VkGpuTexture>()
                {
                    Ok(vk_texture.vk_format())
                } else {
                    Ok(vk::Format::D32_SFLOAT) // Default depth format
                }
            }
            AttachmentKind::DepthStencil => {
                if let Some(vk_texture) = attachment.texture.as_any().downcast_ref::<VkGpuTexture>()
                {
                    Ok(vk_texture.vk_format())
                } else {
                    Ok(vk::Format::D24_UNORM_S8_UINT) // Default depth-stencil format
                }
            }
        }
    }

    /// Gets the image view of an attachment.
    fn get_attachment_image_view(attachment: &Attachment) -> Result<vk::ImageView, FrameworkError> {
        if let Some(vk_texture) = attachment.texture.as_any().downcast_ref::<VkGpuTexture>() {
            Ok(vk_texture.vk_image_view())
        } else {
            Err(FrameworkError::Custom("Invalid texture type".to_string()))
        }
    }

    /// Gets the dimensions of an attachment.
    fn get_attachment_dimensions(attachment: &Attachment) -> Result<(u32, u32), FrameworkError> {
        match attachment.texture.kind() {
            GpuTextureKind::Rectangle { width, height } => Ok((width as u32, height as u32)),
            GpuTextureKind::Cube { size } => Ok((size as u32, size as u32)),
            _ => Err(FrameworkError::Custom(
                "Unsupported texture kind for framebuffer".to_string(),
            )),
        }
    }

    /// Gets the Vulkan render pass.
    pub fn vk_render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }

    /// Gets the Vulkan framebuffer.
    pub fn vk_framebuffer(&self) -> vk::Framebuffer {
        self.framebuffer
    }
}

impl GpuFrameBufferTrait for VkGpuFrameBuffer {
    fn color_attachments(&self) -> &[Attachment] {
        &self.color_attachments
    }

    fn depth_attachment(&self) -> Option<&Attachment> {
        self.depth_attachment.as_ref()
    }

    fn set_cubemap_face(
        &self,
        _attachment_index: usize,
        _face: fyrox_graphics::gpu_texture::CubeMapFace,
        _level: usize,
    ) {
        // Implementation would update the attachment's cubemap face and level
        // For now, this is a no-op
    }

    fn blit_to(
        &self,
        _dest: &fyrox_graphics::framebuffer::GpuFrameBuffer,
        _src_x0: i32,
        _src_y0: i32,
        _src_x1: i32,
        _src_y1: i32,
        _dst_x0: i32,
        _dst_y0: i32,
        _dst_x1: i32,
        _dst_y1: i32,
        _copy_color: bool,
        _copy_depth: bool,
        _copy_stencil: bool,
    ) {
        // Implementation would perform framebuffer blit operation
        // For now, this is a no-op
    }

    fn clear(
        &self,
        _viewport: fyrox_graphics::core::math::Rect<i32>,
        _color: Option<fyrox_graphics::core::color::Color>,
        _depth: Option<f32>,
        _stencil: Option<i32>,
    ) {
        // Implementation would clear the framebuffer
        // For now, this is a no-op
    }

    fn read_pixels(
        &self,
        _read_target: fyrox_graphics::framebuffer::ReadTarget,
    ) -> Option<Vec<u8>> {
        // Implementation would read pixels from framebuffer
        // For now, return None
        None
    }

    fn draw(
        &self,
        _geometry: &fyrox_graphics::geometry_buffer::GpuGeometryBuffer,
        _viewport: fyrox_graphics::core::math::Rect<i32>,
        _program: &fyrox_graphics::gpu_program::GpuProgram,
        _params: &fyrox_graphics::DrawParameters,
        _resources: &[fyrox_graphics::framebuffer::ResourceBindGroup],
        _element_range: fyrox_graphics::ElementRange,
    ) -> Result<fyrox_graphics::framebuffer::DrawCallStatistics, FrameworkError> {
        // Implementation would perform draw call
        // For now, return placeholder statistics
        Ok(fyrox_graphics::framebuffer::DrawCallStatistics { triangles: 0 })
    }

    fn draw_instances(
        &self,
        _instance_count: usize,
        _geometry: &fyrox_graphics::geometry_buffer::GpuGeometryBuffer,
        _viewport: fyrox_graphics::core::math::Rect<i32>,
        _program: &fyrox_graphics::gpu_program::GpuProgram,
        _params: &fyrox_graphics::DrawParameters,
        _resources: &[fyrox_graphics::framebuffer::ResourceBindGroup],
        _element_range: fyrox_graphics::ElementRange,
    ) -> Result<fyrox_graphics::framebuffer::DrawCallStatistics, FrameworkError> {
        // Implementation would perform instanced draw call
        // For now, return placeholder statistics
        Ok(fyrox_graphics::framebuffer::DrawCallStatistics { triangles: 0 })
    }
}

impl Drop for VkGpuFrameBuffer {
    fn drop(&mut self) {
        unsafe {
            // Clean up dummy backbuffer resources if present
            if let Some(image_view) = self.dummy_image_view {
                self.device.device.destroy_image_view(image_view, None);
            }
            if let Some(image) = self.dummy_image {
                self.device.device.destroy_image(image, None);
            }
            if let Some(memory) = self.dummy_image_memory {
                self.device.device.free_memory(memory, None);
            }

            // Only destroy valid (non-null) handles
            if self.framebuffer != vk::Framebuffer::null() {
                self.device
                    .device
                    .destroy_framebuffer(self.framebuffer, None);
            }
            if self.render_pass != vk::RenderPass::null() {
                self.device
                    .device
                    .destroy_render_pass(self.render_pass, None);
            }
        }
    }
}

/// Creates a Vulkan GPU framebuffer.
pub fn create_framebuffer(
    device: Arc<VkDevice>,
    depth_attachment: Option<Attachment>,
    color_attachments: Vec<Attachment>,
) -> Result<Rc<dyn GpuFrameBufferTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuFrameBuffer::new(
        device,
        depth_attachment,
        color_attachments,
    )?))
}
