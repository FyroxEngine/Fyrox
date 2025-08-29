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

//! A [`GpuAsyncReadBufferTrait`] object handles the transfering pixel data from a frame buffer
//! into a color buffer in the background, and provides a method to poll the object and take the
//! data once it arrives.

use crate::{
    core::math::Rect, define_shared_wrapper, error::FrameworkError,
    framebuffer::GpuFrameBufferTrait,
};
use bytemuck::Pod;
use fyrox_core::define_as_any_trait;

define_as_any_trait!(GpuAsyncReadBufferAsAny => GpuAsyncReadBufferTrait);

/// Trait for objects that represent a request to transfer pixel data from some frame buffer
/// into a color buffer.
pub trait GpuAsyncReadBufferTrait: GpuAsyncReadBufferAsAny {
    /// Begin the pixel data transfer by specifying a particular attachment of a particular framebuffer to read from.
    /// * `framebuffer`: The source of the data.
    /// * `color_buffer_index`: The index of the color attachment of the framebuffer.
    /// * `rect`: The portion of the framebuffer to read.
    ///
    /// After this is called, [`is_request_running`](GpuAsyncReadBufferTrait::is_request_running) will
    /// return true.
    fn schedule_pixels_transfer(
        &self,
        framebuffer: &dyn GpuFrameBufferTrait,
        color_buffer_index: u32,
        rect: Option<Rect<i32>>,
    ) -> Result<(), FrameworkError>;
    /// Return true if a request has been made and the data has not yet been
    /// retrieved with [`try_read`](GpuAsyncReadBufferTrait::try_read).
    /// Returning false does *not* indicate that the data is read; this method
    /// will continue to return true long after the data is read, until `try_read` is
    /// called to actually read the data.
    fn is_request_running(&self) -> bool;
    /// Check the state of the data and return the data if it is read, or else
    /// return None. After the data is returned, [`is_request_running`](GpuAsyncReadBufferTrait::is_request_running)
    /// will return false.
    fn try_read(&self) -> Option<Vec<u8>>;
}

impl dyn GpuAsyncReadBufferTrait {
    /// Check the state of the data and return the data if it is read, or else
    /// return None. The data is converted into the given type before being returned.
    /// After the data is returned, [`is_request_running`](GpuAsyncReadBufferTrait::is_request_running)
    /// will return false.
    pub fn try_read_of_type<T>(&self) -> Option<Vec<T>>
    where
        T: Pod,
    {
        let mut bytes = self.try_read()?;
        let typed = unsafe {
            Some(Vec::from_raw_parts(
                bytes.as_mut_ptr() as *mut T,
                bytes.len() / size_of::<T>(),
                bytes.capacity() / size_of::<T>(),
            ))
        };
        std::mem::forget(bytes);
        typed
    }
}

define_shared_wrapper!(GpuAsyncReadBuffer<dyn GpuAsyncReadBufferTrait>);
