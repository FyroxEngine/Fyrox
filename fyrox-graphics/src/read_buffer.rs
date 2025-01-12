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

use crate::{
    core::{math::Rect, Downcast},
    error::FrameworkError,
    framebuffer::FrameBuffer,
};
use bytemuck::Pod;

pub trait AsyncReadBuffer: Downcast {
    fn schedule_pixels_transfer(
        &mut self,
        framebuffer: &dyn FrameBuffer,
        color_buffer_index: u32,
        rect: Option<Rect<i32>>,
    ) -> Result<(), FrameworkError>;
    fn is_request_running(&self) -> bool;
    fn try_read(&mut self) -> Option<Vec<u8>>;
}

impl dyn AsyncReadBuffer {
    pub fn try_read_of_type<T>(&mut self) -> Option<Vec<T>>
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
