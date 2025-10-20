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
// DeviceV1_0 is no longer needed in ash 0.38
use ash::vk;
use fyrox_graphics::{
    error::FrameworkError,
    query::{GpuQueryTrait, QueryKind, QueryResult},
};
use std::rc::Rc;
use std::sync::Arc;

/// Vulkan query implementation.
pub struct VkGpuQuery {
    /// The query pool.
    query_pool: vk::QueryPool,
    /// Query kind.
    kind: QueryKind,
    /// Device reference.
    device: Arc<VkDevice>,
}

impl std::fmt::Debug for VkGpuQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VkGpuQuery")
            .field("query_pool", &self.query_pool)
            .field("kind", &self.kind)
            .finish()
    }
}

impl VkGpuQuery {
    /// Creates a new Vulkan query.
    pub fn new(device: Arc<VkDevice>) -> Result<Self, FrameworkError> {
        // Create an occlusion query pool with 1 query
        let mut query_pool_info = vk::QueryPoolCreateInfo::default();
        query_pool_info.query_type = vk::QueryType::OCCLUSION;
        query_pool_info.query_count = 1;

        let query_pool = unsafe {
            device
                .device
                .create_query_pool(&query_pool_info, None)
                .map_err(|e| {
                    FrameworkError::Custom(format!("Failed to create query pool: {:?}", e))
                })?
        };

        Ok(Self {
            query_pool,
            kind: QueryKind::SamplesPassed,
            device,
        })
    }

    /// Gets the Vulkan query pool.
    pub fn vk_query_pool(&self) -> vk::QueryPool {
        self.query_pool
    }
}

impl GpuQueryTrait for VkGpuQuery {
    fn begin(&self, _kind: QueryKind) {
        // Query begin is handled by command buffer recording in Vulkan
        // This is a placeholder implementation
    }

    fn end(&self) {
        // Query end is handled by command buffer recording in Vulkan
        // This is a placeholder implementation
    }

    fn is_started(&self) -> bool {
        // For now, assume query is never started
        false
    }

    fn try_get_result(&self) -> Option<QueryResult> {
        // For now, return None as no results are available
        None
    }
}

impl Drop for VkGpuQuery {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_query_pool(self.query_pool, None);
        }
    }
}

/// Creates a Vulkan GPU query.
pub fn create_query(device: Arc<VkDevice>) -> Result<Rc<dyn GpuQueryTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuQuery::new(device)?))
}
